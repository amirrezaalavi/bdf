//! TDD for the search normaliser: every rule in isolation first, then the
//! equivalences that motivated the module, then idempotence and property loops.
//!
//! Scope reminder (the whole point of the module): normalisation applies to
//! **search/index keys only**. Extracted text stays verbatim — ZWNJ, ligatures,
//! harakat and the producer's digits all survive — so nothing here asserts
//! anything about extraction output, and no test feeds a normalised string
//! back into the text pipeline.
//!
//! Rule table and rationale: `docs/SEARCH-NORMALIZATION.md`.

use pdfrtl_core::search::NormalizationStep::*;
use pdfrtl_core::search::{normalize_digits, normalize_for_search, NormalizationStep};

/// Everything the property-style tests iterate over: the motivating cases,
/// one representative per rule, the pass-throughs we knowingly keep, and a
/// few hostile inputs (NUL, tabs, lone formatting characters, emoji).
const CORPUS: &[&str] = &[
    "",
    "hello world",
    "Hello, World! 123",
    // می‌روم (as stored, with ZWNJ) / میروم (as typed, without)
    "\u{0645}\u{06CC}\u{200C}\u{0631}\u{0648}\u{0645}",
    "\u{0645}\u{06CC}\u{0631}\u{0648}\u{0645}",
    // مَرْحَبًا plus a tatweel — harakat + kashida
    "\u{0645}\u{064E}\u{0631}\u{0652}\u{062D}\u{064E}\u{0628}\u{064B}\u{0640}",
    // كتاب (Arabic kaf) / کتاة (Persian keheh + teh marbuta) / علی vs علي
    "\u{0643}\u{062A}\u{0627}\u{0628}",
    "\u{06A9}\u{062A}\u{0627}\u{0629}",
    "\u{0639}\u{0644}\u{064A}",
    "\u{0639}\u{0644}\u{06CC}",
    // alef variants: madda, hamza above, hamza below, wasla, bare
    "\u{0622}\u{0623}\u{0625}\u{0671}\u{0627}",
    // digits: Persian, Arabic-Indic, ASCII, mixed
    "\u{06F1}\u{06F2}\u{06F3}",
    "\u{0661}\u{0662}\u{0663}",
    "123",
    "\u{06F1}\u{06F2}\u{0663}12",
    "قیمت ۱۲۳ تومان",
    // lam-alef: every spelling
    "\u{FEFB}",
    "\u{FEF5}",
    "\u{FEF7}",
    "\u{0644}\u{0627}",
    "\u{0644}\u{0623}",
    // presentation forms: plain, keheh, tav-with-dagesh, yeh, kaf-alef ligature
    "\u{FE83}",
    "\u{FB8E}",
    "\u{FB4A}",
    "\u{FEF2}",
    "\u{FC37}",
    // presentation forms with NO decomposition — knowingly passed through
    "\u{FDFD}",
    "\u{FE73}",
    "\u{FDFA}",
    // Hebrew: niqqud, dagesh, gershayim, plain
    "\u{05E9}\u{05B8}\u{05C1}\u{05DC}\u{05D5}\u{05B9}\u{05DD}",
    "\u{05E9}\u{05DC}\u{05D5}\u{05DD}",
    "\u{05D0}\u{05BC}",
    "\u{05E6}\u{05D4}\u{05F4}\u{05DC}",
    // Yiddish digraphs + geresh
    "\u{05D5}\u{05F0}\u{05D9}\u{05F1}\u{05F2}\u{05F3}",
    // hostile / non-script inputs
    "a\u{200C}b",
    "\u{0627}\u{200D}\u{0627}",
    "\u{FD3E}x\u{FD3F}",
    "\u{FEFF}x",
    "\u{200F}\u{0633}\u{0644}\u{0627}\u{0645}",
    "\u{0645}\u{0654}\u{0651}",
    "\u{0640}\u{0640}\u{0640}",
    "\u{0}tab\tnewline\n",
    "🔍\u{0633}\u{0644}\u{0627}",
];

fn in_ranges(c: char, ranges: &[(char, char)]) -> bool {
    ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi)
}

// --------------------------------------------------------------------------
// Step 1 — presentation forms
// --------------------------------------------------------------------------

#[test]
fn presentation_forms_expand_to_base_letters() {
    // ARABIC LETTER ALEF WITH HAMZA ABOVE ISOLATED FORM, ALEF WASLA, KEHEH,
    // YEH FINAL — each folds to its base letter.
    assert_eq!(PresentationForms.apply("\u{FE83}"), "\u{0623}");
    assert_eq!(PresentationForms.apply("\u{FB50}"), "\u{0671}");
    assert_eq!(PresentationForms.apply("\u{FB8E}"), "\u{06A9}");
    assert_eq!(PresentationForms.apply("\u{FEF2}"), "\u{064A}");
    // The mark survives *this* step; stripping it is the next step's job.
    // That ordering is the reason the pipeline is a fixed sequence.
    assert_eq!(PresentationForms.apply("\u{FB4A}"), "\u{05EA}\u{05BC}");
    assert_eq!(PresentationForms.apply("plain latin"), "plain latin");
}

#[test]
fn presentation_forms_without_a_unicode_decomposition_pass_through() {
    // Honesty pin, documented as a known limit: U+FDFD (BISMILLAH) and
    // U+FE73 (ARABIC TAIL FRAGMENT) have no compatibility decomposition, so
    // NFKC leaves them alone. They are the reason `stripped_ranges()` is
    // empty for this step — we do not claim what we cannot keep.
    assert_eq!(PresentationForms.apply("\u{FDFD}"), "\u{FDFD}");
    assert_eq!(PresentationForms.apply("\u{FE73}"), "\u{FE73}");
    assert!(PresentationForms.stripped_ranges().is_empty());
}

// --------------------------------------------------------------------------
// Step 2 — harakat and tatweel
// --------------------------------------------------------------------------

#[test]
fn harakat_and_tatweel_are_stripped() {
    assert_eq!(
        HarakatAndTatweel
            .apply("\u{0645}\u{064E}\u{0631}\u{0652}\u{062D}\u{064E}\u{0628}\u{064B}\u{0640}"),
        "\u{0645}\u{0631}\u{062D}\u{0628}"
    );
    // superscript alef (U+0670) is in the rule even though it sits outside
    // the U+064B–U+065F block
    assert_eq!(HarakatAndTatweel.apply("\u{0627}\u{0670}"), "\u{0627}");
    // the letters themselves are untouched
    assert_eq!(
        HarakatAndTatweel.apply("\u{0645}\u{0631}\u{062D}\u{0628}"),
        "\u{0645}\u{0631}\u{062D}\u{0628}"
    );
}

// --------------------------------------------------------------------------
// Step 3 — ZWNJ / ZWJ
// --------------------------------------------------------------------------

#[test]
fn zwnj_and_zwj_are_stripped() {
    assert_eq!(
        InvisibleJoiners.apply("\u{0645}\u{06CC}\u{200C}\u{0631}\u{0648}\u{0645}"),
        "\u{0645}\u{06CC}\u{0631}\u{0648}\u{0645}"
    );
    assert_eq!(
        InvisibleJoiners.apply("\u{0627}\u{200D}\u{0627}"),
        "\u{0627}\u{0627}"
    );
}

#[test]
fn raw_comparison_misses_the_zwnj_word_that_normalisation_finds() {
    // The motivating case: identical on screen, different codepoint
    // sequences, so a naive `contains()` search fails.
    let stored = "\u{0645}\u{06CC}\u{200C}\u{0631}\u{0648}\u{0645}";
    let typed = "\u{0645}\u{06CC}\u{0631}\u{0648}\u{0645}";
    assert_ne!(stored, typed);
    assert_eq!(normalize_for_search(stored), normalize_for_search(typed));
}

// --------------------------------------------------------------------------
// Step 4 — letter variants
// --------------------------------------------------------------------------

#[test]
fn letter_variants_fold_onto_one_codepoint_each() {
    // kaf U+0643 -> keheh U+06A9
    assert_eq!(
        LetterVariants.apply("\u{0643}\u{062A}\u{0627}\u{0628}"),
        "\u{06A9}\u{062A}\u{0627}\u{0628}"
    );
    // yeh U+064A -> farsi yeh U+06CC
    assert_eq!(
        LetterVariants.apply("\u{0639}\u{0644}\u{064A}"),
        "\u{0639}\u{0644}\u{06CC}"
    );
    // every alef variant -> bare alef
    assert_eq!(
        LetterVariants.apply("\u{0622}\u{0623}\u{0625}\u{0671}"),
        "\u{0627}\u{0627}\u{0627}\u{0627}"
    );
    // teh marbuta -> heh
    assert_eq!(
        LetterVariants.apply("\u{0645}\u{062F}\u{0631}\u{0633}\u{0629}"),
        "\u{0645}\u{062F}\u{0631}\u{0633}\u{0647}"
    );
    // the canonical forms themselves pass through
    assert_eq!(
        LetterVariants.apply("\u{0627}\u{06CC}\u{06A9}\u{0647}"),
        "\u{0627}\u{06CC}\u{06A9}\u{0647}"
    );
}

#[test]
fn full_pipeline_makes_arabic_and_persian_spellings_equal() {
    // كتاب (Arabic kaf) vs کتاب (keheh), differing only in codepoints
    assert_ne!(
        "\u{0643}\u{062A}\u{0627}\u{0628}",
        "\u{06A9}\u{062A}\u{0627}\u{0628}"
    );
    assert_eq!(
        normalize_for_search("\u{0643}\u{062A}\u{0627}\u{0628}"),
        normalize_for_search("\u{06A9}\u{062A}\u{0627}\u{0628}")
    );
    // علي (Arabic yeh) vs علی (Persian yeh)
    assert_eq!(
        normalize_for_search("\u{0639}\u{0644}\u{064A}"),
        normalize_for_search("\u{0639}\u{0644}\u{06CC}")
    );
    // a shaped yeh (U+FEF2) reaches the same key as the typed letter
    assert_eq!(normalize_for_search("\u{FEF2}"), "\u{06CC}");
}

// --------------------------------------------------------------------------
// Step 5 — digits
// --------------------------------------------------------------------------

#[test]
fn both_digit_blocks_fold_to_ascii() {
    assert_eq!(normalize_digits("\u{06F1}\u{06F2}\u{06F3}"), "123");
    assert_eq!(normalize_digits("\u{0661}\u{0662}\u{0663}"), "123");
    assert_eq!(normalize_digits("123"), "123");
    // Persian and Arabic-Indic digits in one string agree digit-for-digit
    assert_eq!(normalize_digits("\u{06F1}\u{0662}"), "12");
    // letters and spaces are not digits and stay put
    assert_eq!(
        normalize_digits("\u{0642}\u{06CC}\u{0645}\u{062A} \u{06F1}\u{06F2}"),
        "\u{0642}\u{06CC}\u{0645}\u{062A} 12"
    );
}

#[test]
fn full_pipeline_makes_all_three_digit_scripts_equal() {
    assert_eq!(
        normalize_for_search("\u{06F1}\u{06F2}"),
        normalize_for_search("\u{0661}\u{0662}")
    );
    assert_eq!(
        normalize_for_search("\u{06F1}\u{06F2}"),
        normalize_for_search("12")
    );
}

// --------------------------------------------------------------------------
// Step 6 — lam-alef
// --------------------------------------------------------------------------

#[test]
fn every_lam_alef_spelling_collapses_to_the_two_letter_sequence() {
    for ligature in [
        "\u{FEF5}", "\u{FEF6}", "\u{FEF7}", "\u{FEF8}", "\u{FEF9}", "\u{FEFA}", "\u{FEFB}",
        "\u{FEFC}",
    ] {
        assert_eq!(LamAlefLigature.apply(ligature), "\u{0644}\u{0627}");
    }
    // the sequence is already canonical and stays as it is
    assert_eq!(
        LamAlefLigature.apply("\u{0644}\u{0627}"),
        "\u{0644}\u{0627}"
    );
}

#[test]
fn full_pipeline_treats_ligature_and_sequence_as_one_query() {
    let canonical = normalize_for_search("\u{0644}\u{0627}");
    for spelling in [
        "\u{FEFB}",         // lam-alef isolated ligature
        "\u{FEFC}",         // lam-alef final ligature
        "\u{0644}\u{0627}", // lam + bare alef
        "\u{FEF7}",         // lam-alef with hamza above
        "\u{0644}\u{0623}", // lam + alef with hamza above
        "\u{FEF5}",         // lam-alef with madda
        "\u{0644}\u{0622}", // lam + alef with madda
    ] {
        assert_eq!(
            normalize_for_search(spelling),
            canonical,
            "spelling {spelling:?}"
        );
    }
}

// --------------------------------------------------------------------------
// Step 7 — Hebrew marks
// --------------------------------------------------------------------------

#[test]
fn hebrew_niqqud_and_cantillation_are_stripped() {
    // שָׁלוֹם (with qamats, shin dot, holam) -> שלום
    assert_eq!(
        HebrewMarks.apply("\u{05E9}\u{05B8}\u{05C1}\u{05DC}\u{05D5}\u{05B9}\u{05DD}"),
        "\u{05E9}\u{05DC}\u{05D5}\u{05DD}"
    );
    // dagesh (U+05BC) is inside the documented range
    assert_eq!(HebrewMarks.apply("\u{05D0}\u{05BC}"), "\u{05D0}");
    // plain letters pass through
    assert_eq!(
        HebrewMarks.apply("\u{05E9}\u{05DC}\u{05D5}\u{05DD}"),
        "\u{05E9}\u{05DC}\u{05D5}\u{05DD}"
    );
}

#[test]
fn full_pipeline_finds_vocalised_and_unvocalised_hebrew_alike() {
    assert_eq!(
        normalize_for_search("\u{05E9}\u{05B8}\u{05C1}\u{05DC}\u{05D5}\u{05B9}\u{05DD}"),
        normalize_for_search("\u{05E9}\u{05DC}\u{05D5}\u{05DD}")
    );
}

// --------------------------------------------------------------------------
// Step 8 — Hebrew digraphs and gershayim
// --------------------------------------------------------------------------

#[test]
fn yiddish_digraphs_and_gershayim_fold() {
    assert_eq!(HebrewDigraphs.apply("\u{05F0}"), "\u{05D5}\u{05D5}"); // double vav
    assert_eq!(HebrewDigraphs.apply("\u{05F1}"), "\u{05D5}\u{05D9}"); // vav yod
    assert_eq!(HebrewDigraphs.apply("\u{05F2}"), "\u{05D9}\u{05D9}"); // double yod
                                                                      // Judgement call, pinned as current behaviour: gershayim/geresh become
                                                                      // ASCII apostrophe (the character a Latin-layout typist can actually
                                                                      // type). See the open-questions section of docs/SEARCH-NORMALIZATION.md.
    assert_eq!(HebrewDigraphs.apply("\u{05F4}"), "'");
    assert_eq!(HebrewDigraphs.apply("\u{05F3}"), "'");
    // a query typed with ASCII matches the text stored with gershayim
    assert_eq!(
        normalize_for_search("\u{05E6}\u{05D4}\u{05F4}\u{05DC}"),
        normalize_for_search("\u{05E6}\u{05D4}'\u{05DC}")
    );
}

// --------------------------------------------------------------------------
// Pipeline-level properties
// --------------------------------------------------------------------------

#[test]
fn latin_and_untouched_text_comes_back_identically() {
    // No rule fires for Latin: no case folding, no whitespace collapsing —
    // documented deliberate non-goals, pinned here.
    assert_eq!(
        normalize_for_search("Hello, World! 123"),
        "Hello, World! 123"
    );
    assert_eq!(normalize_for_search(""), "");
}

#[test]
fn step_ids_are_stable_snake_case() {
    assert_eq!(PresentationForms.as_str(), "presentation_forms");
    assert_eq!(HarakatAndTatweel.as_str(), "harakat_and_tatweel");
    assert_eq!(InvisibleJoiners.as_str(), "invisible_joiners");
    assert_eq!(LetterVariants.as_str(), "letter_variants");
    assert_eq!(DigitVariants.as_str(), "digit_variants");
    assert_eq!(LamAlefLigature.as_str(), "lam_alef_ligature");
    assert_eq!(HebrewMarks.as_str(), "hebrew_marks");
    assert_eq!(HebrewDigraphs.as_str(), "hebrew_digraphs");
    for step in NormalizationStep::ALL {
        assert!(
            !step.description().is_empty(),
            "{} has no why",
            step.as_str()
        );
    }
}

#[test]
fn all_steps_are_listed_once_in_canonical_order() {
    let expected = [
        "presentation_forms",
        "harakat_and_tatweel",
        "invisible_joiners",
        "letter_variants",
        "digit_variants",
        "lam_alef_ligature",
        "hebrew_marks",
        "hebrew_digraphs",
    ];
    let ids: Vec<&str> = NormalizationStep::ALL.iter().map(|s| s.as_str()).collect();
    assert_eq!(ids, expected);
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), ids.len(), "duplicate step id");
}

#[test]
fn the_pipeline_is_idempotent_over_the_corpus() {
    for input in CORPUS {
        let once = normalize_for_search(input);
        let twice = normalize_for_search(&once);
        assert_eq!(once, twice, "not idempotent for {input:?}");
    }
}

#[test]
fn every_single_step_is_idempotent_over_the_corpus() {
    for step in NormalizationStep::ALL {
        for input in CORPUS {
            let once = step.apply(input);
            let twice = step.apply(&once);
            assert_eq!(
                once,
                twice,
                "{} not idempotent for {input:?}",
                step.as_str()
            );
        }
    }
}

#[test]
fn each_step_never_returns_a_codepoint_it_claims_to_strip() {
    for step in NormalizationStep::ALL {
        for input in CORPUS {
            let out = step.apply(input);
            for c in out.chars() {
                assert!(
                    !in_ranges(c, step.stripped_ranges()),
                    "{} (U+{:04X}) survived a step whose stripped_ranges() forbids it, in {out:?}",
                    step.as_str(),
                    c as u32
                );
            }
        }
    }
}

#[test]
fn the_full_pipeline_never_returns_a_codepoint_any_step_claims_to_strip() {
    let union: Vec<(char, char)> = NormalizationStep::ALL
        .iter()
        .flat_map(|step| step.stripped_ranges().iter().copied())
        .collect();
    assert!(!union.is_empty(), "the claim set should not be empty");
    for input in CORPUS {
        let out = normalize_for_search(input);
        for c in out.chars() {
            assert!(
                !in_ranges(c, &union),
                "U+{:04X} leaked into {out:?}",
                c as u32
            );
        }
    }
}

#[test]
fn every_individually_claimed_codepoint_is_gone_after_the_full_pipeline() {
    // Stronger than a corpus sweep: walk each claimed range codepoint by
    // codepoint and feed it through the whole pipeline on its own.
    for step in NormalizationStep::ALL {
        for &(lo, hi) in step.stripped_ranges() {
            for c in lo..=hi {
                let out = normalize_for_search(&c.to_string());
                assert!(
                    !out.contains(c),
                    "U+{:04X} (claimed stripped by {}) survived as {out:?}",
                    c as u32,
                    step.as_str()
                );
            }
        }
    }
}

#[test]
fn never_panics_and_never_leaks_over_all_of_unicode() {
    // Property-style loop: every Unicode scalar value, concatenated, goes
    // through the pipeline twice (second pass doubles as an idempotence
    // check at maximum coverage). A panic here fails the test; a leak of any
    // claimed codepoint fails it too.
    let every: String = (0u32..=0x10FFFF).filter_map(char::from_u32).collect();
    let once = normalize_for_search(&every);
    let twice = normalize_for_search(&once);
    assert_eq!(
        once, twice,
        "pipeline is not idempotent over all of Unicode"
    );
    let union: Vec<(char, char)> = NormalizationStep::ALL
        .iter()
        .flat_map(|step| step.stripped_ranges().iter().copied())
        .collect();
    for c in once.chars() {
        assert!(
            !in_ranges(c, &union),
            "U+{:04X} leaked over the full sweep",
            c as u32
        );
    }
}
