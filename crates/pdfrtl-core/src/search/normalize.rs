//! The rules, one [`NormalizationStep`] at a time.
//!
//! **Search/index text only.** Nothing here may be applied to extracted text:
//! pdfrtl's extraction contract returns what the PDF actually encodes, and a
//! user who searches for `می‌روم` (with ZWNJ) must still *see* the ZWNJ in the
//! excerpt we hand back. Each function here produces a comparison key; see
//! `docs/SEARCH-NORMALIZATION.md` for the full rule table and its limits.
//!
//! Why the pipeline is eight small steps instead of one fused pass: a single
//! pass would be faster but untestable per rule, and the rules are exactly the
//! part that needs review — someone will eventually disagree with one of them
//! (the Hebrew gershayim rule already invites disagreement), and when they do
//! the disagreement must be a one-line change to one step with its own test.
//!
//! Every step is idempotent, so [`normalize_for_search`] is idempotent too:
//! callers may normalise an already-normalised key without thinking about it.

use unicode_normalization::UnicodeNormalization;

/// One rule of the search normalisation pipeline.
///
/// The variants double as stable identifiers ([`NormalizationStep::as_str`])
/// so tests, docs and callers can say *which* rule did *what* instead of
/// pointing at a blob of code. [`NormalizationStep::ALL`] is the canonical
/// order; [`normalize_for_search`] applies exactly that sequence, and each
/// step's [`NormalizationStep::apply`] runs it in isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NormalizationStep {
    /// NFKC-fold Arabic (U+FB50–U+FDFF, U+FE70–U+FEFF) and Hebrew
    /// (U+FB1D–U+FB4F) presentation forms back to their base letters.
    PresentationForms,
    /// Strip harakat (U+064B–U+065F, U+0670) and tatweel (U+0640).
    HarakatAndTatweel,
    /// Strip ZWNJ/ZWJ (U+200C, U+200D).
    InvisibleJoiners,
    /// Unify Persian/Arabic letter variants onto one codepoint each.
    LetterVariants,
    /// Fold Arabic-Indic and Extended Arabic-Indic digits to ASCII 0–9.
    DigitVariants,
    /// Collapse every lam-alef ligature spelling onto U+0644 U+0627.
    LamAlefLigature,
    /// Strip Hebrew niqqud/cantillation (U+0591–U+05C7).
    HebrewMarks,
    /// Fold Hebrew ligatures/punctuation (U+05F0–U+05F4) to plain letters.
    HebrewDigraphs,
}

impl NormalizationStep {
    /// Canonical pipeline order — the order [`normalize_for_search`] applies.
    pub const ALL: &'static [NormalizationStep] = &[
        NormalizationStep::PresentationForms,
        NormalizationStep::HarakatAndTatweel,
        NormalizationStep::InvisibleJoiners,
        NormalizationStep::LetterVariants,
        NormalizationStep::DigitVariants,
        NormalizationStep::LamAlefLigature,
        NormalizationStep::HebrewMarks,
        NormalizationStep::HebrewDigraphs,
    ];

    /// Stable machine identifier, snake_case, like [`crate::Reason::as_str`].
    pub const fn as_str(self) -> &'static str {
        match self {
            NormalizationStep::PresentationForms => "presentation_forms",
            NormalizationStep::HarakatAndTatweel => "harakat_and_tatweel",
            NormalizationStep::InvisibleJoiners => "invisible_joiners",
            NormalizationStep::LetterVariants => "letter_variants",
            NormalizationStep::DigitVariants => "digit_variants",
            NormalizationStep::LamAlefLigature => "lam_alef_ligature",
            NormalizationStep::HebrewMarks => "hebrew_marks",
            NormalizationStep::HebrewDigraphs => "hebrew_digraphs",
        }
    }

    /// Why this rule exists — the failure it prevents. Kept next to the code
    /// so the reason survives a refactor; the longer version (including what
    /// the rule deliberately does *not* do) lives in `docs/SEARCH-NORMALIZATION.md`.
    pub const fn description(self) -> &'static str {
        match self {
            NormalizationStep::PresentationForms => {
                "PDFs shaped with legacy fonts encode letters as presentation \
                 forms; a query typed as base letters would never match them"
            }
            NormalizationStep::HarakatAndTatweel => {
                "vowel marks and kashida are optional in running text; the same \
                 word is usually stored with and without them"
            }
            NormalizationStep::InvisibleJoiners => {
                "ZWNJ is invisible but changes the codepoints: می‌روم with ZWNJ \
                 and میروم without render identically"
            }
            NormalizationStep::LetterVariants => {
                "Arabic and Persian use different codepoints for the letters \
                 that look the same (yeh, kaf, alef, teh marbuta)"
            }
            NormalizationStep::DigitVariants => {
                "the same number is written with Arabic-Indic, Persian or ASCII \
                 digits depending on the producer and the typist"
            }
            NormalizationStep::LamAlefLigature => {
                "lam+alef may be one ligature codepoint or two letters; a query \
                 spelled either way must find both"
            }
            NormalizationStep::HebrewMarks => {
                "niqqud and cantillation are pronunciation aids a searcher never \
                 types, and they sit between the letters as separate codepoints"
            }
            NormalizationStep::HebrewDigraphs => {
                "Yiddish digraphs and gershayim are typographic ligatures; the \
                 same word is usually typed with plain letters"
            }
        }
    }

    /// Codepoints this step guarantees are **absent from its own output**.
    ///
    /// A claim, not a hint: tests assert it for every corpus input
    /// (`tests/search_normalize.rs`), so an inaccurate range fails the build.
    /// Ranges are inclusive.
    ///
    /// Empty for [`NormalizationStep::PresentationForms`] on purpose: that step
    /// *expands* rather than strips, and a handful of presentation-form
    /// codepoints (U+FDFD "BISMILLAH", U+FBB2… Arabic symbols, unassigned gaps)
    /// have no Unicode decomposition and pass through unchanged. Claiming the
    /// whole range would be a claim we cannot keep — see the known-limits
    /// section of `docs/SEARCH-NORMALIZATION.md`.
    pub const fn stripped_ranges(self) -> &'static [(char, char)] {
        match self {
            NormalizationStep::PresentationForms => &[],
            NormalizationStep::HarakatAndTatweel => &[
                ('\u{0640}', '\u{0640}'),
                ('\u{064B}', '\u{065F}'),
                ('\u{0670}', '\u{0670}'),
            ],
            NormalizationStep::InvisibleJoiners => &[('\u{200C}', '\u{200D}')],
            // Sources only: each maps to a target that is itself not a source
            // (U+06CC, U+06A9, U+0627, U+0647), so nothing re-enters the set.
            NormalizationStep::LetterVariants => &[
                ('\u{0622}', '\u{0623}'),
                ('\u{0625}', '\u{0625}'),
                ('\u{0629}', '\u{0629}'),
                ('\u{0643}', '\u{0643}'),
                ('\u{064A}', '\u{064A}'),
                ('\u{0671}', '\u{0671}'),
            ],
            NormalizationStep::DigitVariants => {
                &[('\u{0660}', '\u{0669}'), ('\u{06F0}', '\u{06F9}')]
            }
            NormalizationStep::LamAlefLigature => &[('\u{FEF5}', '\u{FEFC}')],
            NormalizationStep::HebrewMarks => &[('\u{0591}', '\u{05C7}')],
            NormalizationStep::HebrewDigraphs => &[('\u{05F0}', '\u{05F4}')],
        }
    }

    /// Apply this rule on its own. Exposed so tests — and callers debugging a
    /// surprising match — can see what one rule contributes.
    #[must_use]
    pub fn apply(self, input: &str) -> String {
        match self {
            NormalizationStep::PresentationForms => expand_presentation_forms(input),
            NormalizationStep::HarakatAndTatweel => strip(input, is_harakat_or_tatweel),
            NormalizationStep::InvisibleJoiners => strip(input, is_joiner),
            NormalizationStep::LetterVariants => map_chars(input, fold_letter_variant),
            NormalizationStep::DigitVariants => map_chars(input, fold_digit),
            NormalizationStep::LamAlefLigature => collapse_lam_alef(input),
            NormalizationStep::HebrewMarks => strip(input, is_hebrew_mark),
            NormalizationStep::HebrewDigraphs => fold_hebrew_digraphs(input),
        }
    }
}

/// The full pipeline, in [`NormalizationStep::ALL`] order.
///
/// **Search/index keys only — never feed the result to output.** Extracted
/// text stays verbatim (ZWNJ, ligatures, harakat, original digits all
/// preserved); this is the string you compare a query against, and nothing
/// else. Apply it to *both* sides of a comparison — an index built with
/// different rules than the query matches nothing.
///
/// Idempotent: `normalize_for_search(normalize_for_search(x))` equals
/// `normalize_for_search(x)` for every input (pinned by test).
#[must_use]
pub fn normalize_for_search(input: &str) -> String {
    let mut current = input.to_owned();
    for step in NormalizationStep::ALL {
        current = step.apply(&current);
    }
    current
}

/// Digit folding only ([`NormalizationStep::DigitVariants`]): Arabic-Indic
/// (U+0660–U+0669) and Extended Arabic-Indic/Persian (U+06F0–U+06F9) to ASCII.
///
/// For callers that index digits separately — a phone number or a price should
/// match across `۱۲۳`, `١٢٣` and `123` without the rest of the pipeline
/// rewriting their letters. Search/index keys only, like every function here.
#[must_use]
pub fn normalize_digits(input: &str) -> String {
    NormalizationStep::DigitVariants.apply(input)
}

/// NFKC applied *only* to presentation-form codepoints.
///
/// Deliberately not whole-string NFKC: that would also rewrite Latin
/// ligatures, fullwidth forms and superscripts — characters outside this
/// module's mandate and outside everything the rule table claims. Char-local
/// NFKC keeps the blast radius exactly what `docs/SEARCH-NORMALIZATION.md`
/// documents. (NFKC of a presentation form depends on no surrounding context,
/// so per-char folding loses nothing.)
fn expand_presentation_forms(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if is_presentation_form(c) {
            out.extend(c.nfkc());
        } else {
            out.push(c);
        }
    }
    out
}

/// True for the Arabic and Hebrew presentation-form blocks we fold.
const fn is_presentation_form(c: char) -> bool {
    matches!(c as u32, 0xFB1D..=0xFB4F | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF)
}

const fn is_harakat_or_tatweel(c: char) -> bool {
    matches!(c as u32, 0x0640 | 0x064B..=0x065F | 0x0670)
}

/// ZWNJ (U+200C) and ZWJ (U+200D). Invisible, untypeable, and the single most
/// common reason a Persian search misses: `می‌روم` and `میروم` render
/// identically but are different codepoint sequences.
const fn is_joiner(c: char) -> bool {
    matches!(c as u32, 0x200C | 0x200D)
}

/// Hebrew niqqud, cantillation and the points around them (U+0591–U+05C7).
/// The range also covers sof pasuq (U+05C3) and nof hafakh (U+05C6), which are
/// punctuation rather than vowels — accepted here because they only occur in
/// liturgical text and never inside a word a searcher queries.
const fn is_hebrew_mark(c: char) -> bool {
    matches!(c as u32, 0x0591..=0x05C7)
}

fn strip(input: &str, drop: fn(char) -> bool) -> String {
    input.chars().filter(|&c| !drop(c)).collect()
}

fn map_chars(input: &str, f: fn(char) -> char) -> String {
    input.chars().map(f).collect()
}

/// One codepoint per equivalence class — chosen so that *both* writing
/// traditions land on a codepoint that exists in normal text:
///
/// * yeh: U+064A (Arabic) and U+06CC (Persian) are the letter people mean
///   interchangeably; U+06CC is the canonical pick because pdfrtl is
///   Persian-first and U+06CC is what Persian keyboards produce.
/// * kaf: U+0643 (Arabic) and U+06A9 (Persian keheh) likewise; U+06A9 wins
///   for the same reason.
/// * alef: U+0622/U+0623/U+0625/U+0671 differ only in the madda/hamza/wasla
///   marks, which step [`NormalizationStep::HarakatAndTatweel`] has already
///   stopped treating as significant — folding them to bare U+0627 makes
///   `آب` and `اب` one query. Deliberate recall-over-precision trade-off.
/// * teh marbuta U+0629 → heh U+0647: the Egyptian/Arabic typing variant of
///   the same word (`مدرسة` vs `مدرسه`).
///
/// U+0649 (alef maksura) and U+06D2 (yeh barree) are deliberately *not*
/// folded — they have distinct final shapes; see the open questions in
/// `docs/SEARCH-NORMALIZATION.md`.
fn fold_letter_variant(c: char) -> char {
    match c {
        '\u{064A}' => '\u{06CC}',
        '\u{0643}' => '\u{06A9}',
        '\u{0622}' | '\u{0623}' | '\u{0625}' | '\u{0671}' => '\u{0627}',
        '\u{0629}' => '\u{0647}',
        other => other,
    }
}

/// Both Arabic-Indic (U+0660–U+0669) and Persian/Extended (U+06F0–U+06F9)
/// digit runs to ASCII, so `۱۲`, `١٢` and `12` produce one key. ASCII input
/// passes through untouched (the match arms never fire for it).
fn fold_digit(c: char) -> char {
    let cp = c as u32;
    let digit = match cp {
        0x0660..=0x0669 => cp - 0x0660,
        0x06F0..=0x06F9 => cp - 0x06F0,
        _ => return c,
    };
    // 0x30 + 0..=9 is '0'..'9'; if that arithmetic were ever wrong the
    // original digit survives and the digit tests fail loudly — no panic on
    // input, which is the house rule.
    char::from_u32(0x30 + digit).unwrap_or(c)
}

/// Canonical lam-alef form is the two-letter sequence U+0644 U+0627, because
/// it is what NFKC produces from every ligature spelling *and* what NFC leaves
/// alone (lam-alef is a composition exclusion, so the sequence never
/// re-merges). [`NormalizationStep::PresentationForms`] already expands the
/// ligatures in the full pipeline; this step re-states the rule for the
/// hamza/madda variants — U+FEF5–U+FEFA decompose to lam + a *variant* alef,
/// which [`NormalizationStep::LetterVariants`] then folds — and hard-codes the
/// family so the equivalence never depends solely on the Unicode table.
fn collapse_lam_alef(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        if ('\u{FEF5}'..='\u{FEFC}').contains(&c) {
            out.push_str("\u{0644}\u{0627}");
        } else {
            out.push(c);
        }
    }
    out
}

/// Yiddish digraphs → the letters they stand for, gershayim/geresh → ASCII
/// apostrophe.
///
/// The digraph folds are uncontroversial: U+05F0/U+05F1/U+05F2 are ligatures
/// of vav (U+05D5) and yod (U+05D9), and a query typed as two letters must
/// find the ligature.
///
/// U+05F3/U+05F4 are punctuation, not ligatures, so folding them is a
/// judgement call: we map them to U+0027 because that is the character a
/// Hebrew typist on a Latin layout reaches for without Shift, and ASCII is
/// the one codepoint no later rule rewrites. Typists who use `"` (U+0022)
/// still miss — recorded as an open question in
/// `docs/SEARCH-NORMALIZATION.md`, pinned by test as current behaviour.
fn fold_hebrew_digraphs(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\u{05F0}' => out.push_str("\u{05D5}\u{05D5}"),
            '\u{05F1}' => out.push_str("\u{05D5}\u{05D9}"),
            '\u{05F2}' => out.push_str("\u{05D9}\u{05D9}"),
            '\u{05F3}' | '\u{05F4}' => out.push('\''),
            other => out.push(other),
        }
    }
    out
}
