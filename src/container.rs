use crate::Codec;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

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
    #[serde(flatten)]
    pub json_value: Option<serde_json::Value>,
    /// Context for encoding errors.
    #[serde(skip)]
    pub encoding_error: Option<String>,
}

impl Default for BytesContainer {
    fn default() -> Self {
        Self {
            codec: Codec::Raw,
            json_value: None,
            raw: Default::default(),
            encoding_error: None,
        }
    }
}

impl BytesContainer {
    pub fn new(ty: BytesContainerType, codec: Codec, raw: Vec<u8>) -> Result<Self> {
        match codec {
            Codec::Raw => Ok(Self {
                raw,
                codec,
                ..Default::default()
            }),
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
                        encoding_error: None,
                    })
                }
                BytesContainerType::Output => {
                    let mut this = Self {
                        raw: raw.clone(),
                        codec,
                        ..Default::default()
                    };

                    match serde_json::from_slice::<serde_json::Value>(&raw) {
                        Ok(json) => {
                            this.json_value = Some(json.clone());
                            this.raw = serde_json::to_vec(&json)?;
                        }
                        Err(e) => {
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
                        encoding_error: None,
                    })
                }
                BytesContainerType::Output => {
                    let mut this = Self {
                        raw: raw.clone(),
                        codec,
                        ..Default::default()
                    };

                    let value: Result<serde_json::Value, _> = rmp_serde::decode::from_slice(&raw);
                    match value {
                        Ok(json) => {
                            this.json_value = Some(json.clone());
                            this.raw = raw;
                        }
                        Err(e) => {
                            this.encoding_error = Some(e.to_string());
                        }
                    };

                    Ok(this)
                }
            },
        }
    }
}

pub struct HumanizedBytes<'a> {
    bytes: &'a BytesContainer,
}

impl<'a> From<&'a BytesContainer> for HumanizedBytes<'a> {
    fn from(bytes: &'a BytesContainer) -> Self {
        Self { bytes }
    }
}

impl fmt::Display for HumanizedBytes<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(json) = &self.bytes.json_value {
            let humanized = serde_json::to_string_pretty(json).map_err(|_| fmt::Error)?;
            return formatter.write_str(&humanized);
        }

        if self.bytes.encoding_error.is_some() {
            return formatter.write_str(&String::from_utf8_lossy(&self.bytes.raw));
        }

        match self.bytes.codec {
            Codec::Raw => {
                for (index, byte) in self.bytes.raw.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(" ")?;
                    }
                    write!(formatter, "{byte:02x}")?;
                }

                Ok(())
            }
            Codec::Json | Codec::Messagepack => {
                formatter.write_str(&String::from_utf8_lossy(&self.bytes.raw))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_input_preserves_object_key_order_in_raw_bytes() {
        let raw = br#"{"msg":{"sq-XK":"5.00 EUR Zbritje","en":"5.00 EUR Discount"}}"#.to_vec();

        let input = BytesContainer::new(BytesContainerType::Input, Codec::Json, raw.clone())
            .expect("valid JSON input");

        assert_eq!(
            String::from_utf8(input.raw).unwrap(),
            String::from_utf8(raw).unwrap()
        );
    }

    #[test]
    fn humanized_bytes_can_be_composed_from_a_bytes_container() {
        let input = BytesContainer::new(
            BytesContainerType::Input,
            Codec::Json,
            br#"{"input_test":"input_value"}"#.to_vec(),
        )
        .expect("valid JSON input");

        assert_eq!(
            HumanizedBytes::from(&input).to_string(),
            "{\n  \"input_test\": \"input_value\"\n}"
        );
    }
}
