//! The order oracle, tested — including the cross-check that makes it trustworthy.
//!
//! `docs/problems/0018` requires an exhaustive oracle before any new hypothesis is built. Two
//! properties matter here:
//!
//! 1. The oracle reports the truth about a line's geometry: one order when positions are distinct,
//!    a product of factorials when they tie, and an explicit "not computed" above its budget.
//! 2. The two implementations agree. One implementation would make "the oracle agrees with the
//!    code" unfalsifiable — the same defect as verifying a gate with the code it gates.
//!
//! Every number here comes from positions alone. No text, no lines, no filenames: this is a proof
//! tool, not a data dump.

use pdfrtl_core::text::oracle::{
    admissible_orders_by_enumeration, admissible_orders_closed_form, ambiguity_of,
    base_direction_is_fixed, groups_at_shared_positions, position_profile, Ambiguity, BidiClassOf,
    MAX_ENUMERABLE_UNITS,
};

/// A line of strongly-RTL units: UAX #9 P2 fixes the base direction, so the count is the tie
/// product alone. This is the shape of a Persian line.
fn rtl(n: usize) -> Vec<BidiClassOf> {
    vec![BidiClassOf::R; n]
}

/// A line with no strong character at all — neutrals and digits only. UAX #9 P3 inherits the
/// paragraph level, which a painting does not reveal, so the count carries a factor of 2.
fn no_strong(n: usize) -> Vec<BidiClassOf> {
    vec![BidiClassOf::WS; n]
}

/// Every composition of `n` into group sizes, i.e. every possible tie pattern for `n` units.
fn compositions(n: usize) -> Vec<Vec<usize>> {
    fn build(remaining: usize, acc: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
        if remaining == 0 {
            out.push(acc.clone());
            return;
        }
        for take in 1..=remaining {
            acc.push(take);
            build(remaining - take, acc, out);
            acc.pop();
        }
    }
    let mut out = Vec::new();
    build(n, &mut Vec::new(), &mut out);
    out
}

/// Positions for a tie pattern: each group occupies one shared x, groups are 10 apart.
fn positions_for(composition: &[usize]) -> Vec<f64> {
    let mut positions = Vec::new();
    for (index, &size) in composition.iter().enumerate() {
        let x = 10.0 + index as f64 * 10.0;
        for _ in 0..size {
            positions.push(x);
        }
    }
    positions
}

/// The closed form and the enumeration must agree for EVERY tie pattern up to 8 units.
///
/// n ≤ 5 is every possible composition. At 6–8 that would be 64–128 patterns × 40,320
/// permutations, so those lengths use one representative pattern per group size, which is where a
/// closed form actually goes wrong.
#[test]
fn closed_form_agrees_with_enumeration() {
    for n in 1..=5usize {
        for composition in compositions(n) {
            let positions = positions_for(&composition);
            let closed = admissible_orders_closed_form(&positions, &rtl(n));
            let enumerated = admissible_orders_by_enumeration(&positions, &rtl(n));
            assert_eq!(
                closed, enumerated,
                "disagree for composition {composition:?} (n={n}): closed={closed} \
                 enumerated={enumerated}"
            );
        }
    }

    for n in 6..=MAX_ENUMERABLE_UNITS {
        for composition in [
            vec![1usize; n], // every position distinct
            vec![n],         // one big tie
            {
                let mut c = vec![1usize; n];
                c[1] = 2;
                c[2] = 1;
                c
            }, // one tied pair
            {
                let mut c = vec![1usize; n];
                c[2] = 3;
                c
            }, // one tied triple
            {
                let mut c = vec![2usize; n];
                c
            }, // every position a pair
        ] {
            let positions = positions_for(&composition);
            assert_eq!(
                admissible_orders_closed_form(&positions, &rtl(n)),
                admissible_orders_by_enumeration(&positions, &rtl(n)),
                "disagree at n={n} for composition {composition:?}"
            );
        }
    }
}

/// Distinct positions admit exactly one order. If this ever reported 2, the oracle would be
/// measuring nothing and every other number in this file would be meaningless.
#[test]
fn distinct_positions_give_a_unique_order() {
    // Expected: every tie group has size 1, so 1! x 1! x 1! = 1. Base direction is fixed by the
    // strong RTL characters, so no extra factor. A line of two RTL units at distinct x has
    // exactly ONE order that paints as measured.
    let two = [10.0, 20.0];
    assert_eq!(admissible_orders_closed_form(&two, &rtl(2)), 1);
    let four = [10.0, 20.0, 30.0, 40.0];
    assert_eq!(admissible_orders_closed_form(&four, &rtl(4)), 1);
    assert!(ambiguity_of(&two, &rtl(2)).is_unique());
}

/// A full tie admits every permutation. This is the tie refusal as arithmetic: uniqueness is not
/// present to be found, so a solver claiming to find it is guessing.
#[test]
fn a_tie_admits_every_permutation() {
    for n in 1..=MAX_ENUMERABLE_UNITS {
        let expected: usize = (1..=n).product();
        assert_eq!(
            admissible_orders_closed_form(&vec![100.0; n], &rtl(n)),
            expected,
            "n={n} units at one position must admit all n! orders"
        );
    }
}

/// Groups multiply: two tied pairs admit 2!·2! = 4, not 4! = 24.
#[test]
fn independent_groups_multiply() {
    // Expected: two tied pairs -> 2! x 2! = 4. Three tied pairs -> 2! x 2! x 2! = 8.
    let two_pairs = [10.0, 10.0, 20.0, 20.0];
    assert_eq!(admissible_orders_closed_form(&two_pairs, &rtl(4)), 4);
    let three_pairs = [10.0, 10.0, 20.0, 20.0, 30.0, 30.0];
    assert_eq!(admissible_orders_closed_form(&three_pairs, &rtl(6)), 8);
}

/// One tied pair among distinct units doubles the count — sensitivity, not a constant.
#[test]
fn one_tied_pair_admits_exactly_two() {
    // Expected: groups are [1, 2, 1] -> 1! x 2! x 1! = 2. The tied pair is the only thing
    // interchangeable; the other two are pinned by their distinct positions.
    let xs = [10.0, 20.0, 20.0, 40.0];
    assert_eq!(admissible_orders_closed_form(&xs, &rtl(4)), 2);
}

/// The count must depend on the geometry.
#[test]
fn the_count_depends_on_the_geometry() {
    let distinct = ambiguity_of(&[10.0, 20.0, 30.0, 40.0], &rtl(4));
    let tied = ambiguity_of(&[10.0, 10.0, 10.0, 40.0], &rtl(4));
    assert_ne!(distinct, tied);
    assert_eq!(distinct, Ambiguity::Count(1));
    // Expected: one tie group of 3 -> 3! = 6.
    assert_eq!(tied, Ambiguity::Count(6));
}

/// Above the budget the oracle must say it did not compute, never return a number.
#[test]
fn an_unenumerable_line_reports_not_computed() {
    let n = MAX_ENUMERABLE_UNITS + 1;
    let xs: Vec<f64> = (0..n).map(|i| i as f64 * 10.0).collect();
    let got = ambiguity_of(&xs, &rtl(n));
    assert_eq!(got, Ambiguity::NotComputed);
    assert_eq!(
        got.count(),
        None,
        "a not-computed result must not read as a count of 0"
    );
    assert!(!got.is_unique());
}

/// A position difference below the tolerance must not invent a distinction.
#[test]
fn sub_tolerance_positions_are_one_position() {
    let xs = [100.0, 100.0 + 1e-9, 100.0 - 1e-9, 200.0];
    // Expected: the three near-identical positions collapse to one group of 3 -> 3! = 6.
    assert_eq!(
        admissible_orders_closed_form(&xs, &rtl(4)),
        6,
        "coordinates 1e-9 apart describe a distinction no renderer can show"
    );
}

/// Group sizes are what the count is built from, and the profile aggregates them.
#[test]
fn group_sizes_and_profile_are_consistent() {
    let xs = [10.0, 10.0, 20.0, 30.0, 30.0, 30.0];
    assert_eq!(groups_at_shared_positions(&xs), vec![2, 1, 3]);
    let profile = position_profile(&xs);
    assert_eq!(profile.get(&2), Some(&1));
    assert_eq!(profile.get(&1), Some(&1));
    assert_eq!(profile.get(&3), Some(&1));
}

/// The empty line has exactly one order. Treating it as zero would make an empty page look like a
/// contradiction between the file and the model.
#[test]
fn an_empty_line_is_unique() {
    assert_eq!(admissible_orders_closed_form(&[], &[]), 1);
    assert_eq!(admissible_orders_by_enumeration(&[], &[]), 1);
}

/// The script classes ARE an input, and that correction is the finding (docs/problems/0019).
///
/// Two units at distinct positions, strongly RTL: one admissible order. The same geometry with NO
/// strong character: two, because UAX #9 P3 inherits the paragraph level and the painting does not
/// reveal it. Geometry alone cannot tell these apart, which is why the oracle takes the classes.
#[test]
fn the_base_direction_doubles_the_answer_when_it_is_unmeasured() {
    let xs = [10.0, 20.0];
    let fixed = ambiguity_of(&xs, &rtl(2));
    let open = ambiguity_of(&xs, &no_strong(2));

    assert_eq!(fixed, Ambiguity::Count(1));
    assert_eq!(open, Ambiguity::Count(2));
    assert!(
        base_direction_is_fixed(&rtl(2)),
        "a strong character fixes the base direction"
    );
    assert!(
        !base_direction_is_fixed(&no_strong(2)),
        "no strong character means UAX #9 P3 inherits an unmeasured paragraph level"
    );
}

/// A mixed line with a strong character somewhere: the base direction is fixed, so the count is
/// the tie product alone even though digits and neutrals are present.
#[test]
fn a_mixed_line_with_one_strong_character_keeps_the_tie_product() {
    let classes = vec![
        BidiClassOf::R,
        BidiClassOf::WS,
        BidiClassOf::EN,
        BidiClassOf::R,
        BidiClassOf::ON,
    ];
    assert!(base_direction_is_fixed(&classes));
    // Expected: two tie groups of 2 -> 2! x 2! = 4, and no direction factor.
    let xs = [10.0, 10.0, 20.0, 20.0, 30.0];
    assert_eq!(admissible_orders_closed_form(&xs, &classes), 4);
}

/// Both implementations must agree on the class-aware counts too — the cross-check is the point.
#[test]
fn the_two_implementations_agree_on_direction_free_lines() {
    let xs = [10.0, 20.0];
    assert_eq!(
        admissible_orders_closed_form(&xs, &rtl(2)),
        admissible_orders_by_enumeration(&xs, &rtl(2)),
        "a fixed-direction line disagrees"
    );
    assert_eq!(
        admissible_orders_closed_form(&xs, &no_strong(2)),
        admissible_orders_by_enumeration(&xs, &no_strong(2)),
        "a direction-free line disagrees"
    );
    // 4 units all at one position, no strong character.
    //
    // Hand-checked: `walk_matches` holds for EVERY order under BOTH directions (nothing ever
    // moves against itself when there is only one position), so the direction is not a choice
    // that excludes anything — the admissible set is 4! = 24, and NOT 2 x 4! = 48. The factor of 2
    // is a union of two walks, not a sum, and double-counting it is the mistake this pins.
    let tied = vec![100.0; 4];
    assert_eq!(
        admissible_orders_closed_form(&tied, &no_strong(4)),
        admissible_orders_by_enumeration(&tied, &no_strong(4)),
        "an all-tied direction-free line disagrees"
    );
    assert_eq!(
        admissible_orders_closed_form(&tied, &no_strong(4)),
        24,
        "every order already walks both ways, so the direction adds nothing"
    );

    // But when the tie groups do NOT span everything, the direction genuinely excludes orders and
    // the factor of 2 applies: 2 units at distinct positions -> one order per direction -> 2.
    let distinct = [10.0, 20.0];
    assert_eq!(admissible_orders_closed_form(&distinct, &no_strong(2)), 2);
}
