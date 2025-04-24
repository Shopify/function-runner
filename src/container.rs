use crate::Codec;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub enum BytesContainerType {
    /// Input bytes.
    #[default]
    Input,
    /// Output bytes.
    Output,
}

/// A container of bytes to hold either the input or output bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytesContainer {
    /// The raw bytes.
    #[serde(skip)]
    pub raw: Vec<u8>,
    /// Bytes encoding.
    #[serde(skip)]
    pub codec: Codec,
    /// The JSON represantation of the bytes.
    #[serde(skip)]
    pub json_value: Option<serde_json::Value>,
    /// The human readable representation of the bytes.
    pub humanized: String,
    /// Context for encoding errors.
    #[serde(skip)]
    pub encoding_error: Option<String>,
}

impl Default for BytesContainer {
    fn default() -> Self {
        Self {
            codec: Codec::Raw,
            humanized: "<raw codec>".into(),
            json_value: None,
            raw: Default::default(),
            encoding_error: None,
        }
    }
}

impl BytesContainer {
    pub fn new(ty: BytesContainerType, codec: Codec, raw: Vec<u8>) -> Result<Self> {
        match codec {
            Codec::Raw => {
                let humanized = raw
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<Vec<String>>()
                    .join(" ");

                Ok(Self {
                    raw,
                    codec,
                    humanized,
                    ..Default::default()
                })
            }
            Codec::Json => match ty {
                BytesContainerType::Input => {
                    let json = serde_json::from_slice::<serde_json::Value>(&raw)
                        .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;
                    let minified_buffer = serde_json::to_vec(&json)
                        .map_err(|e| anyhow!("Couldn't serialize JSON: {}", e))?;

                    Ok(Self {
                        codec,
                        raw: minified_buffer,
                        json_value: Some(json.clone()),
                        humanized: serde_json::to_string_pretty(&json)?,
                        encoding_error: None,
                    })
                }
                BytesContainerType::Output => {
                    let mut this = Self {
                        codec,
                        ..Default::default()
                    };

                    match serde_json::from_slice::<serde_json::Value>(&raw) {
                        Ok(json) => {
                            this.json_value = Some(json.clone());
                            this.humanized = serde_json::to_string_pretty(&json)?;
                            this.raw = serde_json::to_vec(&json)?;
                        }
                        Err(e) => {
                            this.humanized = String::from_utf8_lossy(&raw).into();
                            this.encoding_error = Some(e.to_string());
                        }
                    };

                    Ok(this)
                }
            },
            Codec::Messagepack => match ty {
                BytesContainerType::Input => {
                    let json: serde_json::Value = serde_json::from_slice(&raw)
                        .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;
                    let bytes = rmp_serde::to_vec(&json)
                        .map_err(|e| anyhow!("Couldn't convert JSON to MessagePack: {}", e))?;

                    Ok(Self {
                        raw: bytes,
                        codec,
                        json_value: Some(json.clone()),
                        humanized: serde_json::to_string_pretty(&json)?,
                        encoding_error: None,
                    })
                }
                BytesContainerType::Output => {
                    let mut this = Self {
                        codec,
                        ..Default::default()
                    };

                    let value: Result<serde_json::Value, _> = rmp_serde::decode::from_slice(&raw);
                    match value {
                        Ok(json) => {
                            this.json_value = Some(json.clone());
                            this.humanized = serde_json::to_string_pretty(&json)?;
                            this.raw = raw;
                        }
                        Err(e) => {
                            this.humanized = String::from_utf8_lossy(&raw).into();
                            this.encoding_error = Some(e.to_string());
                        }
                    };

                    Ok(this)
                }
            },
        }
    }
}
