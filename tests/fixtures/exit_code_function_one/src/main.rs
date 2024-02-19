use std::io::Write;

fn main() -> std::io::Result<()> {
    let _ = std::io::stdout().write("{}".as_bytes());
    std::process::exit(1);
}
