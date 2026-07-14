use std::io;

use crate::Codec;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Default)]
struct ShopifyJsonFormatter;

impl serde_json::ser::Formatter for ShopifyJsonFormatter {
    fn write_string_fragment<W>(&mut self, writer: &mut W, fragment: &str) -> io::Result<()>
    where
        W: ?Sized + io::Write,
    {
        let mut start = 0;
        for (index, character) in fragment.char_indices() {
            match character {
                '/' => {
                    writer.write_all(&fragment.as_bytes()[start..index])?;
                    writer.write_all(br"\/")?;
                    start = index + character.len_utf8();
                }
                '\u{2028}' => {
                    writer.write_all(&fragment.as_bytes()[start..index])?;
                    writer.write_all(br"\u2028")?;
                    start = index + character.len_utf8();
                }
                '\u{2029}' => {
                    writer.write_all(&fragment.as_bytes()[start..index])?;
                    writer.write_all(br"\u2029")?;
                    start = index + character.len_utf8();
                }
                _ => {}
            }
        }
        writer.write_all(&fragment.as_bytes()[start..])
    }
}

fn to_shopify_json_vec(value: &serde_json::Value) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let formatter = ShopifyJsonFormatter;
    let mut serializer = serde_json::Serializer::with_formatter(&mut bytes, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|e| anyhow!("Couldn't serialize JSON: {}", e))?;
    Ok(bytes)
}

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
    /// The human readable representation of the bytes.
    #[serde(skip)]
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
                    let minified_buffer = to_shopify_json_vec(&json)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_input_escapes_solidus_like_shopify_json() -> Result<()> {
        let input = br#"{"id":"gid://shopify/Product/1"}"#.to_vec();

        let container = BytesContainer::new(BytesContainerType::Input, Codec::Json, input)?;

        assert_eq!(
            String::from_utf8(container.raw).unwrap(),
            r#"{"id":"gid:\/\/shopify\/Product\/1"}"#
        );

        Ok(())
    }

    #[test]
    fn json_input_escapes_line_and_paragraph_separators_like_shopify_json() -> Result<()> {
        let input = "{\"line\":\"before\u{2028}after\",\"paragraph\":\"before\u{2029}after\"}"
            .as_bytes()
            .to_vec();

        let container = BytesContainer::new(BytesContainerType::Input, Codec::Json, input)?;

        assert_eq!(
            String::from_utf8(container.raw).unwrap(),
            r#"{"line":"before\u2028after","paragraph":"before\u2029after"}"#
        );

        Ok(())
    }

    #[test]
    fn json_input_preserves_object_order() -> Result<()> {
        let input = br#"{"b":1,"a":2}"#.to_vec();

        let container = BytesContainer::new(BytesContainerType::Input, Codec::Json, input)?;

        assert_eq!(
            String::from_utf8(container.raw).unwrap(),
            r#"{"b":1,"a":2}"#
        );

        Ok(())
    }
}
