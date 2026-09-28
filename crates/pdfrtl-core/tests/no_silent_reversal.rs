//! TDD: **no silent reversal.** (ADR 0004)
//!
//! A correct logical result contains the logical form of a right-to-left word and
//! ZERO occurrences of its character-reversed form (for a pure-RTL word, visual
//! order == reversed). The corpus fixtures only check "does it extract"; this test
//! checks the thing that actually matters — that the order we hand back is the
//! reading order, not the drawing order.
//!
//! Private archive files are a *subset* and are skipped cleanly when absent, so
//! public CI stays green (corpus rule: never require private files locally).

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// One word list + one file, per language. Words are pure RTL: no Latin letters,
/// no digits — a reversed form of such a word is unambiguous.
struct Case {
    /// Repo-relative path; the case is skipped when the file is absent.
    path: &'static str,
    /// Words that MUST appear in logical order in the extracted text.
    words: &'static [&'static str],
}

const CASES: &[Case] = &[
    // Generated Chrome fixtures (redistributable — always present).
    Case {
        path: "corpus/raw/generated/chrome/fa-plain.pdf",
        words: &["دنیا"],
    },
    Case {
        path: "corpus/raw/generated/chrome/mixed-fa-en.pdf",
        words: &["گزارش", "فنی"],
    },
    // Private archive (present only on the maintainer's machine).
    Case {
        path: "corpus/raw/private/desktop-pdfs/persian-7.pdf",
        words: &["شرکت", "قرارداد", "مدیریت"],
    },
    Case {
        path: "corpus/raw/private/desktop-pdfs/arabic-2.pdf",
        words: &["منظمة", "الصحة", "إطار"],
    },
    Case {
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
    }

    // The control is worthless if nothing ran: fixtures are redistributable and
    // must be present everywhere; only the private subset may be missing.
    assert!(
        checked >= 4,
        "no-silent-reversal control ran on only {checked} word(s) ({skipped} file(s) \
         skipped) — the fixture set must cover fa/ar/he"
    );
}

fn reverse_chars(text: &str) -> String {
    text.chars().rev().collect()
}
