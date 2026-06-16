use std::{char, iter::Peekable, str::Chars};

use super::name_tree as nt;
use crate::{Resolver, parser::lexer::Token};

impl Resolver<'_> {
    /// Convert a number token into a number literal
    pub(super) fn number(&mut self, tok: Token) -> nt::NumberId {
        if tok.value.starts_with("0x") || tok.value.starts_with("0X") {
            self.hex_number(tok.value)
        } else {
            self.decimal_number(tok.value)
        }
    }

    /// Parse a hexadecimal number literal
    fn hex_number(&mut self, num: &str) -> nt::NumberId {
        let id = nt::NumberId(
            self.number_table
                .len()
                .try_into()
                .expect("Too many number literals within module"),
        );
        self.number_table.push(nt::Number::Float(1.0));

        if num.contains(['.', 'p', 'P']) {
            self.number_table.push(nt::Number::Float(hex_flt(num)));
            id
        } else {
            // hex integer
            self.number_table
                .push(nt::Number::Integer(hex_int(&num[2..])));
            id
        }
    }

    /// Parse a non-hexadecimal number literal
    fn decimal_number(&mut self, num: &str) -> nt::NumberId {
        let id = nt::NumberId(
            self.number_table
                .len()
                .try_into()
                .expect("Too many number literals within module"),
        );

        let num = if num.contains(['.', 'e', 'E']) {
            nt::Number::Float(num.parse::<f64>().unwrap())
        } else {
            match num.parse::<u64>() {
                Ok(n) => nt::Number::Integer(n),
                Err(_) => nt::Number::Float(num.parse::<f64>().unwrap()),
            }
        };

        self.number_table.push(num);
        id
    }

    /// Convert a string token into a string literal
    pub(super) fn string(&mut self, tok: Token) -> nt::StringId {
        let value = string_remove_quotes(tok.value);

        let value = if tok.value.starts_with('[') {
            value.to_string().into_bytes()
        } else {
            unescape_string(value)
        };

        self.insert_string(value)
    }
}

/// Parse a hexadecimal string into an integer, wrapping if too long.
fn hex_int(s: &str) -> u64 {
    let mut res: u64 = 0;

    for ch in s.chars() {
        let digit = match ch {
            '0' => 0,
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            '7' => 7,
            '8' => 8,
            '9' => 9,
            'a' | 'A' => 0xA,
            'b' | 'B' => 0xB,
            'c' | 'C' => 0xC,
            'd' | 'D' => 0xD,
            'e' | 'E' => 0xE,
            'f' | 'F' => 0xF,
            _ => panic!("Invalid hex digit"),
        };
        res = res.unbounded_shl(4);
        res = res.wrapping_add(digit);
    }

    res
}

/// Parse a hexadecimal floating point literal with the 0x stripped.
fn hex_flt(s: &str) -> f64 {
    hexf_parse::parse_hexf64(s, false).unwrap()
}

/// Remove all valid lua quote marks from the start and end of a string literal
fn string_remove_quotes(s: &str) -> &str {
    if let Some(s) = s.strip_prefix('\'') {
        return s.strip_suffix('\'').unwrap_or(s);
    } else if let Some(s) = s.strip_prefix('"') {
        return s.strip_suffix('"').unwrap_or(s);
    }

    // remove `[=[` / `]=]` long brackets.  Don't bother checking if they match
    // as the parser already did that.
    let s = s.strip_prefix('[').unwrap_or(s);
    let s = s.strip_suffix(']').unwrap_or(s);
    let s = s.trim_start_matches('=').trim_end_matches('=');
    let s = s.strip_prefix('[').unwrap_or(s);
    s.strip_suffix(']').unwrap_or(s)
}

/// Process and remove all escape sequences from a string literal
fn unescape_string(s: &str) -> Vec<u8> {
    let mut output = vec![];
    let mut input = s.chars().peekable();

    while let Some(ch) = input.next() {
        if ch != '\\' {
            let mut arr = [0; 4];
            ch.encode_utf8(&mut arr);
            output.extend(&arr[0..ch.len_utf8()]);
            continue;
        }

        let Some(next) = input.next() else {
            panic!("Unexpected EOF within escape sequence")
        };
        match_escape(next, &mut output, &mut input);
    }

    output
}

/// Process a single escape sequence, following the leading `\`
fn match_escape(ch: char, out: &mut Vec<u8>, input: &mut Peekable<Chars<'_>>) {
    match ch {
        'a' => out.push(b'\x07'),
        'b' => out.push(b'\x08'),
        'f' => out.push(b'\x0C'),
        'n' | '\n' => out.push(b'\n'),
        'r' => out.push(b'\r'),
        't' => out.push(b'\t'),
        'v' => out.push(b'\x0B'),
        '\\' => out.push(b'\\'),
        '"' => out.push(b'"'),
        '\'' => out.push(b'\''),
        'z' => {
            // skip whitespace
            while let Some(' ' | '\x0C' | '\n' | '\r' | '\t' | '\x0B') = input.peek() {
                input.next();
            }
        }
        'x' => out.push(hex_escape(input)),
        ch @ '0'..='9' => out.push(decimal_escape(ch, input)),
        'u' => unicode_escape(out, input),
        ch => panic!("Unknown escape sequence: {ch}"),
    }
}

/// Parse a hex `\xFF` escape sequence
fn hex_escape(input: &mut Peekable<impl Iterator<Item = char>>) -> u8 {
    let (Some(a), Some(b)) = (input.next(), input.next()) else {
        panic!("Unexpected EOF in hex byte escape")
    };
    let Ok(num) = u8::from_str_radix(&format!("{a}{b}"), 16) else {
        panic!("Invalid hex digit")
    };
    num
}

/// Parse a decimal `\152` escape sequence
fn decimal_escape(ch: char, input: &mut Peekable<Chars<'_>>) -> u8 {
    let mut s = String::with_capacity(3);
    s.push(ch);
    for _ in 0..2 {
        if let Some(ch @ '0'..='9') = input.peek() {
            s.push(*ch);
            input.next();
        }
    }
    let Ok(num) = s.parse::<u8>() else {
        panic!("Invalid decimal escape");
    };
    num
}

/// Parse a unicode escape sequence
fn unicode_escape(out: &mut Vec<u8>, input: &mut Peekable<Chars<'_>>) {
    if input.next() != Some('{') {
        panic!("Expected `{{` in unicode escape")
    }
    let mut s = String::new();
    while let Some(ch) = input.peek()
        && ch.is_ascii_hexdigit()
    {
        s.push(*ch);
        input.next();
    }
    if input.next() != Some('}') {
        panic!("Expected `}}` in unicode escape")
    }
    let Ok(num) = s.parse::<u32>() else {
        panic!("Invalid number in unicode_escape")
    };

    // Encode num using utf-8, in the old, 6-byte max length encoding
    match num {
        0x0..0x80 => out.push(num as u8),
        0x80..0x800 => {
            out.push(0b1100_0000 | ((num >> 6) as u8));
            out.push(0b1000_0000 | ((num & 0b11_1111) as u8));
        }
        0x800..0x1_0000 => {
            out.push(0b1110_0000 | ((num >> 12) as u8));
            out.push(0b1000_0000 | (((num >> 6) & 0b11_1111) as u8));
            out.push(0b1000_0000 | ((num & 0b11_1111) as u8));
        }
        0x1_0000..0x20_0000 => {
            out.push(0b1111_0000 | ((num >> 18) as u8));
            out.push(0b1000_0000 | (((num >> 12) & 0b11_1111) as u8));
            out.push(0b1000_0000 | (((num >> 6) & 0b11_1111) as u8));
            out.push(0b1000_0000 | ((num & 0b11_1111) as u8));
        }
        0x20_0000..0x400_0000 => {
            out.push(0b1111_1000 | ((num >> 24) as u8));
            out.push(0b1000_0000 | (((num >> 18) & 0b11_1111) as u8));
            out.push(0b1000_0000 | (((num >> 12) & 0b11_1111) as u8));
            out.push(0b1000_0000 | (((num >> 6) & 0b11_1111) as u8));
            out.push(0b1000_0000 | ((num & 0b11_1111) as u8));
        }
        0x400_0000..0x8000_0000 => {
            out.push(0b1111_1100 | ((num >> 30) as u8));
            out.push(0b1000_0000 | (((num >> 24) & 0b11_1111) as u8));
            out.push(0b1000_0000 | (((num >> 18) & 0b11_1111) as u8));
            out.push(0b1000_0000 | (((num >> 12) & 0b11_1111) as u8));
            out.push(0b1000_0000 | (((num >> 6) & 0b11_1111) as u8));
            out.push(0b1000_0000 | ((num & 0b11_1111) as u8));
        }
        0x8000_0000..=u32::MAX => panic!("Unicode number greater than 2^31-1"),
    }
}
