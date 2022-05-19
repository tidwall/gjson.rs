// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

use std::cmp::Ordering;

// maxDepth is maximum number of nested objects and arrays
const MAX_DEPTH: usize = 500;

struct InnerOptions<'a> {
    // Width is an max column width for single line arrays
    // Default is 80
    width: i64,
    // Prefix is a prefix for all lines
    // Default is an empty string
    prefix: &'a str,
    // Indent is the nested indentation
    // Default is two spaces
    indent: &'a str,
    // sort_keys will sort the keys alphabetically
    // Default is false
    sort_keys: bool,
}

pub struct PrettyOptions<'a> {
    inner: InnerOptions<'a>,
}

impl<'a> Default for PrettyOptions<'a> {
    fn default() -> Self {
        PrettyOptions {
            inner: InnerOptions {
                width: 80,
                prefix: "",
                indent: "  ",
                sort_keys: false,
            },
        }
    }
}

impl<'a> PrettyOptions<'a> {
    pub fn new() -> PrettyOptions<'a> {
        PrettyOptions::default()
    }
    pub fn width(mut self, width: usize) -> Self {
        self.inner.width = width as i64;
        self
    }
    pub fn prefix(mut self, prefix: &'a str) -> Self {
        self.inner.prefix = prefix;
        self
    }
    pub fn indent(mut self, indent: &'a str) -> Self {
        self.inner.indent = indent;
        self
    }
    pub fn sort_keys(mut self, sort_keys: bool) -> Self {
        self.inner.sort_keys = sort_keys;
        self
    }
    pub fn pretty(&self, json: &[u8]) -> Vec<u8> {
        pretty_options(json, self)
    }
}

pub fn pretty(json: &[u8]) -> Vec<u8> {
    PrettyOptions::default().pretty(json)
}

fn pretty_options(json: &[u8], opts: &PrettyOptions) -> Vec<u8> {
    let mut buf = Vec::with_capacity(json.len());
    let prefix = opts.inner.prefix.as_bytes();
    if !prefix.is_empty() {
        buf.extend(prefix);
    }
    extend_pretty_any(
        &mut buf,
        json,
        0,
        true,
        opts.inner.width,
        prefix,
        opts.inner.indent.as_bytes(),
        opts.inner.sort_keys,
        0,
        0,
        -1,
        0,
    );
    if !buf.is_empty() {
        buf.push(b'\n');
    }
    buf
}

fn extend_pretty_any(
    buf: &mut Vec<u8>,
    json: &[u8],
    mut i: usize,
    pretty: bool,
    width: i64,
    prefix: &[u8],
    indent: &[u8],
    sort_keys: bool,
    tabs: i64,
    nl: i64,
    max: i64,
    depth: usize,
) -> (usize, i64, bool) {
    while i < json.len() {
        if json[i] <= b' ' {
            i += 1;
            continue;
        }
        if json[i] == b'"' {
            return extend_pretty_string(buf, json, i, nl);
        }
        if (json[i] >= b'0' && json[i] <= b'9') || json[i] == b'-' {
            return extend_pretty_number(buf, json, i, nl);
        }
        if json[i] == b'{' {
            return extend_pretty_object(
                buf,
                json,
                i,
                b'{',
                b'}',
                pretty,
                width,
                prefix,
                indent,
                sort_keys,
                tabs,
                nl,
                max,
                depth + 1,
            );
        }
        if json[i] == b'[' {
            return extend_pretty_object(
                buf,
                json,
                i,
                b'[',
                b']',
                pretty,
                width,
                prefix,
                indent,
                sort_keys,
                tabs,
                nl,
                max,
                depth + 1,
            );
        }
        match json[i] {
            b't' => {
                buf.extend("true".as_bytes());
                return (i + 4, nl, true);
            }
            b'f' => {
                buf.extend("false".as_bytes());
                return (i + 5, nl, true);
            }
            b'n' => {
                buf.extend("null".as_bytes());
                return (i + 4, nl, true);
            }
            _ => {}
        }
        i += 1;
    }
    (i, nl, true)
}

fn extend_pretty_string(
    buf: &mut Vec<u8>,
    json: &[u8],
    mut i: usize,
    nl: i64,
) -> (usize, i64, bool) {
    let s = i;
    i += 1;
    while i < json.len() {
        if json[i] == b'"' {
            let mut sc = 0;
            let mut j = i - 1;
            while j > s {
                if json[j] == b'\\' {
                    sc += 1;
                } else {
                    break;
                }
                j -= 1;
            }
            if sc % 2 == 1 {
                i += 1;
                continue;
            }
            i += 1;
            break;
        }
        i += 1;
    }
    buf.extend(&json[s..i]);
    (i, nl, true)
}

fn extend_pretty_number(
    buf: &mut Vec<u8>,
    json: &[u8],
    mut i: usize,
    nl: i64,
) -> (usize, i64, bool) {
    let s = i;
    i += 1;
    while i < json.len() {
        if json[i] <= b' '
            || json[i] == b','
            || json[i] == b':'
            || json[i] == b']'
            || json[i] == b'}'
        {
            break;
        }
        i += 1;
    }
    buf.extend(&json[s..i]);
    (i, nl, true)
}

#[derive(Default)]
struct Pair {
    kstart: usize,
    kend: usize,
    vstart: usize,
    vend: usize,
}

fn extend_pretty_object(
    buf: &mut Vec<u8>,
    json: &[u8],
    mut i: usize,
    open: u8,
    close: u8,
    pretty: bool,
    width: i64,
    prefix: &[u8],
    indent: &[u8],
    sort_keys: bool,
    tabs: i64,
    mut nl: i64,
    max: i64,
    depth: usize,
) -> (usize, i64, bool) {
    if depth == MAX_DEPTH {
        let fragment = ugly_bytes(&json[i..]);
        buf.extend(fragment);
        return (json.len(), nl, true);
    }
    let mut ok;
    if width > 0 {
        if pretty && open == b'[' && max == -1 {
            // here we try to create a single line array
            let max = (width as i64) - ((buf.len() as i64) - nl);
            if max > 3 {
                let (s1, s2) = (buf.len(), i);
                let res = extend_pretty_object(
                    buf,
                    json,
                    i,
                    b'[',
                    b']',
                    false,
                    width,
                    prefix,
                    "".as_bytes(),
                    sort_keys,
                    0,
                    0,
                    max,
                    depth,
                );
                i = res.0;
                ok = res.2;
                if ok && (buf.len() as i64) - (s1 as i64) <= max {
                    return (i, nl, true);
                }
                buf.truncate(s1);
                i = s2;
            }
        } else if max != -1 && open == b'{' {
            return (i, nl, false);
        }
    }
    buf.push(open);
    i += 1;
    let mut pairs = Vec::new();
    let mut n = 0;
    while i < json.len() {
        if json[i] <= b' ' {
            i += 1;
            continue;
        }
        if json[i] == close {
            if pretty {
                if open == b'{' && sort_keys {
                    sort_pairs(json, buf, &mut pairs);
                }
                if n > 0 {
                    nl = buf.len() as i64;
                    if buf[(nl - 1) as usize] != b' ' {
                        buf.push(b'\n');
                    }
                }
                if buf[buf.len() - 1] != open {
                    extend_tabs(buf, prefix, indent, tabs);
                }
            }
            buf.push(close);
            return (i + 1, nl, open != b'{');
        }
        if open == b'[' || json[i] == b'"' {
            if n > 0 {
                buf.push(b',');
                if width != -1 && open == b'[' {
                    buf.push(b' ');
                }
            }
            let mut p = Pair::default();
            if pretty {
                nl = buf.len() as i64;
                if buf[(nl - 1) as usize] == b' ' {
                    buf[(nl - 1) as usize] = b'\n';
                } else {
                    buf.push(b'\n');
                }
                if open == b'{' && sort_keys {
                    p.kstart = i;
                    p.vstart = buf.len();
                }
                extend_tabs(buf, prefix, indent, tabs + 1);
            }
            if open == b'{' {
                let res = extend_pretty_string(buf, json, i, nl);
                i = res.0;
                nl = res.1;
                if sort_keys {
                    p.kend = i;
                }
                buf.push(b':');
                if pretty {
                    buf.push(b' ');
                }
            }
            let r = extend_pretty_any(
                buf,
                json,
                i,
                pretty,
                width,
                prefix,
                indent,
                sort_keys,
                tabs + 1,
                nl,
                max,
                depth,
            );
            i = r.0;
            nl = r.1;
            ok = r.2;
            if max != -1 && !ok {
                return (i, nl, false);
            }
            if pretty && open == b'{' && sort_keys {
                p.vend = buf.len();
                if p.kstart <= p.kend && p.vstart <= p.vend {
                    pairs.push(p);
                }
            }
            i -= 1;
            n += 1;
        }
        i += 1;
    }
    (i, nl, open != b'{')
}

fn sort_pairs(json: &[u8], buf: &mut Vec<u8>, pairs: &mut Vec<Pair>) {
    if pairs.is_empty() {
        return;
    }
    let vstart = pairs[0].vstart;
    let vend = pairs[pairs.len() - 1].vend;
    pairs.sort_by(|a, b| {
        let key1 = &json[a.kstart + 1..a.kend - 1];
        let key2 = &json[b.kstart + 1..b.kend - 1];
        let cmp = key1.cmp(key2);
        if cmp == Ordering::Equal {
            a.vstart.cmp(&b.vstart)
        } else {
            cmp
        }
    });
    let mut nbuf: Vec<u8> = Vec::with_capacity(vend - vstart);
    for i in 0..pairs.len() {
        let p = &pairs[i];
        nbuf.extend(&buf[p.vstart..p.vend]);
        if i < pairs.len() - 1 {
            nbuf.push(b',');
            nbuf.push(b'\n');
        }
    }
    buf.truncate(vstart);
    buf.extend(nbuf);
}

fn extend_tabs(buf: &mut Vec<u8>, prefix: &[u8], indent: &[u8], tabs: i64) {
    if !prefix.is_empty() {
        buf.extend(prefix);
    }
    for _ in 0..tabs {
        buf.extend(indent);
    }
}

pub fn ugly_bytes(src: &[u8]) -> Vec<u8> {
    let mut dst = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] > b' ' {
            dst.push(src[i]);
            if src[i] == b'"' {
                i += 1;
                while i < src.len() {
                    dst.push(src[i]);
                    if src[i] == b'"' {
                        let mut j = i - 1;
                        loop {
                            if src[j] != b'\\' {
                                break;
                            }
                            j -= 1;
                        }
                        if (i - j) % 2 != 0 {
                            break;
                        }
                    }
                    i += 1;
                }
            }
        }
        i += 1;
    }
    dst
}

pub fn ugly(json: &str) -> Vec<u8> {
    ugly_bytes(json.as_bytes())
}

#[cfg(test)]
mod test {

    const EXAMPLE_PRETTY: &str = r#"{
  "name": {
    "last": "Sanders",
    "first": "Janet"
  },
  "children": ["Andy", "Carol", "Mike"],
  "values": [
    10.10,
    true,
    false,
    null,
    "hello",
    {
      "a": "b",
      "c": "d"
    },
    []
  ],
  "values2": {},
  "values3": [],
  "deep": {
    "deep": {
      "deep": [1, 2, 3, 4, 5]
    }
  }
}
"#;
    const EXAMPLE_UGLY: &str = r#"{"name":{"last":"Sanders","first":"Janet"},"children":["Andy","Carol","Mike"],"values":[10.10,true,false,null,"hello",{"a":"b","c":"d"},[]],"values2":{},"values3":[],"deep":{"deep":{"deep":[1,2,3,4,5]}}}"#;

    #[test]
    fn ugly() {
        assert_eq!(super::ugly(EXAMPLE_PRETTY), EXAMPLE_UGLY.as_bytes());
        assert_eq!(
            super::pretty(&super::ugly(EXAMPLE_PRETTY)),
            EXAMPLE_PRETTY.as_bytes()
        );
    }
    #[test]
    fn pretty() {
        assert_eq!(
            super::pretty(EXAMPLE_UGLY.as_bytes()),
            EXAMPLE_PRETTY.as_bytes()
        );
        let res = super::PrettyOptions::new()
            .prefix("\t")
            .width(10)
            .sort_keys(true)
            .indent("   ")
            .pretty(EXAMPLE_UGLY.as_bytes());
        let expect = r#"	{
	   "children": [
	      "Andy",
	      "Carol",
	      "Mike"
	   ],
	   "deep": {
	      "deep": {
	         "deep": [
	            1,
	            2,
	            3,
	            4,
	            5
	         ]
	      }
	   },
	   "name": {
	      "first": "Janet",
	      "last": "Sanders"
	   },
	   "values": [
	      10.10,
	      true,
	      false,
	      null,
	      "hello",
	      {
	         "a": "b",
	         "c": "d"
	      },
	      []
	   ],
	   "values2": {},
	   "values3": []
	}
"#;
        assert_eq!(res, expect.as_bytes());
    }

    #[test]
    fn xcover() {
        let res = super::ugly_bytes(
            &super::PrettyOptions::new()
                .sort_keys(true)
                .pretty(r#"{"hello":"JELLO","hello":"HELLO"}"#.as_bytes()),
        );
        assert_eq!(res, r#"{"hello":"JELLO","hello":"HELLO"}"#.as_bytes());
        super::PrettyOptions::new()
            .sort_keys(true)
            .pretty(r#"{"hello":"JELLO","hello":"HELLO"}"#.as_bytes());

        super::pretty(r#"{"#.as_bytes());
        super::pretty(r#"r"#.as_bytes());
    }

    #[test]
    fn depth() {
        const JSON: &str = r#"
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[
      "hello", "jello", {"hello": "jello"}, true, false, null, 0.0
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]      
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]
      ]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]]      
      "#;

        println!(
            "{}",
            String::from_utf8_lossy(&super::pretty(JSON.as_bytes()))
        );
    }
}
