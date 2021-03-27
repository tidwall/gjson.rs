// Copyright 2021 Joshua J Baker. All rights reserved.
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file.

// Bit flags passed to the "info" parameter of the iter function which
// provides additional information about the data

#[cfg(test)]
use super::*;

#[test]
fn various() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    assert_eq!(get(&json, "@valid.statuses.#").u64(), 100);
    assert_eq!(get(&json, "@ugly.@valid.statuses.#").u64(), 100);
    assert_eq!(get(&json, "@pretty.@ugly.@valid.statuses.#").u64(), 100);
    assert_eq!(
        get(&json, "@pretty.@ugly.@valid.statuses.50.user.name").str(),
        "イイヒト"
    );
    assert_eq!(get(&json, "search_metadata.count").u64(), 100);
}

#[test]
fn modifiers() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let res1 = get(&json, "statuses.#.user.id|@valid");
    let res2 = get(&json, "statuses.#.user.id|@reverse|@valid");
    let mut all1 = Vec::new();
    res1.each(|_, value| {
        all1.push(value.str().to_owned());
        true
    });
    let mut all2 = Vec::new();
    res2.each(|_, value| {
        all2.push(value.str().to_owned());
        true
    });
    assert_eq!(all1.len(), 100);
    assert_eq!(all1.len(), all2.len());
    for i in 0..all1.len() {
        assert_eq!(all1[i], all2[all2.len() - 1 - i]);
    }

    let res1 = get(&json, "statuses.50.user|@valid");
    let res2 = get(&json, "statuses.50.user|@reverse|@valid");
    let mut all1 = Vec::new();
    res1.each(|key, value| {
        all1.push((key.str().to_owned(), value.str().to_owned()));
        true
    });
    let mut all2 = Vec::new();
    res2.each(|key, value| {
        all2.push((key.str().to_owned(), value.str().to_owned()));
        true
    });
    assert_eq!(all1.len(), 40);
    assert_eq!(all1.len(), all2.len());
    for i in 0..all1.len() {
        assert_eq!(all1[i].0, all2[all2.len() - 1 - i].0);
        assert_eq!(all1[i].1, all2[all2.len() - 1 - i].1);
    }

    let res = get(
        r#"{"user":[
        {"first":"tom","age":72},
        {"last":"anderson","age":68}
    ]}"#,
        "user.@join.@ugly",
    );
    assert_eq!(res.json(), r#"{"first":"tom","age":68,"last":"anderson"}"#);
    let res = get(
        r#"{"user":[
        {"first":"tom","age":72},
        {"last":"anderson","age":68}
    ]}"#,
        r#"user.@join:{"preserve":true}.@ugly"#,
    );
    assert_eq!(
        res.json(),
        r#"{"first":"tom","age":72,"last":"anderson","age":68}"#
    );

    assert_eq!(
        get("[1,[2],[3,4],[5,[6,7]]]", "@flatten").json(),
        "[1,2,3,4,5,[6,7]]"
    );
    assert_eq!(
        get("[1,[2],[3,4],[5,[6,7]]]", r#"@flatten:{"deep":true}"#).json(),
        "[1,2,3,4,5,6,7]"
    );
}

#[test]
fn iterator() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let mut index = 0;
    let mut res = String::new();
    res.push_str("[");
    parse(&json).each(|key, value| -> bool {
        if key.str() == "statuses" {
            value.each(|_, value| -> bool {
                if index > 0 {
                    res.push_str(",");
                }
                res.push_str(value.get("user.name").json());
                index += 1;
                return true;
            })
        }
        return true;
    });
    res.push_str("]");
    assert_eq!(index, 100);
    assert_eq!(get(&res, "50").str(), "イイヒト");
}

#[test]
fn array() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let res1 = get(&json, "statuses.#.user.name");
    let res2 = parse(&json);
    let res3 = res2.get("statuses.#.user.name");
    assert_eq!(res1.get("#").u64(), 100);
    assert_eq!(res3.get("#").u64(), 100);
    assert_eq!(res1.str(), res3.str());
    assert_eq!(get(&json, "statuses.#.user.name|50").str(), "イイヒト");
}

#[test]
fn query() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let res = get(
        &json,
        "statuses.#(user.name==イイヒト).user.profile_link_color",
    );
    assert_eq!(res.str(), "0084B4");
    let res = get(
        &json,
        "statuses.#(user.profile_link_color!=0084B4)#.id|@ugly",
    );
    assert_eq!(
        res.str(),
        pretty::ugly(
            "[505874919020699648,505874915338104833,505874914897690624,
        505874893154426881,505874882870009856,505874882228281345,
        505874874275864576,505874873248268288,505874856089378816,
        505874855770599425,505874852754907136,505874847260352513]"
        )
    );
    assert_eq!(
        res.get("#(=505874874275864576)").str(),
        "505874874275864576"
    );
    let json = r#"{
        "friends": [
            {"first": "Dale", "last": "Murphy", "age": 44, "nets": [{"net":"ig"}, "fb", "tw"]},
            {"first": "Roger", "last": "Craig", "age": 68, "nets": ["fb", "tw"]},
            {"first": "Jane", "last": "Murphy", "age": 47, "nets": ["ig", "tw"]}
            ]
        }
        "#;
    assert_eq!(
        get(json, r#"frie\nds.#(ne\ts.#(ne\t=ig)).@ugly"#).json(),
        r#"{"first":"Dale","last":"Murphy","age":44,"nets":[{"net":"ig"},"fb","tw"]}"#
    );
}

#[test]
fn multipath() {
    let json = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let res = get(
        &json,
        r#"[[statuses.#,statuses.#],statuses.10.user.name,[statuses.10.user.id,statuses.56.user.id,statuses.42.user.id].@reverse]"#,
    );
    assert_eq!(
        res.json(),
        r#"[[100,100],"モテモテ大作戦★男子編",[2278053589,2714868440,2714526565]]"#
    );
    let res = get(
        &json,
        r#"{[statuses.#,statuses.#],statuses.10.user.name,[statuses.10.user.id,statuses.56.user.id,statuses.42.user.id].@reverse}"#,
    );
    assert_eq!(
        res.json(),
        r#"{"_":[100,100],"name":"モテモテ大作戦★男子編","@reverse":[2278053589,2714868440,2714526565]}"#
    );
    let res = get(
        &json,
        r#"{counts:[statuses.#,statuses.#],statuses.10.user.name,[statuses.10.user.id,statuses.56.user.id,statuses.42.user.id].@reverse}"#,
    );
    assert_eq!(
        res.json(),
        r#"{"counts":[100,100],"name":"モテモテ大作戦★男子編","@reverse":[2278053589,2714868440,2714526565]}"#
    );
}

#[test]
fn jsonlines() {
    let json = r#"
        {"a": 1 }
        {"a": 2 }
        true
        false
        4
    "#;
    assert_eq!(get(json, "..#").i32(), 5);
    assert_eq!(get(json, "..0.a").i32(), 1);
    assert_eq!(get(json, "..1.a").i32(), 2);
    assert_eq!(
        get(json, "..#.@this|@ugly").json(),
        r#"[{"a":1},{"a":2},true,false,4]"#
    );
    assert_eq!(get(json, "..#.@this|@join|@ugly").json(), r#"{"a":2}"#);
}

#[test]
fn escaped() {
    let json1 = std::fs::read_to_string("testfiles/twitter.json").unwrap();
    let json2 = std::fs::read_to_string("testfiles/twitterescaped.json").unwrap();
    assert_eq!(
        get(&json1, "statuses.#").i32(),
        get(&json2, "statuses.#").i32()
    );
    for i in 0..100 {
        let path = format!("statuses.{}.text", i);
        assert_eq!(get(&json1, &path).str(), get(&json2, &path).str());
        let path = format!("statuses.{}.user.name", i);
        assert_eq!(get(&json1, &path).str(), get(&json2, &path).str());
        break;
    }
}

#[cfg(test)]
const EXAMPLE: &str = r#"
{
  "name": {"first": "Tom", "last": "Anderson"},
  "age":37,
  "children": ["Sara","Alex","Jack"],
  "fav.movie": "Deer Hunter",
  "friends": [
    {"first": "Dale", "last": "Murphy", "age": 44, "nets": ["ig", "fb", "tw"]},
    {"first": "Roger", "last": "Craig", "age": 68, "nets": ["fb", "tw"]},
    {"first": "Jane", "last": "Murphy", "age": 47, "nets": ["ig", "tw"]}
  ]
}
"#;

#[cfg(test)]
fn exec_simple_fuzz(data: &[u8]) {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = std::str::from_utf8(get(s, s).json().as_bytes()).unwrap();
        let _ = std::str::from_utf8(get(EXAMPLE, s).json().as_bytes()).unwrap();
    }
}

#[test]
fn fuzz() {
    // This only runs on crash files in the fuzz directory.
    let crash_dir = "extra/fuzz/out/default/crashes";
    if !std::path::Path::new(crash_dir).exists() {
        return;
    }
    let mut files = std::fs::read_dir(crash_dir)
        .unwrap()
        .map(|res| res.map(|e| e.path()))
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap();
    files.sort();
    for file in files {
        let fname = file.as_path().to_str().unwrap().to_owned();
        eprintln!("{}", fname);
        let data = std::fs::read(file).unwrap();
        exec_simple_fuzz(&data);
    }
}

#[test]
fn array_value() {
    const PROGRAMMERS: &str = r#"
    {
        "programmers": [
          {
            "firstName": "Janet", 
            "lastName": "McLaughlin", 
          }, {
            "firstName": "Elliotte", 
            "lastName": "Hunter", 
          }, {
            "firstName": "Jason", 
            "lastName": "Harold", 
          }
        ]
      }
    "#;
    let mut res = String::new();
    let value = get(PROGRAMMERS, "programmers.#.lastName");
    for name in value.array() {
        res.extend(format!("{}\n", name).chars());
    }
    assert_eq!(res, "McLaughlin\nHunter\nHarold\n");
}
