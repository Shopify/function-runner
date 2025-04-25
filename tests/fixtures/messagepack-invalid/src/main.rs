use std::io::{Read, Write};

fn main() {
    let mut buf: Vec<u8> = vec![];
    std::io::stdin().read_to_end(&mut buf).unwrap();
    std::io::stdout()
        // Invalid messagepack.
        .write_all(&[192, 193, 194, 195, 196, 197, 198, 199, 200, 201])
        .unwrap();
}
