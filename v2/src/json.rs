//! JSON.parse / JSON.stringify(null, 2) semantics without a JavaScript runtime.
//! Strings and object keys are UTF-16, including lone surrogate escapes.
use crate::{input, output, OUTPUT_LIMIT};

#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Number(f64),
    String(Vec<u16>),
    Array(Vec<Json>),
    Object(Vec<(Vec<u16>, Json)>),
}
const INVALID: &str = "Invalid JSON";
const DEPTH: usize = 128;

pub fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, '\u{0009}'..='\u{000d}' | ' ' | '\u{00a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'))
}
pub fn format(s: &str) -> Result<String, String> {
    input(s)?;
    let s = trim_js(s);
    if s.is_empty() {
        return Err("JSON is required".into());
    }
    let mut p = Parser {
        s: s.as_bytes(),
        i: 0,
    };
    let value = p.value(0)?;
    p.ws();
    if p.i != p.s.len() {
        return Err(INVALID.into());
    }
    let mut out = String::new();
    stringify(&value, 0, &mut out)?;
    output(out)
}
pub fn validate(s: &str) -> Option<String> {
    format(s).err()
}
struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\r' | b'\n') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Result<Json, String> {
        if depth > DEPTH {
            return Err("JSON nesting limit exceeded (128)".into());
        }
        self.ws();
        match self.s.get(self.i).copied() {
            Some(b'n') => {
                self.word(b"null")?;
                Ok(Json::Null)
            }
            Some(b't') => {
                self.word(b"true")?;
                Ok(Json::Bool(true))
            }
            Some(b'f') => {
                self.word(b"false")?;
                Ok(Json::Bool(false))
            }
            Some(b'"') => Ok(Json::String(self.string()?)),
            Some(b'[') => {
                self.i += 1;
                self.ws();
                let mut v = Vec::new();
                if self.eat(b']') {
                    return Ok(Json::Array(v));
                }
                loop {
                    v.push(self.value(depth + 1)?);
                    self.ws();
                    if self.eat(b']') {
                        break;
                    }
                    if !self.eat(b',') {
                        return Err(INVALID.into());
                    }
                }
                Ok(Json::Array(v))
            }
            Some(b'{') => {
                self.i += 1;
                self.ws();
                let mut v: Vec<(Vec<u16>, Json)> = Vec::new();
                // Map index avoids quadratic duplicate-key lookup on large objects.
                let mut indices = std::collections::BTreeMap::new();
                if self.eat(b'}') {
                    return Ok(Json::Object(v));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    if !self.eat(b':') {
                        return Err(INVALID.into());
                    }
                    let val = self.value(depth + 1)?;
                    if let Some(&idx) = indices.get(&key) {
                        v[idx] = (key, val);
                    } else {
                        indices.insert(key.clone(), v.len());
                        v.push((key, val));
                    }
                    self.ws();
                    if self.eat(b'}') {
                        break;
                    }
                    if !self.eat(b',') {
                        return Err(INVALID.into());
                    }
                }
                Ok(Json::Object(v))
            }
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(INVALID.into()),
        }
    }
    fn word(&mut self, word: &[u8]) -> Result<(), String> {
        if self.s.get(self.i..self.i + word.len()) != Some(word) {
            return Err(INVALID.into());
        }
        self.i += word.len();
        Ok(())
    }
    fn digits(&mut self) -> usize {
        let start = self.i;
        while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
        }
        self.i - start
    }
    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        self.eat(b'-');
        if !self.eat(b'0') && self.digits() == 0 {
            return Err(INVALID.into());
        }
        if self.eat(b'.') && self.digits() == 0 {
            return Err(INVALID.into());
        }
        if self.eat(b'e') || self.eat(b'E') {
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if self.digits() == 0 {
                return Err(INVALID.into());
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| INVALID)?;
        let n: f64 = text.parse().map_err(|_| INVALID)?;
        Ok(Json::Number(n))
    }
    fn string(&mut self) -> Result<Vec<u16>, String> {
        if !self.eat(b'"') {
            return Err(INVALID.into());
        }
        let mut units = Vec::new();
        loop {
            let c = *self.s.get(self.i).ok_or(INVALID)?;
            self.i += 1;
            match c {
                b'"' => return Ok(units),
                0..=31 => return Err(INVALID.into()),
                b'\\' => {
                    let c = *self.s.get(self.i).ok_or(INVALID)?;
                    self.i += 1;
                    units.push(match c {
                        b'"' => 34,
                        b'\\' => 92,
                        b'/' => 47,
                        b'b' => 8,
                        b'f' => 12,
                        b'n' => 10,
                        b'r' => 13,
                        b't' => 9,
                        b'u' => {
                            let bytes = self.s.get(self.i..self.i + 4).ok_or(INVALID)?;
                            let mut n = 0;
                            for b in bytes {
                                n = n * 16 + (*b as char).to_digit(16).ok_or(INVALID)? as u16;
                            }
                            self.i += 4;
                            n
                        }
                        _ => return Err(INVALID.into()),
                    });
                }
                32..=127 => units.push(u16::from(c)),
                _ => {
                    self.i -= 1;
                    let len = if c < 0xe0 {
                        2
                    } else if c < 0xf0 {
                        3
                    } else {
                        4
                    };
                    let text =
                        std::str::from_utf8(self.s.get(self.i..self.i + len).ok_or(INVALID)?)
                            .map_err(|_| INVALID)?;
                    let ch = text.chars().next().ok_or(INVALID)?;
                    units.extend(ch.encode_utf16(&mut [0; 2]).iter().copied());
                    self.i += ch.len_utf8();
                }
            }
        }
    }
}
fn index(key: &[u16]) -> Option<u32> {
    if key.is_empty() || (key.len() > 1 && key[0] == 48) {
        return None;
    }
    let mut n = 0u32;
    for &u in key {
        if !(48..=57).contains(&u) {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u32::from(u - 48))?;
    }
    if n == u32::MAX {
        None
    } else {
        Some(n)
    }
}
fn quoted(units: &[u16], out: &mut String) {
    out.push('"');
    for ch in char::decode_utf16(units.iter().copied()) {
        match ch {
            Err(e) => out.push_str(&format!("\\u{:04x}", e.unpaired_surrogate())),
            Ok('"') => out.push_str("\\\""),
            Ok('\\') => out.push_str("\\\\"),
            Ok('\u{8}') => out.push_str("\\b"),
            Ok('\u{c}') => out.push_str("\\f"),
            Ok('\n') => out.push_str("\\n"),
            Ok('\r') => out.push_str("\\r"),
            Ok('\t') => out.push_str("\\t"),
            Ok(c) if (c as u32) < 32 => out.push_str(&format!("\\u{:04x}", c as u32)),
            Ok(c) => out.push(c),
        }
    }
    out.push('"');
}
fn stringify(v: &Json, depth: usize, out: &mut String) -> Result<(), String> {
    if out.len() > OUTPUT_LIMIT {
        return Err(crate::LIMIT_ERROR.into());
    }
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Number(n) => {
            if !n.is_finite() {
                out.push_str("null")
            } else {
                out.push_str(ryu_js::Buffer::new().format(*n))
            }
        }
        Json::String(s) => quoted(s, out),
        Json::Array(v) => {
            out.push('[');
            for (i, x) in v.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&"  ".repeat(depth + 1));
                stringify(x, depth + 1, out)?;
            }
            if !v.is_empty() {
                out.push('\n');
                out.push_str(&"  ".repeat(depth));
            }
            out.push(']');
        }
        Json::Object(v) => {
            let mut order: Vec<_> = v.iter().enumerate().collect();
            order.sort_by_key(|(i, (key, _))| (index(key).is_none(), index(key).unwrap_or(0), *i));
            out.push('{');
            for (i, (_, (key, x))) in order.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&"  ".repeat(depth + 1));
                quoted(key, out);
                out.push_str(": ");
                stringify(x, depth + 1, out)?;
            }
            if !v.is_empty() {
                out.push('\n');
                out.push_str(&"  ".repeat(depth));
            }
            out.push('}');
        }
    }
    if out.len() > OUTPUT_LIMIT {
        Err(crate::LIMIT_ERROR.into())
    } else {
        Ok(())
    }
}
