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

    let result = increase_runtime();

    eprint!("result is: {:?}", result);
    eprintln!("config message is: {:?}", message_config);
    eprintln!("input message is: {:?}", input.context.suffix);

    return Ok(Output {
        message: "Hello World!".to_string(),
    });
}

fn increase_runtime() -> f64 {
    let mut result = 0.0001;
    for _ in 0..2_500_000 {
        result = result * 0.0000032;
    }
    result
}
