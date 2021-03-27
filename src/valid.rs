// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

const SPACE: u8 = 1 << 1;
const STRING: u8 = 1 << 2;

static TABLE: [u8; 256] = {
    let mut table = [0; 256];
    table[b'\t' as usize] |= SPACE;
    table[b'\n' as usize] |= SPACE;
    table[b'\r' as usize] |= SPACE;
    table[b' ' as usize] |= SPACE;

    table[0x00] |= STRING;
    table[0x01] |= STRING;
    table[0x02] |= STRING;
    table[0x03] |= STRING;
    table[0x04] |= STRING;
    table[0x05] |= STRING;
    table[0x06] |= STRING;
    table[0x07] |= STRING;
    table[0x08] |= STRING;
    table[0x09] |= STRING;
    table[0x0A] |= STRING;
    table[0x0B] |= STRING;
    table[0x0C] |= STRING;
    table[0x0D] |= STRING;
    table[0x0E] |= STRING;
    table[0x0F] |= STRING;
    table[0x10] |= STRING;
    table[0x11] |= STRING;
    table[0x12] |= STRING;
    table[0x13] |= STRING;
    table[0x14] |= STRING;
    table[0x15] |= STRING;
    table[0x16] |= STRING;
    table[0x17] |= STRING;
    table[0x18] |= STRING;
    table[0x19] |= STRING;
    table[0x1A] |= STRING;
    table[0x1B] |= STRING;
    table[0x1C] |= STRING;
    table[0x1D] |= STRING;
    table[0x1E] |= STRING;
    table[0x1F] |= STRING;
    table[b'"' as usize] |= STRING;
    table[b'\\' as usize] |= STRING;

    table
};

fn isspace(c: u8) -> bool {
    TABLE[c as usize] & SPACE == SPACE
}

/// Returns true if the input is valid json.
///
/// ```
/// if !gjson::valid(json) {
/// 	return Err("invalid json");
/// }
/// let value = gjson::get(json, "name.last");
/// ```
pub fn valid(json: &str) -> bool {
    let json = json.as_bytes();
    let mut i = 0;
    let (valid, next_i) = valid_any(json, i);
    if !valid {
        return false;
    }
    i = next_i;
    while i < json.len() {
        if !isspace(json[i]) {
            return false;
        }
        i += 1;
    }
    true
}

fn valid_any(json: &[u8], mut i: usize) -> (bool, usize) {
    while i < json.len() {
        if isspace(json[i]) {
            i += 1;
            continue;
        }
        return match json[i] {
            b'{' => valid_object(json, i),
            b'[' => valid_array(json, i),
            b'"' => valid_string(json, i),
            b't' => valid_true(json, i),
            b'f' => valid_false(json, i),
            b'n' => valid_null(json, i),
            _ => {
                if json[i] == b'-' || (json[i] >= b'0' && json[i] <= b'9') {
                    valid_number(json, i)
                } else {
                    break;
                }
            }
        };
    }
    (false, i)
}

fn strip_ws(json: &[u8], mut i: usize) -> usize {
    loop {
        if i + 16 < json.len() {
            for ch in &json[i..i + 16] {
                if TABLE[*ch as usize] & SPACE != SPACE {
                    return i;
                }
                i += 1;
            }
        }
        while i < json.len() {
            if TABLE[json[i] as usize] & SPACE != SPACE {
                return i;
            }
            i += 1;
        }
        return i;
    }
}

fn valid_object(json: &[u8], mut i: usize) -> (bool, usize) {
    i = strip_ws(json, i + 1);
    if i == json.len() {
        return (false, i);
    }
    if json[i] == b'}' {
        return (true, i + 1);
    }
    loop {
        if json[i] != b'"' {
            return (false, i);
        }
        let (valid, next_i) = valid_string(json, i);
        if !valid {
            return (false, i);
        }
        i = next_i;
        i = strip_ws(json, i);
        if i == json.len() {
            return (false, i);
        }
        if json[i] != b':' {
            return (false, i);
        }
        let (valid, next_i) = valid_any(json, i + 1);
        if !valid {
            return (false, i);
        }
        i = next_i;
        i = strip_ws(json, i);
        if i == json.len() {
            return (false, i);
        }
        if json[i] == b'}' {
            return (true, i + 1);
        }
        if json[i] != b',' {
            return (false, i);
        }
        i = strip_ws(json, i + 1);
        if i == json.len() {
            return (false, i);
        }
    }
}

fn valid_array(json: &[u8], mut i: usize) -> (bool, usize) {
    i = strip_ws(json, i + 1);
    if i == json.len() {
        return (false, i);
    }
    if json[i] == b']' {
        return (true, i + 1);
    }
    loop {
        let (valid, next_i) = valid_any(json, i);
        if !valid {
            return (false, i);
        }
        i = next_i;
        i = strip_ws(json, i);
        if i == json.len() {
            return (false, i);
        }
        if json[i] == b']' {
            return (true, i + 1);
        }
        if json[i] != b',' {
            return (false, i);
        }
        i += 1;
    }
}

fn ishexdigit(c: u8) -> bool {
    (c >= b'0' && c <= b'9') || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}

fn valid_string(json: &[u8], mut i: usize) -> (bool, usize) {
    i += 1;
    loop {
        let mut ch: u8;
        'tok: loop {
            if i + 32 < json.len() {
                for c in &json[i..i + 32] {
                    ch = *c;
                    if TABLE[ch as usize] & STRING == STRING {
                        break 'tok;
                    }
                    i += 1;
                }
            }
            while i < json.len() {
                ch = json[i];
                if TABLE[ch as usize] & STRING == STRING {
                    break 'tok;
                }
                i += 1;
            }
            return (false, i);
        }
        if json[i] < b' ' {
            return (false, i);
        }
        if json[i] == b'"' {
            return (true, i + 1);
        }
        if json[i] == b'\\' {
            i += 1;
            if i == json.len() {
                return (false, i);
            }
            match json[i] {
                b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't' => {}
                b'u' => {
                    for _ in 0..4 {
                        i += 1;
                        if i == json.len() {
                            return (false, i);
                        }
                        if !ishexdigit(json[i]) {
                            return (false, i);
                        }
                    }
                }
                _ => return (false, i),
            }
        }
        i += 1;
    }
}

fn valid_number(json: &[u8], mut i: usize) -> (bool, usize) {
    // sign
    if json[i] == b'-' {
        i += 1;
    }
    // int
    if i == json.len() {
        return (false, i);
    }
    if json[i] == b'0' {
        i += 1;
    } else {
        while i < json.len() {
            if json[i] >= b'0' && json[i] <= b'9' {
                i += 1;
                continue;
            }
            break;
        }
    }
    // frac
    if i == json.len() {
        return (true, i);
    }
    if json[i] == b'.' {
        i += 1;
        if i == json.len() {
            return (false, i);
        }
        if json[i] < b'0' || json[i] > b'9' {
            return (false, i);
        }
        i += 1;
        while i < json.len() {
            if json[i] >= b'0' && json[i] <= b'9' {
                i += 1;
                continue;
            }
            break;
        }
    }
    // exp
    if i == json.len() {
        return (true, i);
    }
    if json[i] == b'e' || json[i] == b'E' {
        i += 1;
        if i == json.len() {
            return (false, i);
        }
        if json[i] == b'+' || json[i] == b'-' {
            i += 1;
        }
        if i == json.len() {
            return (false, i);
        }
        if json[i] < b'0' || json[i] > b'9' {
            return (false, i);
        }
        i += 1;
        while i < json.len() {
            if json[i] >= b'0' && json[i] <= b'9' {
                i += 1;
                continue;
            }
            break;
        }
    }
    return (true, i);
}

fn valid_true(json: &[u8], i: usize) -> (bool, usize) {
    if i + 4 <= json.len() && json[i..i + 4].eq("true".as_bytes()) {
        (true, i + 4)
    } else {
        (false, i)
    }
}

fn valid_false(json: &[u8], i: usize) -> (bool, usize) {
    if i + 5 <= json.len() && json[i..i + 5].eq("false".as_bytes()) {
        (true, i + 5)
    } else {
        (false, i)
    }
}
fn valid_null(json: &[u8], i: usize) -> (bool, usize) {
    if i + 4 <= json.len() && json[i..i + 4].eq("null".as_bytes()) {
        (true, i + 4)
    } else {
        (false, i)
    }
}

#[cfg(test)]
mod test {
    use super::valid;

    #[test]
    fn basic() {
        assert_eq!(valid("0"), true);
        assert_eq!(valid("00"), false);
        assert_eq!(valid("-00"), false);
        assert_eq!(valid("-."), false);
        assert_eq!(valid("0.0"), true);
        assert_eq!(valid("10.0"), true);
        assert_eq!(valid("10e1"), true);
        assert_eq!(valid("10EE"), false);
        assert_eq!(valid("10E-"), false);
        assert_eq!(valid("10E+"), false);
        assert_eq!(valid("10E123"), true);
        assert_eq!(valid("10E-123"), true);
        assert_eq!(valid("10E-0123"), true);
        assert_eq!(valid(""), false);
        assert_eq!(valid(" "), false);
        assert_eq!(valid("{}"), true);
        assert_eq!(valid("{"), false);
        assert_eq!(valid("-"), false);
        assert_eq!(valid("-1"), true);
        assert_eq!(valid("-1."), false);
        assert_eq!(valid("-1.0"), true);
        assert_eq!(valid(" -1.0"), true);
        assert_eq!(valid(" -1.0 "), true);
        assert_eq!(valid("-1.0 "), true);
        assert_eq!(valid("-1.0 i"), false);
        assert_eq!(valid("-1.0 i"), false);
        assert_eq!(valid("true"), true);
        assert_eq!(valid(" true"), true);
        assert_eq!(valid(" true "), true);
        assert_eq!(valid(" True "), false);
        assert_eq!(valid(" tru"), false);
        assert_eq!(valid("false"), true);
        assert_eq!(valid(" false"), true);
        assert_eq!(valid(" false "), true);
        assert_eq!(valid(" False "), false);
        assert_eq!(valid(" fals"), false);
        assert_eq!(valid("null"), true);
        assert_eq!(valid(" null"), true);
        assert_eq!(valid(" null "), true);
        assert_eq!(valid(" Null "), false);
        assert_eq!(valid(" nul"), false);
        assert_eq!(valid(" []"), true);
        assert_eq!(valid(" [true]"), true);
        assert_eq!(valid(" [ true, null ]"), true);
        assert_eq!(valid(" [ true,]"), false);
        assert_eq!(valid(r#"{"hello":"world"}"#), true);
        assert_eq!(valid(r#"{ "hello": "world" }"#), true);
        assert_eq!(valid(r#"{ "hello": "world", }"#), false);
        assert_eq!(valid(r#"{"a":"b",}"#), false);
        assert_eq!(valid(r#"{"a":"b","a"}"#), false);
        assert_eq!(valid(r#"{"a":"b","a":}"#), false);
        assert_eq!(valid(r#"{"a":"b","a":1}"#), true);
        assert_eq!(valid(r#"{"a":"b",2"1":2}"#), false);
        assert_eq!(valid(r#"{"a":"b","a": 1, "c":{"hi":"there"} }"#), true);
        assert_eq!(
            valid(r#"{"a":"b","a": 1, "c":{"hi":"there", "easy":["going",{"mixed":"bag"}]} }"#),
            true
        );
        assert_eq!(valid(r#""""#), true);
        assert_eq!(valid(r#"""#), false);
        assert_eq!(valid(r#""\n""#), true);
        assert_eq!(valid(r#""\""#), false);
        assert_eq!(valid(r#""\\""#), true);
        assert_eq!(valid(r#""a\\b""#), true);
        assert_eq!(valid(r#""a\\b\\\"a""#), true);
        assert_eq!(valid(r#""a\\b\\\uFFAAa""#), true);
        assert_eq!(valid(r#""a\\b\\\uFFAZa""#), false);
        assert_eq!(valid(r#""a\\b\\\uFFA""#), false);
    }

    #[test]
    fn xcover() {
        // code coverage
        assert_eq!(valid(r#"{"hel\lo":"world"}"#), false);
        assert_eq!(valid(r#"{"hello"  "#), false);
        assert_eq!(valid(r#"{"hello"  : true "#), false);
        assert_eq!(valid(r#"{"hello"  : true x"#), false);
        assert_eq!(valid(r#"{"hello"  : true , "#), false);
        assert_eq!(valid(r#"[  "#), false);
        assert_eq!(valid(r#"[ true "#), false);
        assert_eq!(valid(r#"[ true x "#), false);
        assert_eq!(valid(r#"[ true , "#), false);

        assert_eq!(valid("[ \"hel\u{0}\" ]"), false);
        assert_eq!(valid(r#"[ "hel\"#), false);
        assert_eq!(valid(r#"[ "hel\u"#), false);

        assert_eq!(valid(r#"[ 123.x ]"#), false);
        assert_eq!(valid(r#"[ 123.0e"#), false);
        assert_eq!(valid(r#"[ 123.0e1f"#), false);
    }
}
