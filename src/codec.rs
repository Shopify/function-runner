use crate::function_run_result::{
    FunctionOutput::{self, InvalidJsonOutput, JsonOutput},
    InvalidOutput,
};
use anyhow::{anyhow, Result};

/// Codec represents the different serialization formats supported for function input/output
#[derive(Debug, Clone, Copy)]
pub enum Codec {
    Json,
    Msgpack,
}

impl Codec {
    pub fn for_io_format(use_msgpack: bool) -> Self {
        if use_msgpack {
            Self::Msgpack
        } else {
            Self::Json
        }
    }

    pub fn transcode_from_json_bytes(&self, bytes: Vec<u8>) -> Result<Vec<u8>> {
        match self {
            Self::Json => Ok(bytes),
            Self::Msgpack => {
                let json_value: serde_json::Value = serde_json::from_slice(&bytes)
                    .map_err(|e| anyhow!("Invalid input JSON for Wasm API function: {}", e))?;
                rmp_serde::to_vec(&json_value).map_err(|e| {
                    anyhow!(
                        "Couldn't convert JSON to MessagePack for Wasm API function: {}",
                        e
                    )
                })
            }
        }
    }

    pub fn parse_output(&self, output_bytes: &[u8]) -> FunctionOutput {
        match self {
            Self::Json => match serde_json::from_slice(output_bytes) {
                Ok(json_output) => JsonOutput(json_output),
                Err(error) => InvalidJsonOutput(InvalidOutput {
                    stdout: std::str::from_utf8(output_bytes)
                        .map_err(|e| anyhow!("Couldn't print Function Output: {}", e))
                        .unwrap_or_default()
                        .to_owned(),
                    error: error.to_string(),
                }),
            },
            Self::Msgpack => match rmp_serde::from_slice::<serde_json::Value>(output_bytes) {
                Ok(json_output) => JsonOutput(json_output),
                Err(error) => InvalidJsonOutput(InvalidOutput {
                    stdout: String::from_utf8_lossy(output_bytes).into_owned(),
                    error: format!("Invalid MessagePack output: {}", error),
                }),
            },
        }
    }
}
