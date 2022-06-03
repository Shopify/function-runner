use serde::Serialize;
mod api;
use api::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payload: Payload = serde_json::from_reader(std::io::BufReader::new(std::io::stdin()))?;
    let mut out = std::io::stdout();
    let mut serializer = serde_json::Serializer::new(&mut out);
    script(payload)?.serialize(&mut serializer)?;
    Ok(())
}

fn script(payload: Payload) -> Result<Output, Box<dyn std::error::Error>> {
    let (input, config) = (payload.input, payload.configuration);
    let message_config = config.message;

    eprintln!("config message is: {:?}", message_config);
    eprintln!("input message is: {:?}", input.context.suffix);

    return Ok(Output {
        message: "Hello World!".to_string(),
    });
}
