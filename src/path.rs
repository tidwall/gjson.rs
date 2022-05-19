// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

use super::util::{tostr, trim};
use super::{proc_value, scan_squash};

#[derive(Copy, Clone)]
pub struct Path<'a> {
    pub comp: &'a [u8],
    pub esc: bool,
    pub pat: bool,
    pub sep: u8,
    pub marg: usize,
    pub extra: &'a [u8],
}

impl<'a> Path<'a> {
    pub fn more(&self) -> bool {
        self.sep != 0
    }
    pub fn new(path: &'a str) -> Self {
        let path = Path {
            comp: "".as_ref(),
            extra: path.as_ref(),
            esc: false,
            pat: false,
            sep: 0,
            marg: 0,
        };
        path_next(&path)
    }
    pub fn is_modifier(&self) -> bool {
        !self.comp.is_empty() && self.comp[0] == b'@'
    }
    pub fn is_multipath(&self) -> bool {
        !self.comp.is_empty() && (self.comp[0] == b'{' || self.comp[0] == b'[')
    }
    // next returns the next component
    pub fn next(&self) -> Path<'a> {
        path_next(self)
    }

    // next_group extracts the next group of components, which is a series of
    // components seperated by dots.
    // For example,
    //   Using the path `statuses.0.user|name.first`
    //   The current comonent is `statuses`
    //   The next component is `0`
    //   The next group is `0.user`
    // This operation returns the next group and the remaining path following
    // the group.
    // For the example above, the next group is `0.user` and reminging path
    // is `|name.first`.
    // -> (next_group, remaining_path)
    pub fn next_group(&self) -> (&'a str, Path<'a>) {
        if self.sep != b'.' {
            return ("", *self);
        }
        let mut remaining = self.next();
        let mut len = remaining.comp.len();
        while remaining.sep == b'.' {
            len += 1;
            remaining = path_next(&remaining);
            len += remaining.comp.len();
        }
        let group = tostr(&self.extra[..len]);
        remaining.comp = "".as_bytes();
        (group, remaining)
    }

    // -> lh, op, rh
    pub fn query_parts(&self) -> (&'a str, &'a str, &'a str) {
        let mut lh = "";
        let mut op = "";
        let mut rh = "";
        'bad: loop {
            // take the inner contents of the query
            let mut query = self.comp;
            if query.len() < 2 || query[0] != b'#' || query[1] != b'(' {
                break 'bad;
            } else if query[query.len() - 1] == b'#' {
                if query[query.len() - 2] == b')' {
                    query = &query[2..query.len() - 2];
                } else {
                    break 'bad;
                }
            } else if query[query.len() - 1] != b')' {
                break 'bad;
            } else {
                query = &query[2..query.len() - 1];
            }

            // trim the query
            query = trim(query);

            // locate the operator
            let mut depth = 0;
            let mut i = 0;
            while i < query.len() {
                if query[i] == b'\\' {
                    if i + 1 == query.len() {
                        break;
                    }
                    i += 2;
                    continue;
                }
                if query[i] == b'(' {
                    depth += 1;
                    i += 1;
                    continue;
                } else if query[i] == b')' {
                    depth -= 1;
                    i += 1;
                    continue;
                } else if depth > 0 {
                    i += 1;
                    continue;
                }

                let mut found = true;
                let mut s = 0;
                let mut e = 0;
                match query[i] {
                    b'(' => {
                        depth += 1;
                    }
                    b'%' => {
                        s = i;
                        e = i + 1;
                    }
                    b'!' => {
                        if i + 1 < query.len() && (query[i + 1] == b'=' || query[i + 1] == b'%') {
                            s = i;
                            e = i + 2;
                        } else {
                            s = i;
                            e = i + 1;
                        }
                    }
                    b'=' | b'<' | b'>' => {
                        if i + 1 < query.len() && query[i + 1] == b'=' {
                            s = i;
                            e = i + 2;
                        } else {
                            s = i;
                            e = i + 1;
                        }
                    }
                    _ => {
                        found = false;
                    }
                }
                if found {
                    lh = tostr(trim(&query[..s]));
                    op = tostr(trim(&query[s..e]));
                    rh = tostr(trim(&query[e..]));
                    if op == "==" {
                        op = &op[0..1];
                    }
                    return (lh, op, rh);
                }
                i += 1;
            }
            return (tostr(query), "", "");
        }
        (lh, op, rh)
    }
}

impl<'a> Default for Path<'a> {
    fn default() -> Self {
        Path::new("")
    }
}

impl<'a> std::fmt::Display for Path<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Path: comp={}, sep={}, extra={}",
            String::from_utf8_lossy(self.comp),
            self.sep as char,
            String::from_utf8_lossy(self.extra),
        )
    }
}

fn path_next_query<'a>(path: &Path<'a>) -> Path<'a> {
    // make sure the component is wrapped in `#()` or `#()#`.
    if path.extra[1] != b'(' {
        return Path::default();
    }
    let (_, next_i) = scan_squash(path.extra, 1);
    let mut i = next_i;
    let mut sep = 0;
    if i < path.extra.len() {
        match path.extra[i] {
            b'#' => {
                i += 1;
                if i < path.extra.len() {
                    match path.extra[i] {
                        b'.' | b'|' => {
                            sep = path.extra[i];
                        }
                        _ => {
                            return Path::default();
                        }
                    }
                }
            }
            b'.' | b'|' => {
                sep = path.extra[i];
            }
            _ => {
                return Path::default();
            }
        }
    }
    let extra = if sep == 0 {
        &path.extra[i..]
    } else {
        &path.extra[i + 1..]
    };
    let path = Path {
        comp: &path.extra[..i],
        esc: false,
        pat: false,
        sep,
        marg: 0,
        extra,
    };
    if path.comp[path.comp.len() - 1] == b'#' {
        if path.comp[path.comp.len() - 2] != b')' {
            return Path::default();
        }
    } else if path.comp[path.comp.len() - 1] != b')' {
        return Path::default();
    }
    path
}

fn path_next_multipath<'a>(path: &Path<'a>) -> Path<'a> {
    let (_, i) = scan_squash(path.extra, 0);
    let e;
    let s;
    let sep;
    if i == path.extra.len() || (path.extra[i] != b'.' && path.extra[i] != b'|') {
        e = i;
        s = path.extra.len();
        sep = 0;
    } else {
        sep = path.extra[i];
        e = i;
        s = i + 1;
    }
    Path {
        comp: &path.extra[..e],
        esc: false,
        pat: false,
        sep,
        marg: 0,
        extra: &path.extra[s..],
    }
}

// path_next returns the next path component
fn path_next<'a>(path: &Path<'a>) -> Path<'a> {
    let mut i = 0;
    let mut sep = 0;
    let mut esc = false;
    let mut pat = false;
    let mut modi = false;
    let mut marg = 0;
    if !path.extra.is_empty() {
        if path.extra[0] == b'@' {
            modi = true;
        } else if path.extra[0] == b'#' {
            if !(path.extra.len() == 1 || path.extra[1] == b'.' || path.extra[1] == b'|') {
                let next = path_next_query(path);
                return next;
            }
        } else if path.extra[0] == b'{' || path.extra[0] == b'[' {
            return path_next_multipath(path);
        }
    }
    while i < path.extra.len() {
        if path.extra[i] == b'\\' {
            esc = true;
            i += 1;
            if i == path.extra.len() {
                break;
            }
            i += 1;
            continue;
        } else if modi && path.extra[i] == b':' {
            marg = i;
            i += 1;
            if i == path.extra.len() {
                break;
            }
            if path.extra[i] == b'{' || path.extra[i] == b'[' || path.extra[i] == b'"' {
                let res = proc_value(path.extra, i, Path::new(""), true);
                i = res.1;
                if i < path.extra.len() && (path.extra[i] == b'|' || path.extra[i] == b'.') {
                    sep = path.extra[i];
                    i += 1;
                }
                break;
            }
            while i < path.extra.len() {
                if path.extra[i] == b'.' || path.extra[i] == b'|' {
                    sep = path.extra[i];
                    i += 1;
                    break;
                }
                i += 1;
            }
            break;
        } else if path.extra[i] == b'*' || path.extra[i] == b'?' {
            pat = true;
        } else if path.extra[i] == b'.' || path.extra[i] == b'|' {
            sep = path.extra[i];
            i += 1;
            break;
        }
        i += 1;
    }
    Path {
        comp: if sep == 0 {
            path.extra
        } else {
            &path.extra[..i - 1]
        },
        esc,
        pat,
        sep,
        marg,
        extra: &path.extra[i..],
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_path(path: &Path, comp: &str, sep: char, esc: bool, pat: bool) {
        assert_eq!(tostr(path.comp), comp);
        assert_eq!(path.sep as char, sep);
        assert_eq!(path.esc, esc);
        assert_eq!(path.pat, pat);
    }

    #[test]
    fn next() {
        let path = Path::new("hello.j*ello|12\\3.ab*\\c.[hel.lo,je\\.llo]");

        assert_eq!(
            format!("{}", path),
            r#"Path: comp=hello, sep=., extra=j*ello|12\3.ab*\c.[hel.lo,je\.llo]"#
        );
        assert_path(&path, "hello", '.', false, false);

        let path = path_next(&path);
        assert_path(&path, "j*ello", '|', false, true);

        let path = path_next(&path);
        assert_path(&path, "12\\3", '.', true, false);

        let path = path_next(&path);
        assert_path(&path, "ab*\\c", '.', true, true);

        let path = path_next(&path);
        assert_path(&path, "[hel.lo,je\\.llo]", '\0', false, false);

        let path = path_next(&path);
        assert_path(&path, "", '\0', false, false);

        let (comp, path) = path.next_group();
        assert_eq!(comp, "");
        assert_path(&path, "", '\0', false, false);
    }
    #[test]
    fn query() {
        let path = Path::new("#(hello=world)");
        let (lh, op, rh) = path.query_parts();
        assert_eq!("hello", lh);
        assert_eq!("=", op);
        assert_eq!("world", rh);

        let path = Path::new("");
        let (lh, op, rh) = path.query_parts();
        assert_eq!("", lh);
        assert_eq!("", op);
        assert_eq!("", rh);
    }
}
