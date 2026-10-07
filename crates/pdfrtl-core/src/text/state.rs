//! Graphics state machine, content stream walker, and font resource metrics.

use crate::text::encoding::Font;
use crate::text::tokenizer::{parse_value, tokenize, TextOp, Token, Value};
use crate::text::unit_text::UnitText;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

/// Simple-font subtypes whose byte codes come from `/Encoding` (PDF spec §9.6.6).
pub const SIMPLE_FONT_SUBTYPES: &[&[u8]] = &[b"Type1", b"MMType1", b"TrueType"];

/// One unit of producer evidence on a visual line.
#[derive(Debug, Clone)]
pub struct Unit {
    pub text: UnitText,
    /// Baseline vertical position (the matrix `f`), grouping units onto one line.
    pub line: f64,
    /// Where the PEN was when this unit was painted (CTM ∘ the advanced text matrix).
    pub paint_x: f64,
    /// False when the composed matrix does not run left-to-right (`a ≤ 0`).
    pub x_ok: bool,
    /// Inside `/ReversedChars`: the producer stored this run in visual order.
    pub reversed: bool,
    /// Bumped by each `Tm`; a new text matrix starts a new run.
    pub epoch: u64,
}

pub struct Walk {
    pub units: Vec<Unit>,
    pub used_actual_text: bool,
    pub used_to_unicode: bool,
    pub used_encoding: bool,
    pub undecodable: bool,
    pub refused_encoding: bool,
}

#[derive(Clone, Copy)]
pub struct Mat {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Mat {
    pub fn identity() -> Mat {
        Mat {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// `T(tx,ty) × self` — PDF composes row-vector matrices on the left.
    pub fn translated(&self, tx: f64, ty: f64) -> Mat {
        Mat {
            a: self.a,
            b: self.b,
            c: self.c,
            d: self.d,
            e: tx * self.a + ty * self.c + self.e,
            f: tx * self.b + ty * self.d + self.f,
        }
    }

    /// `self × rhs` for row-vector matrices: a point goes through `rhs` first, then `self`.
    pub fn multiplied(&self, rhs: &Mat) -> Mat {
        Mat {
            a: self.a * rhs.a + self.b * rhs.c,
            b: self.a * rhs.b + self.b * rhs.d,
            c: self.c * rhs.a + self.d * rhs.c,
            d: self.c * rhs.b + self.d * rhs.d,
            e: self.e * rhs.a + self.f * rhs.c + rhs.e,
            f: self.e * rhs.b + self.f * rhs.d + rhs.f,
        }
    }
}

pub struct Mark {
    pub tag: Vec<u8>,
    pub actual_text: Option<String>,
}

/// What the file says each code is WIDE, in text-space units (already divided by 1000).
#[derive(Debug, Clone, Default)]
pub struct Metrics {
    pub widths: HashMap<u16, f64>,
    pub missing: f64,
    pub known: bool,
    pub two_byte: bool,
}

impl Metrics {
    /// Width of one code in text-space units, or `None` when the file does not say.
    pub fn width(&self, code: u16) -> Option<f64> {
        if !self.known {
            return None;
        }
        self.widths.get(&code).copied().or(Some(self.missing))
    }
}

pub fn as_number(value: &Object) -> Option<f64> {
    match value {
        Object::Integer(number) => Some(*number as f64),
        Object::Real(number) => Some(f64::from(*number)),
        _ => None,
    }
}

pub fn num(ops: &[Value], index: usize) -> Option<f64> {
    ops.get(index).and_then(|value| match value {
        Value::Num(n) => Some(*n),
        _ => None,
    })
}

pub struct Walker<'a> {
    pub fonts: &'a HashMap<Vec<u8>, Font>,
    pub marks: Vec<Mark>,
    pub font: Option<Vec<u8>>,
    pub line: Mat,
    pub pen: f64,
    pub ctm: Mat,
    pub ctm_stack: Vec<Mat>,
    pub leading: f64,
    pub metrics: &'a HashMap<Vec<u8>, Metrics>,
    pub font_size: f64,
    pub char_spacing: f64,
    pub word_spacing: f64,
    pub h_scale: f64,
    pub epoch: u64,
    pub units: Vec<Unit>,
    pub used_actual_text: bool,
    pub used_to_unicode: bool,
    pub used_encoding: bool,
    pub undecodable: bool,
    pub refused_encoding: bool,
}

impl<'a> Walker<'a> {
    pub fn new(
        fonts: &'a HashMap<Vec<u8>, Font>,
        metrics: &'a HashMap<Vec<u8>, Metrics>,
    ) -> Walker<'a> {
        Walker {
            fonts,
            metrics,
            font_size: 0.0,
            char_spacing: 0.0,
            word_spacing: 0.0,
            h_scale: 1.0,
            marks: Vec::new(),
            font: None,
            line: Mat::identity(),
            pen: 0.0,
            ctm: Mat::identity(),
            ctm_stack: Vec::new(),
            leading: 0.0,
            epoch: 0,
            units: Vec::new(),
            used_actual_text: false,
            used_to_unicode: false,
            used_encoding: false,
            undecodable: false,
            refused_encoding: false,
        }
    }

    pub fn kern(&mut self, number: f64) {
        self.pen -= number / 1000.0 * self.font_size * self.h_scale;
    }

    pub fn advance(&mut self, code: u16) {
        let tx = {
            let Some(name) = self.font.as_ref() else {
                return;
            };
            let Some(metrics) = self.metrics.get(name) else {
                return;
            };
            let Some(width) = metrics.width(code) else {
                return;
            };
            let mut tx = (width * self.font_size + self.char_spacing) * self.h_scale;
            if !metrics.two_byte && code == 32 {
                tx += self.word_spacing * self.h_scale;
            }
            tx
        };
        if tx != 0.0 {
            self.pen += tx;
        }
    }

    pub fn advance_over(&mut self, bytes: &[u8]) {
        let two_byte = self
            .font
            .as_ref()
            .and_then(|name| self.metrics.get(name))
            .is_some_and(|metrics| metrics.two_byte);
        let stride = if two_byte { 2 } else { 1 };
        let mut index = 0usize;
        while index + stride <= bytes.len() {
            let code = if two_byte {
                u16::from_be_bytes([bytes[index], bytes[index + 1]])
            } else {
                u16::from(bytes[index])
            };
            self.advance(code);
            index += stride;
        }
    }

    pub fn op(&mut self, op: TextOp, ops: &[Value]) {
        match op {
            TextOp::BT => {
                self.line = Mat::identity();
                self.pen = 0.0;
            }
            TextOp::Q => self.ctm_stack.push(self.ctm),
            TextOp::PopQ => {
                if let Some(saved) = self.ctm_stack.pop() {
                    self.ctm = saved;
                }
            }
            TextOp::Cm => {
                if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                    num(ops, 0),
                    num(ops, 1),
                    num(ops, 2),
                    num(ops, 3),
                    num(ops, 4),
                    num(ops, 5),
                ) {
                    self.ctm = Mat { a, b, c, d, e, f }.multiplied(&self.ctm);
                }
            }
            TextOp::Tm => {
                if let (Some(a), Some(b), Some(c), Some(d), Some(e), Some(f)) = (
                    num(ops, 0),
                    num(ops, 1),
                    num(ops, 2),
                    num(ops, 3),
                    num(ops, 4),
                    num(ops, 5),
                ) {
                    self.line = Mat { a, b, c, d, e, f };
                    self.pen = 0.0;
                    self.epoch += 1;
                }
            }
            TextOp::Td => {
                if let (Some(tx), Some(ty)) = (num(ops, 0), num(ops, 1)) {
                    self.line = self.line.translated(tx, ty);
                    self.pen = 0.0;
                }
            }
            TextOp::TD => {
                if let (Some(tx), Some(ty)) = (num(ops, 0), num(ops, 1)) {
                    self.leading = -ty;
                    self.line = self.line.translated(tx, ty);
                    self.pen = 0.0;
                }
            }
            TextOp::TStar => {
                self.line = self.line.translated(0.0, -self.leading);
                self.pen = 0.0;
            }
            TextOp::TL => {
                if let Some(value) = num(ops, 0) {
                    self.leading = value;
                }
            }
            TextOp::Tf => {
                self.font = match ops.first() {
                    Some(Value::Name(name)) => Some(name.to_vec()),
                    _ => None,
                };
                self.font_size = num(ops, 1).unwrap_or(0.0);
            }
            TextOp::Tc => self.char_spacing = num(ops, 0).unwrap_or(0.0),
            TextOp::Tw => self.word_spacing = num(ops, 0).unwrap_or(0.0),
            TextOp::Tz => self.h_scale = num(ops, 0).unwrap_or(100.0) / 100.0,
            TextOp::Tj => {
                if let Some(Value::Str(bytes)) = ops.first() {
                    self.show(bytes.as_ref());
                }
            }
            TextOp::TJ => {
                if let Some(Value::Arr(items)) = ops.first() {
                    for item in items {
                        match item {
                            Value::Str(bytes) => self.show(bytes.as_ref()),
                            Value::Num(number) => self.kern(*number),
                            _ => {}
                        }
                    }
                }
            }
            TextOp::Quote => {
                self.line = self.line.translated(0.0, -self.leading);
                if let Some(Value::Str(bytes)) = ops.first() {
                    self.show(bytes.as_ref());
                }
            }
            TextOp::DoubleQuote => {
                if let Some(Value::Str(bytes)) = ops.get(2) {
                    self.line = self.line.translated(0.0, -self.leading);
                    self.show(bytes.as_ref());
                }
            }
            TextOp::BMC => {
                if let Some(Value::Name(tag)) = ops.first() {
                    self.marks.push(Mark {
                        tag: tag.to_vec(),
                        actual_text: None,
                    });
                }
            }
            TextOp::BDC => {
                if let (Some(Value::Name(tag)), Some(value)) = (ops.first(), ops.get(1)) {
                    let actual_text = match value {
                        Value::Dict(entries) => dict_actual_text(entries),
                        _ => None,
                    };
                    self.marks.push(Mark {
                        tag: tag.to_vec(),
                        actual_text,
                    });
                }
            }
            TextOp::EMC => {
                self.marks.pop();
            }
            _ => {}
        }
    }

    pub fn show(&mut self, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }
        let actual = self
            .marks
            .iter()
            .rev()
            .find_map(|mark| mark.actual_text.clone());
        if let Some(text) = actual.filter(|text| !text.is_empty()) {
            self.used_actual_text = true;
            self.push_unit(&text);
            self.advance_over(bytes);
            return;
        }
        let fonts = self.fonts;
        let font = match self.font.as_ref().and_then(|name| fonts.get(name)) {
            Some(font) => font,
            None => {
                self.undecodable = true;
                return;
            }
        };
        match font {
            Font::Refused => self.refused_encoding = true,
            Font::Simple(encoding) => {
                for &code in bytes {
                    match encoding.decode(code) {
                        Some(text) => {
                            self.used_encoding = true;
                            if !text.is_empty() {
                                self.push_unit(&text);
                            }
                        }
                        None => self.refused_encoding = true,
                    }
                    self.advance(u16::from(code));
                }
            }
            Font::ToUnicode(cmap) => {
                let width = cmap.code_len();
                if width == 0 || width > 2 {
                    self.undecodable = true;
                    return;
                }
                let mut i = 0usize;
                while i < bytes.len() {
                    if i + width > bytes.len() {
                        self.undecodable = true;
                        break;
                    }
                    let code = match &bytes[i..i + width] {
                        [b] => u16::from(*b),
                        [hi, lo] => u16::from_be_bytes([*hi, *lo]),
                        _ => {
                            self.undecodable = true;
                            break;
                        }
                    };
                    match cmap.get(code) {
                        Some(text) => {
                            self.used_to_unicode = true;
                            if !text.is_empty() {
                                self.push_unit(text);
                            }
                        }
                        None => {
                            self.undecodable = true;
                        }
                    }
                    i += width;
                    self.advance(code);
                }
            }
        }
    }

    pub fn push_unit(&mut self, text: &str) {
        let reversed = self.marks.iter().any(|mark| mark.tag == b"ReversedChars");
        let line = self.line.f;
        let origin_x = self.ctm.a * self.line.e + self.ctm.c * self.line.f + self.ctm.e;
        let x_scale = self.ctm.a * self.line.a + self.ctm.c * self.line.b;
        let paint_x = origin_x + x_scale * self.pen;
        self.units.push(Unit {
            text: UnitText::new(text),
            line,
            paint_x,
            x_ok: x_scale != 0.0 && x_scale.is_finite() && origin_x.is_finite(),
            reversed,
            epoch: self.epoch,
        });
    }

    pub fn finish(self) -> Walk {
        Walk {
            units: self.units,
            used_actual_text: self.used_actual_text,
            used_to_unicode: self.used_to_unicode,
            used_encoding: self.used_encoding,
            undecodable: self.undecodable,
            refused_encoding: self.refused_encoding,
        }
    }
}

pub fn dict_actual_text(entries: &[(std::borrow::Cow<'_, [u8]>, Value<'_>)]) -> Option<String> {
    entries
        .iter()
        .find(|(key, _)| key.as_ref() == b"ActualText")
        .and_then(|(_, value)| match value {
            Value::Str(bytes) => Some(decode_text_string(bytes.as_ref())),
            _ => None,
        })
}

pub fn decode_text_string(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFE, 0xFF]) {
        return lossy_utf16(&bytes[2..], true);
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return lossy_utf16(&bytes[2..], false);
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    }
}

pub fn lossy_utf16(bytes: &[u8], big_endian: bool) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(bytes.len() / 2);
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        let pair = [bytes[i], bytes[i + 1]];
        units.push(if big_endian {
            u16::from_be_bytes(pair)
        } else {
            u16::from_le_bytes(pair)
        });
        i += 2;
    }
    char::decode_utf16(units)
        .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

pub fn walk(
    stream: &[u8],
    fonts: &HashMap<Vec<u8>, Font>,
    metrics: &HashMap<Vec<u8>, Metrics>,
) -> Walk {
    let tokens = tokenize(stream);
    let mut walker = Walker::new(fonts, metrics);
    let mut pending: Vec<Value<'_>> = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Op(op, _) => {
                walker.op(*op, &pending);
                pending.clear();
                i += 1;
            }
            Token::ArrStart | Token::DictStart => {
                let before = i;
                if let Some(val) = parse_value(&tokens, &mut i) {
                    pending.push(val);
                } else if i == before {
                    i += 1;
                }
            }
            Token::Name(name) => {
                pending.push(Value::Name(name.clone()));
                i += 1;
            }
            Token::Num(n) => {
                pending.push(Value::Num(*n));
                i += 1;
            }
            Token::Str(bytes) => {
                pending.push(Value::Str(bytes.clone()));
                i += 1;
            }
            Token::ArrEnd | Token::DictEnd | Token::Reference(..) => {
                i += 1;
            }
        }
    }
    walker.finish()
}

pub fn metrics_of(doc: &Document, dict: &Dictionary) -> Metrics {
    let subtype = dict
        .get(b"Subtype")
        .ok()
        .and_then(|value| value.as_name().ok())
        .map(|name| name.to_vec())
        .unwrap_or_default();
    let is_simple = SIMPLE_FONT_SUBTYPES.contains(&subtype.as_slice());
    let mut metrics = Metrics {
        two_byte: !is_simple,
        ..Metrics::default()
    };
    if is_simple {
        let first = dict
            .get(b"FirstChar")
            .ok()
            .and_then(as_number)
            .unwrap_or(0.0) as i64;
        if let Ok(Object::Array(widths)) = dict.get(b"Widths") {
            for (index, value) in widths.iter().enumerate() {
                if let Some(width) = as_number(value) {
                    let code = first + index as i64;
                    if (0..=u16::MAX as i64).contains(&code) {
                        metrics.widths.insert(code as u16, width / 1000.0);
                        metrics.known = true;
                    }
                }
            }
        }
        metrics.missing = dict
            .get(b"MissingWidth")
            .ok()
            .and_then(as_number)
            .unwrap_or(0.0)
            / 1000.0;
        return metrics;
    }
    let descendant = dict
        .get_deref(b"DescendantFonts", doc)
        .ok()
        .and_then(|value| match value {
            Object::Array(items) => items.first(),
            other => Some(other),
        });
    let cid = descendant
        .and_then(|value| resolve(doc, value))
        .and_then(|font| font.as_dict().ok().cloned());
    let Some(cid) = cid else {
        return metrics;
    };
    metrics.missing = cid.get(b"DW").ok().and_then(as_number).unwrap_or(1000.0) / 1000.0;
    let Ok(Object::Array(w)) = cid.get(b"W") else {
        return metrics;
    };
    let mut index = 0usize;
    while index + 1 < w.len() {
        let Some(first) = as_number(&w[index]) else {
            break;
        };
        match w.get(index + 1) {
            Some(next) if as_number(next).is_some() => {
                let Some(width) = w.get(index + 2).and_then(as_number) else {
                    break;
                };
                let last = as_number(next).unwrap_or(first);
                let mut code = first as i64;
                while code <= last as i64 && code <= u16::MAX as i64 {
                    metrics.widths.insert(code as u16, width / 1000.0);
                    metrics.known = true;
                    code += 1;
                }
                index += 3;
            }
            Some(Object::Array(run)) => {
                for (offset, value) in run.iter().enumerate() {
                    if let Some(width) = as_number(value) {
                        let code = first as i64 + offset as i64;
                        if (0..=u16::MAX as i64).contains(&code) {
                            metrics.widths.insert(code as u16, width / 1000.0);
                            metrics.known = true;
                        }
                    }
                }
                index += 2;
            }
            _ => break,
        }
    }
    metrics
}

pub fn collect_fonts(
    doc: &Document,
    page_id: ObjectId,
) -> (HashMap<Vec<u8>, Font>, HashMap<Vec<u8>, Metrics>) {
    let mut fonts = HashMap::new();
    let mut metrics = HashMap::new();
    let resources = match find_resources(doc, page_id) {
        Some(resources) => resources,
        None => return (fonts, metrics),
    };
    let font_dict = match resources
        .get_deref(b"Font", doc)
        .ok()
        .and_then(|value| value.as_dict().ok())
    {
        Some(dict) => dict,
        None => return (fonts, metrics),
    };

    for (name, value) in font_dict.iter() {
        let font = match resolve(doc, value) {
            Some(font) => font,
            None => continue,
        };
        let dict = match font.as_dict() {
            Ok(dict) => dict,
            Err(_) => continue,
        };
        metrics.insert(name.clone(), metrics_of(doc, dict));
        // `/ToUnicode` wins when it is readable and maps anything.
        if let Some(cmap) = to_unicode(doc, dict) {
            fonts.insert(name.clone(), Font::ToUnicode(cmap));
            continue;
        }
        // Only simple fonts have a byte-level `/Encoding`. A CID font without a
        // usable `/ToUnicode` has no mapping we could read — it stays out of the
        // map and its codes surface as `unsupported_broken_to_unicode`.
        let is_simple = dict
            .get_deref(b"Subtype", doc)
            .ok()
            .and_then(|value| match value {
                Object::Name(subtype) => Some(subtype.as_slice()),
                _ => None,
            })
            .is_some_and(|subtype| SIMPLE_FONT_SUBTYPES.contains(&subtype));
        if !is_simple {
            continue;
        }
        let font = match simple_font_encoding(doc, dict) {
            Some(encoding) => Font::Simple(encoding),
            None => Font::Refused,
        };
        fonts.insert(name.clone(), font);
    }
    (fonts, metrics)
}

fn to_unicode(doc: &Document, font: &Dictionary) -> Option<crate::text::cmap::ToUnicode> {
    let value = font.get_deref(b"ToUnicode", doc).ok()?;
    let stream = value.as_stream().ok()?;
    let bytes = stream.decompressed_content().ok()?;
    let cmap = crate::text::cmap::ToUnicode::parse(&bytes);
    if cmap.is_empty() {
        None
    } else {
        Some(cmap)
    }
}

fn simple_font_encoding(
    doc: &Document,
    font: &Dictionary,
) -> Option<crate::text::encoding::SimpleEncoding> {
    use crate::text::encoding::{BaseEncoding, SimpleEncoding};
    let encoding = resolve(doc, font.get(b"Encoding").ok()?)?;
    match encoding {
        Object::Name(name) => BaseEncoding::by_name(name).map(SimpleEncoding::new),
        Object::Dictionary(entries) => {
            let base = match entries.get(b"BaseEncoding") {
                Ok(value) => match resolve(doc, value) {
                    Some(Object::Name(name)) => BaseEncoding::by_name(name)?,
                    _ => return None,
                },
                Err(_) => {
                    if looks_symbolic(doc, font) {
                        return None;
                    }
                    BaseEncoding::Standard
                }
            };
            let mut decoded = SimpleEncoding::new(base);
            decoded.apply_differences(differences(doc, entries));
            Some(decoded)
        }
        _ => None,
    }
}

fn differences(doc: &Document, entries: &Dictionary) -> Vec<(u8, String)> {
    let mut pairs = Vec::new();
    let array = match entries.get_deref(b"Differences", doc) {
        Ok(Object::Array(array)) => array,
        _ => return pairs,
    };
    let mut code: Option<u8> = None;
    for item in array {
        match item {
            Object::Integer(value) => code = u8::try_from(*value).ok(),
            Object::Name(name) => {
                if let Some(current) = code {
                    pairs.push((current, String::from_utf8_lossy(name).into_owned()));
                    code = current.checked_add(1);
                }
            }
            _ => {}
        }
    }
    pairs
}

fn looks_symbolic(doc: &Document, font: &Dictionary) -> bool {
    let base_font = match font.get_deref(b"BaseFont", doc) {
        Ok(Object::Name(name)) => name.clone(),
        _ => return false,
    };
    let lower = base_font.to_ascii_lowercase();
    let needles: &[&[u8]] = &[b"symbol", b"zapfdingbats", b"dingbats"];
    needles
        .iter()
        .any(|needle| lower.windows(needle.len()).any(|w| w == *needle))
}

pub fn find_resources(doc: &Document, page_id: ObjectId) -> Option<&Dictionary> {
    let mut current = page_id;
    while let Ok(page) = doc.get_object(current).and_then(|obj| obj.as_dict()) {
        if let Ok(resources) = page.get_deref(b"Resources", doc) {
            if let Ok(dict) = resources.as_dict() {
                return Some(dict);
            }
        }
        match page
            .get(b"Parent")
            .ok()
            .and_then(|obj| obj.as_reference().ok())
        {
            Some(parent) => current = parent,
            None => break,
        }
    }
    None
}

pub fn resolve<'a>(doc: &'a Document, object: &'a Object) -> Option<&'a Object> {
    match object {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}
