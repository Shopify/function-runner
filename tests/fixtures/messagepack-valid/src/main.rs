use std::io::{Read, Write};

fn main() {
    let mut buf: Vec<u8> = vec![];
    std::io::stdin().read_to_end(&mut buf).unwrap();
    std::io::stdout().write_all(&buf).unwrap();
}
