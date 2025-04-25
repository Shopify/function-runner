use std::io::{Read, Write};

fn main() {
    let mut buf: Vec<u8> = vec![];
    std::io::stdin().read_to_end(&mut buf).unwrap();
    let mut cursor = std::io::Cursor::new(&buf);
    rmpv::decode::read_value(&mut cursor).expect("Valid messagepack");
    std::io::stdout().write_all(&buf).unwrap();
}
