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
    if has_genuine_tie {
        return LineOrder::Ambiguous;
    }

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

    if std::env::var_os("PDFRTL_TRACE_ORDER").is_some() {
        record_pattern(if keeps && inverts {
            "both_fit"
        } else if keeps {
            "stored_only"
        } else if inverts {
            "reversed_only"
        } else {
            "neither"
        });
    }

    match (keeps, inverts) {
        (true, false) => LineOrder::Keep,
        (false, true) => LineOrder::Invert(inverted),
        (true, true) => LineOrder::Ambiguous,
        (false, false) => LineOrder::Unexplained,
    }
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
        Dir::Ltr | Dir::Neutral => Level::from(even),
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
