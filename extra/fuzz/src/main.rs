#[macro_use]
extern crate afl;
extern crate gjson;

use std::str::from_utf8;

const JSON: &str = r#"
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

fn main() {
    fuzz!(|data: &[u8]| {
        if let Ok(s) = from_utf8(data) {
            let _ = from_utf8(gjson::get(s, s).raw().as_bytes()).unwrap();
            let _ = from_utf8(gjson::get(JSON, s).raw().as_bytes()).unwrap();
        }
    });
}
