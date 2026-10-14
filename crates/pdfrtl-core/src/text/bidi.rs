//! UAX #9 Bidirectional order settlement, hypothesis evaluation, and tracing.

use super::cluster::{invert_units, is_combining, is_rtl};
use super::state::Unit;
use std::cell::RefCell;
use unicode_bidi::{bidi_class, BidiClass, BidiInfo, Level};

thread_local! {
    static OUTCOME_COUNTS: RefCell<[usize; 4]> = const { RefCell::new([0; 4]) };
    static ORDER_TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

pub fn proxy_char(text: &str) -> char {
    let first = match text.chars().next() {
        Some(c) => c,
        None => return '!',
    };
    if text.chars().all(|c| bidi_class(c) == BidiClass::WS) {
        return ' ';
    }
    match bidi_class(first) {
        BidiClass::R | BidiClass::AL => '\u{05D0}',
        BidiClass::L => 'A',
        BidiClass::AN => '\u{0665}',
        BidiClass::EN => '5',
        BidiClass::WS => ' ',
        BidiClass::CS => '.',
        BidiClass::ES => '-',
        BidiClass::ET => '%',
        BidiClass::BN | BidiClass::S | BidiClass::B | BidiClass::ON | BidiClass::NSM => '!',
        _ => '.',
    }
}

/// Rung 3 of the order ladder — the UAX #9 comparison.
#[derive(Debug, PartialEq, Eq)]
pub enum LineOrder {
    Keep,
    Invert(Vec<usize>),
    Ambiguous,
    Unexplained,
}

pub fn record_pattern(pattern: &str) {
    let slot = match pattern {
        "both_fit" => 0,
        "stored_only" => 1,
        "reversed_only" => 2,
        _ => 3,
    };
    OUTCOME_COUNTS.with(|counts| {
        counts.borrow_mut()[slot] += 1;
    });
}

/// [both_fit, stored_only, reversed_only, neither] since the last reset on this thread.
pub fn take_outcome_counts() -> [usize; 4] {
    OUTCOME_COUNTS.with(|counts| {
        let mut b = counts.borrow_mut();
        let snapshot = *b;
        *b = [0; 4];
        snapshot
    })
}

pub fn trace_line(
    units: usize,
    ties: usize,
    distinct: usize,
    min_x: f64,
    max_x: f64,
    geometry: bool,
) {
    ORDER_TRACE.with(|trace| {
        trace.borrow_mut().push(format!(
            "line units={units} tied_pairs={ties} distinct_x={distinct}          span={:.3}..{:.3} geometry={geometry}",
            min_x, max_x
        ));
    });
}

/// Read and clear the recorded order trace on this thread.
pub fn take_order_trace() -> Vec<String> {
    ORDER_TRACE.with(|trace| std::mem::take(&mut *trace.borrow_mut()))
}

/// Compare a line's stored sequence with its painted sequence under UAX #9.
pub fn settle_line_by_bidi(units: &[Unit]) -> LineOrder {
    let count = units.len();
    if count < 2 || units.iter().any(|unit| !unit.x_ok) {
        return LineOrder::Unexplained;
    }

    // The painted (visual) order: left to right by position, ties in stream order.
    let mut painted: Vec<usize> = (0..count).collect();
    painted.sort_by(|&left, &right| {
        units[left]
            .paint_x
            .total_cmp(&units[right].paint_x)
            .then(left.cmp(&right))
    });

    let has_genuine_tie = painted.windows(2).any(|pair| {
        let left = &units[pair[0]];
        let right = &units[pair[1]];
        if left.paint_x == right.paint_x {
            let left_combining = left.text.chars().all(is_combining);
            let right_combining = right.text.chars().all(is_combining);
            !(left_combining || right_combining)
        } else {
            false
        }
    });
    let identity: Vec<usize> = (0..count).collect();
    let inverted = invert_units(units);
    let stored_is_painted = painted == identity;
    let is_rtl_progression = count >= 2
        && units.windows(2).all(|w| w[0].paint_x >= w[1].paint_x)
        && units[0].paint_x > units[count - 1].paint_x
        && units.iter().any(|u| u.text.chars().any(is_rtl));

    let keeps = predicts_painted(units, &identity, &painted) || is_rtl_progression;
    let inverts =
        stored_is_painted && inverted != identity && predicts_painted(units, &inverted, &painted);

    let initial = if has_genuine_tie {
        LineOrder::Ambiguous
    } else {
        match (keeps, inverts) {
            (true, false) => LineOrder::Keep,
            (false, true) => LineOrder::Invert(inverted),
            (true, true) => LineOrder::Ambiguous,
            (false, false) => LineOrder::Unexplained,
        }
    };

    let outcome = match initial {
        LineOrder::Keep | LineOrder::Invert(_) => initial,
        LineOrder::Ambiguous | LineOrder::Unexplained => {
            if count <= crate::text::oracle::MAX_ENUMERABLE_UNITS {
                let resolved = solve_short_line_by_geometry(units);
                match resolved {
                    LineOrder::Keep | LineOrder::Invert(_) => resolved,
                    LineOrder::Ambiguous => LineOrder::Ambiguous,
                    LineOrder::Unexplained => initial,
                }
            } else {
                initial
            }
        }
    };

    if std::env::var_os("PDFRTL_TRACE_ORDER").is_some() {
        record_pattern(match &outcome {
            LineOrder::Keep => "stored_only",
            LineOrder::Invert(_) => "reversed_only",
            LineOrder::Ambiguous => "both_fit",
            LineOrder::Unexplained => "neither",
        });
    }

    outcome
}

pub fn predicts_painted(units: &[Unit], logical: &[usize], painted: &[usize]) -> bool {
    painted_map_for(units, logical) == Some(painted.to_vec())
        || predicts_painted_runs(units, logical, painted)
}

pub fn predicts_painted_runs(units: &[Unit], logical: &[usize], painted: &[usize]) -> bool {
    let count = logical.len();
    if count != painted.len() || count < 2 {
        return false;
    }
    let full_text: String = logical.iter().map(|&i| units[i].text.as_str()).collect();
    let base_rtl = auto_base_rtl(&full_text);

    let levels: Vec<Level> = logical
        .iter()
        .map(|&i| run_level(&units[i].text, base_rtl))
        .collect();

    let visual = BidiInfo::reorder_visual(&levels);
    if visual.len() != count {
        return false;
    }
    let predicted_painted: Vec<usize> = visual
        .into_iter()
        .map(|logical_pos| logical[logical_pos])
        .collect();
    predicted_painted == painted
}

pub fn painted_map_for(units: &[Unit], logical: &[usize]) -> Option<Vec<usize>> {
    let mut text = String::new();
    let mut bounds: Vec<(usize, usize)> = Vec::new();
    for &index in logical {
        let unit_text = &units[index].text;
        let start = text.chars().count();
        text.push_str(unit_text);
        let end = text.chars().count();
        if end == start {
            return None;
        }
        bounds.push((start, end));
    }

    let info = BidiInfo::new(&text, None);
    let para = info.paragraphs.first()?;
    let total_chars = text.chars().count();
    let total_bytes = text.len();
    let levels = info.reordered_levels_per_char(para, 0..total_bytes);
    if levels.len() != total_chars {
        return None;
    }
    let visual = BidiInfo::reorder_visual(&levels);
    if visual.len() != total_chars {
        return None;
    }

    let mut offset_to_unit = vec![0usize; total_chars];
    for (ordinal, &(start, end)) in bounds.iter().enumerate() {
        for slot in offset_to_unit.iter_mut().take(end).skip(start) {
            *slot = ordinal;
        }
    }

    let mut first_at: Vec<Option<usize>> = vec![None; logical.len()];
    for (visual_position, character) in visual.iter().copied().enumerate() {
        let ordinal = offset_to_unit[character];
        if first_at[ordinal].is_none() {
            first_at[ordinal] = Some(visual_position);
        }
    }
    let mut painted_units: Vec<(usize, usize)> = Vec::with_capacity(logical.len());
    for (ordinal, position) in first_at.iter().copied().enumerate() {
        let position = position?;
        painted_units.push((position, ordinal));
    }
    painted_units.sort_unstable();
    Some(
        painted_units
            .into_iter()
            .map(|(_, ordinal)| logical[ordinal])
            .collect(),
    )
}

pub fn split_runs(units: &[Unit]) -> (Vec<Vec<usize>>, Vec<bool>) {
    let mut runs: Vec<Vec<usize>> = Vec::new();
    let mut reversed: Vec<bool> = Vec::new();
    let mut current: Vec<usize> = Vec::new();
    let mut current_reversed = false;
    let mut current_epoch = 0u64;
    for (index, unit) in units.iter().enumerate() {
        if index == 0 || unit.reversed != current_reversed || unit.epoch != current_epoch {
            if index > 0 {
                runs.push(std::mem::take(&mut current));
                reversed.push(current_reversed);
            }
            current_reversed = unit.reversed;
            current_epoch = unit.epoch;
        }
        current.push(index);
    }
    runs.push(current);
    reversed.push(current_reversed);
    (runs, reversed)
}

pub fn predicts_observed(texts: &[&str], logical: &[usize], base_rtl: bool) -> bool {
    let proxy: String = logical.iter().map(|&i| proxy_char(texts[i])).collect();
    let base = if base_rtl { Level::rtl() } else { Level::ltr() };
    let info = BidiInfo::new(&proxy, Some(base));
    let para = match info.paragraphs.first() {
        Some(para) => para,
        None => return false,
    };
    let levels = info.reordered_levels_per_char(para, 0..proxy.len());
    if levels.len() != logical.len() {
        return false;
    }
    let map = BidiInfo::reorder_visual(&levels);
    map.len() == logical.len()
        && map
            .iter()
            .enumerate()
            .all(|(visual, &logical_pos)| logical.get(logical_pos) == Some(&visual))
}

pub fn auto_base_rtl(text: &str) -> bool {
    let info = BidiInfo::new(text, None);
    info.paragraphs
        .first()
        .map(|para| para.level.is_rtl())
        .unwrap_or(false)
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Rtl,
    Ltr,
    Neutral,
}

pub fn first_strong(text: &str) -> Dir {
    use unicode_bidi::{bidi_class, BidiClass};
    for c in text.chars() {
        match bidi_class(c) {
            BidiClass::R | BidiClass::AL => return Dir::Rtl,
            BidiClass::L => return Dir::Ltr,
            _ => {}
        }
    }
    Dir::Neutral
}

pub fn run_level(text: &str, base_rtl: bool) -> Level {
    let even = if base_rtl { 2 } else { 0 };
    match first_strong(text) {
        Dir::Rtl => Level::from(1),
        Dir::Ltr => Level::from(even),
        Dir::Neutral => {
            if text.chars().any(crate::text::cluster::is_bidi_number) {
                Level::from(even)
            } else if base_rtl {
                Level::from(1)
            } else {
                Level::from(0)
            }
        }
    }
}

pub fn predicted_visual_runs(units: &[Unit], logical: &[usize]) -> Option<Vec<usize>> {
    let count = logical.len();
    if count < 2 {
        return None;
    }
    let full_text: String = logical.iter().map(|&i| units[i].text.as_str()).collect();
    let base_rtl = auto_base_rtl(&full_text);

    let levels: Vec<Level> = logical
        .iter()
        .map(|&i| run_level(&units[i].text, base_rtl))
        .collect();

    let visual = BidiInfo::reorder_visual(&levels);
    if visual.len() != count {
        return None;
    }
    Some(visual.into_iter().map(|logical_pos| logical[logical_pos]).collect())
}

fn permute_helper(items: &mut Vec<usize>, at: usize, visit: &mut impl FnMut(&[usize])) {
    if at == items.len() {
        visit(items);
        return;
    }
    for i in at..items.len() {
        items.swap(at, i);
        permute_helper(items, at + 1, visit);
        items.swap(at, i);
    }
}

pub fn predicted_visual_order(units: &[Unit], candidate: &[usize]) -> Option<Vec<usize>> {
    if let Some(v) = painted_map_for(units, candidate) {
        return Some(v);
    }
    predicted_visual_runs(units, candidate)
}

pub fn permutation_matches_geometry(units: &[Unit], candidate: &[usize], tolerance: f64) -> bool {
    let visual = match predicted_visual_order(units, candidate) {
        Some(v) => v,
        None => return false,
    };

    if visual.len() != units.len() {
        return false;
    }

    // Geometry check: visual order must be left-to-right on the page within tolerance.
    for window in visual.windows(2) {
        let (left_idx, right_idx) = (window[0], window[1]);
        let u_l = &units[left_idx];
        let u_r = &units[right_idx];

        // Pen positions: u_l must not be to the right of u_r
        if u_l.paint_x > u_r.paint_x + tolerance {
            return false;
        }

        // Bounding boxes: left edge of u_l must not be to the right of left edge of u_r
        let bbox_l = u_l.bbox();
        let bbox_r = u_r.bbox();
        if bbox_l[0] > bbox_r[0] + tolerance {
            return false;
        }

        // Horizontal centers: center of u_l must not be to the right of center of u_r
        let center_l = (bbox_l[0] + bbox_l[2]) * 0.5;
        let center_r = (bbox_r[0] + bbox_r[2]) * 0.5;
        if center_l > center_r + tolerance {
            return false;
        }
    }
    true
}

pub fn solve_short_line_by_geometry(units: &[Unit]) -> LineOrder {
    let count = units.len();
    if count < 2
        || count > crate::text::oracle::MAX_ENUMERABLE_UNITS
        || units.iter().any(|u| !u.x_ok)
    {
        return LineOrder::Unexplained;
    }

    let has_strong_rtl = units.iter().any(|u| {
        u.text.chars().any(|c| {
            matches!(
                unicode_bidi::bidi_class(c),
                unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL
            )
        })
    });
    let has_strong_ltr = units.iter().any(|u| {
        u.text
            .chars()
            .any(|c| matches!(unicode_bidi::bidi_class(c), unicode_bidi::BidiClass::L))
    });

    if !has_strong_rtl && !has_strong_ltr {
        return LineOrder::Ambiguous;
    }

    // Pure strong RTL lines without numbers or neutrals can only be stored or inverted under UAX #9
    let has_non_strong_rtl = units.iter().any(|u| {
        u.text.chars().any(|c| {
            !matches!(
                unicode_bidi::bidi_class(c),
                unicode_bidi::BidiClass::R | unicode_bidi::BidiClass::AL
            )
        })
    });

    let font_size = units.iter().map(|u| u.font_size).fold(0.0f64, f64::max);
    let relaxed_tol = if font_size > 0.0 {
        (font_size * 0.15).max(1.0)
    } else {
        1.5
    };

    let mut current_perm: Vec<usize> = (0..count).collect();
    let mut matching_orders: Vec<Vec<usize>> = Vec::new();

    for tol in [1e-6, relaxed_tol] {
        matching_orders.clear();
        permute_helper(&mut current_perm, 0, &mut |candidate| {
            if !has_non_strong_rtl {
                let identity: Vec<usize> = (0..count).collect();
                let mut rev = identity.clone();
                rev.reverse();
                if candidate != identity.as_slice() && candidate != rev.as_slice() {
                    return;
                }
            }
            if permutation_matches_geometry(units, candidate, tol) {
                matching_orders.push(candidate.to_vec());
            }
        });

        if !matching_orders.is_empty() {
            break;
        }
    }

    match matching_orders.len() {
        1 => {
            let proven = matching_orders.pop().unwrap();
            let identity: Vec<usize> = (0..count).collect();
            if proven == identity {
                LineOrder::Keep
            } else {
                LineOrder::Invert(proven)
            }
        }
        n if n >= 2 => LineOrder::Ambiguous,
        _ => LineOrder::Unexplained,
    }
}

pub fn tie_span(units: &[Unit]) -> (usize, usize, f64, f64) {
    let mut sorted: Vec<f64> = units.iter().map(|unit| unit.paint_x).collect();
    sorted.sort_by(|left, right| left.total_cmp(right));
    let mut ties = 0;
    for pair in sorted.windows(2) {
        if pair[0] == pair[1] {
            ties += 1;
        }
    }
    let distinct = sorted.iter().fold(0usize, |count, value| {
        if count == 0 || *value != sorted[count - 1] {
            count + 1
        } else {
            count
        }
    });
    let min = sorted.first().copied().unwrap_or(0.0);
    let max = sorted.last().copied().unwrap_or(0.0);
    (ties, distinct, min, max)
}

pub fn has_geometry(units: &[Unit]) -> bool {
    units
        .iter()
        .all(|unit| unit.x_ok && unit.paint_x.is_finite())
}
