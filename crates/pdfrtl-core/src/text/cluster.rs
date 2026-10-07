//! Script classification and cluster inversion for visual/logical RTL runs.

use super::state::Unit;

pub fn is_rtl(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x0590..=0x05FF
            | 0x0600..=0x06FF
            | 0x0750..=0x077F
            | 0x08A0..=0x08FF
            | 0xFB50..=0xFDFF
            | 0xFE70..=0xFEFF
    )
}

/// Combining marks and bidi/format controls that must stay glued to their base
/// when a line is inverted — reversing a base away from its marks corrupts the
/// text just as surely as reversing the line.
pub fn is_combining(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x0300..=0x036F
            | 0x0483..=0x0489
            | 0x0591..=0x05C7
            | 0x0610..=0x061A
            | 0x064B..=0x065F
            | 0x0670
            | 0x06D6..=0x06DC
            | 0x06DF..=0x06E4
            | 0x06E7..=0x06E8
            | 0x06EA..=0x06ED
            | 0x0711
            | 0x0730..=0x074A
            | 0x07A6..=0x07B0
            | 0x0900..=0x0903
            | 0x093A..=0x094F
            | 0x0951..=0x0957
            | 0x0E31
            | 0x0E34..=0x0E3A
            | 0x0E47..=0x0E4E
            | 0x200C..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0xFE00..=0xFE0F
            | 0xFE20..=0xFE2F
    )
}

/// A character that can *start* an embedded left-to-right run: Latin/Greek
/// letters and digits (never a right-to-left one).
pub fn ltr_start(ch: char) -> bool {
    ch.is_alphanumeric() && !is_rtl(ch) && (ch as u32) < 0x0900
}

/// A unit whose text begins with a combining/format character: it belongs to the
/// preceding unit's cluster and has to move with it when the line is inverted.
pub fn starts_combining(text: &str) -> bool {
    text.chars().next().is_some_and(is_combining)
}

/// Characters allowed *inside* an LTR run: the ASCII punctuation that occurs in
/// Latin tokens (URLs, decimals, dates).
pub fn ltr_run_char(ch: char) -> bool {
    matches!(
        ch,
        '.' | ',' | ':' | '/' | '-' | '_' | '@' | '#' | '%' | '?' | '!' | '&' | '=' | '+' | '\''
    ) || ltr_start(ch)
}

/// A line worth settling: at least two right-to-left letters (a stray mark is not
/// evidence of an RTL run).
pub fn has_rtl_run(text: &str) -> bool {
    text.chars().filter(|ch| is_rtl(*ch)).count() >= 2
}

/// Visual -> logical for one line of base-direction RTL: reverse the line in
/// clusters (base + combining marks), then put embedded left-to-right runs back
/// into reading order. This is the inverse of the UAX #9 L2 reversal the renderer
/// performed when it drew the line left to right.
pub fn visual_to_logical(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    if !chars.iter().any(|ch| is_rtl(*ch)) {
        return line.to_string();
    }

    // Cluster: a base character with the marks/format controls glued to it.
    let mut clusters: Vec<Vec<char>> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let mut j = i + 1;
        while j < chars.len() && is_combining(chars[j]) {
            j += 1;
        }
        clusters.push(chars[i..j].to_vec());
        i = j;
    }
    clusters.reverse();

    let starts_ltr = |cluster: &[char]| cluster.first().is_some_and(|ch| ltr_start(*ch));
    let is_space = |cluster: &[char]| cluster.first() == Some(&' ');
    let ltr_after_space = |clusters: &[Vec<char>], index: usize| {
        let mut next = index;
        while next < clusters.len() && is_space(&clusters[next]) {
            next += 1;
        }
        next < clusters.len() && starts_ltr(&clusters[next])
    };

    let mut k = 0;
    while k < clusters.len() {
        if starts_ltr(&clusters[k]) {
            let mut end = k + 1;
            while end < clusters.len() {
                let head = clusters[end].first().copied();
                if head.is_some_and(ltr_run_char)
                    || (is_space(&clusters[end]) && ltr_after_space(&clusters, end))
                {
                    end += 1;
                } else {
                    break;
                }
            }
            clusters[k..end].reverse();
            k = end;
        } else {
            k += 1;
        }
    }

    clusters.into_iter().flatten().collect()
}

/// Visual -> logical for one line of base-direction RTL, at UNIT granularity:
/// reverse the unit order in clusters (a base unit plus the combining/format units
/// glued to it), then put embedded left-to-right unit runs back into reading order.
pub fn invert_units(units: &[Unit]) -> Vec<usize> {
    if !units.iter().any(|unit| unit.text.chars().any(is_rtl)) {
        return (0..units.len()).collect();
    }

    let mut clusters: Vec<Vec<usize>> = Vec::new();
    let mut index = 0;
    while index < units.len() {
        let mut end = index + 1;
        while end < units.len() && starts_combining(&units[end].text) {
            end += 1;
        }
        clusters.push((index..end).collect());
        index = end;
    }
    clusters.reverse();

    let first_char = |cluster: &[usize]| {
        cluster
            .first()
            .and_then(|&unit| units[unit].text.chars().next())
    };
    let starts_ltr = |cluster: &[usize]| first_char(cluster).is_some_and(ltr_start);
    let is_space = |cluster: &[usize]| first_char(cluster) == Some(' ');
    let ltr_after_space = |clusters: &[Vec<usize>], position: usize| {
        let mut next = position;
        while next < clusters.len() && is_space(&clusters[next]) {
            next += 1;
        }
        next < clusters.len() && starts_ltr(&clusters[next])
    };

    let mut at = 0;
    while at < clusters.len() {
        if starts_ltr(&clusters[at]) {
            let mut end = at + 1;
            while end < clusters.len() {
                let head = first_char(&clusters[end]);
                if head.is_some_and(ltr_run_char)
                    || (is_space(&clusters[end]) && ltr_after_space(&clusters, end))
                {
                    end += 1;
                } else {
                    break;
                }
            }
            clusters[at..end].reverse();
            at = end;
        } else {
            at += 1;
        }
    }

    clusters.into_iter().flatten().collect()
}
