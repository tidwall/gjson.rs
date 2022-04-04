// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

mod modifiers;
mod multipath;
mod path;
mod pretty;
mod test;
/// Additional tools for working with JSON data.
pub mod tools;
mod util;
mod valid;

use path::*;
use std::cmp::Ordering;
use std::fmt;
use util::{pmatch, tostr, unescape};
pub use valid::valid;

type InfoBits = u32;

/// The kind of json `Value`.
#[derive(Copy, Clone, Eq)]
pub enum Kind {
    Null,
    False,
    Number,
    String,
    True,
    Array,
    Object,
}

impl PartialOrd for Kind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Kind {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(&other) == Ordering::Equal
    }
}

impl Ord for Kind {
    fn cmp(&self, other: &Self) -> Ordering {
        (*self as i32).cmp(&(*other as i32))
    }
}

// first eight bits are reserved for json-kind flags
const INFO_NULL: InfoBits = 1 << 1;
const INFO_FALSE: InfoBits = 1 << 2;
const INFO_NUMBER: InfoBits = 1 << 3;
const INFO_STRING: InfoBits = 1 << 4;
const INFO_TRUE: InfoBits = 1 << 5;
const INFO_OBJECT: InfoBits = 1 << 6;
const INFO_ARRAY: InfoBits = 1 << 7;
// remaing 8-31 bits used for extra details
const INFO_ESC: InfoBits = 1 << 8;
const INFO_SIGN: InfoBits = 1 << 9;
const INFO_DOT: InfoBits = 1 << 10;
const INFO_E: InfoBits = 1 << 11;
const INFO_FOG: InfoBits = 1 << 12;

static KINDMAP: [Kind; 256] = {
    let mut map = [Kind::Null; 256];
    map[INFO_NULL as usize] = Kind::Null;
    map[INFO_FALSE as usize] = Kind::False;
    map[INFO_NUMBER as usize] = Kind::Number;
    map[INFO_STRING as usize] = Kind::String;
    map[INFO_TRUE as usize] = Kind::True;
    map[INFO_OBJECT as usize] = Kind::Object;
    map[INFO_ARRAY as usize] = Kind::Array;
    map
};

/// Value is the JSON value returned from the `get` function.
pub struct Value<'a> {
    slice: &'a str,
    owned: String,
    uescstr: String,
    info: InfoBits,
    index: Option<usize>,
}

impl<'a> Eq for Value<'a> {}

impl<'a> PartialOrd for Value<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for Value<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(&other) == Ordering::Equal
    }
}

impl<'a> Ord for Value<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        let cmp = self.kind().cmp(&other.kind());
        if cmp != Ordering::Equal {
            cmp
        } else if self.kind() == Kind::String {
            self.str().cmp(other.str())
        } else if self.kind() == Kind::Number {
            let x = self.f64();
            let y = other.f64();
            if x < y {
                Ordering::Less
            } else if x > y {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        } else {
            self.json().cmp(other.json())
        }
    }
}

impl<'a> Default for Value<'a> {
    fn default() -> Self {
        return Value {
            slice: "",
            owned: String::default(),
            uescstr: String::default(),
            info: 0,
            index: None,
        };
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.str())
    }
}

fn json_clone_from_ref<'a>(json: &'a Value<'a>) -> Value<'a> {
    Value {
        slice: json.json(),
        owned: String::new(),
        uescstr: json.uescstr.to_owned(),
        info: json.info,
        index: json.index,
    }
}

fn json_from_slice<'a>(slice: &'a [u8], index: Option<usize>, info: InfoBits) -> Value<'a> {
    let mut json = Value {
        slice: tostr(slice),
        owned: String::new(),
        uescstr: String::new(),
        info,
        index,
    };
    json_unescape_string(&mut json);
    return json;
}

fn json_from_owned<'a>(owned: String, index: Option<usize>, info: InfoBits) -> Value<'a> {
    let mut json = Value {
        slice: "",
        owned: owned,
        uescstr: String::new(),
        info,
        index,
    };
    json_unescape_string(&mut json);
    return json;
}

fn json_unescape_string<'a>(json: &mut Value<'a>) {
    if json.info & (INFO_STRING | INFO_ESC) == (INFO_STRING | INFO_ESC) {
        // Escaped string. We must unescape it into a new allocated string.
        json.uescstr = unescape(json.json());
    }
}

impl<'a> Value<'a> {
    pub fn get(&'a self, path: &'a str) -> Value<'a> {
        let mut json = if self.slice.len() > 0 {
            get(&self.slice, path)
        } else {
            json_into_owned(get(&self.owned, path))
        };
        let mut index = None;
        if let Some(index1) = self.index {
            if let Some(index2) = json.index {
                index = Some(index1 + index2);
            }
        }
        json.index = index;
        json
    }

    pub fn exists(&self) -> bool {
        self.json().len() > 0
    }

    pub fn kind(&self) -> Kind {
        KINDMAP[(self.info << 24 >> 24) as usize]
    }

    pub fn json(&self) -> &str {
        if self.owned.len() > 0 {
            self.owned.as_str()
        } else {
            self.slice
        }
    }

    pub fn f64(&'a self) -> f64 {
        let raw = self.json().as_bytes();
        match self.kind() {
            Kind::True => 1.0,
            Kind::String => {
                if self.info & INFO_ESC == INFO_ESC {
                    raw_to_f64(&unescape(tostr(raw)))
                } else {
                    raw_to_f64(tostr(&raw[1..raw.len() - 1]))
                }
            }
            Kind::Number => raw_to_f64(tostr(raw)),
            _ => 0.0,
        }
    }

    pub fn f32(&'a self) -> f32 {
        self.f64() as f32
    }

    pub fn i64(&'a self) -> i64 {
        let raw = self.json().as_bytes();
        match self.kind() {
            Kind::True => 1,
            Kind::String => {
                if self.info & INFO_ESC == INFO_ESC {
                    raw_to_i64(&unescape(tostr(raw)))
                } else {
                    raw_to_i64(tostr(&raw[1..raw.len() - 1]))
                }
            }
            Kind::Number => raw_to_i64(tostr(raw)),
            _ => 0,
        }
    }

    pub fn u64(&'a self) -> u64 {
        let raw = self.json().as_bytes();
        match self.kind() {
            Kind::True => 1,
            Kind::String => {
                if self.info & INFO_ESC == INFO_ESC {
                    raw_to_u64(&unescape(tostr(raw)))
                } else {
                    raw_to_u64(tostr(&raw[1..raw.len() - 1]))
                }
            }
            Kind::Number => raw_to_u64(tostr(raw)),
            _ => 0,
        }
    }

    pub fn i32(&'a self) -> i32 {
        let x = self.i64();
        (if x < -2147483648 {
            -2147483648
        } else if x > 2147483648 {
            2147483648
        } else {
            x
        }) as i32
    }

    pub fn i16(&'a self) -> i16 {
        let x = self.i64();
        (if x < -32768 {
            -32768
        } else if x > 32767 {
            32767
        } else {
            x
        }) as i16
    }

    pub fn i8(&'a self) -> i8 {
        let x = self.i64();
        (if x < -128 {
            -128
        } else if x > 127 {
            127
        } else {
            x
        }) as i8
    }

    pub fn u32(&'a self) -> u32 {
        let x = self.u64();
        (if x > 4294967295 { 4294967295 } else { x }) as u32
    }

    pub fn u16(&'a self) -> u16 {
        let x = self.u64();
        (if x > 65535 { 65535 } else { x }) as u16
    }

    pub fn u8(&'a self) -> u8 {
        let x = self.u64();
        (if x > 255 { 255 } else { x }) as u8
    }

    pub fn bool(&'a self) -> bool {
        let raw = self.json();
        match raw {
            r#"1"# | r#"true"# => true,
            r#"0"# | r#"false"# => false,

            r#""t""# | r#""1""# | r#""T""# => true,
            r#""f""# | r#""0""# | r#""F""# => false,

            r#""true""# | r#""TRUE""# | r#""True""# => true,
            r#""false""# | r#""FALSE""# | r#""False""# => false,
            _ => self.i64() != 0,
        }
    }

    pub fn str(&'a self) -> &'a str {
        match self.kind() {
            Kind::True => "true",
            Kind::False => "false",
            Kind::Object | Kind::Array | Kind::Number => self.json(),
            Kind::String => {
                if self.info & INFO_ESC == INFO_ESC {
                    self.uescstr.as_ref()
                } else {
                    let raw = self.json().as_bytes();
                    tostr(&raw[1..raw.len() - 1])
                }
            }
            // Return an empty string for null. Use raw() to return the
            // raw json.
            Kind::Null => "",
        }
    }

    pub fn each(&'a self, mut iter: impl FnMut(Value<'a>, Value<'a>) -> bool) {
        if !self.exists() {
            return;
        }
        let kind = self.kind();
        if kind != Kind::Object && kind != Kind::Array {
            iter(Value::default(), json_clone_from_ref(&self));
            return;
        }
        let json = self.json().as_bytes();
        for_each(json, 0, false, kind, iter);
    }

    pub fn array(&'a self) -> Vec<Value<'a>> {
        let mut arr = Vec::new();
        if self.kind() == Kind::Array {
            self.each(|_, value| {
                arr.push(value);
                true
            })
        }
        arr
    }
}

fn for_each<'a>(
    json: &'a [u8],
    mut i: usize,
    lines: bool,
    kind: Kind,
    mut iter: impl FnMut(Value<'a>, Value<'a>) -> bool,
) -> usize {
    if i == json.len() {
        return i;
    }
    if !lines {
        i += 1;
    }
    let mut index = 0;
    let mut tmp_key = Value::default();
    while i < json.len() {
        if json[i] <= b' ' || json[i] == b',' || json[i] == b':' {
            i += 1;
            continue;
        }
        if json[i] == b'}' || json[i] == b']' {
            return i + 1;
        }
        let (res, next_i, _) = proc_value(json, i, Path::default(), true);
        i = next_i;
        if res.exists() {
            if kind == Kind::Object {
                if index % 2 == 0 {
                    tmp_key = res;
                } else {
                    let key = tmp_key;
                    tmp_key = Value::default();
                    if !iter(key, res) {
                        break;
                    }
                }
            } else {
                if !iter(Value::default(), res) {
                    break;
                }
            }
            index += 1;
        }
    }
    i
}

fn raw_to_f64(raw: &str) -> f64 {
    raw.parse().unwrap_or(0.0)
}

fn raw_to_i64(raw: &str) -> i64 {
    raw.parse().unwrap_or(raw_to_f64(raw) as i64)
}

fn raw_to_u64(raw: &str) -> u64 {
    raw.parse().unwrap_or(raw_to_f64(raw) as u64)
}

const CHQUOTE: u8 = 1 << 1;
const CHOPEN: u8 = 1 << 2;
const CHCLOSE: u8 = 1 << 3;
const CHSTRTOK: u8 = 1 << 4;
const CHSQUASH: u8 = 1 << 5;

static CHTABLE: [u8; 256] = {
    let mut table = [0; 256];
    table[b'{' as usize] |= CHSQUASH | CHOPEN;
    table[b'[' as usize] |= CHSQUASH | CHOPEN;
    table[b'(' as usize] |= CHSQUASH | CHOPEN;
    table[b'}' as usize] |= CHSQUASH | CHCLOSE;
    table[b']' as usize] |= CHSQUASH | CHCLOSE;
    table[b')' as usize] |= CHSQUASH | CHCLOSE;
    table[b'"' as usize] |= CHSQUASH | CHQUOTE;
    table[b'\\' as usize] |= CHSQUASH;
    table[b'"' as usize] |= CHSTRTOK;
    table[b'\\' as usize] |= CHSTRTOK;
    table
};

// -> (val, info, next_i)
fn scan_number<'a>(json: &'a [u8], mut i: usize) -> (&'a [u8], InfoBits, usize) {
    let s = i;
    let mut info = 0;
    if json[i] == b'-' {
        info |= INFO_SIGN;
    }
    i += 1;
    while i < json.len() {
        let ch = json[i];
        if ch == b'.' {
            info |= INFO_DOT;
        } else if ch == b'e' || ch == b'E' {
            info |= INFO_E;
        } else if (ch < b'0' || ch > b'9') && ch != b'-' && ch != b'+' {
            break;
        }
        i += 1;
    }
    (&json[s..i], info, i)
}

#[inline(always)]
fn scan_string<'a>(json: &'a [u8], mut i: usize) -> (&'a [u8], InfoBits, usize) {
    let mut info = 0;
    let s = i;
    i += 1;
    'outer: loop {
        let mut ch;
        'tok: loop {
            while i + 8 < json.len() {
                for _ in 0..8 {
                    // SAFETY: bounds already checked.
                    ch = unsafe { *json.get_unchecked(i) } as usize;
                    if CHTABLE[ch] & CHSTRTOK == CHSTRTOK {
                        break 'tok;
                    }
                    i += 1;
                }
            }
            while i < json.len() {
                ch = json[i] as usize;
                if CHTABLE[ch] & CHSTRTOK == CHSTRTOK {
                    break 'tok;
                }
                i += 1;
            }
            break 'outer;
        }
        if ch as u8 == b'"' {
            i += 1;
            return (&json[s..i], info, i);
        } else {
            // must be a escape character '\'
            info |= INFO_ESC;
            i += 1;
            if i == json.len() {
                break;
            }
            i += 1;
        }
    }
    ("".as_bytes(), 0, json.len())
}

// -> (val, next_i)
fn scan_squash<'a>(json: &'a [u8], mut i: usize) -> (&'a [u8], usize) {
    let s = i;
    i += 1;
    let mut depth = 1;
    'outer: loop {
        let mut ch;
        'tok: loop {
            while i + 8 < json.len() {
                for _ in 0..8 {
                    // SAFETY: bounds already checked.
                    ch = unsafe { *json.get_unchecked(i) } as usize;
                    if CHTABLE[ch] & CHSQUASH == CHSQUASH {
                        break 'tok;
                    }
                    i += 1;
                }
            }
            while i < json.len() {
                ch = json[i] as usize;
                if CHTABLE[ch] & CHSQUASH == CHSQUASH {
                    break 'tok;
                }
                i += 1;
            }
            break 'outer;
        }
        if ch as u8 == b'"' {
            i = scan_string(json, i).2;
            continue;
        } else if CHTABLE[ch] & CHOPEN == CHOPEN {
            depth += 1;
        } else if CHTABLE[ch] & CHCLOSE == CHCLOSE {
            depth -= 1;
            if depth == 0 {
                i += 1;
                return (&json[s..i], i);
            }
        } else if ch == b'\\' as usize {
            i += 1;
            if i == json.len() {
                break 'outer;
            }
        }
        i += 1;
    }
    ("".as_bytes(), json.len())
}

fn proc_value<'a>(
    json: &'a [u8],
    mut i: usize,
    path: Path<'a>,
    is_match: bool,
) -> (Value<'a>, usize, Path<'a>) {
    if json[i] == b'"' {
        let s = i;
        let (val, info, next_i) = scan_string(json, i);
        i = next_i;
        if is_match {
            return (json_from_slice(val, Some(s), info | INFO_STRING), i, path);
        }
    } else if json[i] == b'{' || json[i] == b'[' {
        if is_match {
            let mut squash = true;
            if path.sep == b'.' {
                let next_path = path.next();
                if !next_path.is_modifier() && !next_path.is_multipath() {
                    squash = false;
                    let (res, next_i, next_path) = if json[i] == b'{' {
                        get_obj(json, i, next_path)
                    } else {
                        get_arr(json, i, false, next_path)
                    };
                    if res.exists() {
                        return (res, next_i, next_path);
                    }
                }
            }
            if squash {
                // returns the squashed (entire) json value
                let s = i;
                let (val, next_i) = scan_squash(json, i);
                i = next_i;
                let kind = if json[s] == b'{' {
                    INFO_OBJECT
                } else {
                    INFO_ARRAY
                };
                return (json_from_slice(val, Some(s), 0 | kind), i, path);
            }
        } else {
            i = scan_squash(json, i).1;
        }
    } else if (json[i] >= b'0' && json[i] <= b'9') || json[i] == b'-' {
        let s = i;
        let (val, info, next_i) = scan_number(json, i);
        i = next_i;
        if is_match {
            return (json_from_slice(val, Some(s), info | INFO_NUMBER), i, path);
        }
    } else {
        let s = i;
        let kind;
        if json[i] == b't' {
            if i + 3 >= json.len() {
                return (Value::default(), json.len(), path);
            }
            i += 4;
            kind = INFO_TRUE;
        } else if json[i] == b'f' {
            if i + 4 >= json.len() {
                return (Value::default(), json.len(), path);
            }
            i += 5;
            kind = INFO_FALSE;
        } else if json[i] == b'n' {
            if i + 3 >= json.len() {
                return (Value::default(), json.len(), path);
            }
            i += 4;
            kind = INFO_NULL;
        } else {
            // unknown character
            return (Value::default(), json.len(), Path::default());
        }
        if is_match {
            return (json_from_slice(&json[s..i], Some(s), kind), i, path);
        }
    }
    (Value::default(), i, path)
}

fn get_obj<'a>(json: &'a [u8], mut i: usize, path: Path<'a>) -> (Value<'a>, usize, Path<'a>) {
    if i == json.len() || json[i] != b'{' {
        return (Value::default(), i, path);
    }
    i += 1;
    while i < json.len() {
        if json[i] == b'}' {
            i += 1;
            break;
        }
        if json[i] != b'"' {
            i += 1;
            continue;
        }
        // key
        let (key, info, next_i) = scan_string(json, i);
        i = next_i;
        while i < json.len() {
            if json[i] <= b' ' || json[i] == b':' {
                i += 1;
                continue;
            }
            break;
        }
        if i == json.len() {
            break;
        }
        let is_match = key_match(key, info, &path);
        let (res, next_i, next_path) = proc_value(json, i, path, is_match);
        i = next_i;
        if res.exists() {
            return (res, i, next_path);
        }
    }
    (Value::default(), i, path)
}

fn key_match(key: &[u8], info: InfoBits, path: &Path) -> bool {
    let comp = tostr(path.comp);
    if info & INFO_ESC == INFO_ESC {
        let key = unescape(tostr(key));
        if path.pat || path.esc {
            pmatch(comp, key)
        } else {
            key.eq(comp)
        }
    } else {
        let key = tostr(&key[1..key.len() - 1]);
        if path.pat || path.esc {
            pmatch(comp, key)
        } else {
            key.eq(comp)
        }
    }
}

fn get_arr<'a>(
    json: &'a [u8],
    i: usize,
    lines: bool,
    path: Path<'a>,
) -> (Value<'a>, usize, Path<'a>) {
    // Array paths are special.
    // There are a few different ways to handling arrays:
    // - By Index: Return a single child at a specified index.
    // - Count: Return a count of all children.
    // - Query: Return a single child using a query.
    // - Sub path (recomposition): Creates a new array from child paths.
    // - Query + Sub path (recomp): Create a new array from child querys.
    // The `lines` param allows for the input to be in JSON Lines format,
    // where, rather than having [value1,value2,value3], each value is on
    // a separate line like:
    // ```
    // value1
    // value2
    // value3
    // ```
    if path.comp.len() > 0 && path.comp[0] == b'#' {
        if path.comp.len() == 1 {
            if path.sep == b'.' {
                get_arr_children_with_subpath(json, i, lines, path)
            } else {
                get_arr_count(json, i, lines, path)
            }
        } else if path.comp[path.comp.len() - 1] == b'#' {
            get_arr_children_with_query_subpath(json, i, lines, path)
        } else {
            get_arr_child_with_query(json, i, lines, path)
        }
    } else {
        get_arr_child_at_index(json, i, lines, path)
    }
}

fn get_arr_count<'a>(
    json: &'a [u8],
    mut i: usize,
    lines: bool,
    path: Path<'a>,
) -> (Value<'a>, usize, Path<'a>) {
    let mut count = 0;
    i = for_each(json, i, lines, Kind::Array, |_, _| {
        count += 1;
        true
    });
    let res = json_from_owned(format!("{}", count), None, INFO_NUMBER);
    (res, i, path)
}

fn get_arr_child_at_index<'a>(
    json: &'a [u8],
    mut i: usize,
    lines: bool,
    path: Path<'a>,
) -> (Value<'a>, usize, Path<'a>) {
    let comp_index = tostr(path.comp).parse::<i64>().unwrap_or(-1);
    let mut res = Value::default();
    let mut index = 0;
    let mut next_i = 0;
    i = for_each(json, i, lines, Kind::Array, |_, value| {
        if index == comp_index {
            res = value;
            next_i = i;
            return false;
        }
        index += 1;
        true
    });
    if res.exists() {
        (res, next_i, path)
    } else {
        (Value::default(), i, path)
    }
}

fn query_matches<'a>(valin: &Value<'a>, op: &str, rpv: &str) -> bool {
    let uesc_str: String;
    let mut rpv = rpv.as_bytes();
    if rpv.len() > 2 && rpv[0] == b'"' && rpv[rpv.len() - 1] == b'"' {
        let mut overwrite = false;
        for c in rpv {
            if *c == b'\\' {
                overwrite = true;
                uesc_str = unescape(tostr(rpv));
                rpv = uesc_str.as_bytes();
                break;
            }
        }
        if !overwrite {
            rpv = &rpv[1..rpv.len() - 1];
        }
    }
    let mut value = valin;
    let mut tvalue = Value::default();
    if rpv.len() > 0 && rpv[0] == b'~' {
        // convert to bool
        rpv = &rpv[1..];
        if value.bool() {
            tvalue.slice = "true";
            tvalue.info = INFO_TRUE;
        } else {
            tvalue.slice = "false";
            tvalue.info = INFO_FALSE;
        }
        value = &tvalue;
    }
    let rpv = tostr(rpv);
    if !value.exists() {
        return false;
    }
    if op == "" {
        // the query is only looking for existence, such as:
        //   friends.#(name)
        // which makes sure that the array "friends" has an element of
        // "name" that exists
        return true;
    }
    match value.kind() {
        Kind::String => match op {
            "=" => value.str() == rpv,
            "!=" => value.str() != rpv,
            "<" => value.str() < rpv,
            "<=" => value.str() <= rpv,
            ">" => value.str() > rpv,
            ">=" => value.str() >= rpv,
            "%" => pmatch(rpv, value.str()),
            "!%" => !pmatch(rpv, value.str()),
            _ => false,
        },
        Kind::Number => {
            let rpvn = rpv.parse().unwrap_or(0.0);
            match op {
                "=" => value.f64() == rpvn,
                "!=" => value.f64() != rpvn,
                "<" => value.f64() < rpvn,
                "<=" => value.f64() <= rpvn,
                ">" => value.f64() > rpvn,
                ">=" => value.f64() >= rpvn,
                _ => false,
            }
        }
        Kind::True => match op {
            "=" => rpv == "true",
            "!=" => rpv != "true",
            ">" => rpv == "false",
            ">=" => true,
            _ => false,
        },
        Kind::False => match op {
            "=" => rpv == "false",
            "!=" => rpv != "false",
            "<" => rpv == "true",
            "<=" => true,
            _ => false,
        },
        _ => false,
    }
}

fn get_arr_child_with_query<'a>(
    json: &'a [u8],
    mut i: usize,
    lines: bool,
    path: Path<'a>,
) -> (Value<'a>, usize, Path<'a>) {
    let (lh, op, rhv) = path.query_parts();
    let mut res = Value::default();
    i = for_each(json, i, lines, Kind::Array, |_, value| {
        let is_match = if lh != "" {
            query_matches(&value.get(lh), op, rhv)
        } else {
            query_matches(&value, op, rhv)
        };
        if is_match {
            res = value;
            return false;
        }
        true
    });
    if res.exists() {
        (res, i, path)
    } else {
        (Value::default(), i, path)
    }
}

fn get_arr_children_with_query_subpath<'a>(
    json: &'a [u8],
    mut i: usize,
    lines: bool,
    mut path: Path<'a>,
) -> (Value<'a>, usize, Path<'a>) {
    let (lh, op, rhv) = path.query_parts();
    let mut subpath = None;
    let r = path.next_group();
    if path.sep == b'.' {
        subpath = Some(r.0);
    }
    path = r.1;
    let mut res = Vec::new();
    res.push(b'[');
    let mut index = 0;
    i = for_each(json, i, lines, Kind::Array, |_, value| {
        let is_match = if lh != "" {
            query_matches(&value.get(lh), op, rhv)
        } else {
            query_matches(&value, op, rhv)
        };
        if is_match {
            let value = if let Some(subpath) = subpath {
                value.get(subpath)
            } else {
                value
            };
            if value.exists() {
                if index > 0 {
                    res.push(b',');
                }
                res.extend(value.json().as_bytes());
                index += 1;
            }
        }
        true
    });
    res.push(b']');
    let res = json_from_owned(
        // SAFETY: buffer was constructed from known utf8 parts.
        unsafe { String::from_utf8_unchecked(res) },
        None,
        INFO_ARRAY,
    );
    (res, i, path)
}

fn get_arr_children_with_subpath<'a>(
    json: &'a [u8],
    mut i: usize,
    lines: bool,
    mut path: Path<'a>,
) -> (Value<'a>, usize, Path<'a>) {
    let r = path.next_group();
    let subpath = r.0;
    path = r.1;
    let mut res = Vec::new();
    res.push(b'[');
    let mut index = 0;
    i = for_each(json, i, lines, Kind::Array, |_, value| {
        let value = value.get(subpath);
        if value.exists() {
            if index > 0 {
                res.push(b',');
            }
            res.extend(value.json().as_bytes());
            index += 1;
        }
        true
    });
    res.push(b']');
    let res = json_from_owned(
        // SAFETY: buffer was constructed from known utf8 parts.
        unsafe { String::from_utf8_unchecked(res) },
        None,
        INFO_ARRAY,
    );
    (res, i, path)
}

/// Searches json for the specified path.
/// A path is in dot syntax, such as "name.last" or "age".
/// When the value is found it's returned immediately.
///
/// A path is a series of keys separated by a dot.
/// A key may contain special wildcard characters '*' and '?'.
/// To access an array value use the index as the key.
/// To get the number of elements in an array or to access a child path, use
/// the '#' character.
/// The dot and wildcard character can be escaped with '\'.
///
/// ```json
/// {
///   "name": {"first": "Tom", "last": "Anderson"},
///   "age":37,
///   "children": ["Sara","Alex","Jack"],
///   "friends": [
///     {"first": "James", "last": "Murphy"},
///     {"first": "Roger", "last": "Craig"}
///   ]
/// }
/// ```
///
/// ```json
///  "name.last"          >> "Anderson"
///  "age"                >> 37
///  "children"           >> ["Sara","Alex","Jack"]
///  "children.#"         >> 3
///  "children.1"         >> "Alex"
///  "child*.2"           >> "Jack"
///  "c?ildren.0"         >> "Sara"
///  "friends.#.first"    >> ["James","Roger"]
/// ```
///
/// This function expects that the json is valid, and does not validate.
/// Invalid json will not panic, but it may return back unexpected results.
/// If you are consuming JSON from an unpredictable source then you may want to
/// use the `valid` function first.
#[inline]
pub fn get<'a>(json: &'a str, path: &'a str) -> Value<'a> {
    unsafe { get_bytes(json.as_bytes(), path) }
}

/// Searches json for the specified path.
/// A path is in dot syntax, such as "name.last" or "age".
/// When the value is found it's returned immediately.
///
/// A path is a series of keys separated by a dot.
/// A key may contain special wildcard characters '*' and '?'.
/// To access an array value use the index as the key.
/// To get the number of elements in an array or to access a child path, use
/// the '#' character.
/// The dot and wildcard character can be escaped with '\'.
///
/// ```json
/// {
///   "name": {"first": "Tom", "last": "Anderson"},
///   "age":37,
///   "children": ["Sara","Alex","Jack"],
///   "friends": [
///     {"first": "James", "last": "Murphy"},
///     {"first": "Roger", "last": "Craig"}
///   ]
/// }
/// ```
///
/// ```json
///  "name.last"          >> "Anderson"
///  "age"                >> 37
///  "children"           >> ["Sara","Alex","Jack"]
///  "children.#"         >> 3
///  "children.1"         >> "Alex"
///  "child*.2"           >> "Jack"
///  "c?ildren.0"         >> "Sara"
///  "friends.#.first"    >> ["James","Roger"]
/// ```
///
/// This function expects that the json is valid, and does not validate.
/// Invalid json will not panic, but it may return back unexpected results.
/// If you are consuming JSON from an unpredictable source then you may want to
/// use the `valid` function first.
pub unsafe fn get_bytes<'a>(json: &'a [u8], path: &'a str) -> Value<'a> {
    let mut path = path;
    let mut lines = false;
    if path.len() >= 2 && path.as_bytes()[0] == b'.' && path.as_bytes()[1] == b'.' {
        // json lines
        path = tostr(&path.as_bytes()[2..]);
        lines = true;
    }
    let path = Path::new(path);
    let (res, path) = {
        if lines {
            let res = get_arr(json, 0, true, path);
            (res.0, res.2)
        } else if path.is_modifier() {
            modifiers::exec(json, path)
        } else if path.is_multipath() {
            multipath::exec(json, path)
        } else {
            let mut i = 0;
            loop {
                if i == json.len() {
                    break (Value::default(), path);
                }
                if json[i] <= b' ' {
                    i += 1;
                    continue;
                }
                if json[i] == b'{' {
                    let res = get_obj(json, i, path);
                    break (res.0, res.2);
                }
                if json[i] == b'[' {
                    let res = get_arr(json, i, false, path);
                    break (res.0, res.2);
                }
                break (Value::default(), path);
            }
        }
    };
    if !path.more() {
        return res;
    }
    let path = tostr(path.extra);
    let mut json = if res.slice.len() > 0 {
        get(&res.slice, path)
    } else {
        json_into_owned(get(&res.owned, path))
    };
    let mut index = None;
    if let Some(index1) = res.index {
        if let Some(index2) = json.index {
            index = Some(index1 + index2);
        }
    }
    json.index = index;
    json
}

fn json_into_owned<'a>(json: Value) -> Value<'a> {
    Value {
        slice: "",
        owned: if json.slice.len() > 0 {
            json.slice.to_owned()
        } else {
            json.owned
        },
        uescstr: json.uescstr,
        info: json.info,
        index: json.index,
    }
}

/// Parse the json and return it as a value.
///
/// This function expects that the json is valid, and does not validate.
/// Invalid json will not panic, but it may return back unexpected results.
/// If you are consuming JSON from an unpredictable source then you may want to
/// use the `valid` function first.
pub fn parse<'a>(json: &'a str) -> Value<'a> {
    let json = json.as_bytes();
    let mut i = 0;
    while i < json.len() {
        if json[i] <= b' ' {
            i += 1;
            continue;
        }
        match json[i] {
            b'{' => return json_from_slice(&json[i..], Some(i), INFO_OBJECT | INFO_FOG),
            b'[' => return json_from_slice(&json[i..], Some(i), INFO_ARRAY | INFO_FOG),
            b't' | b'f' | b'n' | b'"' | b'0' | b'1' | b'2' => {}
            b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'-' => {}
            _ => break,
        }
        return proc_value(json, i, Path::default(), true).0;
    }
    return Value::default();
}
