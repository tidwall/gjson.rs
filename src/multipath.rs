// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

use super::path::Path;
use super::util::*;
use super::*;

// name_of_last returns the name of the last component
fn name_of_last<'a>(path: &'a [u8]) -> &'a [u8] {
    for i in 0..path.len() {
        let i = path.len() - 1 - i;
        if path[i] == b'|' || path[i] == b'.' {
            if i > 0 {
                if path[i - 1] == b'\\' {
                    continue;
                }
            }
            return &path[i + 1..];
        }
    }
    path
}

// is_simple_name returns true if the component name is simple enough to use as
// a multipath key.
fn is_simple_name(comp: &[u8]) -> bool {
    for i in 0..comp.len() {
        if comp[i] < b' ' {
            return false;
        }
        match comp[i] {
            b'[' | b']' | b'{' | b'}' | b'(' | b')' | b'#' | b'|' => {
                return false;
            }
            _ => {}
        }
    }
    true
}

fn key_for_path<'a>(path: &'a [u8]) -> &'a [u8] {
    let key = name_of_last(path);
    if is_simple_name(key) {
        key
    } else {
        "_".as_bytes()
    }
}

fn each_comp(path: &[u8], mut iter: impl FnMut(&[u8], &[u8])) {
    let path = &path[1..path.len() - 1];
    let mut i = 0;
    let mut c = None;
    let mut s = 0;
    loop {
        if i == path.len() {
            if let Some(c) = c {
                iter(&path[s..c], &path[c + 1..i]);
            } else {
                iter(key_for_path(&path[s..i]), &path[s..i]);
            }
            break;
        }
        match path[i] {
            b'\\' => {
                i += 1;
                if i == path.len() {
                    break;
                }
            }
            b'(' | b'[' | b'{' => {
                let (_, next_i) = scan_squash(path, i);
                i = next_i;
                continue;
            }
            b',' => {
                if let Some(c) = c {
                    iter(&path[s..c], &path[c + 1..i]);
                } else {
                    iter(key_for_path(&path[s..i]), &path[s..i]);
                }
                s = i + 1;
                c = None;
            }
            b':' => {
                c = Some(i);
            }
            _ => {}
        }
        i += 1;
    }
}

pub fn exec<'a>(json: &'a [u8], path: Path<'a>) -> (Value<'a>, Path<'a>) {
    // it's expected that path.comp starts with a '[' or '{'
    if path.comp[0] == b'[' {
        exec_arr(json, path)
    } else {
        exec_obj(json, path)
    }
}

fn exec_arr<'a>(json: &'a [u8], path: Path<'a>) -> (Value<'a>, Path<'a>) {
    if path.comp[0] == b'[' && path.comp[path.comp.len() - 1] != b']' {
        return (Value::default(), Path::default());
    }
    let mut out = Vec::new();
    out.push(b'[');
    let mut index = 0;
    each_comp(path.comp, |_, path| {
        let res = get(tostr(json), tostr(path));
        if res.exists() {
            if index > 0 {
                out.push(b',');
            }
            out.extend(res.json().as_bytes());
            index += 1;
        }
    });
    out.push(b']');
    let json = unsafe { String::from_utf8_unchecked(out) };
    (json_from_owned(json, None, INFO_ARRAY), path)
}

fn exec_obj<'a>(json: &'a [u8], path: Path<'a>) -> (Value<'a>, Path<'a>) {
    if path.comp[0] == b'{' && path.comp[path.comp.len() - 1] != b'}' {
        return (Value::default(), Path::default());
    }
    let mut out = Vec::new();
    out.push(b'{');
    let mut index = 0;
    each_comp(path.comp, |key, path| {
        let res = get(tostr(json), tostr(path));
        if res.exists() {
            if index > 0 {
                out.push(b',');
            }
            extend_json_string(&mut out, key);
            out.push(b':');
            out.extend(res.json().as_bytes());
            index += 1;
        }
    });
    out.push(b'}');
    let json = unsafe { String::from_utf8_unchecked(out) };
    (json_from_owned(json, None, INFO_OBJECT), path)
}
