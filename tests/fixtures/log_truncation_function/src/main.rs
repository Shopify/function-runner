use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
struct Output {
    output: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut out = std::io::stdout();
    let mut serializer = serde_json::Serializer::new(&mut out);
    let result = Output {
        output: "hello world".to_string(),
    };
    result.serialize(&mut serializer)?;
    for _ in 0..6_667 {
        eprint!("☠");
    }
    Ok(())
}
