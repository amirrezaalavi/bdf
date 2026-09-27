# Search normalisation — rules, reasons, limits

> **Search/index keys only.** Every function in `crates/pdfrtl-core/src/search/`
> builds a string for *comparison*. pdfrtl's extraction contract — text comes
> back **verbatim** (ZWNJ preserved, lam-alef ligatures preserved, harakat
> preserved, the producer's digits preserved; see `AGENTS.md` and
> `docs/decisions/0002-logical-order-invariant.md`) — is not affected by a
> single byte of this module. Nothing under `src/text/` imports `src/search/`.
> Never display, never store, never return `normalize_for_search()` output to a
> user; it is the key you compare a query against, and nothing else.

## API

| item | purpose |
|---|---|
| `search::normalize_for_search(&str) -> String` | the full pipeline, canonical order |
| `search::normalize_digits(&str) -> String` | rule 5 alone — digits only |
| `search::NormalizationStep` | one enum variant per rule, so tests and callers can say *which* rule did *what* |
| `NormalizationStep::ALL` | canonical order; `normalize_for_search` is defined as "apply every step in this order" |
| `NormalizationStep::apply(step, &str)` | run one rule in isolation |
| `NormalizationStep::as_str()` / `description()` | stable snake_case id and the one-line *why* (house style: every non-obvious decision carries a reason) |
| `NormalizationStep::stripped_ranges()` | the codepoints a rule guarantees are **absent from its own output** — a claim, asserted by `tests/search_normalize.rs` |

Use it on **both** sides of a comparison. An index normalised with these rules
and a query typed raw (or the reverse) matches nothing.

## The rules

Order matters — column "order" is the pipeline position, not a ranking.

| # | `NormalizationStep` | codepoints | effect | why |
|---|---|---|---|---|
| 1 | `presentation_forms` | U+FB1D–U+FB4F (Hebrew), U+FB50–U+FDFF, U+FE70–U+FEFF (Arabic) | NFKC per character, only inside those blocks | PDFs shaped with legacy fonts store letters as presentation forms (`ﻓ`, `ﻼ`, `תּ`). A query typed as base letters would never match a codepoint the font never emitted. |
| 2 | `harakat_and_tatweel` | U+064B–U+065F, U+0670, U+0640 | removed | Vowel marks are optional in running text: the same word is stored vocalised in one PDF and bare in another. Tatweel (kashida) is pure decoration. |
| 3 | `invisible_joiners` | U+200C (ZWNJ), U+200D (ZWJ) | removed | The single most common Persian search miss: `می‌روم` (with ZWNJ) and `میروم` (without) render **identically** but are different codepoint sequences, so a raw `contains()` fails. ZWNJ is required for correct *rendering* (it selects the medial form), which is exactly why it must go before *matching*. |
| 4 | `letter_variants` | U+064A→U+06CC (yeh), U+0643→U+06A9 (kaf/keheh), U+0622/U+0623/U+0625/U+0671→U+0627 (alef), U+0629→U+0647 (teh marbuta→heh) | mapped to one canonical codepoint | Arabic and Persian keyboards emit different codepoints for letters that are the same letter (`كتاب`/`کتاب`, `علي`/`علی`); hamza and madda on the alef are marks, and rule 2 has already declared marks insignificant. Direction is Persian-first (U+06CC, U+06A9) because pdfrtl is Persian-first. |
| 5 | `digit_variants` | U+0660–U+0669, U+06F0–U+06F9 → `0`–`9` | mapped to ASCII | The same number is written with Arabic-Indic, Persian or ASCII digits depending on producer and typist. One number, one key. |
| 6 | `lam_alef_ligature` | U+FEF5–U+FEFC → U+0644 U+0627 | mapped to the two-letter sequence | `لا` may be one codepoint (ligature) or two (lam + alef); a query spelled either way must find both. The canonical form is the **sequence** because NFKC produces it from every ligature and NFC leaves it alone (lam-alef is a composition exclusion, so the sequence never re-merges). Rule 6 restates the family in code so the equivalence does not depend solely on the Unicode table, and folds the hamza/madda variants to the bare alef — consistent with rule 4. |
| 7 | `hebrew_marks` | U+0591–U+05C7 | removed | Niqqud and cantillation are pronunciation aids a searcher never types; they sit *between* the letters as separate codepoints. Includes dagesh (U+05BC), the shin/sin dots (U+05C1/U+05C2), and — see limits — sof pasuq (U+05C3). |
| 8 | `hebrew_digraphs` | U+05F0→`וו`, U+05F1→`וי`, U+05F2→`יי`, U+05F3/U+05F4→`'` | mapped | The digraphs are ligatures of vav (U+05D5) and yod (U+05D9); a query typed as two letters must find the ligature. Gershayim/geresh are punctuation — folding them is a judgement call, see open questions. |

### Why this order

* **1 before 2:** expanding a presentation form can *produce* harakat
  (`U+FB4A` → `U+05EA U+05BC`, and shaped forms decompose with hamza/shadda
  marks), so stripping has to happen after the expansion.
* **1 before 4:** shaped forms decompose to the *variant* base letters
  (`U+FEF2` → U+064A, `U+FB8E` → U+06A9), so the variant fold has to run after
  them or the shaped spelling would survive.
* **6 after 1 and 4:** `U+FEF7` decomposes to lam + *alef-with-hamza*; rule 4
  then folds that alef. The ligature rule runs last among the Arabic rules so
  its output is already canonical.
* **7 before 8:** independent scripts, but keeping Hebrew last keeps the rule
  table readable — Arabic (1–6), Hebrew (7–8).

### Design decisions worth defending

* **Char-local NFKC, not whole-string NFKC.** Whole-string NFKC would also
  rewrite Latin ligatures (`ﬁ`), fullwidth forms and superscripts — outside
  this module's mandate and outside anything the table above claims. NFKC of a
  presentation form needs no context, so nothing is lost.
* **The pipeline is the composition of the tested steps**, not a separate
  fused implementation: what the tests exercise per rule is exactly what runs.
  Eight passes over a query string is noise next to the clarity.
* **Every step is idempotent**, hence so is `normalize_for_search`:
  `normalize(normalize(x)) == normalize(x)` is asserted in the tests, so a
  caller may re-normalise a key without thinking.
* **Recall over precision, deliberately.** Rules 4 and 6 merge things a
  linguist might keep apart (madda-alef vs bare alef, `ة` vs `ه`). For search
  that trade is right — a miss costs a user an empty result page; a false hit
  costs them a glance — but it is a choice, listed below with its cost.

## What this deliberately does NOT do (limits)

* **No extraction output changes.** Not one byte. This is a comparison key.
* **No case folding, no whitespace collapsing, no accent stripping for Latin.**
  `Hello, World! 123` comes back byte-identical (pinned by test).
* **No stemming, no definite-article (`ال`) removal, no lemmatisation.** A
  search for `كتاب` will not find `الكتاب` unless the caller strips `ال`.
  Those are language-specific and belong above this layer.
* **Presentation forms with no Unicode decomposition pass through unchanged.**
  Verified against `unicode-normalization` 0.1.25: U+FB1E, U+FBB2–U+FBC2
  (Arabic symbol marks), U+FD3E/U+FD3F (ornate parentheses), U+FD40–U+FD4F and
  U+FDCF (unexpandable praise ligatures), U+FDFD–U+FDFF (BISMILLAH and friends),
  U+FE73 (Arabic tail fragment), U+FEFF (zero-width no-break space), plus the
  unassigned gaps inside those blocks (U+FB37, U+FB3D, U+FB3F, U+FB42, U+FB45,
  U+FBC3–U+FBD2, U+FD90–U+FD91, U+FDC8–U+FDCE, U+FDD0–U+FDEF, U+FE75,
  U+FEFD–U+FEFE). Expanding `U+FDFD` needs a curated
  phrase table (data + licence question we have not answered), and *deleting*
  a word would lose meaning — so we do neither and pin the behaviour by test.
  That is also why `PresentationForms::stripped_ranges()` is **empty**: we do
  not claim what we cannot keep.
* **U+0649 (alef maksura) and U+06D2 (yeh barree) are not folded** into the
  yeh class — they have distinct final shapes. Note that rule 1's decompositions
  do produce U+0649 (e.g. `U+FDFA` → `صلى الله عليه وسلم`), so the two
  spellings of that word do *not* currently match. Open question below.
* **Arabic marks outside U+064B–U+065F/U+0670 are left alone**: the
  Quranic annotation block U+0610–U+061A and U+06D6–U+06ED, and the FBB2…
  symbols above. Correct for our corpora (they do not appear), wrong for a
  Quran corpus — widen the range if pdfrtl ever takes one on.
* **Bidi and format controls are untouched**: U+200E/U+200F, U+202A–U+202E,
  U+FEFF. They are rare in extracted PDF text and stripping them is a separate,
  separately-testable rule.
* **Hebrew punctuation other than U+05F3/U+05F4 is untouched** (quotation
  marks, maqaf U+05BE), and Latin `"` (U+0022) does *not* match gershayim —
  see open questions.
* **The guarantees depend on the Unicode tables** of `unicode-normalization`
  0.1.25 (already a pdfrtl dependency; no new dependency was added). If a table
  update ever breaks a `stripped_ranges()` claim, the property tests fail the
  build rather than silently changing search behaviour.

## Open questions (honest list)

1. **Gershayim → ASCII apostrophe (rule 8).** A judgement call, pinned by
   test. Alternatives considered: drop the marks entirely (loses the
   abbreviation boundary), or fold to `"` (U+0022, the shifted key). We chose
   U+0027 because a Latin-layout typist produces it without Shift and ASCII is
   never rewritten by any later rule. Cost: a user who types `"` still misses.
   Worth revisiting together with rule "no Latin folding" above.
2. **Madda/hamza on the alef collapse to bare alef (rule 4).** In Persian,
   `آب` (with madda) and `اب` are different words to a reader; they now share a
   key. Accepted for recall, but if pdfrtl ever ships a "strict" search mode,
   alef variants should be split out of this rule.
3. **`ة` → `ه` (rule 4)** merges `مدرسة` and `مدرسه`. Standard practice for
   Arabic search; confirm nobody expects the reverse precision.
4. **U+0649 / U+06D2** — fold them into U+06CC or not? Affects words produced
   by rule 1's decompositions (see limits). Needs a corpus decision, not a
   Unicode fact, which is why it is not implemented.
5. **Phrase-table ligatures (U+FDFD etc.)** — expand via a curated table?
   Requires data we can ship under the project's licence policy.
6. **U+05C3 (sof pasuq) and U+05C6 (nof hafakh)** are inside the stripped
   Hebrew range though they are punctuation, not vowels. Harmless in running
   text, wrong for a critically-edited biblical corpus. If that corpus ever
   matters, split them out of rule 7.

## Verification

`crates/pdfrtl-core/tests/search_normalize.rs` covers: each rule in isolation,
the motivating equivalences (ZWNJ, yeh/kaf, digits, lam-alef both spellings,
Hebrew niqqud), idempotence per step and for the pipeline, the
`stripped_ranges()` claims rule by rule, every claimed codepoint fed through
the pipeline on its own, and a sweep over **every Unicode scalar value**
asserting no panic and no leaked codepoint. Gate: `bash scripts/wsl-build.sh`
(fmt, clippy `-D warnings`, `cargo test --workspace`, `cargo deny check`) run
in WSL Ubuntu-26.04.
