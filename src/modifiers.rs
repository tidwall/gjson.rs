// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

use super::path::Path;
use super::pretty;
use super::valid::valid;
use super::*;
use std::collections::HashMap;
use std::str;

pub fn exec<'a>(json: &'a [u8], path: Path<'a>) -> (Value<'a>, Path<'a>) {
    let (name, json_str, arg);
    // SAFETY: all json and path parts are prechecked utf8
    unsafe {
        if path.marg == 0 {
            name = str::from_utf8_unchecked(&path.comp[1..]);
            json_str = str::from_utf8_unchecked(json);
            arg = "";
        } else {
            name = str::from_utf8_unchecked(&path.comp[1..path.marg]);
            json_str = str::from_utf8_unchecked(json);
            arg = str::from_utf8_unchecked(&path.comp[path.marg + 1..]);
        }
    };
    let json = json_str;
    let json = match name {
        "this" => mod_this(json, arg),
        "reverse" => mod_reverse(json, arg),
        "ugly" => mod_ugly(json, arg),
        "pretty" => mod_pretty(json, arg),
        "valid" => mod_valid(json, arg),
        "flatten" => mod_flatten(json, arg),
        "join" => mod_join(json, arg),
        _ => String::new(),
    };
    (json_into_owned(parse(&json)), path)
}

fn mod_this(json: &str, _: &str) -> String {
    json.to_owned()
}

fn mod_valid(json: &str, _: &str) -> String {
    if valid(json) {
        json.to_owned()
    } else {
        String::new()
    }
}

fn mod_pretty(json: &str, arg: &str) -> String {
    if arg.len() > 0 {
        let mut opts = pretty::PrettyOptions::new();
        let indent = super::get(arg, "indent");
        let prefix = super::get(arg, "prefix");
        let sort_keys = super::get(arg, "sortKeys");
        let width = super::get(arg, "width");
        if indent.exists() {
            opts = opts.indent(indent.str());
        }
        if prefix.exists() {
            opts = opts.prefix(prefix.str());
        }
        if sort_keys.exists() {
            opts = opts.sort_keys(sort_keys.bool());
        }
        if width.exists() {
            opts = opts.width(width.u32() as usize);
        }
        opts.pretty(json)
    } else {
        pretty::pretty(json)
    }
}

fn mod_ugly(json: &str, _: &str) -> String {
    pretty::ugly(json)
}

fn mod_reverse(json: &str, _: &str) -> String {
    let res = parse(json);
    let json = res.slice.as_bytes();
    let mut slices = Vec::new();
    let endcaps;
    let mut cap = 2;
    match res.kind() {
        Kind::Object => {
            endcaps = (b'{', b'}');
            res.each(|key, value| {
                let kindex = key.index.unwrap();
                let vindex = value.index.unwrap();
                let slice = &json[kindex..vindex + value.slice.len()];
                slices.push(slice);
                cap += 1 + slice.len();
                return true;
            });
        }
        Kind::Array => {
            endcaps = (b'[', b']');
            res.each(|_, value| {
                let vindex = value.index.unwrap();
                let slice = &json[vindex..vindex + value.slice.len()];
                slices.push(slice);
                cap += 1 + slice.len();
                return true;
            });
        }
        _ => return tostr(json).to_owned(),
    }
    let mut out: Vec<u8> = Vec::with_capacity(cap);
    out.push(endcaps.0);
    for i in 0..slices.len() {
        if i > 0 {
            out.push(b',');
        }
        out.extend(slices[slices.len() - 1 - i]);
    }
    out.push(endcaps.1);
    // SAFETY: buffer was constructed from known utf8 parts.
    unsafe { String::from_utf8_unchecked(out) }
}

fn mod_join(json: &str, arg: &str) -> String {
    let res = parse(json);
    if res.kind() != Kind::Array {
        return json.to_owned();
    }
    let preserve = get(arg, "preserve").bool();
    let mut out = Vec::new();
    out.push(b'{');
    if preserve {
        // Preserve duplicate keys.
        let mut idx = 0;
        res.each(|_, value| {
            if value.kind() != Kind::Object {
                return true;
            }
            if idx > 0 {
                out.push(b',');
            }
            out.extend(unwrap(value.slice.as_bytes()));
            idx += 1;
            return true;
        })
    } else {
        // Deduplicate keys and generate an object with stable ordering.
        let mut keys = Vec::new();
        let mut kvals: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
        res.each(|_, value| {
            if value.kind() != Kind::Object {
                return true;
            }
            value.each(|key, value| {
                let k = key.str().as_bytes().to_owned();
                if !kvals.contains_key(&k) {
                    let key = key.json().as_bytes().to_owned();
                    keys.push((key, k.clone()));
                }
                kvals.insert(k, value.json().as_bytes().to_owned());
                return true;
            });
            return true;
        });

        for i in 0..keys.len() {
            if i > 0 {
                out.push(b',');
            }
            out.extend(&keys[i].0);
            out.push(b':');
            out.extend(kvals.get(&keys[i].1).unwrap());
        }
    }
    out.push(b'}');
    // SAFETY: buffer was constructed from known utf8 parts.
    unsafe { String::from_utf8_unchecked(out) }
}

// @flatten an array with child arrays.
//   [1,[2],[3,4],[5,[6,7]]] -> [1,2,3,4,5,[6,7]]
// The {"deep":true} arg can be provide for deep flattening.
//   [1,[2],[3,4],[5,[6,7]]] -> [1,2,3,4,5,6,7]
// The original json is returned when the json is not an array.
fn mod_flatten(json: &str, arg: &str) -> String {
    let res = parse(json);
    if res.kind() != Kind::Array {
        return json.to_owned();
    }
    let deep = get(arg, "deep").bool();
    let mut out = Vec::new();
    out.push(b'[');
    let mut idx = 0;
    res.each(|_, value| {
        let raw;
        if value.kind() == Kind::Array {
            if deep {
                raw = unwrap(mod_flatten(value.json(), arg).as_bytes()).to_owned();
            } else {
                raw = unwrap(value.json().as_bytes()).to_owned();
            }
        } else {
            raw = value.slice.as_bytes().to_owned();
        }
        if raw.len() > 0 {
            if idx > 0 {
                out.push(b',');
            }
            out.extend(&raw);
            idx += 1;
        }
        return true;
    });
    out.push(b']');
    // SAFETY: buffer was constructed from known utf8 parts.
    unsafe { String::from_utf8_unchecked(out) }
}

fn unwrap<'a>(mut json: &'a [u8]) -> &'a [u8] {
    while json.len() > 0 && json[0] <= b' ' {
        json = &json[1..];
    }
    while json.len() > 0 && json[json.len() - 1] <= b' ' {
        json = &json[..json.len() - 1];
    }
    if json.len() >= 2 && (json[0] == b'[' || json[0] == b'{') {
        json = &json[1..json.len() - 1];
    }
    json
}
