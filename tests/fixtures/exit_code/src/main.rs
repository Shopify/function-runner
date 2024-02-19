use std::io;
use std::io::Write;

use nanoserde::DeJson;

#[derive(DeJson)]
pub struct Input {
    code: i32,
}

fn main() -> std::io::Result<()> {
    let input_string = io::read_to_string(io::stdin())?;
    let input: Input = DeJson::deserialize_json(&input_string).expect("Invalid input JSON");
    let output = format!("{{\"exit\":{}}}", input.code);
    std::io::stdout().write_all(output.as_bytes())?;
    std::io::stdout().flush()?;
    std::process::exit(input.code);
}
