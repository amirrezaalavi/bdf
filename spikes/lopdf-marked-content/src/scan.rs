// Structural snapshot + marked-content analysis for the lopdf round-trip spike.
// Throwaway code: its only job is to produce evidence for FINDINGS.md.
use std::collections::BTreeMap;
use std::path::Path;

use lopdf::xref::{XrefEntry, XrefType};
use lopdf::{Document, Object};

// ---------------------------------------------------------------- fingerprints

pub fn fnv1a64(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

pub fn hex(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

// ------------------------------------------------------------------- tokenizer

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Keyword,
    Str,
    HexStr,
    Name,
    OpenDict,
    CloseDict,
    OpenArr,
    CloseArr,
    Num,
    Other,
}

#[derive(Debug, Clone)]
pub struct Tok {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    /// decoded payload (strings, names) or raw token bytes (keywords, numbers)
    pub val: Vec<u8>,
}

fn is_ws(b: u8) -> bool {
    matches!(b, 0x00 | b' ' | b'\t' | b'\r' | b'\n' | 0x0c)
}

fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// A PDF token scanner good enough for content streams: keywords are only ever
/// produced from real token positions, so counting `BDC`/`EMC` can never match
/// bytes that live inside a string or hex string.
pub fn tokenize(d: &[u8]) -> Vec<Tok> {
    let mut out: Vec<Tok> = Vec::new();
    let n = d.len();
    let mut i = 0usize;
    while i < n {
        let c = d[i];
        if is_ws(c) {
            i += 1;
            continue;
        }
        if c == b'%' {
            while i < n && d[i] != b'\n' && d[i] != b'\r' {
                i += 1;
            }
            continue;
        }
        let start = i;
        match c {
            b'(' => {
                i += 1;
                let mut depth: usize = 1;
                let mut val: Vec<u8> = Vec::new();
                while i < n {
                    let b = d[i];
                    if b == b'\\' {
                        i += 1;
                        if i >= n {
                            break;
                        }
                        let e = d[i];
                        if e.is_ascii_digit() {
                            let mut v: u32 = 0;
                            let mut cnt = 0;
                            while i < n && cnt < 3 && (b'0'..=b'7').contains(&d[i]) {
                                v = v * 8 + (d[i] - b'0') as u32;
                                i += 1;
                                cnt += 1;
                            }
                            val.push((v & 0xff) as u8);
                        } else {
                            i += 1;
                            match e {
                                b'n' => val.push(b'\n'),
                                b'r' => val.push(b'\r'),
                                b't' => val.push(b'\t'),
                                b'b' => val.push(0x08),
                                b'f' => val.push(0x0c),
                                b'(' => val.push(b'('),
                                b')' => val.push(b')'),
                                b'\\' => val.push(b'\\'),
                                b'\n' => {}
                                b'\r' => {
                                    if i < n && d[i] == b'\n' {
                                        i += 1;
                                    }
                                }
                                other => val.push(other),
                            }
                        }
                        continue;
                    }
                    if b == b'(' {
                        depth += 1;
                        val.push(b'(');
                        i += 1;
                        continue;
                    }
                    if b == b')' {
                        depth -= 1;
                        i += 1;
                        if depth == 0 {
                            break;
                        }
                        val.push(b')');
                        continue;
                    }
                    val.push(b);
                    i += 1;
                }
                out.push(Tok {
                    kind: Kind::Str,
                    start,
                    end: i,
                    val,
                });
            }
            b'<' => {
                if i + 1 < n && d[i + 1] == b'<' {
                    out.push(Tok {
                        kind: Kind::OpenDict,
                        start,
                        end: i + 2,
                        val: Vec::new(),
                    });
                    i += 2;
                } else {
                    i += 1;
                    let mut digits: Vec<u8> = Vec::new();
                    while i < n && d[i] != b'>' {
                        let b = d[i];
                        i += 1;
                        if hex_val(b).is_some() {
                            digits.push(b);
                        }
                    }
                    if i < n && d[i] == b'>' {
                        i += 1;
                    }
                    if digits.len() % 2 == 1 {
                        digits.push(b'0');
                    }
                    let mut val = Vec::with_capacity(digits.len() / 2);
                    for pair in digits.chunks(2) {
                        let hi = hex_val(pair[0]).unwrap_or(0);
                        let lo = hex_val(pair[1]).unwrap_or(0);
                        val.push(hi * 16 + lo);
                    }
                    out.push(Tok {
                        kind: Kind::HexStr,
                        start,
                        end: i,
                        val,
                    });
                }
            }
            b'>' => {
                if i + 1 < n && d[i + 1] == b'>' {
                    out.push(Tok {
                        kind: Kind::CloseDict,
                        start,
                        end: i + 2,
                        val: Vec::new(),
                    });
                    i += 2;
                } else {
                    out.push(Tok {
                        kind: Kind::Other,
                        start,
                        end: i + 1,
                        val: b">".to_vec(),
                    });
                    i += 1;
                }
            }
            b'[' => {
                out.push(Tok {
                    kind: Kind::OpenArr,
                    start,
                    end: i + 1,
                    val: Vec::new(),
                });
                i += 1;
            }
            b']' => {
                out.push(Tok {
                    kind: Kind::CloseArr,
                    start,
                    end: i + 1,
                    val: Vec::new(),
                });
                i += 1;
            }
            b'/' => {
                i += 1;
                let mut val: Vec<u8> = Vec::new();
                while i < n && !is_ws(d[i]) && !is_delim(d[i]) {
                    if d[i] == b'#' && i + 2 < n {
                        if let (Some(hi), Some(lo)) = (hex_val(d[i + 1]), hex_val(d[i + 2])) {
                            val.push(hi * 16 + lo);
                            i += 3;
                            continue;
                        }
                    }
                    val.push(d[i]);
                    i += 1;
                }
                out.push(Tok {
                    kind: Kind::Name,
                    start,
                    end: i,
                    val,
                });
            }
            b'{' | b'}' => {
                out.push(Tok {
                    kind: Kind::Other,
                    start,
                    end: i + 1,
                    val: d[start..=i].to_vec(),
                });
                i += 1;
            }
            _ => {
                while i < n && !is_ws(d[i]) && !is_delim(d[i]) {
                    i += 1;
                }
                if i == start {
                    // defensive: never spin on a byte we did not consume
                    i += 1;
                }
                let val = d[start..i].to_vec();
                let kind = if val[0].is_ascii_digit()
                    || val[0] == b'+'
                    || val[0] == b'-'
                    || val[0] == b'.'
                {
                    Kind::Num
                } else {
                    Kind::Keyword
                };
                out.push(Tok {
                    kind,
                    start,
                    end: i,
                    val,
                });
            }
        }
    }
    out
}

// ------------------------------------------------- marked-content analysis

#[derive(Debug, Clone)]
pub struct AtPayload {
    /// byte offset of the payload token inside the decompressed stream
    pub offset: usize,
    pub enc: &'static str,
    pub decoded: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Show {
    /// byte offset of the string token in the decompressed stream
    pub offset: usize,
    pub end: usize,
    pub val: Vec<u8>,
}

#[derive(Debug, Default, Clone)]
pub struct Mc {
    pub counts: BTreeMap<String, usize>,
    pub actual_text: Vec<AtPayload>,
    /// text-showing payloads (Tj / ' / " and strings inside TJ)
    pub shows: Vec<Show>,
    /// (tag, open offset, close offset) — close is None while unclosed
    pub pairs: Vec<(String, usize, Option<usize>)>,
    pub unmatched_emc: Vec<usize>,
    pub max_depth: usize,
}

impl Mc {
    pub fn balanced(&self) -> bool {
        self.unmatched_emc.is_empty() && self.pairs.iter().all(|p| p.2.is_some())
    }
    pub fn count(&self, op: &str) -> usize {
        *self.counts.get(op).unwrap_or(&0)
    }
}

fn find_open_dict(toks: &[Tok], close_idx: usize) -> Option<usize> {
    // close_idx points at a CloseDict; walk back to its matching OpenDict.
    let mut depth = 0i32;
    let mut j = close_idx;
    loop {
        match toks[j].kind {
            Kind::CloseDict => depth += 1,
            Kind::OpenDict => {
                depth -= 1;
                if depth == 0 {
                    return Some(j);
                }
            }
            _ => {}
        }
        if j == 0 {
            return None;
        }
        j -= 1;
    }
}

pub fn analyze(data: &[u8]) -> Mc {
    let toks = tokenize(data);
    let mut mc = Mc::default();
    let mut stack: Vec<usize> = Vec::new(); // index into mc.pairs
    for (idx, t) in toks.iter().enumerate() {
        if t.kind != Kind::Keyword {
            continue;
        }
        let kw = String::from_utf8_lossy(&t.val).to_string();
        match kw.as_str() {
            "BDC" | "BMC" => {
                *mc.counts.entry(kw.clone()).or_insert(0) += 1;
                let mut tag = "<no-tag>".to_string();
                if kw == "BDC" {
                    if idx >= 1 && toks[idx - 1].kind == Kind::CloseDict {
                        if let Some(open) = find_open_dict(&toks, idx - 1) {
                            if open >= 1 && toks[open - 1].kind == Kind::Name {
                                tag = String::from_utf8_lossy(&toks[open - 1].val).to_string();
                            }
                            // pull /ActualText out of the property list
                            let mut j = open + 1;
                            while j < idx {
                                if toks[j].kind == Kind::Name && toks[j].val == b"ActualText" {
                                    if j + 1 < idx {
                                        let p = &toks[j + 1];
                                        if p.kind == Kind::Str || p.kind == Kind::HexStr {
                                            mc.actual_text.push(AtPayload {
                                                offset: p.start,
                                                enc: if p.kind == Kind::HexStr {
                                                    "hex"
                                                } else {
                                                    "literal"
                                                },
                                                decoded: p.val.clone(),
                                            });
                                        }
                                    }
                                }
                                j += 1;
                            }
                        }
                    }
                } else if idx >= 1 && toks[idx - 1].kind == Kind::Name {
                    tag = String::from_utf8_lossy(&toks[idx - 1].val).to_string();
                }
                stack.push(mc.pairs.len());
                mc.pairs.push((tag, t.start, None));
                if stack.len() > mc.max_depth {
                    mc.max_depth = stack.len();
                }
            }
            "EMC" => {
                *mc.counts.entry(kw).or_insert(0) += 1;
                if let Some(pi) = stack.pop() {
                    mc.pairs[pi].2 = Some(t.start);
                } else {
                    mc.unmatched_emc.push(t.start);
                }
            }
            "Tj" | "'" | "\"" => {
                if idx >= 1 {
                    let p = &toks[idx - 1];
                    if p.kind == Kind::Str || p.kind == Kind::HexStr {
                        mc.shows.push(Show {
                            offset: p.start,
                            end: p.end,
                            val: p.val.clone(),
                        });
                    }
                }
            }
            "TJ" => {
                let mut j = idx;
                while j > 0 && toks[j - 1].kind != Kind::OpenArr {
                    j -= 1;
                }
                for k in j..idx {
                    if toks[k].kind == Kind::Str || toks[k].kind == Kind::HexStr {
                        mc.shows.push(Show {
                            offset: toks[k].start,
                            end: toks[k].end,
                            val: toks[k].val.clone(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    mc
}

// ----------------------------------------------------------------- snapshot

#[derive(Debug, Clone)]
pub struct StreamMeta {
    pub filter: Option<String>,
    pub length: String,
    pub raw_len: usize,
    pub raw_hash: u64,
    /// decompressed (len, fnv) — Err(reason) when a filter could not be decoded
    pub plain: Result<(usize, u64), String>,
}

#[derive(Debug, Clone)]
pub struct ObjInfo {
    pub variant: &'static str,
    pub stream: Option<StreamMeta>,
}

#[derive(Debug, Clone)]
pub struct ContentInfo {
    pub meta: StreamMeta,
    pub mc: Mc,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub label: String,
    pub file_len: usize,
    pub file_hash: u64,
    pub version: String,
    pub lopdf_xref: String,
    pub raw_xref: String,
    pub objstm_in_bytes: bool,
    pub trailer_keys: Vec<String>,
    pub compressed_ids: Vec<u32>,
    pub objects: BTreeMap<u32, ObjInfo>,
    pub contents: BTreeMap<u32, ContentInfo>,
    pub pages: Vec<(u32, u32, Vec<u32>)>,
    /// id -> (raw start, raw end, fnv of the raw file region)
    pub raw_spans: BTreeMap<u32, (usize, usize, u64)>,
    /// the file bytes these regions index into
    pub file_bytes: Vec<u8>,
    pub warnings: Vec<String>,
}

fn stream_meta(s: &lopdf::Stream) -> StreamMeta {
    let filter = match s.dict.get(b"Filter") {
        Ok(o) => Some(filter_repr_docless(o)),
        Err(_) => None,
    };
    let length = match s.dict.get(b"Length") {
        Ok(Object::Integer(i)) => format!("direct({i})"),
        Ok(Object::Reference(r)) => format!("ref({} {} R)", r.0, r.1),
        Ok(other) => format!("other({})", other.enum_variant()),
        Err(_) => "missing".to_string(),
    };
    let plain = match s.decompressed_content() {
        Ok(p) => Ok((p.len(), fnv1a64(&p))),
        Err(e) => Err(format!("{e:?}")),
    };
    StreamMeta {
        filter,
        length,
        raw_len: s.content.len(),
        raw_hash: fnv1a64(&s.content),
        plain,
    }
}

fn filter_repr_docless(o: &Object) -> String {
    match o {
        Object::Name(n) => String::from_utf8_lossy(n).to_string(),
        Object::Array(a) => a
            .iter()
            .map(|x| match x {
                Object::Name(n) => String::from_utf8_lossy(n).to_string(),
                other => format!("<{}>", other.enum_variant()),
            })
            .collect::<Vec<_>>()
            .join("+"),
        other => format!("<{}>", other.enum_variant()),
    }
}

/// Locate `N G obj` markers and give every object its raw file region.
/// Markers must start at the beginning of a line (the standard layout), which
/// keeps compressed stream payloads from faking an object.
pub fn raw_spans(bytes: &[u8], max_id: u32, warnings: &mut Vec<String>) -> BTreeMap<u32, (usize, usize, u64)> {
    let n = bytes.len();
    let mut starts: Vec<(u32, usize)> = Vec::new();
    let mut p = 0usize;
    while p + 4 < n {
        if &bytes[p..p + 4] == b" obj" {
            let space = p; // index of the space before "obj"
            let g_end = space;
            let mut g_start = g_end;
            while g_start > 0 && bytes[g_start - 1].is_ascii_digit() {
                g_start -= 1;
            }
            if g_start < g_end && g_start > 0 && bytes[g_start - 1] == b' ' {
                let o_end = g_start - 1;
                let mut o_start = o_end;
                while o_start > 0 && bytes[o_start - 1].is_ascii_digit() {
                    o_start -= 1;
                }
                if o_start < o_end
                    && (o_start == 0 || bytes[o_start - 1] == b'\n' || bytes[o_start - 1] == b'\r')
                {
                    let id: u32 = std::str::from_utf8(&bytes[o_start..o_end])
                        .ok()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    if id > 0 && id <= max_id {
                        starts.push((id, o_start));
                    }
                }
            }
        }
        p += 1;
    }
    starts.sort_by_key(|(_, off)| *off);
    let mut dups = Vec::new();
    for w in starts.windows(2) {
        if w[0].0 == w[1].0 {
            dups.push(w[0].0);
        }
    }
    if !dups.is_empty() {
        warnings.push(format!(
            "raw scanner: duplicate object markers (multi-revision file?) for ids {dups:?}"
        ));
    }

    // end of the body region: first line-anchored xref/trailer/startxref after the last object
    let last = starts.last().map(|(_, o)| *o).unwrap_or(0);
    let mut body_end = n;
    let pats: [&[u8]; 4] = [
        b"\nxref\n",
        b"\ntrailer\n",
        b"\nstartxref\n",
        b"\nstartxref\r",
    ];
    for pat in pats {
        if let Some(idx) = find_from(bytes, pat, last) {
            body_end = body_end.min(idx + 1);
        }
    }

    let mut out = BTreeMap::new();
    for (i, (id, start)) in starts.iter().enumerate() {
        let end = if i + 1 < starts.len() {
            starts[i + 1].1
        } else {
            body_end.max(*start)
        };
        if end > *start {
            out.insert(*id, (*start, end, fnv1a64(&bytes[*start..end])));
        }
    }
    out
}

pub fn find_from(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    let mut i = from;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

pub fn snapshot(path: &Path, label: &str) -> Result<Snapshot, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("read {path:?}: {e}"))?;
    let doc = Document::load(path).map_err(|e| format!("lopdf load {path:?}: {e:?}"))?;
    let mut warnings: Vec<String> = Vec::new();

    let mut objects: BTreeMap<u32, ObjInfo> = BTreeMap::new();
    for (&(id, gen), obj) in &doc.objects {
        if gen != 0 {
            warnings.push(format!("object {id} has generation {gen}"));
        }
        let stream = match obj {
            Object::Stream(s) => Some(stream_meta(s)),
            _ => None,
        };
        objects.insert(
            id,
            ObjInfo {
                variant: obj.enum_variant(),
                stream,
            },
        );
    }

    let mut pages: Vec<(u32, u32, Vec<u32>)> = Vec::new();
    let mut contents: BTreeMap<u32, ContentInfo> = BTreeMap::new();
    for (num, pid) in doc.get_pages() {
        let ids: Vec<u32> = doc.get_page_contents(pid).iter().map(|r| r.0).collect();
        for cid in &ids {
            if contents.contains_key(cid) {
                continue;
            }
            if let Ok(Object::Stream(s)) = doc.get_object((*cid, 0)) {
                let meta = stream_meta(s);
                let mc = match s.decompressed_content() {
                    Ok(p) => analyze(&p),
                    Err(e) => {
                        warnings.push(format!("content stream {cid}: decompress failed {e:?}"));
                        Mc::default()
                    }
                };
                contents.insert(*cid, ContentInfo { meta, mc });
            } else {
                warnings.push(format!("page {num}: content stream {cid} not found/not a stream"));
            }
        }
        pages.push((num, pid.0, ids));
    }

    let compressed_ids: Vec<u32> = doc
        .reference_table
        .entries
        .iter()
        .filter(|(_, e)| matches!(e, XrefEntry::Compressed { .. }))
        .map(|(id, _)| *id)
        .collect();

    let lopdf_xref = match doc.reference_table.cross_reference_type {
        XrefType::CrossReferenceStream => "CrossReferenceStream",
        XrefType::CrossReferenceTable => "CrossReferenceTable",
    }
    .to_string();

    let raw_xref = if find_from(&bytes, b"\ntrailer\n", 0).is_some()
        || find_from(&bytes, b"\ntrailer\r", 0).is_some()
    {
        "classic-table"
    } else if find_from(&bytes, b"/XRef", 0).is_some() {
        "xref-stream"
    } else {
        "unknown"
    }
    .to_string();

    let raw_spans = raw_spans(&bytes, doc.max_id, &mut warnings);
    let expected_raw = objects.len() - compressed_ids.len();
    if raw_spans.len() != expected_raw {
        warnings.push(format!(
            "raw scanner found {} object markers, expected {expected_raw} ({} in object streams)",
            raw_spans.len(),
            compressed_ids.len()
        ));
    }

    let trailer_keys: Vec<String> = doc
        .trailer
        .iter()
        .map(|(k, _)| String::from_utf8_lossy(k).to_string())
        .collect();

    Ok(Snapshot {
        label: label.to_string(),
        file_len: bytes.len(),
        file_hash: fnv1a64(&bytes),
        version: doc.version.clone(),
        lopdf_xref,
        raw_xref,
        objstm_in_bytes: find_from(&bytes, b"/ObjStm", 0).is_some(),
        trailer_keys,
        compressed_ids,
        objects,
        contents,
        pages,
        raw_spans,
        file_bytes: bytes,
        warnings,
    })
}

// ------------------------------------------------------------------ printers

pub fn print_summary(s: &Snapshot) {
    println!("--- snapshot: {} ---", s.label);
    println!(
        "file: len={} fnv={:016x} header=PDF-{} xref(raw)={} xref(lopdf)={} objstm_in_bytes={} trailer_keys=[{}]",
        s.file_len,
        s.file_hash,
        s.version,
        s.raw_xref,
        s.lopdf_xref,
        s.objstm_in_bytes,
        s.trailer_keys.join(",")
    );
    println!(
        "objects: {} (in object streams: {:?})",
        s.objects.len(),
        s.compressed_ids
    );
    for (id, info) in &s.objects {
        match &info.stream {
            Some(m) => println!(
                "  obj {id}: Stream filter={} length={} raw_len={} raw_fnv={:016x} plain={}",
                m.filter.as_deref().unwrap_or("<none>"),
                m.length,
                m.raw_len,
                m.raw_hash,
                match &m.plain {
                    Ok((l, h)) => format!("len={l} fnv={h:016x}"),
                    Err(e) => format!("ERR {e}"),
                }
            ),
            None => println!("  obj {id}: {}", info.variant),
        }
    }
    println!(
        "pages: {}",
        s.pages
            .iter()
            .map(|(n, pid, c)| format!("page{n}=obj{pid} contents={c:?}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    print_contents(s);
    for w in &s.warnings {
        println!("  WARNING: {w}");
    }
}

pub fn print_contents(s: &Snapshot) {
    for (id, c) in &s.contents {
        println!(
            "  content stream {id}: BDC={} BMC={} EMC={} AT={} shows={} balanced={} max_depth={} filter={} length={} plain_len={}",
            c.mc.count("BDC"),
            c.mc.count("BMC"),
            c.mc.count("EMC"),
            c.mc.actual_text.len(),
            c.mc.shows.len(),
            c.mc.balanced(),
            c.mc.max_depth,
            c.meta.filter.as_deref().unwrap_or("<none>"),
            c.meta.length,
            c.meta.plain.as_ref().map(|(l, _)| *l).unwrap_or(0),
        );
    }
}

/// Full operator-level dump for one content stream: ActualText table, pairing
/// trace, text-showing payloads.
pub fn print_content_detail(s: &Snapshot, id: u32) {
    let Some(c) = s.contents.get(&id) else {
        println!("  (no content stream {id} in snapshot {})", s.label);
        return;
    };
    println!(
        "  content stream {id} [{}] BDC={} BMC={} EMC={} balanced={}:",
        s.label,
        c.mc.count("BDC"),
        c.mc.count("BMC"),
        c.mc.count("EMC"),
        c.mc.balanced()
    );
    println!("    ActualText payloads (offset, enc, decoded hex, utf16be text):");
    for (i, p) in c.mc.actual_text.iter().enumerate() {
        println!(
            "      [{i:02}] @{:<6} {} {} \"{}\"",
            p.offset,
            p.enc,
            hex(&p.decoded),
            utf16be_text(&p.decoded)
        );
    }
    println!("    marked-content pairing trace (tag open@off -> close@off):");
    for (i, (tag, open, close)) in c.mc.pairs.iter().enumerate() {
        match close {
            Some(cl) => println!("      [{i:02}] {tag:<14} @{open} -> @{cl}"),
            None => println!("      [{i:02}] {tag:<14} @{open} -> UNCLOSED"),
        }
    }
    for e in &c.mc.unmatched_emc {
        println!("      EMC @{e} with EMPTY stack (unmatched)");
    }
    println!("    text-showing payloads (offset..end, hex):");
    for sh in &c.mc.shows {
        println!("      @{}..{} <{}>", sh.offset, sh.end, hex(&sh.val));
    }
}

/// Decode a UTF-16BE payload (as Chrome emits: BOM + UTF-16BE) for display.
fn utf16be_text(b: &[u8]) -> String {
    if b.len() < 2 {
        return String::new();
    }
    let mut units: Vec<u16> = Vec::new();
    let body = if b[0] == 0xfe && b[1] == 0xff { &b[2..] } else { b };
    let mut i = 0;
    while i + 1 < body.len() {
        units.push(u16::from_be_bytes([body[i], body[i + 1]]));
        i += 2;
    }
    String::from_utf16(&units).unwrap_or_else(|_| format!("<invalid utf16> {}", hex(b)))
}

// -------------------------------------------------------------------- diffs

fn at_values(mc: &Mc) -> Vec<Vec<u8>> {
    mc.actual_text.iter().map(|p| p.decoded.clone()).collect()
}

fn show_values(mc: &Mc) -> Vec<Vec<u8>> {
    mc.shows.iter().map(|s| s.val.clone()).collect()
}

/// Compare two sequences of payload values; returns a human-readable issue line
/// (or None when nothing changed). Reports exactly WHICH values differ, so a
/// "the other spans survived" claim is backed by indices, not vibes.
fn values_issue(name: &str, base: &[Vec<u8>], new: &[Vec<u8>]) -> Option<String> {
    if base == new {
        return None;
    }
    if base.len() == new.len() {
        let idx: Vec<usize> = base
            .iter()
            .zip(new.iter())
            .enumerate()
            .filter(|(_, (b, n))| b != n)
            .map(|(i, _)| i)
            .collect();
        return Some(format!(
            "{name}: VALUES changed at {len} of {tot} indices: {idx:?}",
            len = idx.len(),
            tot = base.len()
        ));
    }
    // different counts: how many of the base values still occur in the new file?
    let mut remaining = new.to_vec();
    let mut kept = 0usize;
    for v in base {
        if let Some(pos) = remaining.iter().position(|x| x == v) {
            remaining.remove(pos);
            kept += 1;
        }
    }
    Some(format!(
        "{name}: count {} -> {}; base values still present: {kept}/{}, missing: {}, new: {}",
        base.len(),
        new.len(),
        base.len(),
        base.len().saturating_sub(kept),
        remaining.len()
    ))
}

fn fmt_ids(v: &[u32]) -> String {
    if v.is_empty() {
        "[]".to_string()
    } else {
        format!("{v:?}")
    }
}

pub fn print_diff(base: &Snapshot, new: &Snapshot) {
    println!(
        "--- diff {} -> {} ---",
        base.label, new.label
    );
    println!(
        "file: len {} -> {} ({}), fnv {:016x} -> {:016x}, identical={}",
        base.file_len,
        new.file_len,
        if new.file_len as i64 - base.file_len as i64 >= 0 {
            format!("+{}", new.file_len as i64 - base.file_len as i64)
        } else {
            format!("{}", new.file_len as i64 - base.file_len as i64)
        },
        base.file_hash,
        new.file_hash,
        base.file_hash == new.file_hash
    );
    println!(
        "xref: raw {} -> {}, lopdf {} -> {} ; objstm_in_bytes {} -> {}",
        base.raw_xref, new.raw_xref, base.lopdf_xref, new.lopdf_xref, base.objstm_in_bytes, new.objstm_in_bytes
    );

    let base_ids: Vec<u32> = base.objects.keys().copied().collect();
    let new_ids: Vec<u32> = new.objects.keys().copied().collect();
    let added: Vec<u32> = new_ids.iter().filter(|i| !base_ids.contains(i)).copied().collect();
    let removed: Vec<u32> = base_ids.iter().filter(|i| !new_ids.contains(i)).copied().collect();
    let mut type_changed: Vec<String> = Vec::new();
    let mut meta_changed: Vec<String> = Vec::new();
    for (id, bo) in &base.objects {
        let Some(no) = new.objects.get(id) else { continue };
        if bo.variant != no.variant {
            type_changed.push(format!("{id}:{}->{}", bo.variant, no.variant));
        }
        match (&bo.stream, &no.stream) {
            (Some(bs), Some(ns)) => {
                let mut ch = Vec::new();
                if bs.filter != ns.filter {
                    ch.push(format!("filter {:?}->{:?}", bs.filter, ns.filter));
                }
                if bs.length != ns.length {
                    ch.push(format!("length {}->{}", bs.length, ns.length));
                }
                if bs.raw_hash != ns.raw_hash {
                    ch.push(format!("raw {}->{}b", bs.raw_len, ns.raw_len));
                }
                match (&bs.plain, &ns.plain) {
                    (Ok(a), Ok(b)) if a.1 != b.1 => ch.push("DECOMPRESSED-DATA-CHANGED".into()),
                    (Ok(_), Err(e)) => ch.push(format!("plain became ERR {e}")),
                    (Err(e), Ok(_)) => ch.push(format!("plain recovered from ERR {e}")),
                    (Err(a), Err(b)) if a != b => ch.push(format!("decode error changed: {a} -> {b}")),
                    _ => {}
                }
                if !ch.is_empty() {
                    meta_changed.push(format!("obj {id}: {}", ch.join("; ")));
                }
            }
            (None, Some(_)) => meta_changed.push(format!("obj {id}: became a stream")),
            (Some(_), None) => meta_changed.push(format!("obj {id}: no longer a stream")),
            _ => {}
        }
    }
    println!(
        "objects: base={} new={} added={} removed={} type_changed={}",
        base.objects.len(),
        new.objects.len(),
        fmt_ids(&added),
        fmt_ids(&removed),
        if type_changed.is_empty() {
            "[]".to_string()
        } else {
            type_changed.join(", ")
        }
    );
    if !meta_changed.is_empty() {
        println!("stream meta changes:");
        for m in &meta_changed {
            println!("  {m}");
        }
    }

    // raw byte-level object comparison
    let mut identical = 0usize;
    let mut changed: Vec<u32> = Vec::new();
    let mut only_base: Vec<u32> = Vec::new();
    let mut only_new: Vec<u32> = Vec::new();
    for (id, bs) in &base.raw_spans {
        match new.raw_spans.get(id) {
            Some(ns) if ns.2 == bs.2 => identical += 1,
            Some(_) => changed.push(*id),
            None => only_base.push(*id),
        }
    }
    for id in new.raw_spans.keys() {
        if !base.raw_spans.contains_key(id) {
            only_new.push(*id);
        }
    }
    println!(
        "raw object regions: identical={identical} changed={} only_base={only_base:?} only_new={only_new:?}",
        changed.len()
    );
    if !changed.is_empty() {
        println!("  changed ids: {changed:?}");
        for id in changed.iter().take(12) {
            let (bs, be, bh) = base.raw_spans[id];
            let (ns, ne, nh) = new.raw_spans[id];
            println!(
                "    obj {id}: base[{bs}..{be}] fnv={bh:016x} -> new[{ns}..{ne}] fnv={nh:016x} (delta {} bytes)",
                (ne - ns) as i64 - (be - bs) as i64
            );
        }
    }

    // marked content
    let mut keys: Vec<u32> = base.contents.keys().copied().collect();
    for k in new.contents.keys() {
        if !keys.contains(k) {
            keys.push(*k);
        }
    }
    keys.sort();
    for id in keys {
        let b = base.contents.get(&id);
        let n = new.contents.get(&id);
        match (b, n) {
            (Some(b), Some(n)) => {
                let mut issues: Vec<String> = Vec::new();
                for op in ["BDC", "BMC", "EMC"] {
                    if b.mc.count(op) != n.mc.count(op) {
                        issues.push(format!("{op} {}->{}", b.mc.count(op), n.mc.count(op)));
                    }
                }
                if let Some(msg) = values_issue("ActualText", &at_values(&b.mc), &at_values(&n.mc)) {
                    issues.push(msg);
                } else if b
                    .mc
                    .actual_text
                    .iter()
                    .map(|p| p.offset)
                    .ne(n.mc.actual_text.iter().map(|p| p.offset))
                {
                    issues.push(format!(
                        "ActualText offsets changed: {:?} -> {:?}",
                        b.mc.actual_text.iter().map(|p| p.offset).collect::<Vec<_>>(),
                        n.mc.actual_text.iter().map(|p| p.offset).collect::<Vec<_>>()
                    ));
                }
                if b.mc.balanced() != n.mc.balanced() {
                    issues.push(format!("PAIRING balanced {}->{}", b.mc.balanced(), n.mc.balanced()));
                }
                if let Some(msg) = values_issue("text-show", &show_values(&b.mc), &show_values(&n.mc)) {
                    issues.push(msg);
                }
                if b.meta.raw_hash != n.meta.raw_hash {
                    issues.push(format!(
                        "stream bytes recompressed/rewritten {}->{}b",
                        b.meta.raw_len, n.meta.raw_len
                    ));
                }
                if issues.is_empty() {
                    println!("content {id}: marked content IDENTICAL (counts, AT values+positions, pairing, shows)");
                } else {
                    println!("content {id}: {}", issues.join("; "));
                }
            }
            (Some(_), None) => println!("content {id}: VANISHED"),
            (None, Some(_)) => println!("content {id}: appeared"),
            (None, None) => {}
        }
    }
    for w in &new.warnings {
        if !base.warnings.contains(w) {
            println!("  NEW WARNING: {w}");
        }
    }
    println!("--- end diff ---");
}

/// Printable one-line excerpt around `at` in `data`.
pub fn excerpt(data: &[u8], at: usize, ctx: usize) -> String {
    let lo = at.saturating_sub(ctx);
    let hi = (at + ctx).min(data.len());
    let mut out = String::new();
    if lo > 0 {
        out.push_str("...");
    }
    out.push_str(&String::from_utf8_lossy(&data[lo..at]).replace('\n', "\\n"));
    out.push_str("[HERE]");
    out.push_str(&String::from_utf8_lossy(&data[at..hi]).replace('\n', "\\n"));
    if hi < data.len() {
        out.push_str("...");
    }
    out
}

/// True when the text-showing payload at `off` sits inside a marked-content run.
pub fn inside_pair(mc: &Mc, off: usize) -> bool {
    mc.pairs
        .iter()
        .any(|(_, open, close)| close.map(|c| *open <= off && off < c).unwrap_or(false))
}

/// True when the payload at `off` sits directly in a `/Span` run (or any run whose
/// tag is `tag`).
pub fn inside_tag(mc: &Mc, off: usize, tag: &str) -> bool {
    mc.pairs.iter().any(|(t, open, close)| {
        t == tag && close.map(|c| *open <= off && off < c).unwrap_or(false)
    })
}

/// Raw file region of `id`, escaped for one-line printing.
pub fn raw_region(s: &Snapshot, id: u32) -> Option<String> {
    let (start, end, _) = *s.raw_spans.get(&id)?;
    let slice = s.file_bytes.get(start..end)?;
    Some(String::from_utf8_lossy(slice).replace('\n', "\\n"))
}
