//! An oracle for the order ladder: how many logical orders does a line's geometry actually admit?
//!
//! This is the measurement `docs/problems/0018` requires before any new candidate generator is
//! built, and it exists because a previous attempt guessed at the blocker and shipped decoration.
//!
//! # The question
//!
//! Given the painted positions of a line's units, **how many** orders of those units are
//! indistinguishable from geometry? The ladder's job is to recover the stored order; its honesty
//! depends on knowing when geometry cannot say which reading that was.
//!
//! * **1** → geometry admits exactly one order. Deciding it is sound.
//! * **2 or more** → the file does not say which reading was stored, and any tool that picks one
//!   is guessing. That is `docs/problems/0015` made executable.
//!
//! # Two implementations, on purpose
//!
//! [`admissible_orders_closed_form`] computes the count from the sizes of the groups of units that
//! share a painted position. [`admissible_orders_by_enumeration`] computes the same number by
//! generating every permutation and testing each one. They are independent, and
//! `closed_form_agrees_with_enumeration` asserts they agree.
//!
//! The second exists because the first is the kind of closed form that can be wrong in a way no
//! test of its own output would catch. One implementation would make "the oracle agrees with the
//! code" unfalsifiable — the same defect as verifying a gate with the code it gates.
//!
//! # Bounded, and the bound is reported rather than hidden
//!
//! Enumeration is factorial: 8 units is 40,320 permutations, 9 is 362,880.
//! [`MAX_ENUMERABLE_UNITS`] caps it, and [`ambiguity_of`] returns [`Ambiguity::NotComputed`] above
//! that. Returning an unexamined number would be precisely the defect this project exists to
//! prevent: a figure that looks measured and is not.
//!
//! # The script classes ARE an input, and the first version was wrong without them
//!
//! The first implementation took positions only and answered "1" for two units at distinct `x`.
//! The enumeration answered "2", and the enumeration was right. Geometry says which unit is left
//! and which is right; it does not say which way the producer was writing, because an RTL
//! paragraph is painted right-to-left and its logical order therefore runs opposite to `x`. Two
//! orders, one painting — the same non-invertibility as `docs/problems/0015`, one level up.
//!
//! The base direction comes from the TEXT (UAX #9 rules P2/P3: the first strong character), so
//! both implementations now take the classes and re-order each candidate under UAX #9 rather than
//! sorting purely by `x`.
//!
//! So the question measured here is precisely: **how many orders does geometry AND the script
//! model admit?** Not "what does geometry say" — that question has the unhelpful answer "two".
//!
//! # Privacy
//!
//! Classes and positions in, a count out. No text, no lines, no filenames.

use std::collections::HashMap;

/// The largest line both implementations will enumerate. 8 units = 40,320 permutations.
pub const MAX_ENUMERABLE_UNITS: usize = 8;

/// Returned by the raw `*_orders` functions when the line is too long to examine.
///
/// **Not** a count. Prefer [`ambiguity_of`], whose [`Ambiguity`] type cannot be mistaken for one.
pub const AMBIGUITY_NOT_COMPUTED: usize = 0;

/// Bidi class of a unit, carried for callers' reasoning. Not an input to either count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BidiClassOf {
    /// Strong right-to-left.
    R,
    /// Strong left-to-right.
    L,
    /// European number.
    EN,
    /// Whitespace.
    WS,
    /// Neutral.
    ON,
}

/// Positions equal within this tolerance count as the same painted position.
const POSITION_EPSILON: f64 = 1e-6;

fn same_position(a: f64, b: f64) -> bool {
    (a - b).abs() < POSITION_EPSILON
}

fn factorial(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        (1..=n).product()
    }
}

/// Group sizes of units sharing a painted position.
///
/// Positions are compared with [`POSITION_EPSILON`]: coordinates differing by 1e-9 describe a
/// distinction no renderer can show, and treating it as real would understate ambiguity.
pub fn groups_at_shared_positions(xs: &[f64]) -> Vec<usize> {
    let mut sorted: Vec<f64> = xs.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mut groups: Vec<usize> = Vec::new();
    let mut run = 0usize;
    for (index, x) in sorted.iter().enumerate() {
        if index > 0 && same_position(*x, sorted[index - 1]) {
            run += 1;
        } else {
            if run > 0 {
                groups.push(run);
            }
            run = 1;
        }
    }
    if run > 0 {
        groups.push(run);
    }
    groups
}

/// Count admissible orders from the sizes of the groups of units sharing a painted position,
/// multiplied by the number of BASE DIRECTIONS the script model permits.
///
/// Two factors, and both are load-bearing:
///
/// * a group of `k` units at one `x` contributes `k!`, because they are interchangeable as far
///   as the painting is concerned;
/// * the base direction contributes a factor of **2** when the line has no strong character to fix
///   it (see [`base_direction_is_fixed`]) AND the tie groups do not already admit every order. A
///   line that is all neutrals or all digits has no first-strong character, so UAX #9 P3 defaults
///   it to the paragraph level, which the ladder has no measurement of. The factor is a UNION of
///   two walks, not a sum: when one tie group spans every unit both walks admit the same orders,
///   and multiplying would double-count them.
///
/// Returns [`AMBIGUITY_NOT_COMPUTED`] above [`MAX_ENUMERABLE_UNITS`].
pub fn admissible_orders_closed_form(xs: &[f64], classes: &[BidiClassOf]) -> usize {
    if xs.is_empty() {
        return 1; // the empty order is the only one
    }
    if xs.len() > MAX_ENUMERABLE_UNITS {
        return AMBIGUITY_NOT_COMPUTED;
    }
    let groups = groups_at_shared_positions(xs);
    let ties = groups.iter().fold(1usize, |acc, &group| {
        acc.checked_mul(factorial(group)).unwrap_or(usize::MAX)
    });
    if base_direction_is_fixed(classes) {
        return ties;
    }
    // The direction factor only adds orders when the direction is genuinely a CHOICE that
    // excludes some. When every unit shares one position, `walk_matches` already holds for every
    // order under BOTH directions, so the union is the same set and multiplying by 2 would
    // double-count it. Hand-checked: 4 units at one x, no strong character -> 4! = 24, not 48.
    let every_order_walks = groups.len() == 1 && groups[0] == xs.len();
    if every_order_walks {
        ties
    } else {
        ties.saturating_mul(2)
    }
}

/// Whether UAX #9 fixes the paragraph's base direction from the line's own characters.
///
/// P2/P3: the first strong character sets the level; a line with no strong character (all
/// whitespace, neutrals or digits) inherits the paragraph level, which a page's painting does not
/// reveal. Returns `true` when at least one unit is strongly RTL or strongly LTR.
pub fn base_direction_is_fixed(classes: &[BidiClassOf]) -> bool {
    classes
        .iter()
        .any(|c| matches!(c, BidiClassOf::R | BidiClassOf::L))
}

/// Count admissible orders by generating every permutation and testing each one.
///
/// **The admissibility test, stated so it cannot be vacuous.** Painting a candidate and comparing
/// it to the stored painting must be sensitive to the candidate's OWN sequence, or every
/// permutation trivially matches and the answer is always `n!`. Sorting units by `x` and reversing
/// by direction discards the candidate entirely — it depends only on the multiset of positions —
/// which is exactly the bug this function's first version had, and it reported 2 admissible orders
/// for two units at *distinct* positions.
///
/// So a candidate is admissible when the sequence a renderer would traverse for it — the units in
/// the candidate's own order, walked right-to-left for an RTL paragraph — lands each unit where the
/// measurement put it. Concretely: walking the candidate from its first unit, each next unit must
/// be the next one measured *in the direction the paragraph runs*. A candidate is admissible when
/// that walk visits the painted positions in the measured order, which is a genuine constraint on
/// the sequence.
///
/// Returns [`AMBIGUITY_NOT_COMPUTED`] above [`MAX_ENUMERABLE_UNITS`].
pub fn admissible_orders_by_enumeration(xs: &[f64], classes: &[BidiClassOf]) -> usize {
    let n = xs.len();
    if n == 0 {
        return 1;
    }
    if n > MAX_ENUMERABLE_UNITS {
        return AMBIGUITY_NOT_COMPUTED;
    }
    // When the base direction is NOT fixed (UAX #9 P3 inherits an unmeasured paragraph level),
    // BOTH walks are possible readings and both must be counted. Tried by hand on two units at
    // [10, 20] with no strong character: the RTL walk admits only [1,0] and the LTR walk only
    // [0,1] — one order each, two in total. An enumeration that fixed one direction reported 1
    // and disagreed with the closed form, which was the direction assumption being wrong rather
    // than the arithmetic.
    let rtl = base_direction_is_rtl(classes);
    let directions: &[bool] = if base_direction_is_fixed(classes) {
        &[rtl]
    } else {
        &[true, false]
    };

    let mut order: Vec<usize> = (0..n).collect();
    let mut count = 0usize;
    permute(&mut order, 0, &mut |candidate: &[usize]| {
        if directions
            .iter()
            .any(|&dir| walk_matches(candidate, xs, dir))
        {
            count += 1;
        }
    });
    count
}

/// Walk `candidate` the way a renderer traverses an RTL paragraph: the first unit is painted
/// rightmost, so each subsequent unit must sit at the next measured position *below* the previous
/// one. Units sharing a position may follow each other in any order — that is the tie a renderer
/// cannot show.
///
/// `true` means every unit was accounted for.
fn walk_matches(candidate: &[usize], xs: &[f64], rtl: bool) -> bool {
    for window in candidate.windows(2) {
        let (first, second) = (window[0], window[1]);
        let next_measured = if rtl {
            // Walking right to left: the next position must not be to the RIGHT.
            xs[second] <= xs[first] + POSITION_EPSILON
        } else {
            xs[second] >= xs[first] - POSITION_EPSILON
        };
        if !next_measured {
            return false;
        }
    }
    true
}

/// The base direction a line's own characters fix. UAX #9 P2/P3: the first strong character sets
/// it; a line with none inherits an unmeasured paragraph level, which this function reports as
/// *not* RTL by convention — the ambiguity is accounted for by the factor of 2 in the closed form
/// and by [`base_direction_is_fixed`] telling the caller the direction is not evidence.
pub fn base_direction_is_rtl(classes: &[BidiClassOf]) -> bool {
    match classes
        .iter()
        .find(|c| matches!(**c, BidiClassOf::R | BidiClassOf::L))
    {
        Some(BidiClassOf::L) => false,
        _ => true,
    }
}

fn permute(items: &mut Vec<usize>, at: usize, visit: &mut impl FnMut(&[usize])) {
    if at == items.len() {
        visit(items);
        return;
    }
    for i in at..items.len() {
        items.swap(at, i);
        permute(items, at + 1, visit);
        items.swap(at, i);
    }
}

/// A count of admissible orders, saying whether it was computed.
///
/// The type exists so a caller cannot report an uncomputed result as a measurement: a bare `usize`
/// makes `0` read like a finding rather than "I did not look".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ambiguity {
    /// Exactly this many orders fit the geometry.
    Count(usize),
    /// The line was longer than [`MAX_ENUMERABLE_UNITS`]; nothing was computed.
    NotComputed,
}

impl Ambiguity {
    /// The count, or `None` when it was not computed.
    pub fn count(self) -> Option<usize> {
        match self {
            Ambiguity::Count(n) => Some(n),
            Ambiguity::NotComputed => None,
        }
    }

    /// True when the geometry leaves exactly one admissible order.
    pub fn is_unique(self) -> bool {
        self.count() == Some(1)
    }
}

/// [`Ambiguity`] for a line, using the closed form.
pub fn ambiguity_of(xs: &[f64], classes: &[BidiClassOf]) -> Ambiguity {
    if xs.len() > MAX_ENUMERABLE_UNITS {
        return Ambiguity::NotComputed;
    }
    Ambiguity::Count(admissible_orders_closed_form(xs, classes))
}

/// How units group by painted position: `{group size → how many groups of that size}`.
///
/// Counts only, so a diagnostic can explain a result without naming any text.
pub fn position_profile(xs: &[f64]) -> HashMap<usize, usize> {
    let mut profile: HashMap<usize, usize> = HashMap::new();
    for group in groups_at_shared_positions(xs) {
        *profile.entry(group).or_insert(0) += 1;
    }
    profile
}
