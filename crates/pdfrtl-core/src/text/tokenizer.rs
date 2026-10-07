//! PDF content-stream / CMap zero-copy lexer and typed value parser.
//!
//! Total: malformed input yields best-effort tokens instead of panicking — a PDF is
//! untrusted input. The grammar covered is the text-relevant subset: names, numbers,
//! literal and hex strings, arrays, dictionaries, operators and `%` comments.
//! Inline images (`BI … ID <bytes> EI`) are not part of any text path we walk.

use std::borrow::Cow;

/// Typed opcode for common PDF content-stream and CMap operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOp {
    // Text object operators
    BT,
    ET,
    // Graphics state / CTM
    Q,    // q
    PopQ, // Q
    Cm,   // cm
    // Text positioning
    Tm,
    Td,
    TD,
    TStar, // T*
    // Text state
    TL,
    Tf,
    Tc,
    Tw,
    Tz,
    // Text showing
    Tj,
    TJ,
    Quote,       // '
    DoubleQuote, // "
    // Marked content
    BMC,
    BDC,
    EMC,
    // CMap operators
    BeginCodeSpaceRange,
    EndCodeSpaceRange,
    BeginBfChar,
    EndBfChar,
    BeginBfRange,
    EndBfRange,
    // Other unrecognized / unhandled operator
    Other,
}

impl TextOp {
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        match bytes {
            b"BT" => TextOp::BT,
            b"ET" => TextOp::ET,
            b"q" => TextOp::Q,
            b"Q" => TextOp::PopQ,
            b"cm" => TextOp::Cm,
            b"Tm" => TextOp::Tm,
            b"Td" => TextOp::Td,
            b"TD" => TextOp::TD,
            b"T*" => TextOp::TStar,
            b"TL" => TextOp::TL,
            b"Tf" => TextOp::Tf,
            b"Tc" => TextOp::Tc,
            b"Tw" => TextOp::Tw,
            b"Tz" => TextOp::Tz,
            b"Tj" => TextOp::Tj,
            b"TJ" => TextOp::TJ,
            b"'" => TextOp::Quote,
            b"\"" => TextOp::DoubleQuote,
            b"BMC" => TextOp::BMC,
            b"BDC" => TextOp::BDC,
            b"EMC" => TextOp::EMC,
            b"begincodespacerange" => TextOp::BeginCodeSpaceRange,
            b"endcodespacerange" => TextOp::EndCodeSpaceRange,
            b"beginbfchar" => TextOp::BeginBfChar,
            b"endbfchar" => TextOp::EndBfChar,
            b"beginbfrange" => TextOp::BeginBfRange,
            b"endbfrange" => TextOp::EndBfRange,
            _ => TextOp::Other,
        }
    }
}

/// A lexical token of a content stream or a CMap borrowing from the stream.
#[derive(Debug, Clone, PartialEq)]
pub enum Token<'a> {
    Name(Cow<'a, [u8]>),
    Num(f64),
    Str(Cow<'a, [u8]>),
    ArrStart,
    ArrEnd,
    DictStart,
    DictEnd,
    Op(TextOp, &'a [u8]),
    /// A resolved `n m R` indirect reference.
    Reference(u32, u32),
}

impl<'a> Token<'a> {
    #[inline]
    pub fn op_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Token::Op(_, bytes) => Some(bytes),
            _ => None,
        }
    }

    #[inline]
    pub fn str_bytes(&self) -> Option<&[u8]> {
        match self {
            Token::Str(cow) => Some(cow.as_ref()),
            _ => None,
        }
    }

    #[inline]
    pub fn name_bytes(&self) -> Option<&[u8]> {
        match self {
            Token::Name(cow) => Some(cow.as_ref()),
            _ => None,
        }
    }
}

/// A parsed value (dictionary/array members, operator operands).
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Name(Cow<'a, [u8]>),
    Num(f64),
    Str(Cow<'a, [u8]>),
    Dict(Vec<(Cow<'a, [u8]>, Value<'a>)>),
    Arr(Vec<Value<'a>>),
    Reference(u32, u32),
}

impl<'a> Value<'a> {
    /// The bytes of a name or string value, if this is one.
    #[inline]
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Name(n) | Value::Str(n) => Some(n.as_ref()),
            _ => None,
        }
    }

    #[inline]
    pub fn as_num(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }
}

#[inline]
fn is_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

#[inline]
fn is_delim(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

#[inline]
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Tokenize a content stream or a CMap with zero copy for names, operators, and unescaped strings.
pub fn tokenize<'a>(data: &'a [u8]) -> Vec<Token<'a>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        let b = data[i];
        match b {
            b'%' => {
                while i < data.len() && data[i] != b'\n' && data[i] != b'\r' {
                    i += 1;
                }
            }
            b'/' => {
                i += 1;
                let start = i;
                while i < data.len() && !is_ws(data[i]) && !is_delim(data[i]) {
                    i += 1;
                }
                out.push(Token::Name(unescape_name(&data[start..i])));
            }
            b'(' => {
                let (s, ni) = literal_string(data, i);
                out.push(Token::Str(s));
                i = ni;
            }
            b'<' => {
                if i + 1 < data.len() && data[i + 1] == b'<' {
                    out.push(Token::DictStart);
                    i += 2;
                } else {
                    let (s, ni) = hex_string(data, i);
                    out.push(Token::Str(s));
                    i = ni;
                }
            }
            b'>' => {
                if i + 1 < data.len() && data[i + 1] == b'>' {
                    out.push(Token::DictEnd);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            b'[' => {
                out.push(Token::ArrStart);
                i += 1;
            }
            b']' => {
                out.push(Token::ArrEnd);
                i += 1;
            }
            b'{' | b'}' => i += 1,
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let (n, ni) = number(data, i);
                out.push(Token::Num(n));
                i = ni;
            }
            _ if is_ws(b) => i += 1,
            _ => {
                let start = i;
                while i < data.len() && !is_ws(data[i]) && !is_delim(data[i]) {
                    i += 1;
                }
                if i > start {
                    let slice = &data[start..i];
                    out.push(Token::Op(TextOp::from_bytes(slice), slice));
                } else {
                    i += 1;
                }
            }
        }
    }
    out
}

fn number(data: &[u8], start: usize) -> (f64, usize) {
    let mut i = start;
    if i < data.len() && (data[i] == b'+' || data[i] == b'-') {
        i += 1;
    }
    while i < data.len() && data[i].is_ascii_digit() {
        i += 1;
    }
    if i < data.len() && data[i] == b'.' {
        i += 1;
        while i < data.len() && data[i].is_ascii_digit() {
            i += 1;
        }
    }
    if i < data.len() && (data[i] == b'e' || data[i] == b'E') {
        let save = i;
        i += 1;
        if i < data.len() && (data[i] == b'+' || data[i] == b'-') {
            i += 1;
        }
        let exp_digits = i;
        while i < data.len() && data[i].is_ascii_digit() {
            i += 1;
        }
        if i == exp_digits {
            i = save;
        }
    }
    if i == start {
        return (0.0, start + 1);
    }
    let text = std::str::from_utf8(&data[start..i]).unwrap_or("0");
    (text.parse::<f64>().unwrap_or(0.0), i)
}

/// Literal string starting at `start` (`data[start] == b'('`), with escapes and nesting.
fn literal_string<'a>(data: &'a [u8], start: usize) -> (Cow<'a, [u8]>, usize) {
    let mut i = start + 1;
    let mut depth = 1usize;
    let mut has_escape = false;

    // Fast path: find end without escape
    let body_start = i;
    while i < data.len() {
        match data[i] {
            b'\\' => {
                has_escape = true;
                break;
            }
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return (Cow::Borrowed(&data[body_start..i]), i + 1);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }

    if !has_escape && depth == 0 {
        return (Cow::Borrowed(&data[body_start..i]), i + 1);
    }

    // Slow path with escape decoding
    let mut out = Vec::with_capacity(i - start);
    // Copy what we've seen so far in body_start..i
    out.extend_from_slice(&data[body_start..i]);
    while i < data.len() {
        let b = data[i];
        match b {
            b'\\' => {
                i += 1;
                if i >= data.len() {
                    break;
                }
                let e = data[i];
                match e {
                    b'n' => {
                        out.push(b'\n');
                        i += 1;
                    }
                    b'r' => {
                        out.push(b'\r');
                        i += 1;
                    }
                    b't' => {
                        out.push(b'\t');
                        i += 1;
                    }
                    b'b' => {
                        out.push(0x08);
                        i += 1;
                    }
                    b'f' => {
                        out.push(0x0c);
                        i += 1;
                    }
                    b'(' | b')' | b'\\' => {
                        out.push(e);
                        i += 1;
                    }
                    b'\n' => i += 1,
                    b'\r' => {
                        i += 1;
                        if i < data.len() && data[i] == b'\n' {
                            i += 1;
                        }
                    }
                    b'0'..=b'7' => {
                        let mut v = 0u32;
                        let mut n = 0;
                        while n < 3 && i < data.len() && (b'0'..=b'7').contains(&data[i]) {
                            v = v * 8 + u32::from(data[i] - b'0');
                            i += 1;
                            n += 1;
                        }
                        out.push((v & 0xFF) as u8);
                    }
                    _ => {
                        out.push(e);
                        i += 1;
                    }
                }
            }
            b'(' => {
                depth += 1;
                out.push(b'(');
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    break;
                }
                out.push(b')');
            }
            _ => {
                out.push(b);
                i += 1;
            }
        }
    }
    (Cow::Owned(out), i)
}

/// Hex string starting at `start` (`data[start] == b'<'`), padded if the count is odd.
fn hex_string<'a>(data: &'a [u8], start: usize) -> (Cow<'a, [u8]>, usize) {
    let mut out = Vec::new();
    let mut i = start + 1;
    let mut hi: Option<u8> = None;
    while i < data.len() {
        let b = data[i];
        if b == b'>' {
            i += 1;
            break;
        }
        if is_ws(b) {
            i += 1;
            continue;
        }
        if let Some(v) = hex_val(b) {
            match hi.take() {
                Some(h) => out.push((h << 4) | v),
                None => hi = Some(v),
            }
        }
        i += 1;
    }
    if let Some(h) = hi {
        out.push(h << 4);
    }
    (Cow::Owned(out), i)
}

/// Name `#XX` escapes.
fn unescape_name<'a>(bytes: &'a [u8]) -> Cow<'a, [u8]> {
    if !bytes.contains(&b'#') {
        return Cow::Borrowed(bytes);
    }
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'#' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    Cow::Owned(out)
}

/// Parse one value starting at `*i`, advancing `*i` past it.
pub fn parse_value<'a>(tokens: &[Token<'a>], i: &mut usize) -> Option<Value<'a>> {
    let tok = tokens.get(*i)?;
    match tok {
        Token::Name(n) => {
            *i += 1;
            Some(Value::Name(n.clone()))
        }
        Token::Str(s) => {
            *i += 1;
            Some(Value::Str(s.clone()))
        }
        Token::Num(n) => {
            if let (Some(Token::Num(m)), Some(Token::Op(_, op_bytes))) =
                (tokens.get(*i + 1), tokens.get(*i + 2))
            {
                if *op_bytes == b"R" {
                    let (a, b) = (to_u32(*n), to_u32(*m));
                    *i += 3;
                    return Some(Value::Reference(a, b));
                }
            }
            *i += 1;
            Some(Value::Num(*n))
        }
        Token::DictStart => {
            *i += 1;
            let mut entries = Vec::new();
            loop {
                match tokens.get(*i) {
                    None => return Some(Value::Dict(entries)),
                    Some(Token::DictEnd) => {
                        *i += 1;
                        return Some(Value::Dict(entries));
                    }
                    Some(Token::Name(key)) => {
                        let key = key.clone();
                        *i += 1;
                        match parse_value(tokens, i) {
                            Some(v) => entries.push((key, v)),
                            None => {
                                return Some(Value::Dict(entries));
                            }
                        }
                    }
                    Some(_) => *i += 1,
                }
            }
        }
        Token::ArrStart => {
            *i += 1;
            let mut items = Vec::new();
            loop {
                match tokens.get(*i) {
                    None => return Some(Value::Arr(items)),
                    Some(Token::ArrEnd) => {
                        *i += 1;
                        return Some(Value::Arr(items));
                    }
                    Some(_) => match parse_value(tokens, i) {
                        Some(v) => items.push(v),
                        None => *i += 1,
                    },
                }
            }
        }
        _ => None,
    }
}

fn to_u32(n: f64) -> u32 {
    if n.is_finite() && n >= 0.0 && n <= u32::MAX as f64 {
        n as u32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescaped_literal_string_is_borrowed() {
        let stream = b"(Hello World) Tj";
        let tokens = tokenize(stream);
        assert_eq!(tokens.len(), 2);
        match &tokens[0] {
            Token::Str(Cow::Borrowed(b)) => assert_eq!(*b, b"Hello World"),
            other => panic!("expected borrowed string, got {other:?}"),
        }
        match &tokens[1] {
            Token::Op(TextOp::Tj, slice) => assert_eq!(*slice, b"Tj"),
            other => panic!("expected TextOp::Tj, got {other:?}"),
        }
    }

    #[test]
    fn escaped_literal_string_is_decoded() {
        let stream = b"(Hello\\nWorld\\t\\101) Tj";
        let tokens = tokenize(stream);
        assert_eq!(tokens.len(), 2);
        match &tokens[0] {
            Token::Str(Cow::Owned(b)) => assert_eq!(b.as_slice(), b"Hello\nWorld\tA"),
            other => panic!("expected owned string, got {other:?}"),
        }
    }

    #[test]
    fn hex_string_decoding() {
        let stream = b"<00410042> Tj";
        let tokens = tokenize(stream);
        assert_eq!(tokens.len(), 2);
        match &tokens[0] {
            Token::Str(Cow::Owned(b)) => assert_eq!(b.as_slice(), &[0x00, 0x41, 0x00, 0x42]),
            other => panic!("expected owned hex string, got {other:?}"),
        }
    }

    #[test]
    fn name_borrowed_when_no_hash() {
        let stream = b"/F1 12 Tf";
        let tokens = tokenize(stream);
        assert_eq!(tokens.len(), 3);
        match &tokens[0] {
            Token::Name(Cow::Borrowed(b)) => assert_eq!(*b, b"F1"),
            other => panic!("expected borrowed name, got {other:?}"),
        }
        assert_eq!(tokens[1], Token::Num(12.0));
        assert_eq!(tokens[2], Token::Op(TextOp::Tf, b"Tf"));
    }

    #[test]
    fn name_escaped_with_hash() {
        let stream = b"/Name#20With#20Spaces";
        let tokens = tokenize(stream);
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::Name(Cow::Owned(b)) => assert_eq!(b.as_slice(), b"Name With Spaces"),
            other => panic!("expected owned name, got {other:?}"),
        }
    }

    #[test]
    fn parse_array_and_dict_values() {
        let stream = b"[ (item1) 42 /Item2 ]";
        let tokens = tokenize(stream);
        let mut i = 0;
        let val = parse_value(&tokens, &mut i).expect("parsed array");
        match val {
            Value::Arr(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].as_bytes(), Some(b"item1".as_slice()));
                assert_eq!(items[1], Value::Num(42.0));
                assert_eq!(items[2].as_bytes(), Some(b"Item2".as_slice()));
            }
            other => panic!("expected Arr, got {other:?}"),
        }
    }

    #[test]
    fn all_text_operators_parsed_correctly() {
        let ops = b"BT ET q Q cm Tm Td TD T* TL Tf Tc Tw Tz Tj TJ ' \" BMC BDC EMC";
        let tokens = tokenize(ops);
        let expected = [
            TextOp::BT,
            TextOp::ET,
            TextOp::Q,
            TextOp::PopQ,
            TextOp::Cm,
            TextOp::Tm,
            TextOp::Td,
            TextOp::TD,
            TextOp::TStar,
            TextOp::TL,
            TextOp::Tf,
            TextOp::Tc,
            TextOp::Tw,
            TextOp::Tz,
            TextOp::Tj,
            TextOp::TJ,
            TextOp::Quote,
            TextOp::DoubleQuote,
            TextOp::BMC,
            TextOp::BDC,
            TextOp::EMC,
        ];
        assert_eq!(tokens.len(), expected.len());
        for (tok, exp) in tokens.iter().zip(expected.iter()) {
            match tok {
                Token::Op(op, _) => assert_eq!(op, exp),
                other => panic!("expected Op, got {other:?}"),
            }
        }
    }
}
