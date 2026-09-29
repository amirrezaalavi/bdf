# 0008 — A measurement must not move what it measures

Status: fixed in `2936662` (W1.7b, pen-advance positioning).

## What it looks like

A real page comes back **one glyph per line**:

```
Created with DIALux      ->      C
                                 r
                                 e
                                 a
                                 t
                                 e
                                 d
```

The text is not reversed, not dropped, not refused — it is shredded, and it is shredded
in LTR Latin, on a page whose RTL content was never in question.

## Mechanism

rung 3 needs a measured painting, so W1.7b gave each unit a position taken from the pen.
The first implementation advanced the **text-line matrix** per glyph, exactly as a
conforming viewer does, and read the pen out of it:

```rust
self.line = self.line.translated(tx, 0.0);   // "move the pen by tx"
```

`translated(tx, 0)` is not "move x". It is a matrix composition, so it also moves the
matrix's `f` (the y) whenever the text matrix is **skewed** (`c ≠ 0`) — which real
producers emit for rotated and slanted labels.

The assembler groups units into lines by `Unit.line = (line.f * 1000).round()`. Every
advance therefore produced a **new line key**, and the assembler did what its contract
says: one unit per line.

## Why review and the gate both missed it

The gate was GREEN. The unit tests, the probe fixture and the invariant control all use
**synthetic** pages whose text matrices are axis-aligned (`c = 0`), where the shift is
exactly zero and the defect is invisible. It took the 51-file private archive — real
producers, skewed matrices — to expose it.

What found it was not review either: it was a per-file diff of
`reports/validate-archive.json` against its baseline. The summary numbers alone looked
like a **pure win** at first (+6,852 emitted / +8,385 withheld): the file-level
categories were unchanged, so the damage was invisible at that altitude.

## Fix

Keep the pen in text space and project it once, where the unit is recorded — never in the
shared matrix the rest of the pipeline reads:

```rust
let origin_x = self.ctm.a * self.line.e + self.ctm.c * self.line.f + self.ctm.e;
let paint_x  = origin_x + self.ctm.a * self.pen;
```

`Walker.pen` is reset by every positioning operator (`BT`, `Tm`, `Td`, `TD`, `T*`), so it
is exactly the advance since the run began. `line`, the line key and the assembler's view
of the page are bit-for-bit what they were before widths existed.

## Consequences

- **A new measurement goes beside what it measures, not through it.** Composing the pen
  into the text-line matrix was *more* conforming and still wrong here, because a second
  consumer (line grouping) had built its contract on the old meaning of the same state.
  Shared state is read by layers you are not thinking about.
- **`Unit.x` had no reader at all.** Once rung 3 moved to `paint_x`, clippy's `dead_code`
  reported the old field as never read: the assembler groups on the line key and never
  ordered units by x. "The assembler reads x" was an assumption, and it was false — worth
  knowing before designing around it.
- **Summary numbers cannot see a per-file regression.** `fully_decoded` / `order_verified`
  / `refused` all held steady while a real page was being shredded. The comparison that
  matters is per file, and then per reason.
- **A reversed-word frequency count is not an ordering check.** Words like
  `ثالث`/`ثلاث` or `همان`/`نامه` are real words in both directions, so the metric fires on
  the known-good control files at the same rate as on the changed ones. The decisive
  evidence stays what it was: a construction-verified fixture, plus the named reason each
  page is allowed to emit under.

## Evidence

| state | arabic-1 head | arabic-4 head | chars emitted |
|---|---|---|---|
| baseline (`2af0bb4`) | `املبادئ التوجيهيّة ملنظّمة` | correct | 1,035,606 |
| widths in the line matrix | correct | `\n \nم\nا\nع\nل\nل…` shredded | 1,042,458 |
| pen in text space (`2936662`) | `المبادئ التوجيهيّة لمنظّمة` | correct | 1,038,880 |

Archive after the fix: `fully_decoded 38 · order_verified 37 · order_verified_rtl 14 ·
refused 12 · partial 12 · no_text 4` — unchanged from baseline, `rtl_text_files 14 → 15`,
`chars_withheld 1,275,183 → 1,271,919`, i.e. 3,274 characters now emitted with a named
reason instead of withheld.
