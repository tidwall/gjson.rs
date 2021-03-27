// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

use std::char;
use std::mem;

/// tostr transmutes a byte slice to a string reference. This function must
/// only be used on path components and json data which originated from the
/// super::get() function. The super::get() function only accepts &str
/// references and expects that the inputs are utf8 validated. All slices to
/// the json and path data during the get().
pub fn tostr<'a>(v: &'a [u8]) -> &'a str {
    // SAFETY: All slices to the json and path data during the get()
    // operation are done at ascii codepoints which ensuring that the
    // conversion is safe.
    unsafe { std::str::from_utf8_unchecked(v) }
}

pub fn trim<'a>(mut bin: &'a [u8]) -> &'a [u8] {
    while bin.len() > 0 && bin[0] <= b' ' {
        bin = &bin[1..];
    }
    while bin.len() > 0 && bin[bin.len() - 1] <= b' ' {
        bin = &bin[..bin.len() - 1];
    }
    bin
}

// unescape a json string.
pub fn unescape(json: &str) -> String {
    let json = json.as_bytes();
    if json.len() < 2 || json[0] != b'"' || json[json.len() - 1] != b'"' {
        return String::new();
    }
    let json = &json[1..json.len() - 1];
    let mut out = Vec::with_capacity(json.len());
    let mut i = 0;
    loop {
        if i == json.len() || json[i] < b' ' {
            break;
        } else if json[i] == b'\\' {
            i += 1;
            if i == json.len() {
                break;
            }
            match json[i] {
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                b'/' => out.push(b'/'),
                b'b' => out.push(8),
                b'f' => out.push(12),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b't' => out.push(b'\t'),
                b'u' => {
                    if i + 5 > json.len() {
                        break;
                    }
                    let mut r =
                        u32::from_str_radix(tostr(&json[i + 1..i + 5]), 16).unwrap_or(0xFFFD);
                    i += 5;
                    if utf16_is_surrogate(r) {
                        // need another code
                        if (&json[i..]).len() >= 6 && json[i] == b'\\' && json[i + 1] == b'u' {
                            if let Ok(r2) = u32::from_str_radix(tostr(&json[i + 2..i + 6]), 16) {
                                r = utf16_decode(r, r2);
                            } else {
                                r = 0xFFFD;
                            }
                            i += 6
                        }
                    }
                    let ch = char::from_u32(r).unwrap_or(char::REPLACEMENT_CHARACTER);
                    let mark = out.len();
                    for _ in 0..10 {
                        out.push(0);
                    }
                    let n = ch.encode_utf8(&mut out[mark..]).len();
                    out.truncate(mark + n);
                    continue;
                }
                _ => break,
            }
        } else {
            out.push(json[i]);
        }
        i += 1;
    }
    unsafe { mem::transmute::<Vec<u8>, String>(out) }
}

fn utf16_is_surrogate(r: u32) -> bool {
    0xd800 <= r && r < 0xe000
}

fn utf16_decode(r1: u32, r2: u32) -> u32 {
    if 0xd800 <= r1 && r1 < 0xdc00 && 0xdc00 <= r2 && r2 < 0xe000 {
        (r1 - 0xd800) << 10 | (r2 - 0xdc00) + 0x10000
    } else {
        0xFFFD
    }
}

// fn next_json_encoded_rune(iter: &mut std::str::Chars) -> Option<u16> {
//     (iter.next()?.to_digit(16)? << 16)
//         | (iter.next()?.to_digit(16)? << 8)
//         | (iter.next()?.to_digit(16)? << 4)
//         | (iter.next()?.to_digit(16)? << 0);
//     None
// }

// pub fn need_escaping(s: &str) -> bool {
//     let s = s.as_bytes();
//     for i in 0..s.len() {
//         if s[i] < b' ' || s[i] == b'\n' || s[i] == b'\\' || s[i] == b'"' {
//             return true;
//         }
//     }
//     return false;
// }

pub fn extend_json_string(out: &mut Vec<u8>, s: &[u8]) {
    out.push(b'"');
    for i in 0..s.len() {
        if s[i] < b' ' || s[i] == b'\n' || s[i] == b'\\' || s[i] == b'"' {
            out.push(b'\\');
            match s[i] {
                b'"' => out.push(b'"'),
                b'\\' => out.push(b'\\'),
                8 => out.push(b'b'),
                12 => out.push(b'f'),
                b'\n' => out.push(b'n'),
                b'\r' => out.push(b'r'),
                b'\t' => out.push(b't'),
                _ => {
                    out.push(b'u');
                    out.push(b'0');
                    out.push(b'0');
                    let h = s[i] >> 4;
                    out.push(if h < 10 { h + b'0' } else { (h - 10) + b'A' });
                    let l = s[i] & 0xF;
                    out.push(if l < 10 { l + b'0' } else { (l - 10) + b'A' });
                }
            }
        } else {
            out.push(s[i]);
        }
    }
    out.push(b'"');
}

// escape a json string. includes the
pub fn escape(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len());
    extend_json_string(&mut out, s.as_bytes());
    unsafe { std::mem::transmute::<Vec<u8>, String>(out) }
}

/// pmatch returns true if str matches pattern. This is a very
/// simple wildcard match where '*' matches on any number characters
/// and '?' matches on any one character.
///
/// pattern:
///   { term }
/// term:
/// 	 '*'         matches any sequence of non-Separator characters
/// 	 '?'         matches any single non-Separator character
/// 	 c           matches character c (c != '*', '?')
/// 	'\\' c       matches character c
pub fn pmatch<S, P>(pattern: P, string: S) -> bool
where
    S: AsRef<[u8]>,
    P: AsRef<[u8]>,
{
    let mut string = string.as_ref();
    let mut pattern = pattern.as_ref();
    while pattern.len() > 0 {
        if pattern[0] == b'\\' {
            if pattern.len() == 1 {
                return false;
            }
            pattern = &pattern[1..];
        } else if pattern[0] == b'*' {
            if pattern.len() == 1 {
                return true;
            }
            if pattern[1] == b'*' {
                pattern = &pattern[1..];
                continue;
            }
            if pmatch(&pattern[1..], string) {
                return true;
            }
            if string.len() == 0 {
                return false;
            }
            string = &string[1..];
            continue;
        }
        if string.len() == 0 {
            return false;
        }
        if pattern[0] != b'?' && string[0] != pattern[0] {
            return false;
        }
        pattern = &pattern[1..];
        string = &string[1..];
    }
    return string.len() == 0 && pattern.len() == 0;
}

#[cfg(test)]
mod test {

    #[test]
    fn basic() {
        assert_eq!(true, super::pmatch("*", "",));
        assert_eq!(true, super::pmatch("", "",));
        assert_eq!(false, super::pmatch("", "hello world",));
        assert_eq!(false, super::pmatch("jello world", "hello world",));
        assert_eq!(true, super::pmatch("*", "hello world",));
        assert_eq!(true, super::pmatch("*world*", "hello world",));
        assert_eq!(true, super::pmatch("*world", "hello world",));
        assert_eq!(true, super::pmatch("hello*", "hello world",));
        assert_eq!(false, super::pmatch("jello*", "hello world",));
        assert_eq!(true, super::pmatch("hello?world", "hello world",));
        assert_eq!(false, super::pmatch("jello?world", "hello world",));
        assert_eq!(true, super::pmatch("he*o?world", "hello world",));
        assert_eq!(true, super::pmatch("he*o?wor*", "hello world",));
        assert_eq!(true, super::pmatch("he*o?*r*", "hello world",));
        assert_eq!(true, super::pmatch("h\\*ello", "h*ello",));
        assert_eq!(false, super::pmatch("hello\\", "hello\\",));
        assert_eq!(true, super::pmatch("hello\\?", "hello?",));
        assert_eq!(true, super::pmatch("hello\\\\", "hello\\",));

        // test for fast repeating stars
        let string = ",**,,**,**,**,**,**,**,";
        let pattern = ",**********************************************{**\",**,,**,**,**,**,\"\",**,**,**,**,**,**,**,**,**,**]";
        super::pmatch(pattern, string);
    }
    #[test]
    fn escape() {
        let text = r#"
第一印象:なんか怖っ！
今の印象:とりあえずキモい。噛み合わない
好きなところ:ぶすでキモいとこ😋✨✨
思い出:んーーー、ありすぎ😊❤️
LINE交換できる？:あぁ……ごめん✋
トプ画をみて:照れますがな😘✨
一言:お前は一生もんのダチ💖"#;

        let raw1 = r#""\n第一印象:なんか怖っ！\n今の印象:とりあえずキモい。噛み合わない\n好きなところ:ぶすでキモいとこ😋✨✨\n思い出:んーーー、ありすぎ😊❤️\nLINE交換できる？:あぁ……ごめん✋\nトプ画をみて:照れますがな😘✨\n一言:お前は一生もんのダチ💖""#;
        let raw2 = r#""\n\u7B2C\u4E00\u5370\u8C61:\u306A\u3093\u304B\u6016\u3063\uFF01\n\u4ECA\u306E\u5370\u8C61:\u3068\u308A\u3042\u3048\u305A\u30AD\u30E2\u3044\u3002\u565B\u307F\u5408\u308F\u306A\u3044\n\u597D\u304D\u306A\u3068\u3053\u308D:\u3076\u3059\u3067\u30AD\u30E2\u3044\u3068\u3053\uD83D\uDE0B\u2728\u2728\n\u601D\u3044\u51FA:\u3093\u30FC\u30FC\u30FC\u3001\u3042\u308A\u3059\u304E\uD83D\uDE0A\u2764\uFE0F\nLINE\u4EA4\u63DB\u3067\u304D\u308B\uFF1F:\u3042\u3041\u2026\u2026\u3054\u3081\u3093\u270B\n\u30C8\u30D7\u753B\u3092\u307F\u3066:\u7167\u308C\u307E\u3059\u304C\u306A\uD83D\uDE18\u2728\n\u4E00\u8A00:\u304A\u524D\u306F\u4E00\u751F\u3082\u3093\u306E\u30C0\u30C1\uD83D\uDC96""#;
        assert_eq!(text, super::unescape(raw1));
        assert_eq!(text, super::unescape(raw2));
        assert_eq!(super::escape(&text), raw1);

        assert_eq!(
            super::escape("ad\"\\/\u{08}\u{0C}\n\r\t\u{00}sf"),
            r#""ad\"\\/\b\f\n\r\t\u0000sf""#
        );
    }

    #[test]
    fn unescape() {
        assert_eq!(super::unescape(r#""adsf"#), "");
        assert_eq!(super::unescape(r#""ad\sf""#), "ad");
        assert_eq!(
            super::unescape(r#""ad\"\\\/\b\f\n\r\tsf""#),
            "ad\"\\/\u{08}\u{0C}\n\r\tsf"
        );
        assert_eq!(super::unescape(r#""ad\uD83Dsf""#), "ad�sf");
        assert_eq!(super::unescape(r#""ad\uD83D\usf""#), "ad�");
        assert_eq!(super::unescape(r#""ad\uD83D\uxxxxsf""#), "ad�sf");
        assert_eq!(super::unescape(r#""ad\uD83D\u00FFsf""#), "ad�sf");
    }
}
