<p align="center">
<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/tidwall/gjson/master/.github/images/logo-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/tidwall/gjson/master/.github/images/logo-light.png">
  <img src="/.github/images/logo-light.png" width="240" alt="GJSON" >
</picture>
<br>
<a href="LICENSE"><img src="https://img.shields.io/crates/l/gjson.svg?style=flat-square"></a>
<a href="https://crates.io/crates/gjson"><img src="https://img.shields.io/crates/d/gjson.svg?style=flat-square"></a>
<a href="https://crates.io/crates/gjson/"><img src="https://img.shields.io/crates/v/gjson.svg?style=flat-square"></a>
<a href="https://docs.rs/gjson/"><img src="https://img.shields.io/badge/docs-rustdoc-369?style=flat-square"></a>
<a href="http://tidwall.com/gjson-play"><img src="https://img.shields.io/badge/%F0%9F%8F%90-playground-9900cc.svg?style=flat-square" alt="GJSON Playground"></a>
</p>

<p align="center">get json values quickly</a></p>

GJSON is a Rust crate that provides a fast and [simple](#get-a-value) way to get values from a json document.
It has features such as [one line retrieval](#get-a-value), [dot notation paths](#path-syntax), [iteration](#iterate-through-an-object-or-array), and [parsing json lines](#json-lines).

This library uses the identical path syntax as the [Go version](https://github.com/tidwall/gjson).

Getting Started
===============

## Usage

Put this in your Cargo.toml:

```toml
[dependencies]
gjson = "0.8"
```

## Get a value

Get searches json for the specified path. A path is in dot syntax, such as "name.last" or "age". When the value is found it's returned immediately. 

```rust
const JSON: &str = r#"{"name":{"first":"Janet","last":"Prichard"},"age":47}"#;

fn main() {
    let value = gjson::get(JSON, "name.last");
    println!("{}", value);
}
```

This will print:

```
Prichard
```

## Path Syntax

Below is a quick overview of the path syntax, for more complete information please
check out [GJSON Syntax](https://github.com/tidwall/gjson/blob/master/SYNTAX.md).

A path is a series of keys separated by a dot.
A key may contain special wildcard characters '\*' and '?'.
To access an array value use the index as the key.
To get the number of elements in an array or to access a child path, use the '#' character.
The dot and wildcard characters can be escaped with '\\'.

```json
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
```
```
"name.last"          >> "Anderson"
"age"                >> 37
"children"           >> ["Sara","Alex","Jack"]
"children.#"         >> 3
"children.1"         >> "Alex"
"child*.2"           >> "Jack"
"c?ildren.0"         >> "Sara"
"fav\.movie"         >> "Deer Hunter"
"friends.#.first"    >> ["Dale","Roger","Jane"]
"friends.1.last"     >> "Craig"
```

You can also query an array for the first match by using `#(...)`, or find all 
matches with `#(...)#`. Queries support the `==`, `!=`, `<`, `<=`, `>`, `>=` 
comparison operators and the simple pattern matching `%` (like) and `!%` 
(not like) operators.

```
friends.#(last=="Murphy").first    >> "Dale"
friends.#(last=="Murphy")#.first   >> ["Dale","Jane"]
friends.#(age>45)#.last            >> ["Craig","Murphy"]
friends.#(first%"D*").last         >> "Murphy"
friends.#(first!%"D*").last        >> "Craig"
friends.#(nets.#(=="fb"))#.first   >> ["Dale","Roger"]
```

## Value Type

To convert the json value to a Rust type:

```rust
value.i8()
value.i16()
value.i32()
value.i64()
value.u8()
value.u16()
value.u32()
value.u64()
value.f32()
value.f64()
value.bool()
value.str()    // a string representation
value.json()   // the raw json
```

handy functions that work on a value:

```rust
value.kind()             // String, Number, True, False, Null, Array, or Object
value.exists()           // returns true if value exists in JSON.
value.get(path: &str)    // get a child value
value.each(|key, value|) // iterate over child values
```

### 64-bit integers

The `value.i64()` and `value.u64()` calls are capable of reading all 64 bits, allowing for large JSON integers.

```rust
value.i64() -> i64   // -9223372036854775808 to 9223372036854775807
value.u64() -> u64   // 0 to 18446744073709551615
```

## Modifiers and path chaining 

A modifier is a path component that performs custom processing on the 
json.

Multiple paths can be "chained" together using the pipe character. 
This is useful for getting values from a modified query.

For example, using the built-in `@reverse` modifier on the above json document,
we'll get `children` array and reverse the order:

```
"children|@reverse"           >> ["Jack","Alex","Sara"]
"children|@reverse|0"         >> "Jack"
```

There are currently the following built-in modifiers:

- `@reverse`: Reverse an array or the members of an object.
- `@ugly`: Remove all whitespace from a json document.
- `@pretty`: Make the json document more human readable.
- `@this`: Returns the current element. It can be used to retrieve the root element.
- `@valid`: Ensure the json document is valid.
- `@flatten`: Flattens an array.
- `@join`: Joins multiple objects into a single object.

### Modifier arguments

A modifier may accept an optional argument. The argument can be a valid JSON 
document or just characters.

For example, the `@pretty` modifier takes a json object as its argument. 

```
@pretty:{"sortKeys":true} 
```

Which makes the json pretty and orders all of its keys.

```json
{
  "age":37,
  "children": ["Sara","Alex","Jack"],
  "fav.movie": "Deer Hunter",
  "friends": [
    {"age": 44, "first": "Dale", "last": "Murphy"},
    {"age": 68, "first": "Roger", "last": "Craig"},
    {"age": 47, "first": "Jane", "last": "Murphy"}
  ],
  "name": {"first": "Tom", "last": "Anderson"}
}
```

*The full list of `@pretty` options are `sortKeys`, `indent`, `prefix`, and `width`. 
Please see [Pretty Options](https://github.com/tidwall/pretty#customized-output) for more information.*

## JSON Lines

There's support for [JSON Lines](http://jsonlines.org/) using the `..` prefix, which treats a multilined document as an array. 

For example:

```
{"name": "Gilbert", "age": 61}
{"name": "Alexa", "age": 34}
{"name": "May", "age": 57}
{"name": "Deloise", "age": 44}
```

```
..#                   >> 4
..1                   >> {"name": "Alexa", "age": 34}
..3                   >> {"name": "Deloise", "age": 44}
..#.name              >> ["Gilbert","Alexa","May","Deloise"]
..#(name="May").age   >> 57
```

## Get nested array values

Suppose you want all the last names from the following json:

```json
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
```

You would use the path "programmers.#.lastName" like such:

```rust
value := gjson::get(json, "programmers.#.lastName");
for name in value.array() {
	println!("{}", name);
}
```

You can also query an object inside an array:

```rust
let name = gjson::get(json, "programmers.#(lastName=Hunter).firstName");
println!("{}", name)  // prints "Elliotte"
```

## Iterate through an object or array

The `ForEach` function allows for quickly iterating through an object or array. 
The key and value are passed to the iterator function for objects.
Only the value is passed for arrays.
Returning `false` from an iterator will stop iteration.

```rust
let value := gjson::get(json, "programmers")
value::each(|key, value| {
	println!("{}", value);
	true // keep iterating
});
```

## Simple Parse and Get

There's a `gjson::parse(json)` function that will do a simple parse, and `value.get(path)` that will search a value.

For example, all of these will return the same value:

```rust
gjson::parse(json).get("name").get("last");
gjson::get(json, "name").get("last");
gjson::get(json, "name.last");
```

## Check for the existence of a value

Sometimes you just want to know if a value exists. 

```rust
let value = gjson::get(json, "name.last");
if !value.exists() {
	println!("no last name");
} else {
	println!("{}", value);
}

// Or as one step
if gjson::get(json, "name.last").exists() {
	println!("has a last name");
}
```

## Validate JSON

The `Get*` and `Parse*` functions expects that the json is valid. Bad json will not panic, but it may return back unexpected values.

If you are consuming JSON from an unpredictable source then you may want to validate prior to using GJSON.

```rust
if !gjson::valid(json) {
	return Err("invalid json");
}
let value = gjson::get(json, "name.last");
```
