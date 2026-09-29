# FINDINGS — SPIKE: does `lopdf` survive load → modify → save without destroying marked content?

**Repo:** `pdfrtl` · **spike dir:** `spikes/lopdf-marked-content/` (standalone crate, its own
empty `[workspace]`, never a member of the root workspace) · **crate under test:** `lopdf`
**0.45.0** (the exact version pinned in the root `Cargo.lock`) · **toolchain:** rustc 1.98.1.

**Question (W5 / Q-R6 local half):** is the editor an *incremental update* of the original
file (append-only, everything unknown preserved byte-for-byte) or a *full parse → model →
re-emit* rewrite? The decisive unknown was whether `lopdf` round-trips marked content
(`BDC`/`EMC`/`BMC`, per-glyph `/ActualText`, `/ReversedChars`) and objects/streams it does not
understand. Everything below was measured, not read off documentation.

## Reproduce (one command, from WSL)

```bash
wsl.exe -d Ubuntu-26.04 -- bash /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/run.sh
```

It: generates the adversarial inputs with `qpdf --static-id`, runs the spike
(`cargo run --release` from this directory), runs the whole spike a **second** time and
diffs every artefact's sha256 (AGENTS rule 4), then runs the oracles (`qpdf --check` on
every produced file, `qpdf --show-object` structural comparison original-vs-round-trip,
`pdftotext` identity). Everything is written to
`spikes/lopdf-marked-content/out/report.txt` (1838 lines, complete raw output); the blocks
below are pasted from it verbatim.

To check reproducibility *across* invocations (two separate `run.sh` executions, with
guards that refuse to conclude if the report cannot be read):

```bash
wsl.exe -d Ubuntu-26.04 -- bash /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/check-invocations.sh
```

Every claim below that a `BDC`/`EMC` pairing or an `/ActualText` payload survived is backed
by a raw operator dump or a raw diff line pasted in the section for that experiment — no
claim is made from a screenshot or a paraphrase.

**Oracles used:** `qpdf 12.3.2` (WSL), `pdftotext` = **poppler 26.01.0** (WSL; the MSYS copy
on this host is Xpdf 4.00 and was *not* used for these numbers).

## Method in one paragraph

For every input the spike takes a **structural snapshot**: every object's number, type,
`/Filter`, `/Length` shape (direct vs indirect), the sha of the stream's raw bytes *and* of
its decompressed bytes; a raw-file scan that gives every object its byte region (so
"byte-identical" is a byte comparison, not an impression); and a **tokenizer-based** analysis
of each decompressed page content stream — `BDC`/`BMC`/`EMC` counted as *tokens* (never as
substrings, so a string containing the letters cannot inflate the count), every `/ActualText`
payload with its exact byte offset and decoded bytes, every text-showing payload, and a
nesting stack that proves pairing (an `EMC` with an empty stack, or an unclosed `BDC`, fails
the check). It then saves, re-snapshots, and diffs the two snapshots. Files are additionally
checked by `qpdf --check` (syntax/stream oracle) and `pdftotext` (text oracle).

## Baseline: what is in the fixtures

```
# chrome fixture, content stream 6, decompressed (before any save)
  content stream 6: BDC=22 BMC=1 EMC=23 AT=20 shows=54 balanced=true max_depth=3 filter=FlateDecode length=direct(497) plain_len=2172
  (20 /ActualText payloads, 21 /Span + 2 /NonStruct BDC + 1 /ReversedChars BMC, all 23 EMC matched)
```

```
# actualtext-fa.pdf — line 1 = before, line 2 = after save (identical)
  content stream 4: BDC=2 BMC=0 EMC=2 AT=2 shows=2 balanced=true max_depth=1 filter=<none> length=direct(198) plain_len=198
  content stream 4: BDC=2 BMC=0 EMC=2 AT=2 shows=2 balanced=true max_depth=1 filter=<none> length=direct(198) plain_len=198
# minimal-ltr.pdf — line 1 = before, line 2 = after save (identical, no marked content at all)
  content stream 4: BDC=0 BMC=0 EMC=0 AT=0 shows=1 balanced=true max_depth=0 filter=<none> length=direct(42) plain_len=42
  content stream 4: BDC=0 BMC=0 EMC=0 AT=0 shows=1 balanced=true max_depth=0 filter=<none> length=direct(42) plain_len=42
```

## E1 — unmodified round-trip: `Document::load` → `Document::save`

```
# E1 diff, fa-zwnj-lamalef.pdf (27282 B -> 27065 B)
--- diff before -> after-save ---
file: len 27282 -> 27065 (-217), fnv e087b426387612e9 -> 90dd0bdb9f38dc74, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=25 new=25 added=[] removed=[] type_changed=[]
raw object regions: identical=1 changed=24 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
    obj 1: base[15..303] fnv=462b305473845e07 -> new[15..290] fnv=1aeff97c5b8c6dfd (delta -13 bytes)
    obj 2: base[907..1146] fnv=6500a67b011ba7cf -> new[290..506] fnv=1a70d8b38e9024e6 (delta -23 bytes)
    obj 3: base[303..340] fnv=70a0cc295872a709 -> new[506..541] fnv=00c4023d1f9fc453 (delta -2 bytes)
    obj 4: base[15292..15430] fnv=550578371ba97f34 -> new[541..669] fnv=ec5d342a016b4524 (delta -10 bytes)
    obj 5: base[26538..26684] fnv=36aa126dbe0d1523 -> new[669..805] fnv=d0dddbd21c83c313 (delta -10 bytes)
    obj 6: base[340..907] fnv=d526fde27c314b79 -> new[805..1370] fnv=f71ef1c0987909d1 (delta -2 bytes)
    obj 7: base[1146..1201] fnv=a00408f1b076adce -> new[1370..1421] fnv=865a3ce41b4da9cc (delta -4 bytes)
    obj 8: base[1760..1850] fnv=5d23654295747c02 -> new[1421..1507] fnv=b927441ddf9aa52c (delta -4 bytes)
    obj 9: base[1590..1673] fnv=a423dda965d02cd5 -> new[1507..1583] fnv=4b90057eb4b15125 (delta -7 bytes)
    obj 10: base[1499..1590] fnv=d2e5a19b84d046db -> new[1583..1666] fnv=2e1f206c131d4dc3 (delta -8 bytes)
    obj 11: base[1278..1350] fnv=b75bf49f90d1bc3c -> new[1666..1733] fnv=0d4fae6b437d011c (delta -5 bytes)
    obj 12: base[1201..1278] fnv=e6c6a2fe23b51657 -> new[1733..1804] fnv=2b010bcaf3b8b6e3 (delta -6 bytes)
content 6: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# what a "changed raw region" actually is (raw bytes of object 1, before vs after)
raw region of stream-less object 1:
  BEFORE: 1 0 obj\n<</Title (pdfrtl producer fixture)\n/Creator (Mozilla/5.0 \(Windows NT 10.0; Win64; x64\) AppleWebKit/537.36 \(KHTML, like Gecko\) HeadlessChrome/154.0.0.0 Safari/537.36)\n/Producer (Skia/PDF m154)\n/CreationDate (D:20260927132520+00'00')\n/ModDate (D:20260927132520+00'00')>>\nendobj\n
  AFTER : 1 0 obj\n<</Title(pdfrtl producer fixture)/Creator(Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/154.0.0.0 Safari/537.36)/Producer(Skia/PDF m154)/CreationDate(D:20260927132520+00'00')/ModDate(D:20260927132520+00'00')>>\nendobj\n
```

```
# E1 diff, actualtext-fa.pdf (882 B -> 837 B)
--- diff before -> after-save ---
file: len 882 -> 837 (-45), fnv 42b23b51b167467c -> 2f3fe44cdd49c16b, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=6 new=6 added=[] removed=[] type_changed=[]
raw object regions: identical=0 changed=6 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..247] fnv=09a128f83f48edc3 -> new[111..223] fnv=1745d68585063c49 (delta -14 bytes)
    obj 4: base[247..496] fnv=36de640af5f53437 -> new[223..470] fnv=63452d15a7fc9b37 (delta -2 bytes)
    obj 5: base[496..566] fnv=9eb04bae65694554 -> new[470..533] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[566..667] fnv=95d844f4cf67387e -> new[533..627] fnv=5a35c83c2b8218cc (delta -7 bytes)
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# E1 diff, minimal-ltr.pdf (689 B -> 646 B)
--- diff before -> after-save ---
file: len 689 -> 646 (-43), fnv 02f9848b7176ec8d -> d28303394f839545, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=6 new=6 added=[] removed=[] type_changed=[]
raw object regions: identical=0 changed=6 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..246] fnv=b9b7fc3a72df1041 -> new[111..222] fnv=295472dbe6085deb (delta -14 bytes)
    obj 4: base[246..338] fnv=5335361e4d6cea39 -> new[222..312] fnv=a116cbe9418f1009 (delta -2 bytes)
    obj 5: base[338..408] fnv=9eb04bae65694554 -> new[312..375] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[408..474] fnv=94e71f8285e1eda0 -> new[375..436] fnv=22449968051a0adc (delta -5 bytes)
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

## E2 — `save()` vs `save_to()` vs saving twice

```
# E2
== E2: save() vs save_to() vs save-twice byte equality
==============================================================================
save() == save_to() (same doc state): true
save() twice on the same Document: identical=true (27065 vs 27065 bytes)

==============================================================================
```

## E3 — structured edit: decode ops → replace ONE `Tj` payload → save

```
# E3 diff, fa-zwnj-lamalef.pdf
--- diff before -> after-edit ---
file: len 27282 -> 27058 (-224), fnv e087b426387612e9 -> 8491637641ae9583, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=25 new=25 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 6: length direct(497)->direct(490); raw 497->490b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=1 changed=24 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
    obj 1: base[15..303] fnv=462b305473845e07 -> new[15..290] fnv=1aeff97c5b8c6dfd (delta -13 bytes)
    obj 2: base[907..1146] fnv=6500a67b011ba7cf -> new[290..506] fnv=1a70d8b38e9024e6 (delta -23 bytes)
    obj 3: base[303..340] fnv=70a0cc295872a709 -> new[506..541] fnv=00c4023d1f9fc453 (delta -2 bytes)
    obj 4: base[15292..15430] fnv=550578371ba97f34 -> new[541..669] fnv=ec5d342a016b4524 (delta -10 bytes)
    obj 5: base[26538..26684] fnv=36aa126dbe0d1523 -> new[669..805] fnv=d0dddbd21c83c313 (delta -10 bytes)
    obj 6: base[340..907] fnv=d526fde27c314b79 -> new[805..1363] fnv=251d98b94a69e185 (delta -9 bytes)
    obj 7: base[1146..1201] fnv=a00408f1b076adce -> new[1363..1414] fnv=865a3ce41b4da9cc (delta -4 bytes)
    obj 8: base[1760..1850] fnv=5d23654295747c02 -> new[1414..1500] fnv=b927441ddf9aa52c (delta -4 bytes)
    obj 9: base[1590..1673] fnv=a423dda965d02cd5 -> new[1500..1576] fnv=4b90057eb4b15125 (delta -7 bytes)
    obj 10: base[1499..1590] fnv=d2e5a19b84d046db -> new[1576..1659] fnv=2e1f206c131d4dc3 (delta -8 bytes)
    obj 11: base[1278..1350] fnv=b75bf49f90d1bc3c -> new[1659..1726] fnv=0d4fae6b437d011c (delta -5 bytes)
    obj 12: base[1201..1278] fnv=e6c6a2fe23b51657 -> new[1726..1797] fnv=2b010bcaf3b8b6e3 (delta -6 bytes)
content 6: ActualText offsets changed: [187, 283, 334, 395, 460, 511, 640, 713, 782, 833, 898, 971, 1022, 1073, 1124, 1175, 1226, 1277, 1343, 1394] -> [174, 275, 325, 385, 449, 499, 627, 699, 767, 817, 881, 953, 1003, 1053, 1103, 1153, 1203, 1253, 1318, 1368]; text-show: VALUES changed at 1 of 54 indices: [0]; stream bytes recompressed/rewritten 497->490b
--- end diff ---
```

```
# E3 diff, actualtext-fa.pdf
--- diff before -> after-edit ---
file: len 882 -> 795 (-87), fnv 42b23b51b167467c -> 579e3091a5e4465a, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=6 new=6 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 4: filter None->Some("FlateDecode"); length direct(198)->direct(137); raw 198->137b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=0 changed=6 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..247] fnv=09a128f83f48edc3 -> new[111..223] fnv=1745d68585063c49 (delta -14 bytes)
    obj 4: base[247..496] fnv=36de640af5f53437 -> new[223..428] fnv=d80a33aa025cec11 (delta -44 bytes)
    obj 5: base[496..566] fnv=9eb04bae65694554 -> new[428..491] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[566..667] fnv=95d844f4cf67387e -> new[491..585] fnv=5a35c83c2b8218cc (delta -7 bytes)
content 4: ActualText offsets changed: [43, 142] -> [41, 133]; text-show: VALUES changed at 1 of 2 indices: [0]; stream bytes recompressed/rewritten 198->137b
--- end diff ---
```

```
# operator dump of the edited file (actualtext-fa): payloads + pairing trace
  content stream 4 [after-edit] BDC=2 BMC=0 EMC=2 balanced=true:
    ActualText payloads (offset, enc, decoded hex, utf16be text):
      [00] @41     hex feff06330644062706450020062f064606cc0627 "سلام دنیا"
      [01] @133    hex feff064506cc200c063106480645 "می‌روم"
    marked-content pairing trace (tag open@off -> close@off):
      [00] Span           @86 -> @101
      [01] Span           @166 -> @182
    text-showing payloads (offset..end, hex):
      @90..97 <5350494b45>
      @170..178 <2e2e2e2e2e2e>

```

## E3b — insert a brand-new marked-content run (`/Span` + `/ActualText` `BDC` … `EMC`)

```
# E3b diff, fa-zwnj-lamalef.pdf
--- diff before -> after-insert ---
file: len 27282 -> 27079 (-203), fnv e087b426387612e9 -> ff1369c63bf9acc1, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=25 new=25 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 6: length direct(497)->direct(511); raw 497->511b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=1 changed=24 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
    obj 1: base[15..303] fnv=462b305473845e07 -> new[15..290] fnv=1aeff97c5b8c6dfd (delta -13 bytes)
    obj 2: base[907..1146] fnv=6500a67b011ba7cf -> new[290..506] fnv=1a70d8b38e9024e6 (delta -23 bytes)
    obj 3: base[303..340] fnv=70a0cc295872a709 -> new[506..541] fnv=00c4023d1f9fc453 (delta -2 bytes)
    obj 4: base[15292..15430] fnv=550578371ba97f34 -> new[541..669] fnv=ec5d342a016b4524 (delta -10 bytes)
    obj 5: base[26538..26684] fnv=36aa126dbe0d1523 -> new[669..805] fnv=d0dddbd21c83c313 (delta -10 bytes)
    obj 6: base[340..907] fnv=d526fde27c314b79 -> new[805..1384] fnv=efb66d8009732032 (delta 12 bytes)
    obj 7: base[1146..1201] fnv=a00408f1b076adce -> new[1384..1435] fnv=865a3ce41b4da9cc (delta -4 bytes)
    obj 8: base[1760..1850] fnv=5d23654295747c02 -> new[1435..1521] fnv=b927441ddf9aa52c (delta -4 bytes)
    obj 9: base[1590..1673] fnv=a423dda965d02cd5 -> new[1521..1597] fnv=4b90057eb4b15125 (delta -7 bytes)
    obj 10: base[1499..1590] fnv=d2e5a19b84d046db -> new[1597..1680] fnv=2e1f206c131d4dc3 (delta -8 bytes)
    obj 11: base[1278..1350] fnv=b75bf49f90d1bc3c -> new[1680..1747] fnv=0d4fae6b437d011c (delta -5 bytes)
    obj 12: base[1201..1278] fnv=e6c6a2fe23b51657 -> new[1747..1818] fnv=2b010bcaf3b8b6e3 (delta -6 bytes)
content 6: BDC 22->23; EMC 23->24; ActualText: count 20 -> 21; base values still present: 20/20, missing: 0, new: 1; stream bytes recompressed/rewritten 497->511b
--- end diff ---
```

## E4 — raw byte splice on the decompressed content stream (no op decoding)

```
# E4a: length-preserving payload swap — splice point taken from the tokenizer's own offsets, plus diff
== E4a: raw byte splice, length-preserving (replace one hex Tj payload)
==============================================================================
--- snapshot: before ---
file: len=27282 fnv=e087b426387612e9 header=PDF-1.4 xref(raw)=classic-table xref(lopdf)=CrossReferenceTable objstm_in_bytes=false trailer_keys=[Size,Root,Info]
objects: 25 (in object streams: [])
  obj 1: Dictionary
  obj 2: Dictionary
  obj 3: Dictionary
  obj 4: Dictionary
  obj 5: Dictionary
...
--- diff before -> after-splice ---
file: len 27282 -> 27063 (-219), fnv e087b426387612e9 -> 4fa687c0ab1800a9, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=25 new=25 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 6: length direct(497)->direct(495); raw 497->495b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=1 changed=24 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
    obj 1: base[15..303] fnv=462b305473845e07 -> new[15..290] fnv=1aeff97c5b8c6dfd (delta -13 bytes)
    obj 2: base[907..1146] fnv=6500a67b011ba7cf -> new[290..506] fnv=1a70d8b38e9024e6 (delta -23 bytes)
    obj 3: base[303..340] fnv=70a0cc295872a709 -> new[506..541] fnv=00c4023d1f9fc453 (delta -2 bytes)
    obj 4: base[15292..15430] fnv=550578371ba97f34 -> new[541..669] fnv=ec5d342a016b4524 (delta -10 bytes)
    obj 5: base[26538..26684] fnv=36aa126dbe0d1523 -> new[669..805] fnv=d0dddbd21c83c313 (delta -10 bytes)
    obj 6: base[340..907] fnv=d526fde27c314b79 -> new[805..1368] fnv=126f2af8accb87df (delta -4 bytes)
    obj 7: base[1146..1201] fnv=a00408f1b076adce -> new[1368..1419] fnv=865a3ce41b4da9cc (delta -4 bytes)
    obj 8: base[1760..1850] fnv=5d23654295747c02 -> new[1419..1505] fnv=b927441ddf9aa52c (delta -4 bytes)
    obj 9: base[1590..1673] fnv=a423dda965d02cd5 -> new[1505..1581] fnv=4b90057eb4b15125 (delta -7 bytes)
    obj 10: base[1499..1590] fnv=d2e5a19b84d046db -> new[1581..1664] fnv=2e1f206c131d4dc3 (delta -8 bytes)
    obj 11: base[1278..1350] fnv=b75bf49f90d1bc3c -> new[1664..1731] fnv=0d4fae6b437d011c (delta -5 bytes)
    obj 12: base[1201..1278] fnv=e6c6a2fe23b51657 -> new[1731..1802] fnv=2b010bcaf3b8b6e3 (delta -6 bytes)
content 6: text-show: VALUES changed at 1 of 54 indices: [0]; stream bytes recompressed/rewritten 497->495b
--- end diff ---
```

```
# E4b: length-changing splice — a new /Span BDC...EMC nested around a Tj (after = what lands in the file)
== E4b: raw byte splice, length-changing (nest a NEW /Span BDC...EMC around a Tj)
==============================================================================
target: text-show at 254..260 = <030a> (in /Span run: false)
payload ends at 260, following operator Tj at 261..263
before: ...xt <FEFF06F1> >> BDC\n/F4 24 Tf\n1 0 0 -1 258.59375 40 Tm\n<02FA> Tj\nEMC\n[HERE]<030A> Tj\n/Span<</ActualText <FEFF06F0> >> BDC\n<02F9> Tj\nEMC\n/Span<</A...
after:  ...<FEFF06F1> >> BDC\n/F4 24 Tf\n1 0 0 -1 258.59375 40 Tm\n<02FA> Tj\nEMC\n/Span << /ActualText <feff06270644> >> BDC [HERE]<030A> Tj EMC\n/Span<</ActualText <FEFF06F0> >> BDC\n<02F9> Tj\nEMC\n/Span<</ActualText <FEFF06F3> >> BDC\n<02FC> T...
--- snapshot: after-wrap ---
file: len=27083 fnv=97083d6e57a6963e header=PDF-1.4 xref(raw)=classic-table xref(lopdf)=CrossReferenceTable objstm_in_bytes=false trailer_keys=[Size,Root,Info]
objects: 25 (in object streams: [])
  obj 1: Dictionary
  obj 2: Dictionary
  obj 3: Dictionary
  obj 4: Dictionary
  obj 5: Dictionary
...
--- diff before -> after-wrap ---
file: len 27282 -> 27083 (-199), fnv e087b426387612e9 -> 97083d6e57a6963e, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=25 new=25 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 6: length direct(497)->direct(515); raw 497->515b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=1 changed=24 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]
    obj 1: base[15..303] fnv=462b305473845e07 -> new[15..290] fnv=1aeff97c5b8c6dfd (delta -13 bytes)
    obj 2: base[907..1146] fnv=6500a67b011ba7cf -> new[290..506] fnv=1a70d8b38e9024e6 (delta -23 bytes)
    obj 3: base[303..340] fnv=70a0cc295872a709 -> new[506..541] fnv=00c4023d1f9fc453 (delta -2 bytes)
    obj 4: base[15292..15430] fnv=550578371ba97f34 -> new[541..669] fnv=ec5d342a016b4524 (delta -10 bytes)
    obj 5: base[26538..26684] fnv=36aa126dbe0d1523 -> new[669..805] fnv=d0dddbd21c83c313 (delta -10 bytes)
    obj 6: base[340..907] fnv=d526fde27c314b79 -> new[805..1388] fnv=715cd1840bd6bd33 (delta 16 bytes)
    obj 7: base[1146..1201] fnv=a00408f1b076adce -> new[1388..1439] fnv=865a3ce41b4da9cc (delta -4 bytes)
    obj 8: base[1760..1850] fnv=5d23654295747c02 -> new[1439..1525] fnv=b927441ddf9aa52c (delta -4 bytes)
    obj 9: base[1590..1673] fnv=a423dda965d02cd5 -> new[1525..1601] fnv=4b90057eb4b15125 (delta -7 bytes)
    obj 10: base[1499..1590] fnv=d2e5a19b84d046db -> new[1601..1684] fnv=2e1f206c131d4dc3 (delta -8 bytes)
    obj 11: base[1278..1350] fnv=b75bf49f90d1bc3c -> new[1684..1751] fnv=0d4fae6b437d011c (delta -5 bytes)
    obj 12: base[1201..1278] fnv=e6c6a2fe23b51657 -> new[1751..1822] fnv=2b010bcaf3b8b6e3 (delta -6 bytes)
content 6: BDC 22->23; EMC 23->24; ActualText: count 20 -> 21; base values still present: 20/20, missing: 0, new: 1; stream bytes recompressed/rewritten 497->515b
--- end diff ---
```

## E5 — the adversarial path: poke `Stream::content` directly, leave `/Length` + `/Filter` stale

```
# E5: what was written and what lopdf makes of it on reload
== E5: HAZARD — mutate Stream::content directly (indirect /Length left stale)
==============================================================================
content stream object: (6, 0)
before: /Length = 497, /Filter = "/FlateDecode", content 497 bytes
after: content 2218 bytes, dict untouched
saved /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf
on disk: ...bj\n6 0 obj\n<</Filter/FlateDecode/Length 497>>stream\n.23999999 0 0 -.23999999 0 8
lopdf reload: dict /Length = direct(2218), in-memory raw content = 2218 bytes
lopdf decompressed: 490 bytes, first 60: "4ɓcccPQPQPf^YiccccccPQPSUa^caccSPh�dccPQPQPf^YiccccccPQPSUa"
--- snapshot: after-stale-edit ---
file: len=28786 fnv=1a180c048f215fb1 header=PDF-1.4 xref(raw)=classic-table xref(lopdf)=CrossReferenceTable objstm_in_bytes=false trailer_keys=[Size,Root,Info]
  content stream 6: BDC=0 BMC=0 EMC=0 AT=0 shows=0 balanced=true max_depth=0 filter=FlateDecode length=direct(2218) plain_len=490
=> lopdf reloaded the broken file; see qpdf --check output for the oracle verdict
=> control (change_page_content) reload: OK, /Length now direct(535)
```

That file is the **only** artefact in this spike that fails its oracle. Raw output of a
direct invocation (report.txt keeps only the `ERROR`/`WARNING`/status lines of this, so the
full transcript with the exit code is pasted here):

```
$ qpdf --check out/e5-stale-length.pdf ; echo "qpdf exit code: $?"
checking .../spikes/lopdf-marked-content/out/e5-stale-length.pdf
PDF Version: 1.4
File is not encrypted
File is not linearized
WARNING: .../e5-stale-length.pdf (object 6 0, offset 1351): expected endstream
WARNING: .../e5-stale-length.pdf (object 6 0, offset 854): attempting to recover stream length
WARNING: .../e5-stale-length.pdf (object 6 0, offset 854): recovered stream length: 2219
WARNING: .../e5-stale-length.pdf (offset 854): error decoding stream data for object 6 0: stream inflate: inflate: data: incorrect header check
WARNING: .../e5-stale-length.pdf (offset 854): stream will be re-processed without filtering to avoid data loss
WARNING: .../e5-stale-length.pdf (offset 854): error decoding stream data for object 6 0: stream inflate: inflate: data: incorrect header check
ERROR: page 1: content stream (content stream object 6 0): errors while decoding content stream
qpdf: errors detected
qpdf exit code: 2
```

(The `...` above elide only the absolute directory prefix, which the report's own
`qpdf --check` section shows in full for every file.)

`qpdf` first warns `expected endstream`, recovers a length of 2219, then cannot inflate.
lopdf itself reloads the same file *without complaint* and yields 490 bytes of garbage where
the content stream should be 2218 bytes — i.e. the marked content reports `BDC=0 EMC=0 AT=0`:
silently destroyed. The control (`Document::change_page_content`, the supported API) produces
a file `qpdf --check` passes with `BDC=22 BMC=1 EMC=23 AT=20 balanced=true`.

## E6 — the incremental path: `IncrementalDocument` (append-only revision)

```
# E6: edit performed in the appended revision
prev objects: 25, new_document objects before cloning: 0
replaced first Tj payload <02fa> with SPIKE
output starts with the original file verbatim (27282 bytes): true
--- snapshot: before ---
file: len=27282 fnv=e087b426387612e9 header=PDF-1.4 xref(raw)=classic-table xref(lopdf)=CrossReferenceTable objstm_in_bytes=false trailer_keys=[Size,Root,Info]
objects: 25 (in object streams: [])
  obj 1: Dictionary
  obj 2: Dictionary
  obj 3: Dictionary
  obj 4: Dictionary
  obj 5: Dictionary
```

```
# E6 diff (base = original file, new = 28209-byte incremental file)
--- diff before -> after-incremental ---
file: len 27282 -> 28209 (+927), fnv e087b426387612e9 -> 786d6d80c06fbee3, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=25 new=25 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 6: length direct(497)->direct(490); raw 497->490b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=22 changed=3 only_base=[] only_new=[]
  changed ids: [2, 5, 6]
    obj 2: base[907..1146] fnv=6500a67b011ba7cf -> new[27282..27498] fnv=1a70d8b38e9024e6 (delta -23 bytes)
    obj 5: base[26538..26684] fnv=36aa126dbe0d1523 -> new[26538..27282] fnv=54c7fc90b87aecfd (delta 598 bytes)
    obj 6: base[340..907] fnv=d526fde27c314b79 -> new[27498..28056] fnv=251d98b94a69e185 (delta -9 bytes)
content 6: ActualText offsets changed: [187, 283, 334, 395, 460, 511, 640, 713, 782, 833, 898, 971, 1022, 1073, 1124, 1175, 1226, 1277, 1343, 1394] -> [174, 275, 325, 385, 449, 499, 627, 699, 767, 817, 881, 953, 1003, 1053, 1103, 1153, 1203, 1253, 1318, 1368]; text-show: VALUES changed at 1 of 54 indices: [0]; stream bytes recompressed/rewritten 497->490b
--- end diff ---
```

Reading back the multi-revision file through `Document::load` returns the *newest* revision
(`content stream 6: BDC=22 BMC=1 EMC=23 AT=20 shows=54 balanced=true`), so our own extraction
path needs no special handling, and `qpdf --check` on `e6-incremental.pdf` is clean.

The three "changed" raw regions are fully explained by the printed ranges:
`obj 2` and `obj 6` are the two objects the edit touched and they now exist for the second
time in the appended section (`new[27282..27498]` — offset 27282 is exactly the original
file length); `obj 5` only *looks* changed because its region now runs to the next marker
(`base[26538..26684] -> new[26538..27282]`, +598 bytes = the original xref/trailer that now
follows it) — the prefix assertion above proves bytes 0..27282 were copied verbatim.

## E7 — documented save options: `save_modern()` and `save_with_options`

```
# E7 diff 1: plain load/save -> save_modern (object streams + xref stream)
--- diff before -> save_modern ---
file: len 882 -> 750 (-132), fnv 42b23b51b167467c -> d364e866d0ce9f6b, identical=false
xref: raw classic-table -> xref-stream, lopdf CrossReferenceTable -> CrossReferenceStream ; objstm_in_bytes false -> true
objects: base=6 new=8 added=[7, 8] removed=[] type_changed=[]
raw object regions: identical=0 changed=1 only_base=[1, 2, 3, 5, 6] only_new=[7, 8]
  changed ids: [4]
    obj 4: base[247..496] fnv=36de640af5f53437 -> new[15..262] fnv=63452d15a7fc9b37 (delta -2 bytes)
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# E7 diff 2: plain save with use_xref_streams only
--- diff before -> xref-stream-only ---
file: len 882 -> 803 (-79), fnv 42b23b51b167467c -> 5ec1bbf831de2246, identical=false
xref: raw classic-table -> xref-stream, lopdf CrossReferenceTable -> CrossReferenceStream ; objstm_in_bytes false -> false
objects: base=6 new=7 added=[7] removed=[] type_changed=[]
raw object regions: identical=0 changed=6 only_base=[] only_new=[7]
  changed ids: [1, 2, 3, 4, 5, 6]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..247] fnv=09a128f83f48edc3 -> new[111..223] fnv=1745d68585063c49 (delta -14 bytes)
    obj 4: base[247..496] fnv=36de640af5f53437 -> new[223..470] fnv=63452d15a7fc9b37 (delta -2 bytes)
    obj 5: base[496..566] fnv=9eb04bae65694554 -> new[470..533] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[566..667] fnv=95d844f4cf67387e -> new[533..627] fnv=5a35c83c2b8218cc (delta -7 bytes)
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# E7 diff 3: save_modern output read back and saved again with a plain save()
--- diff save_modern -> save_modern -> plain save ---
file: len 750 -> 808 (+58), fnv d364e866d0ce9f6b -> e1782439d6ce7adc, identical=false
xref: raw xref-stream -> xref-stream, lopdf CrossReferenceStream -> CrossReferenceStream ; objstm_in_bytes true -> false
objects: base=8 new=7 added=[9] removed=[7, 8] type_changed=[]
raw object regions: identical=1 changed=0 only_base=[7, 8] only_new=[1, 2, 3, 5, 6, 9]
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

## E8 — adversarial input (a): object streams + cross-reference stream (made by qpdf)

```
# E8 input snapshot (objects living in an ObjStm) + round trip via plain save()
--- snapshot: input ---
file: len=847 fnv=0510f767a59866f7 header=PDF-1.7 xref(raw)=xref-stream xref(lopdf)=CrossReferenceStream objstm_in_bytes=true trailer_keys=[Type,Root,Size,ID,Info]
objects: 8 (in object streams: [2, 3, 4, 5, 6])
  obj 1: Stream filter=FlateDecode length=direct(226) raw_len=226 raw_fnv=2c9898aa17fd738a plain=len=358 fnv=f285c80fddeb4369
  obj 2: Dictionary
  obj 3: Dictionary
  obj 4: Dictionary
  obj 5: Dictionary
  obj 6: Dictionary
  obj 7: Stream filter=FlateDecode length=direct(134) raw_len=134 raw_fnv=4a733d36cd2f0c21 plain=len=198 fnv=aef464d3fa9d936a
  obj 8: Stream filter=FlateDecode length=direct(34) raw_len=34 raw_fnv=61d117ab51c64ba3 plain=len=36 fnv=14840dbe2b3e0926
pages: page1=obj4 contents=[7]
...
--- diff input -> after-plain-save ---
file: len 847 -> 836 (-11), fnv 0510f767a59866f7 -> 686ab63d2cec5f77, identical=false
xref: raw xref-stream -> xref-stream, lopdf CrossReferenceStream -> CrossReferenceStream ; objstm_in_bytes true -> false
objects: base=8 new=7 added=[9] removed=[1, 8] type_changed=[]
raw object regions: identical=0 changed=1 only_base=[1, 8] only_new=[2, 3, 4, 5, 6, 9]
  changed ids: [7]
    obj 7: base[341..546] fnv=bada5761a2a05357 -> new[380..582] fnv=854b29837f872eed (delta -3 bytes)
content 7: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# E8: same input written with save_modern (ObjStm rebuilt instead of collapsed)
--- diff input -> after-save_modern ---
file: len 847 -> 1050 (+203), fnv 0510f767a59866f7 -> 59ddbc6ffd82fe51, identical=false
xref: raw xref-stream -> xref-stream, lopdf CrossReferenceStream -> CrossReferenceStream ; objstm_in_bytes true -> true
objects: base=8 new=9 added=[9, 10] removed=[1] type_changed=[]
raw object regions: identical=0 changed=2 only_base=[1] only_new=[9, 10]
  changed ids: [7, 8]
    obj 7: base[341..546] fnv=bada5761a2a05357 -> new[15..217] fnv=854b29837f872eed (delta -3 bytes)
    obj 8: base[546..827] fnv=c11c8b3abe43a33f -> new[217..478] fnv=ac6b943a435eaca3 (delta -20 bytes)
content 7: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

## E9 — adversarial input (b): a stream whose `/Length` is an indirect reference

```
# E9 input (hand-built, qpdf-verified) + unmodified round trip
wrote 792 bytes to /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/in-indirect-length.pdf
--- snapshot: input ---
file: len=792 fnv=faa1ec342e6a7be8 header=PDF-1.4 xref(raw)=classic-table xref(lopdf)=CrossReferenceTable objstm_in_bytes=false trailer_keys=[Size,Root]
objects: 6 (in object streams: [])
  obj 1: Dictionary
  obj 2: Dictionary
  obj 3: Dictionary
  obj 4: Stream filter=<none> length=direct(202) raw_len=202 raw_fnv=21e7783a89ed4e16 plain=len=202 fnv=21e7783a89ed4e16
  obj 5: Dictionary
  obj 6: Integer
pages: page1=obj3 contents=[4]
  content stream 4: BDC=2 BMC=0 EMC=2 AT=2 shows=2 balanced=true max_depth=1 filter=<none> length=direct(202) plain_len=202
...
--- diff input -> after-roundtrip ---
file: len 792 -> 757 (-35), fnv faa1ec342e6a7be8 -> 88db26c153672c82, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=6 new=6 added=[] removed=[] type_changed=[]
raw object regions: identical=0 changed=6 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..247] fnv=f4aee3058193b7a1 -> new[111..223] fnv=02bac5fc2fed46db (delta -14 bytes)
    obj 4: base[247..501] fnv=96b53f13508f38d3 -> new[223..474] fnv=d5e696b81459ca29 (delta -3 bytes)
    obj 5: base[501..571] fnv=9eb04bae65694554 -> new[474..537] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[571..590] fnv=a7459b48ea6e72a8 -> new[537..558] fnv=633cd210f4613818 (delta 2 bytes)
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# E9 edit through the supported API
--- diff input -> after-edit ---
file: len 792 -> 715 (-77), fnv faa1ec342e6a7be8 -> ea0bcf0701421057, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=6 new=6 added=[] removed=[] type_changed=[]
stream meta changes:
  obj 4: filter None->Some("FlateDecode"); length direct(202)->direct(141); raw 202->141b; DECOMPRESSED-DATA-CHANGED
raw object regions: identical=0 changed=6 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..247] fnv=f4aee3058193b7a1 -> new[111..223] fnv=02bac5fc2fed46db (delta -14 bytes)
    obj 4: base[247..501] fnv=96b53f13508f38d3 -> new[223..432] fnv=241aa65d28e6c92f (delta -45 bytes)
    obj 5: base[501..571] fnv=9eb04bae65694554 -> new[432..495] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[571..590] fnv=a7459b48ea6e72a8 -> new[495..516] fnv=633cd210f4613818 (delta 2 bytes)
content 4: ActualText offsets changed: [43, 144] -> [41, 133]; text-show: VALUES changed at 1 of 2 indices: [0]; stream bytes recompressed/rewritten 202->141b
--- end diff ---
```

The reference itself does not survive the model — `lopdf` resolves it while parsing:

```
$ grep -a -n Length out/in-indirect-length.pdf
13:<< /Length 6 0 R >>
$ grep -a -n Length out/e9-roundtrip.pdf
13:<</Length 202>>stream
$ awk '/^6 0 obj/{f=1} f{print} f&&/endobj/{exit}' out/e9-roundtrip.pdf
6 0 obj
 202
endobj
```

so the value is right, the *shape* is normalised to direct, and the old length object is left
behind as an orphan. (`qpdf --check` passes on both input and output; `pdftotext` output is
identical.)

## E10 — a stream lopdf has no decoder for: `/Filter /DCTDecode`

```
# input in-unknown-filter.pdf
  obj 6: Stream filter=DCTDecode length=direct(22) raw_len=22 raw_fnv=6f135c843499c050 plain=ERR Unimplemented("decompression algorithms")
  content stream 4: BDC=1 BMC=0 EMC=1 AT=1 shows=1 balanced=true max_depth=1 filter=<none> length=direct(124) plain_len=124
# after Document::load -> Document::save (e10-roundtrip.pdf) — byte-for-byte the same lines
  obj 6: Stream filter=DCTDecode length=direct(22) raw_len=22 raw_fnv=6f135c843499c050 plain=ERR Unimplemented("decompression algorithms")
  content stream 4: BDC=1 BMC=0 EMC=1 AT=1 shows=1 balanced=true max_depth=1 filter=<none> length=direct(124) plain_len=124
```

```
# E10 diff
--- diff input -> after-roundtrip ---
file: len 945 -> 893 (-52), fnv cc048249d4733b96 -> 589a057a777338e0, identical=false
xref: raw classic-table -> classic-table, lopdf CrossReferenceTable -> CrossReferenceTable ; objstm_in_bytes false -> false
objects: base=7 new=7 added=[] removed=[] type_changed=[]
raw object regions: identical=0 changed=7 only_base=[] only_new=[]
  changed ids: [1, 2, 3, 4, 5, 6, 7]
    obj 1: base[15..64] fnv=c6e35ab7626add7b -> new[15..60] fnv=535022d3d940266f (delta -4 bytes)
    obj 2: base[64..121] fnv=7d80b193394ea062 -> new[60..111] fnv=0c746669ba80cb8a (delta -6 bytes)
    obj 3: base[121..273] fnv=a521a51ec1da3fa3 -> new[111..245] fnv=4d5de16099f1c8f9 (delta -18 bytes)
    obj 4: base[273..449] fnv=b0d65072f21c9969 -> new[245..418] fnv=f0335e6e049fbe7d (delta -3 bytes)
    obj 5: base[449..519] fnv=9eb04bae65694554 -> new[418..481] fnv=d96e8e7c2a7877b2 (delta -7 bytes)
    obj 6: base[519..704] fnv=a7fd083df8240616 -> new[481..653] fnv=9228d40956eed8b8 (delta -13 bytes)
    obj 7: base[704..723] fnv=50ab8201b381b602 -> new[653..674] fnv=a22c854c55a4235c (delta 2 bytes)
content 4: marked content IDENTICAL (counts, AT values+positions, pairing, shows)
--- end diff ---
```

```
# the "raw file region" of object 6 (whole object incl. dict, not the payload):
raw file region of the undecodable image object 6: base fnv=a7fd083df8240616 after fnv=9228d40956eed8b8 identical=false
  -> only the dictionary spelling changed (delta -13 bytes, see the diff above); the
     payload itself is byte-identical: raw_len 22 / raw_fnv 6f135c843499c050 in both
     snapshots. The oracle emits the SAME warning on input and output:
     "WARNING: ... error decoding stream data for object 6 0: JPEG datastream contains no image"
     (my deliberately-fake JPEG) — i.e. lopdf did no damage it could even have done.
```

## Oracle output (appended to `out/report.txt` by `run.sh`)

```
===== ORACLE: tool versions =====
qpdf version 12.3.2
pdftotext version 26.01.0
rustc 1.98.1 (48a229cea 2026-09-01)

===== ORACLE: determinism - second full run vs first (sha256 of out/*.pdf) =====
  IDENTICAL across two runs:
    61f5ab51198316ec80960ff414b661a5e5cf1f8501aa74bc99ca0667d72ebeb2  e1-actualtext-fa.pdf
    44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  e1-fa-zwnj-lamalef.pdf
    9c22dfd9ab299c16d720e333201ebe54683cbfb22f07866c89cfca510457d3f8  e1-minimal-ltr.pdf
    77d14e8f4431ec10ff79844b01eacfc0d417e2b70fd5df3d841e8bdcdd7d1081  e10-roundtrip.pdf
    44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  e2-save-again.pdf
    44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  e2-save.pdf
    44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  e2-save_to.pdf
    86433421bd05052a3150bb29479a69bc68067a762645e1df1b12fa895874c272  e3-actualtext-fa.pdf
    cb915fa23c4e85f1f6e354a3309a27fdaf985674558a7d6db9b0cd14cf80c349  e3-fa-zwnj-lamalef.pdf
    2545c68611cbc68eb8628ef595258f2d96cf1f80626078da069c1c18f6ef0ddc  e3b-insert-marked-run.pdf
    4871fcc129d08a0db537e115e30bc8181988fcd49c561a2b7392fde8a260dc8f  e4a-splice-same-length.pdf
    cb8711840bc847339d882cdc9c0266d144af58d0fde40f1fbf2ca615c9cd1197  e4b-splice-wrap-span.pdf
    25f003a3f01a1e4167d738ec614958a3742dd45d0852e307b3215788b6508496  e5-stale-length.pdf
    52048b98079a8940094bc4bbf6a2da0702e780d5f61430584eac124ffaeaaebb  e5-supported-api.pdf
    627bb520be8a0067d63a4b3e10c73b03eb9ce55569f2a11b649729df6867c226  e6-incremental.pdf
    9a6c1594a1e3f64613703a1c98a4133c5495109f19ba5e83074be22a41a958ba  e7-modern-rt.pdf
    a660289ee7c0c603b3feb1590c6c56f82dc8b6e3e1edc180ea4c21ed1b3229b5  e7-save-modern.pdf
    1df2c30fc1e1223bb097e910afd49e088648a75ceda105fe023303668d972300  e7-xref-stream-only.pdf
    074f5a8c99698742d7b59988b00e0fa73ee725b40c52589bf2591091628924e8  e8-objstm-modern.pdf
    71cd701fdc4a505a9cd82157ab7b09f96c862c3d146c75b5d2122e1dfa21f639  e8-objstm-rt.pdf
    f9d0830716e0e9dbecb95cafbf3f9c897b84bfe6bf0559044305a1f5877da68a  e9-edit.pdf
    15d979cc59cdd6147ab6dcd7c802b9dd9a00708c084188617a54f66c8d13e423  e9-roundtrip.pdf
    832f203f884f43409c29f55e4f582480bb66291c0494e9238c2c53cc14f39ae8  in-indirect-length.pdf
    b7a46178260d706d35104cd65f27b08118d27a0a298117a696a80bbf83e453c6  in-qpdf-objstm.pdf
    27e0e145c96fde2a01aa9f2231dea484230b48dde2f40eacfeb3caddfcf9d0b6  in-unknown-filter.pdf

===== ORACLE: qpdf --check on every produced file =====
--- e1-actualtext-fa.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e1-fa-zwnj-lamalef.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e1-minimal-ltr.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e10-roundtrip.pdf
    File is not encrypted
    File is not linearized
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e10-roundtrip.pdf (offset 612): error decoding stream data for object 6 0: JPEG datastream contains no image
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e10-roundtrip.pdf (offset 612): stream will be re-processed without filtering to avoid data loss
--- e2-save-again.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e2-save.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e2-save_to.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e3-actualtext-fa.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e3-fa-zwnj-lamalef.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e3b-insert-marked-run.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e4a-splice-same-length.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e4b-splice-wrap-span.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e5-stale-length.pdf
    File is not encrypted
    File is not linearized
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf (object 6 0, offset 1351): expected endstream
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf (object 6 0, offset 854): attempting to recover stream length
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf (object 6 0, offset 854): recovered stream length: 2219
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf (offset 854): error decoding stream data for object 6 0: stream inflate: inflate: data: incorrect header check
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf (offset 854): stream will be re-processed without filtering to avoid data loss
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/e5-stale-length.pdf (offset 854): error decoding stream data for object 6 0: stream inflate: inflate: data: incorrect header check
    ERROR: page 1: content stream (content stream object 6 0): errors while decoding content stream
--- e5-supported-api.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e6-incremental.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e7-modern-rt.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e7-save-modern.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e7-xref-stream-only.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e8-objstm-modern.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e8-objstm-rt.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e9-edit.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- e9-roundtrip.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- in-indirect-length.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- in-qpdf-objstm.pdf
    File is not encrypted
    File is not linearized
    No syntax or stream encoding errors found; the file may still contain
--- in-unknown-filter.pdf
    File is not encrypted
    File is not linearized
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/in-unknown-filter.pdf (offset 664): error decoding stream data for object 6 0: JPEG datastream contains no image
    WARNING: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out/in-unknown-filter.pdf (offset 664): stream will be re-processed without filtering to avoid data loss

===== ORACLE: qpdf --show-object structural identity, original vs lopdf round-trip =====
--- fa-zwnj-lamalef.pdf vs e1-fa-zwnj-lamalef.pdf
    obj 1: structurally IDENTICAL (qpdf-normalised)
    obj 2: DIFFERS:
        1c1
        < << /Contents 6 0 R /MediaBox [ 0 0 594.95996 841.91998 ] /Parent 7 0 R /Resources << /ExtGState << /G3 3 0 R >> /Font << /F4 4 0 R /F5 5 0 R >> /ProcSet [ /PDF /Text /ImageB /ImageC /ImageI ] >> /StructParents 0 /Tabs /S /Type /Page >>
        ---
        > << /Contents 6 0 R /MediaBox [ 0 0 594.95996 841.92 ] /Parent 7 0 R /Resources << /ExtGState << /G3 3 0 R >> /Font << /F4 4 0 R /F5 5 0 R >> /ProcSet [ /PDF /Text /ImageB /ImageC /ImageI ] >> /StructParents 0 /Tabs /S /Type /Page >>
    obj 3: structurally IDENTICAL (qpdf-normalised)
    obj 4: structurally IDENTICAL (qpdf-normalised)
    obj 6: structurally IDENTICAL (qpdf-normalised)
    obj 18: structurally IDENTICAL (qpdf-normalised)
    obj 22: structurally IDENTICAL (qpdf-normalised)
--- actualtext-fa.pdf vs e1-actualtext-fa.pdf
    obj 1: structurally IDENTICAL (qpdf-normalised)
    obj 2: structurally IDENTICAL (qpdf-normalised)
    obj 3: structurally IDENTICAL (qpdf-normalised)
    obj 4: structurally IDENTICAL (qpdf-normalised)
    obj 5: structurally IDENTICAL (qpdf-normalised)
    obj 6: structurally IDENTICAL (qpdf-normalised)
    obj 7: structurally IDENTICAL (qpdf-normalised)
--- minimal-ltr.pdf vs e1-minimal-ltr.pdf
    obj 1: structurally IDENTICAL (qpdf-normalised)
    obj 2: structurally IDENTICAL (qpdf-normalised)
    obj 3: structurally IDENTICAL (qpdf-normalised)
    obj 4: structurally IDENTICAL (qpdf-normalised)
    obj 5: structurally IDENTICAL (qpdf-normalised)
    obj 6: structurally IDENTICAL (qpdf-normalised)
    obj 7: structurally IDENTICAL (qpdf-normalised)

===== ORACLE: pdftotext identity, original vs unmodified round-trip =====
  e1-fa-zwnj-lamalef.pdf: extracted text IDENTICAL to original (129 bytes)
  e1-actualtext-fa.pdf: extracted text IDENTICAL to original (46 bytes)
  e1-minimal-ltr.pdf: extracted text IDENTICAL to original (15 bytes)
    71cd701fdc4a505a9cd82157ab7b09f96c862c3d146c75b5d2122e1dfa21f639  e8-objstm-rt.pdf
  e9-roundtrip.pdf: extracted text IDENTICAL to original (46 bytes)

===== sha256 of inputs (fixtures) and outputs (all produced PDFs) =====
7836f6a21e16d9f360b6ade97be81eb16b1127d732602dbe14a329a55834e86d  corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf
0d6ac126911261200bbd011fc795aa11a6b7cab2d088cbf1ec1c6710524438e5  corpus/raw/synthetic/actualtext-fa.pdf
4218c9566d3cc4ebb9d3956b15583b7df2022e1bae2b8a9d1ea932d9b7b6f636  corpus/raw/synthetic/minimal-ltr.pdf
61f5ab51198316ec80960ff414b661a5e5cf1f8501aa74bc99ca0667d72ebeb2  spikes/lopdf-marked-content/out/e1-actualtext-fa.pdf
44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  spikes/lopdf-marked-content/out/e1-fa-zwnj-lamalef.pdf
9c22dfd9ab299c16d720e333201ebe54683cbfb22f07866c89cfca510457d3f8  spikes/lopdf-marked-content/out/e1-minimal-ltr.pdf
77d14e8f4431ec10ff79844b01eacfc0d417e2b70fd5df3d841e8bdcdd7d1081  spikes/lopdf-marked-content/out/e10-roundtrip.pdf
44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  spikes/lopdf-marked-content/out/e2-save-again.pdf
44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  spikes/lopdf-marked-content/out/e2-save.pdf
44d15881bedf30604121f21bb71a805a388b82dae753bb6165a82e1dcc57fd7c  spikes/lopdf-marked-content/out/e2-save_to.pdf
86433421bd05052a3150bb29479a69bc68067a762645e1df1b12fa895874c272  spikes/lopdf-marked-content/out/e3-actualtext-fa.pdf
cb915fa23c4e85f1f6e354a3309a27fdaf985674558a7d6db9b0cd14cf80c349  spikes/lopdf-marked-content/out/e3-fa-zwnj-lamalef.pdf
2545c68611cbc68eb8628ef595258f2d96cf1f80626078da069c1c18f6ef0ddc  spikes/lopdf-marked-content/out/e3b-insert-marked-run.pdf
4871fcc129d08a0db537e115e30bc8181988fcd49c561a2b7392fde8a260dc8f  spikes/lopdf-marked-content/out/e4a-splice-same-length.pdf
cb8711840bc847339d882cdc9c0266d144af58d0fde40f1fbf2ca615c9cd1197  spikes/lopdf-marked-content/out/e4b-splice-wrap-span.pdf
25f003a3f01a1e4167d738ec614958a3742dd45d0852e307b3215788b6508496  spikes/lopdf-marked-content/out/e5-stale-length.pdf
52048b98079a8940094bc4bbf6a2da0702e780d5f61430584eac124ffaeaaebb  spikes/lopdf-marked-content/out/e5-supported-api.pdf
627bb520be8a0067d63a4b3e10c73b03eb9ce55569f2a11b649729df6867c226  spikes/lopdf-marked-content/out/e6-incremental.pdf
9a6c1594a1e3f64613703a1c98a4133c5495109f19ba5e83074be22a41a958ba  spikes/lopdf-marked-content/out/e7-modern-rt.pdf
a660289ee7c0c603b3feb1590c6c56f82dc8b6e3e1edc180ea4c21ed1b3229b5  spikes/lopdf-marked-content/out/e7-save-modern.pdf
1df2c30fc1e1223bb097e910afd49e088648a75ceda105fe023303668d972300  spikes/lopdf-marked-content/out/e7-xref-stream-only.pdf
074f5a8c99698742d7b59988b00e0fa73ee725b40c52589bf2591091628924e8  spikes/lopdf-marked-content/out/e8-objstm-modern.pdf
71cd701fdc4a505a9cd82157ab7b09f96c862c3d146c75b5d2122e1dfa21f639  spikes/lopdf-marked-content/out/e8-objstm-rt.pdf
f9d0830716e0e9dbecb95cafbf3f9c897b84bfe6bf0559044305a1f5877da68a  spikes/lopdf-marked-content/out/e9-edit.pdf
15d979cc59cdd6147ab6dcd7c802b9dd9a00708c084188617a54f66c8d13e423  spikes/lopdf-marked-content/out/e9-roundtrip.pdf
832f203f884f43409c29f55e4f582480bb66291c0494e9238c2c53cc14f39ae8  spikes/lopdf-marked-content/out/in-indirect-length.pdf
b7a46178260d706d35104cd65f27b08118d27a0a298117a696a80bbf83e453c6  spikes/lopdf-marked-content/out/in-qpdf-objstm.pdf
27e0e145c96fde2a01aa9f2231dea484230b48dde2f40eacfeb3caddfcf9d0b6  spikes/lopdf-marked-content/out/in-unknown-filter.pdf
```

## Reproducibility — raw output, including the check that failed

A control that cannot read its input must not report success (the failure mode recorded in
`docs/problems/0006`). My **first** cross-invocation
check passed *vacuously*: the `cd` into the spike directory was swallowed by the
`wsl.exe` quoting, so `sha256sum out/report.txt` never found the file and `cut -d\" -f1`
received a bare quote as an argument (`cut: '"': No such file or directory`), so both
hashes came out empty - and the unguarded `[ "$A" = "$B" ]` then compared two empty
strings and printed success. Raw output of that run, kept as the counter-example:
strings. Raw output of that run, kept as the counter-example:

```
=== invocation A ===
run.sh exit=0
sha256sum: out/report.txt: No such file or directory
cut: '"': No such file or directory
sha256sum: out/report.txt: No such file or directory
cut: '"': No such file or directory
runA=
runB=
STABLE ACROSS INVOCATIONS
1
```

After putting every command in a script (no `wsl.exe` quoting involved) and adding
existence/non-empty/section assertions, the **same** check reported the truth — the report
was *not* yet reproducible across invocations:

```
=== invocation A ===
run.sh exit=0
report[A] size=96845B expected_sections=2/2 sha256=30ff3cb1ab1f598396d7d79d4751a7ee41741814f4828fa7d397e724560a11b7
=== invocation B ===
run.sh exit=0
report[B] size=96845B expected_sections=2/2 sha256=98c60fb77a369f8d02563be4f425701c98bbe681a52dbbb2f5fa76ef33daedc1
A=30ff3cb1ab1f598396d7d79d4751a7ee41741814f4828fa7d397e724560a11b7 B=98c60fb77a369f8d02563be4f425701c98bbe681a52dbbb2f5fa76ef33daedc1
RESULT: UNSTABLE OR UNVERIFIED (A non-empty=yes; B non-empty=yes; equal=no)
SCRIPT_EXIT=1
```

Diffing the two saved reports located the cause (and proved the PDFs themselves were
already stable):

```
=== report sha ===
34a7b30715ee1f6f09360eca301c3422495dde864122d9640efb15221c7bee3a  /tmp/r1.txt
98c60fb77a369f8d02563be4f425701c98bbe681a52dbbb2f5fa76ef33daedc1  /tmp/r2.txt

=== diff of the two reports (first 60 lines) ===
--- /tmp/r1.txt	2026-09-29 11:32:17.733035931 +0330
+++ /tmp/r2.txt	2026-09-29 11:32:20.365059956 +0330
@@ -1,4 +1,4 @@
-    Finished `release` profile [optimized] target(s) in 0.42s
+    Finished `release` profile [optimized] target(s) in 0.04s
      Running `/home/netcon/target-pdfrtl-spike/release/spike-lopdf-marked-content`
 repo root: /mnt/c/Users/netcon/playground/ai/pdfrtl
 output dir: /mnt/c/Users/netcon/playground/ai/pdfrtl/spikes/lopdf-marked-content/out

=== which produced PDFs changed between invocations ===
  (none - every out/*.pdf identical)
```

Two fixes followed: `qpdf --static-id` when generating the objstm input (qpdf's default
`/ID` is random per invocation — `id1.pdf` and `id2.pdf` from two identical `qpdf` calls
hash-ed to `a6dda4c9…` vs `5178490b…`; with `--static-id` both hash to `b7a46178…`), and
`cargo run --quiet` so cargo's elapsed-time line cannot enter the report. Final state,
from the committed `check-invocations.sh` (assertions on, exit code printed):

```
=== invocation A ===
run.sh exit=0
report[A] size=96700B expected_sections=2/2 sha256=0f3ae752c9a970ea94b17f87d91fc732e25286cf4467c436052d0f2d9e48b5da
=== invocation B ===
run.sh exit=0
report[B] size=96700B expected_sections=2/2 sha256=0f3ae752c9a970ea94b17f87d91fc732e25286cf4467c436052d0f2d9e48b5da
A=0f3ae752c9a970ea94b17f87d91fc732e25286cf4467c436052d0f2d9e48b5da B=0f3ae752c9a970ea94b17f87d91fc732e25286cf4467c436052d0f2d9e48b5da
RESULT: STABLE ACROSS INVOCATIONS (both hashes non-empty and equal)

=== run.sh's own inner determinism gate (two runs inside one invocation) ===
  IDENTICAL across two runs:
grep -c "IDENTICAL across two runs": 1
SCRIPT_EXIT=0
```

## Verdict

**Patch — build W5 as an append-only incremental update** (`lopdf::IncrementalDocument::create_from` + `save`, exactly what experiment E6 runs), and keep a full `Document::save` rewrite behind an explicit *compact / save-as* operation rather than making it the per-edit path. Marked content is **not** the discriminator: every full rewrite in this spike (E1, E3, E3b, E4a, E4b, E7, E8, E9, E10) returned identical `BDC`/`BMC`/`EMC` counts, balanced pairing, identical `/ActualText` payloads (only their *file offsets* moved, because lopdf writes tighter whitespace), `qpdf --show-object` structures identical except one number, and byte-identical `pdftotext` output for every unmodified round-trip — lopdf does not lose the parts we read. What discriminates is **bytes**: a plain `Document::save` re-serialises *every* object (24 of 25 in the chrome fixture: dict spacing, escaped parentheses, xref layout; plus `Object::Real(f32)` costing `841.91998 → 841.92`; plus object-stream collapse unless `save_modern` is asked for), whereas `IncrementalDocument::save` copied the original 27282-byte file verbatim as a prefix and appended only the two objects the edit actually touched. The editing contract is "everything unknown preserved byte-for-byte" for material we do not fully model — that contract is met by the incremental mode and *not* by the rewrite mode, so rewrite is a compaction feature, not the editing architecture.

## Table: what survives / what does not / why

| Thing under test | Mode | Survives? | Evidence (experiment) | Why |
|---|---|---|---|---|
| `BDC`/`BMC`/`EMC` counts, nesting, pairing | plain `save()` (unmodified) | **yes** — all 3 fixtures | E1: `marked content IDENTICAL (counts, AT values+positions, pairing, shows)` ×3 | untouched content stream is emitted from its raw bytes; never re-parsed |
| `/ActualText` payloads + their positions | plain `save()` | **yes** — 20/20, 2/2, offsets shift only | E1 diffs (chrome, actualtext-fa) | stream payload byte-identical; file offsets move because whitespace shrinks (`fnv` of payload unchanged) |
| text-showing payloads (`Tj`/`'`/`"`/`TJ`) | plain `save()` | **yes** | E1: `shows=54 … shows=2 … shows=1` before and after | ditto |
| marked content after a *decoded* edit (replace one `Tj`) | `Content` ops → save | **yes**, pairing balanced | E3 ×2: `marked content IDENTICAL (counts, AT values+positions, pairing, shows)`, `qpdf --check` clean | op re-serialisation is lossless for tokens and string/hex payloads |
| marked content after *inserting* a new `/Span` `BDC`…`EMC` run | `Content` ops → save | **yes** | E3b: `content 6: BDC 22->23; EMC 23->24; ActualText: count 20 -> 21; base values still present: 20/20, missing: 0, new: 1`, `balanced=true max_depth=3` | pairing stack verified over the whole stream |
| marked content after a raw byte splice (same length) | decompressed bytes → save | **yes** (only the spliced payload differs) | E4a: `content 6: text-show: VALUES changed at 1 of 54 indices: [0]; stream bytes recompressed/rewritten 497->495b`, `obj 6: … DECOMPRESSED-DATA-CHANGED`, `qpdf --check` clean | splice is byte-exact, `/Length` recomputed by lopdf, rest of the stream untouched |
| marked content after a length-changing splice (new nested `/Span`) | decompressed bytes → save | **yes** | E4b: `after` file `BDC=23 BMC=1 EMC=24 AT=21 balanced=true`, `qpdf --check` clean | `/Length` re-derived from the new stream bytes |
| stream lopdf has **no decoder** for (`/Filter /DCTDecode`) | plain `save()` | **payload yes, spelling no** | E10: `raw_len=22 raw_fnv=6f135c843499c050` identical before/after, filter preserved, `plain=ERR Unimplemented(...)` both times, `qpdf --check` emits the *same* warning on input and output | unsupported filter is copied through as raw bytes; only the dict is re-spelled |
| objects/streams lopdf does not understand (orphans, unknown keys) | plain `save()` | **yes** | E9: orphan `6 0 obj 202` still present; `added=[] removed=[] type_changed=[]` in every fixture diff | writer emits everything in `doc.objects`, unknown keys included |
| object **bytes** (whitespace, escapes, dict spelling, xref layout) | plain `save()` | **no** | E1: `raw object regions: identical=1 changed=24`, per-object deltas −2 … −23 bytes; raw dump of obj 1: `1 0 obj\n<</Title (pdfrtl producer fixture)\n/Creator (Mozilla/5.0 \(Windows NT 10.0…` → `1 0 obj\n<</Title(pdfrtl producer fixture)/Creator(Mozilla/5.0 (Windows NT 10.0…` (same values, tighter spelling, parens no longer escaped — still balanced, so valid) | full re-serialisation from the parsed model |
| real-number precision | plain `save()` | **no** | oracle: `obj 2: DIFFERS: … 841.91998 → 841.92`; `qpdf` rewrite keeps `841.91998` | `Object::Real(f32)` is the parsed number type |
| shape of an **indirect `/Length`** | plain `save()` | **resolved, not preserved** | E9: `<< /Length 6 0 R >>` → `<</Length 202>>` + orphan `6 0 obj 202`; value correct, `qpdf --check` clean, `pdftotext` identical | reader resolves the reference while parsing |
| object streams (ObjStm) in the input | plain `save()` | **collapsed** | E8: `objstm_in_bytes true → false`, ObjStm container object *removed*, its 5 members rewritten as top-level objects | writer only builds ObjStm when asked |
| object streams (ObjStm) in the input | `save_modern()` | **rebuilt** (as a new object) | E7: `raw classic-table → xref-stream … objstm_in_bytes false → true` with objects 1,2,3,5,6 moved into the new ObjStm; E8: `objstm_in_bytes true → true … removed=[1] added=[9, 10]`, content payload unchanged (`raw_len 134 → 134, raw_fnv 4a733d36cd2f0c21`), `marked content IDENTICAL` | `SaveOptions::use_object_streams` |
| object streams of an *already modern* file | plain `save()` | **collapsed again** | E7 diff 3: `objstm_in_bytes true → false`, `removed=[7, 8] added=[9]`, file 750 → 808 B | plain `save_internal` never builds object streams — ObjStm presence is decided by the *save call*, not by the input |
| cross-reference type | plain `save()` | **follows input** | E1/E9: `raw classic-table → classic-table`; E8: `xref-stream → xref-stream` | reader's `cross_reference_type` is carried over |
| cross-reference type | `SaveOptions::use_xref_streams` | switchable | E7 diff 2: `cross_reference_type CrossReferenceTable → CrossReferenceStream` | documented option, honoured |
| everything untouched, byte-for-byte | `IncrementalDocument::save` | **yes** | E6: `output starts with the original file verbatim (27282 bytes): true`, `file: len 27282 -> 28209 (+927)`, `objects: base=25 new=25 added=[] removed=[] type_changed=[]`, `raw object regions: identical=22 changed=3 changed ids: [2, 5, 6]` — obj 2 and obj 6 are the appended copies of the two edited objects (`new[27282..27498]`, `new[27498..28056]`, and 27282 is exactly the original file length), obj 5 only grew because its region now reaches the next marker (`+598 bytes` = the old xref/trailer following it); `qpdf --check` clean | append-only: original bytes are copied verbatim, only touched objects are re-emitted in the new revision |
| a content stream whose `/Length` was made stale (unsupported direct mutation) | plain `save()` | **silently corrupts** | E5 — the *only* oracle failure: `expected endstream` → `recovered stream length: 2219` → `stream inflate: data: incorrect header check` → `ERROR: page 1: content stream … errors while decoding content stream`; lopdf reloads it happily as 490 garbage bytes, `BDC=0 EMC=0 AT=0` | not a writer bug (any writer would emit a lie), but lopdf performs **no validation** — the editor must go through `change_page_content` / keep `/Length` in sync |
| `save()` vs `save_to()` vs saving twice | all | **identical output** | E2: `e1-…`, `e2-save.pdf`, `e2-save_to.pdf`, `e2-save-again.pdf` share sha256 `44d15881…` | `save_to` is `save` without the file handle |
| determinism (AGENTS rule 4) | everything | **byte-identical on rerun** | oracle: `IDENTICAL across two runs` for all 26 PDFs | — |

## Options checked before concluding (was the loss avoidable?)

Documented options in lopdf **0.45.0** (source: `save_options.rs`, `writer.rs`), all exercised here:

- `Document::save_to` ≡ `Document::save` → E2, byte-identical.
- `Document::save_with_options(target, SaveOptions { use_object_streams, use_xref_streams, linearize, object_stream_config })` (`writer.rs:30`) and `Document::save_modern` (`writer.rs:57`) → E7/E8 fix the **object-stream collapse** and choose the xref type. They do **not** restore untouched object bytes or fix `f32` reals — every object is still rewritten.
- **Incremental save exists, but as a type, not a flag:** there is no `save_increment` function; the API is `IncrementalDocument` (`incremental_document.rs`, `impl IncrementalDocument` in `writer.rs:292`) with `save` / `save_to`. It guards itself: a still-encrypted previous revision (or a decrypted doc without a recorded `/Encrypt` id) returns `Unsupported` *before* `File::create`, so it cannot truncate an existing file (lopdf issue #520). E6 proves the append-only behaviour and its byte-level result.
- **Not avoidable inside a `Document` full save:** `Object::Real` is `f32`, and there is no "keep original spelling" or "don't rewrite unmodified objects" option in `SaveOptions`. So `841.91998 → 841.92` is inherent to the rewrite path; the only in-crate remedy is to not rewrite that object (incremental), or use a different writer.

## Alternatives (task item 5, measured where possible)

- **qpdf 12.3.2** (already installed as the oracle): full rewrite — `qpdf --help=all | grep -i incremental` reports **no incremental/append-only mode** — but its rewrite *preserves the original decimal spelling*: original `MediaBox [0 0 594.95996 841.91998]` → qpdf `MediaBox [ 0 0 594.95996 841.91998 ]`, while lopdf gives `MediaBox[0 0 594.95996 841.92]`. qpdf's rewrite of the chrome fixture also measures `BDC=22 EMC=23 BMC=1 ActualText=20` — the same marked-content counts as ours (independent cross-check of the tokenizer). Technique observed at the byte level (numbers re-emitted verbatim); qpdf internals were not read.
- **Non-Rust incremental/append-only tooling:** not measured in this spike. The cross-tool question is already owned by **Q-R6** in `docs/RESEARCH-QUESTIONS.md` (outside researcher, still `open`); this spike answers only its local half — *lopdf itself can do it*. I added no new entry: every outside-knowledge question I had is either answered locally above or already Q-R6's scope. That file was not touched.

## What I could not determine (limits of the spike)

- Only **one** appended revision was tested (E6); chaining two successive incremental saves (and the resulting 3-revision file) was not.
- Encrypted/signed inputs were not tested (no fixture; lopdf itself refuses incremental save of a still-encrypted revision). Whether a signature over the *original* byte range stays verifiable after an append is **not** claimed here — untested.
- No visual rendering check: oracles are `qpdf --check`, `qpdf --show-object` and `pdftotext` (poppler 26.01.0). PDFium/Acrobat acceptance of multi-revision files was not measurable on this host.
- The mechanism behind E5's 490-byte garbage decode (why an inflate of plain text returned bytes instead of an error) was not root-caused; the observable facts (wrong bytes, no error, `qpdf --check` ERROR) are what matters for the decision.
- Fixtures with `/Annots`, optional content, linearisation, or 16-bit CID beyond the corpus were not available.

## Contents of this spike

```
spikes/lopdf-marked-content/
  Cargo.toml     standalone crate (lopdf = "0.45", empty [workspace], publish = false)
  src/scan.rs    snapshot + diff engine (object/type/stream-metadata/content-op/pairing analysis)
  src/main.rs    experiments E1..E10 + the two hand-built adversarial inputs
  run.sh         one-command reproduction: generate inputs -> run twice (determinism gate)
                 -> qpdf --check -> qpdf --show-object identity -> pdftotext identity -> sha256
  check-invocations.sh  cross-invocation check: run.sh twice, assert inputs read, compare report sha256
  FINDINGS.md    this file
  out/report.txt complete raw output of the spike + all oracle output (1838 lines / 96700 bytes)
  out/*.pdf      every input and output artefact (regenerated by run.sh; gitignored)
```

`out/` and `target/` are gitignored; `run.sh` recreates them.
