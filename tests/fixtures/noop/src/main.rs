use std::io;
use std::io::Write;

fn main() -> std::io::Result<()> {
    let input_string = io::read_to_string(io::stdin())?;
    std::io::stdout().write_all(input_string.as_bytes())?;
    std::io::stdout().flush()?;
    Ok(())
}
