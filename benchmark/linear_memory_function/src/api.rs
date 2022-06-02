use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct Payload {
    pub input: Input,
    pub configuration: Config,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub message: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Input {
    pub context: HelloWorldContext,
}

#[derive(Clone, Debug, Deserialize)]
pub struct HelloWorldContext {
    pub suffix: String
}

#[derive(Clone, Debug, Serialize)]
pub struct Output {
    pub message: String
}