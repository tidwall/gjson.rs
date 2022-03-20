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
    let (name, arg);
    // SAFETY: all json and path parts are prechecked utf8
    unsafe {
        if path.marg == 0 {
            name = str::from_utf8_unchecked(&path.comp[1..]);
            arg = "";
        } else {
            name = str::from_utf8_unchecked(&path.comp[1..path.marg]);
            arg = str::from_utf8_unchecked(&path.comp[path.marg + 1..]);
        }
    };
    let json = match name {
        "this" => mod_this(json, arg),
        "reverse" => mod_reverse(json, arg),
        "ugly" => mod_ugly(json, arg),
        "pretty" => mod_pretty(json, arg),
        "valid" => mod_valid(json, arg),
        "flatten" => mod_flatten(json, arg),
        "join" => mod_join(json, arg),
        _ => Vec::new(),
    };
    (json_into_owned(parse_bytes(&json)), path)
}

fn mod_this(json: &[u8], _: &str) -> Vec<u8> {
    json.to_vec()
}

fn mod_valid(json: &[u8], _: &str) -> Vec<u8> {
    if valid(json) {
        json.to_vec()
    } else {
        Vec::new()
    }
}

fn mod_pretty(json: &[u8], arg: &str) -> Vec<u8> {
    if arg.is_empty() {
        pretty::pretty(json)
    } else {
        let mut opts = pretty::PrettyOptions::new();
        let indent = super::get(arg, "indent");
        let prefix = super::get(arg, "prefix");
        let sort_keys = super::get(arg, "sortKeys");
        let width = super::get(arg, "width");
        let indent_ref = indent.str();
        let prefix_ref = prefix.str();

        if indent.exists() {
            opts = opts.indent(&indent_ref);
        }
        if prefix.exists() {
            opts = opts.prefix(&prefix_ref);
        }
        if sort_keys.exists() {
            opts = opts.sort_keys(sort_keys.bool());
        }
        if width.exists() {
            opts = opts.width(width.u32() as usize);
        }
        opts.pretty(json)
    }
}

fn mod_ugly(json: &[u8], _: &str) -> Vec<u8> {
    pretty::ugly_bytes(json)
}

fn mod_reverse(json: &[u8], _: &str) -> Vec<u8> {
    let res = parse_bytes(json);
    let json = &res.data;
    let mut slices = Vec::new();
    let endcaps;
    let mut cap = 2;
    match res.kind() {
        Kind::Object => {
            endcaps = (b'{', b'}');
            res.each(|key, value| {
                let kindex = key.index.unwrap();
                let vindex = value.index.unwrap();
                let slice = &json[kindex..vindex + value.data.len()];
                slices.push(slice);
                cap += 1 + slice.len();
                true
            });
        }
        Kind::Array => {
            endcaps = (b'[', b']');
            res.each(|_, value| {
                let vindex = value.index.unwrap();
                let slice = &json[vindex..vindex + value.data.len()];
                slices.push(slice);
                cap += 1 + slice.len();
                true
            });
        }
        _ => return json.to_vec(),
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
    out
}

fn mod_join(json: &[u8], arg: &str) -> Vec<u8> {
    let res = parse_bytes(json);
    if res.kind() != Kind::Array {
        return json.to_owned();
    }
    let preserve = get(arg, "preserve").bool();
    let mut out = vec![b'{'];
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
            out.extend(unwrap(&value.data));
            idx += 1;
            true
        });
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
                    let key = key.data.into_owned();
                    keys.push((key, k.clone()));
                }
                kvals.insert(k, value.data.into_owned());
                true
            });
            true
        });

        for (i, key) in keys.iter().enumerate() {
            if i > 0 {
                out.push(b',');
            }
            out.extend(&key.0);
            out.push(b':');
            out.extend(kvals.get(&key.1).unwrap());
        }
    }
    out.push(b'}');
    out
}

// @flatten an array with child arrays.
//   [1,[2],[3,4],[5,[6,7]]] -> [1,2,3,4,5,[6,7]]
// The {"deep":true} arg can be provide for deep flattening.
//   [1,[2],[3,4],[5,[6,7]]] -> [1,2,3,4,5,6,7]
// The original json is returned when the json is not an array.
fn mod_flatten(json: &[u8], arg: &str) -> Vec<u8> {
    let res = parse_bytes(json);
    if res.kind() != Kind::Array {
        return Vec::from(json);
    }
    let deep = get(arg, "deep").bool();
    let mut out = vec![b'['];
    let mut idx = 0;
    res.each(|_, value| {
        let raw;
        if value.kind() == Kind::Array {
            if deep {
                raw = unwrap(&mod_flatten(&value.data, arg)).to_owned();
            } else {
                raw = unwrap(&value.data).to_owned();
            }
        } else {
            raw = Vec::from(value.data);
        }
        if !raw.is_empty() {
            if idx > 0 {
                out.push(b',');
            }
            out.extend(&raw);
            idx += 1;
        }
        true
    });
    out.push(b']');
    out
}

fn unwrap<'a>(mut json: &'a [u8]) -> &'a [u8] {
    while !json.is_empty() && json[0] <= b' ' {
        json = &json[1..];
    }
    while !json.is_empty() && json[json.len() - 1] <= b' ' {
        json = &json[..json.len() - 1];
    }
    if json.len() >= 2 && (json[0] == b'[' || json[0] == b'{') {
        json = &json[1..json.len() - 1];
    }
    json
}
