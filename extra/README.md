## Extra tools

- parity: This tool does a bunch of various `gjson::get` operations on the JSON files in the `testfiles` directory. Each `get` operation is run using both the Go and Rust library and it checks that the output is valid json/utf8, and are compared to make sure that they have binary equivalency between Go and Rust. To run, execute `extra/parity/run.sh` from the project root.

- fuzz: A fuzzing for gjson paths. My initial test cases are in the `in` directory. The fuzz suite is https://github.com/rust-fuzz/afl.rs. Instructions on how to build `alf.rs` are at https://rust-fuzz.github.io/book/afl/setup.html. To run, execute `cargo afl fuzz -i in -o out target/debug/fuzz` from the `fuzz` directory

- cover: The cover.sh script does code coverage on the Rust gjson library. Right now it's hardcoded to work with the [cargo-tarpaulin](https://github.com/xd009642/tarpaulin) project over ssh in my lab.
