//! PDF content-stream / CMap lexer and value parser.
//!
//! Total: malformed input yields best-effort tokens instead of panicking — a PDF is
//! untrusted input. The grammar covered is the text-relevant subset: names, numbers,
//! literal and hex strings, arrays, dictionaries, operators and `%` comments.
//! Inline images (`BI … ID <bytes> EI`) are not part of any text path we walk.

/// A lexical token of a content stream or a CMap.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Name(Vec<u8>),
    Num(f64),
    Str(Vec<u8>),
    ArrStart,
    ArrEnd,
    DictStart,
    DictEnd,
    Op(Vec<u8>),
    /// A resolved `n m R` indirect reference.
    Reference(u32, u32),
}

/// A parsed value (dictionary/array members, operator operands).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Name(Vec<u8>),
    Num(f64),
    Str(Vec<u8>),
    Dict(Vec<(Vec<u8>, Value)>),
    Arr(Vec<Value>),
    Reference(u32, u32),
}

impl Value {
    /// The bytes of a name or string value, if this is one.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Name(n) | Value::Str(n) => Some(n),
            _ => None,
        }
    }
}

fn is_ws(b: u8) -> bool {
    matches!(b, 0x00 | 0x09 | 0x0A | 0x0C | 0x0D | 0x20)
}

fn is_delim(b: u8) -> bool {
    matches!(b, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Tokenize a content stream or a CMap. Never fails.
pub fn tokenize(data: &[u8]) -> Vec<Token> {
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
                    out.push(Token::Op(data[start..i].to_vec()));
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
fn literal_string(data: &[u8], start: usize) -> (Vec<u8>, usize) {
    let mut out = Vec::new();
    let mut i = start + 1;
    let mut depth = 1usize;
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
    (out, i)
}

/// Hex string starting at `start` (`data[start] == b'<'`), padded if the count is odd.
fn hex_string(data: &[u8], start: usize) -> (Vec<u8>, usize) {
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
    (out, i)
}

/// Name `#XX` escapes.
fn unescape_name(bytes: &[u8]) -> Vec<u8> {
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
    out
}

/// Parse one value starting at `*i`, advancing `*i` past it.
pub fn parse_value(tokens: &[Token], i: &mut usize) -> Option<Value> {
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
            if let (Some(Token::Num(m)), Some(Token::Op(op))) = (tokens.get(*i + 1), tokens.get(*i + 2))
            {
                if op.as_slice() == b"R" {
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
            let mut entries: Vec<(Vec<u8>, Value)> = Vec::new();
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
                                // A key with no parsable value: stop here, the caller
                                // resumes at the offending token.
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
