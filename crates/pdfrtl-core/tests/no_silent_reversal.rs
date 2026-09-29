//! TDD: **no silent reversal.** (ADR 0004)
//!
//! A correct logical result contains the logical form of a right-to-left word and
//! ZERO occurrences of its character-reversed form (for a pure-RTL word, visual
//! order == reversed). The corpus fixtures only check "does it extract"; this test
//! checks the thing that actually matters — that the order we hand back is the
//! reading order, not the drawing order.
//!
//! Private archive files are a *subset* and are skipped cleanly when absent, so
//! public CI stays green (corpus rule: never require private files locally). The
//! fa/ar/he coverage the guard demands comes from redistributable fixtures
//! (`corpus/raw/generated/chrome/*`, `corpus/raw/synthetic/actualtext-{fa,ar,he}.pdf`),
//! so the control still runs — and still fails when a language's fixtures are missing —
//! on the public mirror, where `corpus/raw/private/` does not exist at all.

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// One word list + one file, per language. Words are pure RTL: no Latin letters,
/// no digits — a reversed form of such a word is unambiguous. Each word has FOUR OR
/// MORE letters: three-letter words can match by coincidence across a boundary
/// (`דוח` inside `חודש`), which would make the reversed-form assertion vacuous.
struct Case {
    /// Language this case proves coverage for; the coverage guard requires fa, ar and
    /// he each to have at least one case that actually ran.
    lang: &'static str,
    /// Repo-relative path; the case is skipped when the file is absent.
    path: &'static str,
    /// Words that MUST appear in logical order in the extracted text.
    words: &'static [&'static str],
}

const CASES: &[Case] = &[
    // Redistributable fixtures (committed — present on every clone, public mirror
    // included). These are what make the coverage guard satisfiable without the
    // private archive.
    Case {
        lang: "fa",
        path: "corpus/raw/generated/chrome/fa-plain.pdf",
        words: &["دنیا"],
    },
    Case {
        lang: "fa",
        path: "corpus/raw/generated/chrome/mixed-fa-en.pdf",
        words: &["گزارش", "فنی"],
    },
    Case {
        lang: "ar",
        path: "corpus/raw/synthetic/actualtext-ar.pdf",
        words: &["السلام", "مرحبا"],
    },
    Case {
        lang: "he",
        path: "corpus/raw/synthetic/actualtext-he.pdf",
        words: &["שלום", "עולם", "תודה"],
    },
    // Private archive (present only on the maintainer's machine).
    Case {
        lang: "fa",
        path: "corpus/raw/private/desktop-pdfs/persian-7.pdf",
        words: &["شرکت", "قرارداد", "مدیریت"],
    },
    Case {
        lang: "ar",
        path: "corpus/raw/private/desktop-pdfs/arabic-2.pdf",
        words: &["منظمة", "الصحة", "إطار"],
    },
    Case {
        lang: "he",
        path: "corpus/raw/private/desktop-pdfs/hebrew-4.pdf",
        words: &["פניות", "ציבור"],
    },
];

/// Word-boundary count: `דוח` inside `חודש` does not count, in either direction.
fn count_word(text: &str, word: &str) -> usize {
    let word_char = |c: char| c.is_alphanumeric();
    text.match_indices(word)
        .filter(|(index, matched)| {
            let before_ok = text[..*index]
                .chars()
                .next_back()
                .is_none_or(|c| !word_char(c));
            let after_ok = text[index + matched.len()..]
                .chars()
                .next()
                .is_none_or(|c| !word_char(c));
            before_ok && after_ok
        })
        .count()
}

#[test]
fn rtl_text_is_never_silently_reversed() {
    let root = repo_root();
    let mut checked = 0usize;
    let mut skipped = 0usize;
    let mut covered: Vec<&str> = Vec::new();

    for case in CASES {
        let path = root.join(case.path);
        if !path.exists() {
            skipped += 1;
            eprintln!("skip (not present locally): {}", case.path);
            continue;
        }
        let pages = pdfrtl_core::extract(&path).unwrap_or_else(|err| {
            panic!("extract failed on {}: {err}", case.path);
        });
        let text: String = pages
            .iter()
            .map(|page| page.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        for &word in case.words {
            let logical = count_word(&text, word);
            let reversed = count_word(&text, &reverse_chars(word));
            assert!(
                logical > 0,
                "{}: expected the LOGICAL form {word:?} at least once, found none — \
                 the extractor is returning visual order",
                case.path
            );
            assert_eq!(
                reversed, 0,
                "{}: found {reversed} occurrence(s) of the REVERSED form of {word:?} — \
                 silent reversal (ADR 0002/0004)",
                case.path
            );
            checked += 1;
        }
        if !covered.contains(&case.lang) {
            covered.push(case.lang);
        }
    }

    // The control is worthless if nothing ran: fixtures are redistributable and
    // must be present everywhere; only the private subset may be missing.
    assert!(
        checked >= 4,
        "no-silent-reversal control ran on only {checked} word(s) ({skipped} file(s) \
         skipped) — the fixture set must cover fa/ar/he"
    );
    // Word count alone is not enough: without the private archive the public fixture
    // set has to cover every language on its own, and losing one language's fixtures
    // (an accidental delete, a bad .gitignore, a partial publish) must fail loudly
    // instead of shrinking the control silently.
    for required in ["fa", "ar", "he"] {
        assert!(
            covered.contains(&required),
            "no-silent-reversal control never ran a {required} fixture ({skipped} file(s) \
             skipped, {checked} word(s) checked) — fa/ar/he must each be covered by a \
             redistributable fixture"
        );
    }
}

fn reverse_chars(text: &str) -> String {
    text.chars().rev().collect()
}
