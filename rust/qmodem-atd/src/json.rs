//! Minimal JSON value model, parser and serializer (std-only, replaces
//! serde/serde_json for the Unix-socket line protocol).
//!
//! Covers exactly what the daemon protocol needs: objects (insertion
//! ordered), arrays, strings with the full escape set (including `\uXXXX`
//! surrogate pairs), integers, floats, booleans and `null`.

use std::fmt;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    /// JSON number that round-trips losslessly through `i64`.
    Int(i64),
    /// JSON number with a fractional/exponent part, or beyond `i64` range.
    Float(f64),
    String(String),
    Array(Vec<Value>),
    /// Insertion-ordered object (keeps protocol output stable).
    Object(Vec<(String, Value)>),
}

static NULL: Value = Value::Null;

impl Value {
    /// Object member lookup (`None` for non-objects / missing keys).
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Int(i) if *i >= 0 => Some(*i as u64),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(x) => Some(*x),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(pairs) => Some(pairs),
            _ => None,
        }
    }
}

/// `value["key"]` / `value[0]` accessors; missing members yield `Null`
/// (mirrors serde_json's lenient indexing used by the tests).
impl std::ops::Index<&str> for Value {
    type Output = Value;
    fn index(&self, key: &str) -> &Value {
        self.get(key).unwrap_or(&NULL)
    }
}

impl std::ops::Index<usize> for Value {
    type Output = Value;
    fn index(&self, idx: usize) -> &Value {
        match self {
            Value::Array(items) => items.get(idx).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

macro_rules! value_partial_eq_int {
    ($($t:ty),*) => {
        $(impl PartialEq<$t> for Value {
            fn eq(&self, other: &$t) -> bool {
                self.as_i64() == Some(i64::try_from(*other).unwrap_or(i64::MAX))
            }
        })*
    };
}

value_partial_eq_int!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

macro_rules! value_from_int {
    ($($t:ty),*) => {
        $(impl From<$t> for Value {
            fn from(v: $t) -> Value { Value::Int(i64::from(v)) }
        })*
    };
}

value_from_int!(i8, i16, i32, u8, u16, u32);

impl From<usize> for Value {
    fn from(v: usize) -> Value {
        Value::Int(v as i64)
    }
}

impl From<isize> for Value {
    fn from(v: isize) -> Value {
        Value::Int(v as i64)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Value {
        Value::Int(v)
    }
}

impl From<u64> for Value {
    fn from(v: u64) -> Value {
        // Numbers beyond i64 range keep full u64 precision as float.
        if v <= i64::MAX as u64 {
            Value::Int(v as i64)
        } else {
            Value::Float(v as f64)
        }
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Value {
        Value::Float(v)
    }
}

impl From<f32> for Value {
    fn from(v: f32) -> Value {
        Value::Float(f64::from(v))
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Value {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Value {
        Value::String(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Value {
        Value::String(v)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Value {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}

impl From<Vec<(String, Value)>> for Value {
    fn from(v: Vec<(String, Value)>) -> Value {
        Value::Object(v)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Value {
        match v {
            Some(x) => x.into(),
            None => Value::Null,
        }
    }
}

/// Conversion trait used by the [`json!`] macro so `$v:expr` accepts any
/// `Into<Value>` argument uniformly.
pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl<T: Into<Value>> IntoValue for T {
    fn into_value(self) -> Value {
        self.into()
    }
}

/// Build a [`Value`] literal:
///
/// * `json!({"key": value, ...})` — object (keys must be literals);
/// * `json!([value, ...])` — array;
/// * `json!(expr)` — any `Into<Value>` expression.
///
/// Nested containers are written as `json!({...})` / `json!([...])`
/// (macro-call expressions), not as raw brace/bracket literals.
#[macro_export]
macro_rules! json {
    ({ $($k:literal : $v:expr),* $(,)? }) => {{
        let pairs: Vec<(String, $crate::json::Value)> = vec![
            $( ($k.to_string(), $crate::json::IntoValue::into_value($v)) ),*
        ];
        $crate::json::Value::Object(pairs)
    }};
    ([ $($v:expr),* $(,)? ]) => {{
        let items: Vec<$crate::json::Value> =
            vec![ $( $crate::json::IntoValue::into_value($v) ),* ];
        $crate::json::Value::Array(items)
    }};
    ($v:expr) => {
        $crate::json::IntoValue::into_value($v)
    };
}

/// Compact serialization (`Display`); also available via `.to_string()`.
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => f.write_str(if *b { "true" } else { "false" }),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(x) => {
                if x.is_finite() {
                    write!(f, "{x}")
                } else {
                    // serde_json parity: non-finite floats serialize as null.
                    f.write_str("null")
                }
            }
            Value::String(s) => write_escaped(f, s),
            Value::Array(items) => {
                f.write_str("[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }
            Value::Object(pairs) => {
                f.write_str("{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write_escaped(f, k)?;
                    f.write_str(":")?;
                    write!(f, "{v}")?;
                }
                f.write_str("}")
            }
        }
    }
}

fn write_escaped(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_str("\"")?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            '\u{08}' => f.write_str("\\b")?,
            '\u{0c}' => f.write_str("\\f")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => write!(f, "{c}")?,
        }
    }
    f.write_str("\"")
}

/// Parse error with byte offset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub msg: String,
    pub pos: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "json error at byte {}: {}", self.pos, self.msg)
    }
}

impl std::error::Error for JsonError {}

/// Parse a complete JSON document (trailing whitespace allowed).
pub fn parse(input: &str) -> Result<Value, JsonError> {
    let mut p = Parser { b: input.as_bytes(), pos: 0 };
    p.skip_ws();
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos != p.b.len() {
        return Err(p.err("trailing characters"));
    }
    Ok(v)
}

struct Parser<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> JsonError {
        JsonError { msg: msg.to_string(), pos: self.pos }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, c: u8) -> Result<(), JsonError> {
        if self.peek() == Some(c) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{c}'")))
        }
    }

    fn parse_value(&mut self) -> Result<Value, JsonError> {
        match self.peek() {
            None => Err(self.err("unexpected end of input")),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(Value::String(self.parse_string()?)),
            Some(b't') => self.parse_lit("true", Value::Bool(true)),
            Some(b'f') => self.parse_lit("false", Value::Bool(false)),
            Some(b'n') => self.parse_lit("null", Value::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.parse_number(),
            Some(_) => Err(self.err("unexpected character")),
        }
    }

    fn parse_lit(&mut self, lit: &str, v: Value) -> Result<Value, JsonError> {
        if self.b[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(v)
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn parse_object(&mut self) -> Result<Value, JsonError> {
        self.expect(b'{')?;
        let mut pairs: Vec<(String, Value)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let val = self.parse_value()?;
            pairs.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(pairs));
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn parse_array(&mut self) -> Result<Value, JsonError> {
        self.expect(b'[')?;
        let mut items: Vec<Value> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.parse_value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            self.pos += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
                    self.pos += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.parse_hex4()?;
                            let cp = if (0xd800..0xdc00).contains(&hi) {
                                // Must be followed by a low surrogate escape.
                                if self.peek() != Some(b'\\') {
                                    return Err(self.err("lone high surrogate"));
                                }
                                self.pos += 1;
                                if self.peek() != Some(b'u') {
                                    return Err(self.err("lone high surrogate"));
                                }
                                self.pos += 1;
                                let lo = self.parse_hex4()?;
                                if !(0xdc00..0xe000).contains(&lo) {
                                    return Err(self.err("invalid low surrogate"));
                                }
                                0x1_0000
                                    + ((u32::from(hi) - 0xd800) << 10)
                                    + (u32::from(lo) - 0xdc00)
                            } else if (0xdc00..0xe000).contains(&hi) {
                                return Err(self.err("lone low surrogate"));
                            } else {
                                u32::from(hi)
                            };
                            out.push(
                                char::from_u32(cp)
                                    .ok_or_else(|| self.err("invalid code point"))?,
                            );
                        }
                        _ => return Err(self.err("invalid escape")),
                    }
                }
                c if c < 0x20 => {
                    return Err(self.err("control character in string"));
                }
                c => {
                    // Input is a &str, so multi-byte UTF-8 is well-formed;
                    // copy the whole character from the original bytes.
                    let start = self.pos - 1;
                    let end = start + utf8_len(c);
                    if end > self.b.len() {
                        return Err(self.err("truncated utf-8"));
                    }
                    let s = std::str::from_utf8(&self.b[start..end])
                        .map_err(|_| self.err("invalid utf-8"))?;
                    out.push_str(s);
                    self.pos = end;
                }
            }
        }
    }

    fn parse_hex4(&mut self) -> Result<u16, JsonError> {
        if self.pos + 4 > self.b.len() {
            return Err(self.err("truncated \\u escape"));
        }
        let s = std::str::from_utf8(&self.b[self.pos..self.pos + 4])
            .map_err(|_| self.err("bad \\u escape"))?;
        let v = u16::from_str_radix(s, 16).map_err(|_| self.err("bad \\u escape"))?;
        self.pos += 4;
        Ok(v)
    }

    fn parse_number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let mut digits = 0;
        while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
            self.pos += 1;
            digits += 1;
        }
        if digits == 0 {
            return Err(self.err("invalid number"));
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            let mut frac = 0;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
                frac += 1;
            }
            if frac == 0 {
                return Err(self.err("invalid number"));
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            let mut exp = 0;
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.pos += 1;
                exp += 1;
            }
            if exp == 0 {
                return Err(self.err("invalid number"));
            }
        }
        let text = std::str::from_utf8(&self.b[start..self.pos])
            .map_err(|_| self.err("invalid number"))?;
        if !is_float {
            if let Ok(i) = text.parse::<i64>() {
                return Ok(Value::Int(i));
            }
        }
        text.parse::<f64>()
            .map(Value::Float)
            .map_err(|_| self.err("invalid number"))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_object_array_scalar() {
        let src = r#"{"cmd":"sendat","port":"/dev/ttyUSB2","timeout_ms":3000,"retry":true,"tags":["a","b"],"ratio":2.5,"none":null}"#;
        let v = parse(src).unwrap();
        assert_eq!(v["cmd"].as_str(), Some("sendat"));
        assert_eq!(v["timeout_ms"].as_u64(), Some(3000));
        assert_eq!(v["retry"].as_bool(), Some(true));
        assert_eq!(v["tags"].as_array().unwrap().len(), 2);
        assert_eq!(v["ratio"].as_f64(), Some(2.5));
        assert!(matches!(v["none"], Value::Null));
        // Compact re-serialization parses back to the same value.
        let rt = parse(&v.to_string()).unwrap();
        assert_eq!(v, rt);
        // Key order is preserved on output.
        assert!(v.to_string().starts_with(r#"{"cmd":"sendat","#));
    }

    #[test]
    fn string_escapes_roundtrip() {
        let src = r#"{"s":"line\nquote\"back\\slash/tabs\t\b\f and 😀"}"#;
        let v = parse(src).unwrap();
        assert_eq!(
            v["s"].as_str(),
            Some("line\nquote\"back\\slash/tabs\t\u{08}\u{0c} and \u{1f600}")
        );
        // Serializing re-escapes everything; the output is valid JSON again.
        let out = v.to_string();
        assert!(out.contains("\\n"));
        assert!(out.contains("\\\""));
        assert!(out.contains("\\\\"));
        assert!(out.contains("\\t"));
        let rt = parse(&out).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn unicode_escape_forms() {
        // BMP escape.
        let v = parse(r#""A""#).unwrap();
        assert_eq!(v.as_str(), Some("A"));
        // Surrogate pair (U+1F600).
        let v = parse(r#""😀""#).unwrap();
        assert_eq!(v.as_str(), Some("\u{1f600}"));
        // Lone surrogates are rejected.
        assert!(parse(r#""\ud83d""#).is_err());
        assert!(parse(r#""\ude00""#).is_err());
        assert!(parse(r#""\ud83dX""#).is_err());
        // Raw non-ASCII UTF-8 passes through and serializes unescaped.
        let v = parse("\"你好\"").unwrap();
        assert_eq!(v.as_str(), Some("你好"));
        assert_eq!(v.to_string(), "\"你好\"");
    }

    #[test]
    fn serialize_control_chars() {
        let v = Value::String("\u{01}\u{1f}".to_string());
        assert_eq!(v.to_string(), "\"\\u0001\\u001f\"");
        let rt = parse(&v.to_string()).unwrap();
        assert_eq!(v, rt);
    }

    #[test]
    fn number_forms() {
        assert_eq!(parse("0").unwrap(), Value::Int(0));
        assert_eq!(parse("-17").unwrap(), Value::Int(-17));
        assert_eq!(parse("9223372036854775807").unwrap(), Value::Int(i64::MAX));
        // Beyond i64 range -> float.
        assert!(matches!(parse("9223372036854775808").unwrap(), Value::Float(_)));
        assert_eq!(parse("2.5").unwrap(), Value::Float(2.5));
        assert_eq!(parse("-1.25e2").unwrap(), Value::Float(-125.0));
        assert_eq!(parse("7E3").unwrap(), Value::Float(7000.0));
        // Malformed numbers are rejected.
        assert!(parse("-").is_err());
        assert!(parse("1.").is_err());
        assert!(parse("1e").is_err());
        assert!(parse("01x").is_err());
    }

    #[test]
    fn parse_errors() {
        assert!(parse("").is_err());
        assert!(parse("{").is_err());
        assert!(parse("{\"a\":1,}").is_err());
        assert!(parse("[1 2]").is_err());
        assert!(parse("nul").is_err());
        assert!(parse("\"unterminated").is_err());
        assert!(parse("\"bad\\x\"").is_err());
        assert!(parse("1 2").is_err());
        assert!(parse("{}extra").is_err());
    }

    #[test]
    fn macro_construction_and_indexing() {
        let v = json!({
            "status": "ok",
            "elapsed_ms": 12u64,
            "lines": vec!["a".to_string(), "b".to_string()],
            "nested": json!({"x": 1}),
            "arr": json!([1, 2, 3]),
            "maybe": None::<String>,
        });
        assert_eq!(v["status"], "ok");
        assert_eq!(v["elapsed_ms"], 12);
        assert_eq!(v["lines"][1], "b");
        assert_eq!(v["nested"]["x"], 1);
        assert_eq!(v["arr"][2], 3);
        assert!(matches!(v["maybe"], Value::Null));
        assert!(matches!(v["missing"], Value::Null));
        // Round-trips through parse.
        assert_eq!(parse(&v.to_string()).unwrap(), v);
    }
}
