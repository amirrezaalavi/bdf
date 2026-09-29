// SPIKE: does lopdf survive load -> modify -> save without destroying marked content?
// Reproduce:  ./run.sh   (from spikes/lopdf-marked-content/)
// Throwaway code — the only deliverable is FINDINGS.md.
mod scan;

use std::fs;
use std::path::{Path, PathBuf};

use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, IncrementalDocument, Object, SaveOptions, StringFormat};

use scan::*;

type R<T> = Result<T, Box<dyn std::error::Error>>;

fn le<E: std::fmt::Debug>(e: E) -> Box<dyn std::error::Error> {
    format!("lopdf error: {e:?}").into()
}

fn length_repr(s: &lopdf::Stream) -> String {
    match s.dict.get(b"Length") {
        Ok(Object::Integer(i)) => format!("direct({i})"),
        Ok(Object::Reference(r)) => format!("ref({} {} R)", r.0, r.1),
        Ok(o) => format!("other({})", o.enum_variant()),
        Err(_) => "missing".to_string(),
    }
}

fn section(title: &str) {
    println!("\n{}", "=".repeat(78));
    println!("== {title}");
    println!("{}", "=".repeat(78));
}

fn first_page(doc: &Document) -> R<lopdf::ObjectId> {
    doc.get_pages()
        .values()
        .next()
        .copied()
        .ok_or_else(|| "document has no pages".into())
}

/// Replace the payload of the FIRST text-showing operator. Returns the old payload.
fn replace_first_show(content: &mut Content<Vec<Operation>>) -> Option<Vec<u8>> {
    for op in content.operations.iter_mut() {
        if matches!(op.operator.as_str(), "Tj" | "'" | "\"") {
            for o in op.operands.iter_mut() {
                if let Object::String(bytes, _) = o {
                    let old = std::mem::take(bytes);
                    *bytes = b"SPIKE".to_vec();
                    return Some(old);
                }
            }
        }
    }
    None
}

fn detail_of(s: &Snapshot) {
    for id in s.contents.keys().copied().collect::<Vec<_>>() {
        print_content_detail(s, id);
    }
}

/// E1: load each fixture, save it unmodified, compare structurally.
fn e1(root: &Path, out: &Path) -> R<()> {
    section("E1: unmodified round-trip via Document::save (3 fixtures)");
    let fixtures = [
        ("fa-zwnj-lamalef", root.join("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf")),
        ("actualtext-fa", root.join("corpus/raw/synthetic/actualtext-fa.pdf")),
        ("minimal-ltr", root.join("corpus/raw/synthetic/minimal-ltr.pdf")),
    ];
    for (name, fx) in fixtures {
        println!("\n### fixture: {name} ({})", fx.display());
        let base = snapshot(&fx, "before")?;
        print_summary(&base);
        let mut doc = Document::load(&fx).map_err(le)?;
        let dest = out.join(format!("e1-{name}.pdf"));
        doc.save(&dest).map_err(le)?;
        let after = snapshot(&dest, "after-save")?;
        print_summary(&after);
        print_diff(&base, &after);
        // Show the actual bytes of one small, stream-less object so the reader can
        // see WHAT a "changed raw region" means (formatting vs. data).
        if let Some(id) = base
            .raw_spans
            .iter()
            .filter(|(k, (a, b, _))| {
                b - a <= 400
                    && base
                        .objects
                        .get(k)
                        .map(|o| o.stream.is_none())
                        .unwrap_or(false)
            })
            .map(|(k, _)| *k)
            .min()
        {
            println!("raw region of stream-less object {id}:");
            println!("  BEFORE: {}", raw_region(&base, id).unwrap_or_default());
            println!("  AFTER : {}", raw_region(&after, id).unwrap_or_default());
        }
        detail_of(&after);
    }
    Ok(())
}

/// E2: Document::save vs Document::save_to, and save-twice determinism.
fn e2(root: &Path, out: &Path) -> R<()> {
    section("E2: save() vs save_to() vs save-twice byte equality");
    let fx = root.join("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf");
    let p_save = out.join("e2-save.pdf");
    let mut d1 = Document::load(&fx).map_err(le)?;
    d1.save(&p_save).map_err(le)?;
    let bytes_save = fs::read(&p_save)?;

    let mut d2 = Document::load(&fx).map_err(le)?;
    let mut buf: Vec<u8> = Vec::new();
    d2.save_to(&mut buf).map_err(le)?;
    println!("save() == save_to() (same doc state): {}", bytes_save == buf);
    fs::write(out.join("e2-save_to.pdf"), &buf)?;

    // save the SAME document object twice: does save() mutate state?
    let p_again = out.join("e2-save-again.pdf");
    d1.save(&p_again).map_err(le)?;
    let bytes_again = fs::read(&p_again)?;
    println!(
        "save() twice on the same Document: identical={} ({} vs {} bytes)",
        bytes_save == bytes_again,
        bytes_save.len(),
        bytes_again.len()
    );
    Ok(())
}

/// E3: structured edit — decode ops, replace one Tj payload, re-encode, save.
fn e3(root: &Path, out: &Path) -> R<()> {
    section("E3: structured edit (Content::decode -> replace one Tj payload -> save)");
    for (name, rel) in [
        ("fa-zwnj-lamalef", "corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf"),
        ("actualtext-fa", "corpus/raw/synthetic/actualtext-fa.pdf"),
    ] {
        let fx = root.join(rel);
        println!("\n### fixture: {name}");
        let base = snapshot(&fx, "before")?;
        print_summary(&base);
        let mut doc = Document::load(&fx).map_err(le)?;
        let page = first_page(&doc)?;
        let mut content = doc.get_and_decode_page_content(page).map_err(le)?;
        println!("decoded operations: {}", content.operations.len());
        match replace_first_show(&mut content) {
            Some(old) => println!(
                "replaced first Tj payload <{}> with 5350494b45 (\"SPIKE\")",
                hex(&old)
            ),
            None => println!("NO text-showing payload found to replace!"),
        }
        let encoded = content.encode().map_err(le)?;
        doc.change_page_content(page, encoded).map_err(le)?;
        let dest = out.join(format!("e3-{name}.pdf"));
        doc.save(&dest).map_err(le)?;
        let after = snapshot(&dest, "after-edit")?;
        print_summary(&after);
        print_diff(&base, &after);
        detail_of(&after);
    }
    Ok(())
}

/// E3b: insert a brand new marked-content run (BDC with /ActualText ... EMC).
fn e3b(root: &Path, out: &Path) -> R<()> {
    section("E3b: insert a new marked-content run (/Span with /ActualText BDC ... EMC)");
    let fx = root.join("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf");
    let base = snapshot(&fx, "before")?;
    print_summary(&base);
    let mut doc = Document::load(&fx).map_err(le)?;
    let page = first_page(&doc)?;
    let mut content = doc.get_and_decode_page_content(page).map_err(le)?;
    let mut dict = Dictionary::new();
    dict.set(
        "ActualText",
        Object::String(b"SPIKE-INSERTED-SPAN".to_vec(), StringFormat::Hexadecimal),
    );
    let idx = content
        .operations
        .iter()
        .position(|o| o.operator == "Tj")
        .unwrap_or(0);
    content.operations.insert(
        idx,
        Operation::new("BDC", vec![Object::Name(b"Span".to_vec()), Object::Dictionary(dict)]),
    );
    content.operations.insert(idx + 2, Operation::new("EMC", vec![]));
    println!(
        "inserted /Span << /ActualText <5350494b...> >> BDC + EMC around operation #{idx}"
    );
    let encoded = content.encode().map_err(le)?;
    doc.change_page_content(page, encoded).map_err(le)?;
    let dest = out.join("e3b-insert-marked-run.pdf");
    doc.save(&dest).map_err(le)?;
    let after = snapshot(&dest, "after-insert")?;
    print_summary(&after);
    print_diff(&base, &after);
    detail_of(&after);
    Ok(())
}

/// E4: raw byte-level splice on the decompressed content stream (no op decoding).
/// Splice points are taken from the tokenizer's own offsets, not from string search.
fn e4(root: &Path, out: &Path) -> R<()> {
    section("E4a: raw byte splice, length-preserving (replace one hex Tj payload)");
    let fx = root.join("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf");
    let base = snapshot(&fx, "before")?;
    print_summary(&base);
    let cid = *base.contents.keys().next().ok_or("no content stream in fixture")?;
    let plain = first_content_plain(&fx)?;
    let mc = &base.contents[&cid].mc;
    let show0 = mc.shows.first().ok_or("no text-showing payload").map_err(|e| e.to_string())?.clone();
    println!(
        "target: first text-showing payload at {}..{} = <{}> (inside a marked run: {})",
        show0.offset,
        show0.end,
        hex(&show0.val),
        inside_pair(mc, show0.offset)
    );
    println!("before: {}", excerpt(&plain, show0.offset, 64));
    let mut spliced = plain.clone();
    spliced.splice(show0.offset..show0.end, b"<0041>".iter().copied());
    println!("after:  {}", excerpt(&spliced, show0.offset, 64));
    let mut doc = Document::load(&fx).map_err(le)?;
    let page = first_page(&doc)?;
    doc.change_page_content(page, spliced).map_err(le)?;
    let dest = out.join("e4a-splice-same-length.pdf");
    doc.save(&dest).map_err(le)?;
    let after = snapshot(&dest, "after-splice")?;
    print_summary(&after);
    print_diff(&base, &after);
    detail_of(&after);

    section("E4b: raw byte splice, length-changing (nest a NEW /Span BDC...EMC around a Tj)");
    // Chrome wraps every run in /NonStruct and every span in /Span, so "outside
    // every marked run" does not exist here; take a payload that is NOT in a /Span
    // and nest a fresh /Span BDC...EMC around it (the realistic edit shape).
    let target = mc
        .shows
        .iter()
        .find(|s| !inside_tag(mc, s.offset, "Span"))
        .ok_or("no text-show outside a /Span run")
        .map_err(|e| e.to_string())?
        .clone();
    println!(
        "target: text-show at {}..{} = <{}> (in /Span run: {})",
        target.offset,
        target.end,
        hex(&target.val),
        inside_tag(mc, target.offset, "Span")
    );
    let open = b"/Span << /ActualText <feff06270644> >> BDC ";
    let close = b" EMC";
    // The EMC must close AFTER the Tj operator: "<payload> EMC Tj" would steal the
    // operand from Tj. Find the operator that follows the payload, then splice.
    let tj_at = find_from(&plain, b"Tj", target.end).ok_or("no Tj operator after payload")?;
    println!(
        "payload ends at {}, following operator Tj at {}..{}",
        target.end,
        tj_at,
        tj_at + 2
    );
    let mut spliced2 = plain.clone();
    spliced2.splice(target.offset..target.offset, open.iter().copied());
    let end2 = tj_at + 2 + open.len();
    spliced2.splice(end2..end2, close.iter().copied());
    println!("before: {}", excerpt(&plain, target.offset, 70));
    println!("after:  {}", excerpt(&spliced2, target.offset + open.len(), 110));
    let mut doc2 = Document::load(&fx).map_err(le)?;
    let page2 = first_page(&doc2)?;
    doc2.change_page_content(page2, spliced2).map_err(le)?;
    let dest2 = out.join("e4b-splice-wrap-span.pdf");
    doc2.save(&dest2).map_err(le)?;
    let after2 = snapshot(&dest2, "after-wrap")?;
    print_summary(&after2);
    print_diff(&base, &after2);
    detail_of(&after2);
    Ok(())
}

/// E5: the adversarial path — poke Stream::content directly, leaving the
/// indirect /Length reference and /Filter dictionary untouched.
fn e5(root: &Path, out: &Path) -> R<()> {
    section("E5: HAZARD — mutate Stream::content directly (indirect /Length left stale)");
    let fx = root.join("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf");
    let mut doc = Document::load(&fx).map_err(le)?;
    let page = first_page(&doc)?;
    let cid = doc.get_page_contents(page)[0];
    println!("content stream object: {cid:?}");
    {
        let s = doc
            .get_object_mut(cid)
            .map_err(le)?
            .as_stream_mut()
            .map_err(le)?;
        println!(
            "before: /Length = {}, /Filter = {:?}, content {} bytes",
            s.dict.get(b"Length").map(|o| format!("{o:?}")).unwrap_or_default(),
            s.dict.get(b"Filter").map(|o| format!("{o:?}")).unwrap_or_default(),
            s.content.len()
        );
        let mut new = s.decompressed_content().map_err(le)?;
        new.extend_from_slice(b"\n% spike: appended bytes, /Length NOT updated\n");
        s.content = new; // <-- dict untouched: /Filter still FlateDecode, /Length still 9 0 R
        println!("after: content {} bytes, dict untouched", s.content.len());
    }
    let dest = out.join("e5-stale-length.pdf");
    doc.save(&dest).map_err(le)?;
    println!("saved {}", dest.display());
    if let Ok(txt) = fs::read(&dest) {
        if let Some(i) = find_from(&txt, b"/FlateDecode/Length", 0) {
            println!(
                "on disk: ...{}",
                String::from_utf8_lossy(&txt[i.saturating_sub(20)..(i + 60).min(txt.len())])
                    .replace('\n', "\\n")
            );
        }
    }
    // what does lopdf itself make of the file it just wrote?
    match Document::load(&dest) {
        Ok(d) => {
            let s = d.get_object(cid).map_err(le)?.as_stream().map_err(le)?;
            println!(
                "lopdf reload: dict /Length = {}, in-memory raw content = {} bytes",
                length_repr(s),
                s.content.len()
            );
            match s.decompressed_content() {
                Ok(p) => println!(
                    "lopdf decompressed: {} bytes, first 60: {:?}",
                    p.len(),
                    String::from_utf8_lossy(&p[..60.min(p.len())])
                ),
                Err(e) => println!("lopdf decompress ERROR: {e:?}"),
            }
        }
        Err(e) => println!("lopdf RELOAD FAILED: {e:?}"),
    }
    match snapshot(&dest, "after-stale-edit") {
        Ok(after) => {
            print_summary(&after);
            println!("=> lopdf reloaded the broken file; see qpdf --check output for the oracle verdict");
        }
        Err(e) => println!("=> RELOAD FAILED: {e}"),
    }
    // control: same edit done through the supported API
    let mut doc2 = Document::load(&fx).map_err(le)?;
    let page2 = first_page(&doc2)?;
    let cid2 = doc2.get_page_contents(page2)[0];
    let mut new_plain;
    {
        let s = doc2
            .get_object_mut(cid2)
            .map_err(le)?
            .as_stream_mut()
            .map_err(le)?;
        new_plain = s.decompressed_content().map_err(le)?;
    }
    new_plain.extend_from_slice(b"\n% spike: appended bytes, /Length updated by API\n");
    doc2.change_page_content(page2, new_plain).map_err(le)?;
    let dest2 = out.join("e5-supported-api.pdf");
    doc2.save(&dest2).map_err(le)?;
    let ctrl = snapshot(&dest2, "after-supported-edit")?;
    print_summary(&ctrl);
    println!("=> control (change_page_content) reload: OK, /Length now {}",
        ctrl.contents.get(&cid2.0).map(|c| c.meta.length.clone()).unwrap_or_default());
    Ok(())
}

/// E6: incremental update through IncrementalDocument (appended revision).
fn e6(root: &Path, out: &Path) -> R<()> {
    section("E6: incremental update via IncrementalDocument (append-only revision)");
    let fx = root.join("corpus/raw/generated/chrome/fa-zwnj-lamalef.pdf");
    let original = fs::read(&fx)?;
    let base = snapshot(&fx, "before")?;
    print_summary(&base);

    let doc = Document::load(&fx).map_err(le)?;
    let page = first_page(&doc)?;
    let content_ids = doc.get_page_contents(page);
    let mut inc = IncrementalDocument::create_from(original.clone(), doc);
    println!(
        "prev objects: {}, new_document objects before cloning: {}",
        inc.get_prev_documents().objects.len(),
        inc.new_document.objects.len()
    );
    // Clone ONLY the objects this revision touches into the new document;
    // everything else stays in the previous revision, byte-for-byte.
    inc.opt_clone_object_to_new_document(page).map_err(le)?;
    for cid in &content_ids {
        inc.opt_clone_object_to_new_document(*cid).map_err(le)?;
    }
    println!(
        "new_document objects after cloning page+contents: {} ids={:?}",
        inc.new_document.objects.len(),
        inc.new_document.objects.keys().copied().collect::<Vec<_>>()
    );
    // A real edit in the new revision: replace one Tj payload.
    let mut content = inc.new_document.get_and_decode_page_content(page).map_err(le)?;
    match replace_first_show(&mut content) {
        Some(old) => println!("replaced first Tj payload <{}> with SPIKE", hex(&old)),
        None => println!("no Tj payload replaced"),
    }
    let encoded = content.encode().map_err(le)?;
    inc.new_document.change_page_content(page, encoded).map_err(le)?;
    let dest = out.join("e6-incremental.pdf");
    inc.save(&dest).map_err(le)?;
    let written = fs::read(&dest)?;
    let prefix_ok = written.len() >= original.len() && written[..original.len()] == original[..];
    println!(
        "output starts with the original file verbatim ({} bytes): {}",
        original.len(),
        prefix_ok
    );
    println!(
        "original {} bytes -> incremental {} bytes (appended {})",
        original.len(),
        written.len(),
        written.len() as i64 - original.len() as i64
    );
    match snapshot(&dest, "after-incremental") {
        Ok(after) => {
            print_summary(&after);
            print_diff(&base, &after);
            detail_of(&after);
        }
        Err(e) => println!("RELOAD FAILED: {e}"),
    }
    Ok(())
}

/// E7: save_modern / save_with_options (object streams + xref streams) round-trip.
fn e7(root: &Path, out: &Path) -> R<()> {
    section("E7: save_modern() (object streams + xref stream) and xref-stream-only save");
    let fx = root.join("corpus/raw/synthetic/actualtext-fa.pdf");
    let base = snapshot(&fx, "before")?;
    print_summary(&base);

    let mut doc = Document::load(&fx).map_err(le)?;
    let mut buf: Vec<u8> = Vec::new();
    doc.save_modern(&mut buf).map_err(le)?;
    let p1 = out.join("e7-save-modern.pdf");
    fs::write(&p1, &buf)?;
    let modern = snapshot(&p1, "save_modern")?;
    print_summary(&modern);
    print_diff(&base, &modern);

    let mut doc2 = Document::load(&fx).map_err(le)?;
    let opts = SaveOptions::builder().use_xref_streams(true).build();
    let mut buf2: Vec<u8> = Vec::new();
    doc2.save_with_options(&mut buf2, opts).map_err(le)?;
    let p2 = out.join("e7-xref-stream-only.pdf");
    fs::write(&p2, &buf2)?;
    let xr = snapshot(&p2, "xref-stream-only")?;
    print_summary(&xr);
    print_diff(&base, &xr);

    // round-trip the object-stream output back through a plain save()
    let mut doc3 = Document::load(&p1).map_err(le)?;
    let p3 = out.join("e7-modern-rt.pdf");
    doc3.save(&p3).map_err(le)?;
    let rt = snapshot(&p3, "save_modern -> plain save")?;
    print_summary(&rt);
    print_diff(&modern, &rt);
    Ok(())
}

/// E8: adversarial input — a file produced by qpdf with object streams + xref stream.
fn e8(out: &Path) -> R<()> {
    section("E8: adversarial input — qpdf --object-streams=generate (ObjStm + xref stream)");
    let fx = out.join("in-qpdf-objstm.pdf");
    if !fx.exists() {
        println!("SKIPPED: {} missing — run.sh did not produce it (qpdf absent?)", fx.display());
        return Ok(());
    }
    let base = snapshot(&fx, "input")?;
    print_summary(&base);
    println!(
        "objects living inside object streams (xref type 2 entries): {:?}",
        base.compressed_ids
    );
    let mut doc = Document::load(&fx).map_err(le)?;
    let dest = out.join("e8-objstm-rt.pdf");
    doc.save(&dest).map_err(le)?;
    let after = snapshot(&dest, "after-plain-save")?;
    print_summary(&after);
    print_diff(&base, &after);
    detail_of(&after);

    // and with save_modern, so object streams are rebuilt rather than collapsed
    let mut doc2 = Document::load(&fx).map_err(le)?;
    let mut buf: Vec<u8> = Vec::new();
    doc2.save_modern(&mut buf).map_err(le)?;
    let dest2 = out.join("e8-objstm-modern.pdf");
    fs::write(&dest2, &buf)?;
    let modern = snapshot(&dest2, "after-save_modern")?;
    print_summary(&modern);
    print_diff(&base, &modern);
    Ok(())
}

/// Build a small PDF whose content stream carries an INDIRECT /Length (6 0 R).
/// Hand-built byte by byte so the input is not lopdf-shaped; run.sh verifies it
/// with qpdf before lopdf ever sees it.
fn put_obj(out: &mut Vec<u8>, num: usize, body: &[u8]) -> usize {
    let start = out.len();
    out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
    out.extend_from_slice(body);
    out.extend_from_slice(b"\nendobj\n");
    start
}

fn gen_indirect_length_pdf() -> Vec<u8> {
    let content: &[u8] = b"BT\n/F1 12 Tf\n20 80 Td\n/Span << /ActualText <FEFF06330644062706450020062F064606CC0627> >> BDC (salam donya) Tj EMC\n0 -24 Td\n/Span << /ActualText <FEFF064506CC200C063106480645> >> BDC (miravam) Tj EMC\nET\n";
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut off = [0usize; 7];
    off[1] = put_obj(&mut out, 1, b"<< /Type /Catalog /Pages 2 0 R >>");
    off[2] = put_obj(&mut out, 2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    off[3] = put_obj(
        &mut out,
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 120] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
    );
    off[4] = out.len();
    out.extend_from_slice(b"4 0 obj\n<< /Length 6 0 R >>\nstream\n");
    out.extend_from_slice(content);
    out.extend_from_slice(b"endstream\nendobj\n");
    off[5] = put_obj(&mut out, 5, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    off[6] = put_obj(&mut out, 6, format!("{}", content.len()).as_bytes());
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 7\n0000000000 65535 f \n");
    for i in 1..7 {
        out.extend_from_slice(format!("{:010} 00000 n \n", off[i]).as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 7 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF").as_bytes(),
    );
    out
}

/// E9: adversarial input (b) — indirect /Length, unmodified round-trip + edit.
fn e9(out: &Path) -> R<()> {
    section("E9: adversarial input — content stream with an INDIRECT /Length (hand-built)");
    let p = out.join("in-indirect-length.pdf");
    fs::write(&p, gen_indirect_length_pdf())?;
    println!("wrote {} bytes to {}", fs::metadata(&p)?.len(), p.display());
    let base = snapshot(&p, "input")?;
    print_summary(&base);

    let mut doc = Document::load(&p).map_err(le)?;
    let d1 = out.join("e9-roundtrip.pdf");
    doc.save(&d1).map_err(le)?;
    let after = snapshot(&d1, "after-roundtrip")?;
    print_summary(&after);
    print_diff(&base, &after);
    detail_of(&after);

    let mut doc2 = Document::load(&p).map_err(le)?;
    let page = first_page(&doc2)?;
    let mut content = doc2.get_and_decode_page_content(page).map_err(le)?;
    if let Some(old) = replace_first_show(&mut content) {
        println!("replaced first Tj payload <{}> with SPIKE", hex(&old));
    }
    let encoded = content.encode().map_err(le)?;
    doc2.change_page_content(page, encoded).map_err(le)?;
    let d2 = out.join("e9-edit.pdf");
    doc2.save(&d2).map_err(le)?;
    let after2 = snapshot(&d2, "after-edit")?;
    print_summary(&after2);
    print_diff(&base, &after2);
    detail_of(&after2);
    Ok(())
}

/// E10: a stream lopdf has no decoder for (/Filter /DCTDecode). Does the raw
/// payload and the filter dictionary survive a round trip?
fn gen_unknown_filter_pdf() -> Vec<u8> {
    let content: &[u8] = b"BT\n/F1 12 Tf\n20 80 Td\n/Span << /ActualText <FEFF0633064406270645> >> BDC (salam) Tj EMC\nET\nq\n100 0 0 100 20 20 cm\n/Im1 Do\nQ\n";
    // deliberately NOT a decodable JPEG: SOI + JFIF header + EOI
    let img: &[u8] = b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01\x01\x00\x00\x01\x00\x01\x00\x00\xFF\xD9";
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut off = [0usize; 8];
    off[1] = put_obj(&mut out, 1, b"<< /Type /Catalog /Pages 2 0 R >>");
    off[2] = put_obj(&mut out, 2, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>");
    off[3] = put_obj(
        &mut out,
        3,
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 120] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>",
    );
    off[4] = out.len();
    out.extend_from_slice(b"4 0 obj\n<< /Length 7 0 R >>\nstream\n");
    out.extend_from_slice(content);
    out.extend_from_slice(b"endstream\nendobj\n");
    off[5] = put_obj(&mut out, 5, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    off[6] = out.len();
    out.extend_from_slice(
        format!(
            "6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n",
            img.len()
        )
        .as_bytes(),
    );
    out.extend_from_slice(img);
    out.extend_from_slice(b"\nendstream\nendobj\n");
    off[7] = put_obj(&mut out, 7, format!("{}", content.len()).as_bytes());
    let xref = out.len();
    out.extend_from_slice(b"xref\n0 8\n0000000000 65535 f \n");
    for i in 1..8 {
        out.extend_from_slice(format!("{:010} 00000 n \n", off[i]).as_bytes());
    }
    out.extend_from_slice(
        format!("trailer\n<< /Size 8 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF").as_bytes(),
    );
    out
}

fn e10(out: &Path) -> R<()> {
    section("E10: a stream lopdf cannot decode (/Filter /DCTDecode) — preserved or destroyed?");
    let p = out.join("in-unknown-filter.pdf");
    fs::write(&p, gen_unknown_filter_pdf())?;
    println!("wrote {} bytes to {}", fs::metadata(&p)?.len(), p.display());
    let base = snapshot(&p, "input")?;
    print_summary(&base);
    let mut doc = Document::load(&p).map_err(le)?;
    let d1 = out.join("e10-roundtrip.pdf");
    doc.save(&d1).map_err(le)?;
    let after = snapshot(&d1, "after-roundtrip")?;
    print_summary(&after);
    print_diff(&base, &after);
    // byte-level check of the payload lopdf could not understand
    if let (Some((_, _, bh)), Some((_, _, ah))) = (
        base.raw_spans.get(&6),
        after.raw_spans.get(&6),
    ) {
        println!(
            "raw file region of the undecodable image object 6: base fnv={bh:016x} after fnv={ah:016x} identical={}",
            bh == ah
        );
    }
    Ok(())
}

fn first_content_plain(fx: &Path) -> R<Vec<u8>> {
    let doc = Document::load(fx).map_err(le)?;
    let page = first_page(&doc)?;
    let cid = doc.get_page_contents(page)[0];
    let s = doc.get_object(cid).map_err(le)?.as_stream().map_err(le)?;
    Ok(s.decompressed_content().map_err(le)?)
}

fn main() -> R<()> {
    let spike = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = spike
        .join("../..")
        .canonicalize()
        .map_err(|e| format!("resolve repo root: {e}"))?;
    let out = spike.join("out");
    fs::create_dir_all(&out)?;
    println!("repo root: {}", root.display());
    println!("output dir: {}", out.display());

    e1(&root, &out)?;
    e2(&root, &out)?;
    e3(&root, &out)?;
    e3b(&root, &out)?;
    e4(&root, &out)?;
    e5(&root, &out)?;
    e6(&root, &out)?;
    e7(&root, &out)?;
    e8(&out)?;
    e9(&out)?;
    e10(&out)?;

    println!("\nALL EXPERIMENTS DONE");
    Ok(())
}
