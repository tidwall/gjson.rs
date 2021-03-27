fn main() {
    let file = std::env::args().nth(1).expect("invalid number of arguments");
    let path = std::env::args().nth(2).expect("invalid number of arguments");
    let json = std::fs::read_to_string(file).unwrap();
    let raw = gjson::get(&json, &path).json().to_owned();
    if raw != "" && !gjson::valid(&raw) {
        panic!("invalid json response");
    }
    std::str::from_utf8(raw.as_bytes()).expect("invalid utf8 response");
    print!("{}\n", raw);
}
