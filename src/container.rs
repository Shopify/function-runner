use crate::Codec;
use anyhow::{anyhow, Result};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize};

struct MessagePackJsonValue<'a>(&'a serde_json::Value);

impl Serialize for MessagePackJsonValue<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            serde_json::Value::Null => serializer.serialize_unit(),
            serde_json::Value::Bool(value) => serializer.serialize_bool(*value),
            serde_json::Value::Number(value) => {
                if let Some(value) = value.as_i64() {
                    serializer.serialize_i64(value)
                } else if let Some(value) = value.as_u64() {
                    serializer.serialize_u64(value)
                } else if let Some(value) = value.as_f64() {
                    serializer.serialize_f64(value)
                } else {
                    serializer.serialize_str(&value.to_string())
                }
            }
            serde_json::Value::String(value) => serializer.serialize_str(value),
            serde_json::Value::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(&MessagePackJsonValue(value))?;
                }
                seq.end()
            }
            serde_json::Value::Object(values) => {
                let mut map = serializer.serialize_map(Some(values.len()))?;
                for (key, value) in values {
                    map.serialize_entry(key, &MessagePackJsonValue(value))?;
                }
                map.end()
            }
        }
    }
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
                    let bytes = rmp_serde::to_vec(&MessagePackJsonValue(&json))
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
    use std::collections::BTreeMap;

    #[test]
    fn json_input_preserves_number_representation_when_minifying() -> Result<()> {
        let input = br#"{ "longitude": -0.9191460999999999 }"#.to_vec();

        let container = BytesContainer::new(BytesContainerType::Input, Codec::Json, input)?;

        assert_eq!(
            String::from_utf8(container.raw).unwrap(),
            r#"{"longitude":-0.9191460999999999}"#
        );

        Ok(())
    }

    #[test]
    fn messagepack_input_encodes_arbitrary_precision_numbers_as_numbers() -> Result<()> {
        let input = br#"{"longitude":-0.9191460999999999}"#.to_vec();

        let container = BytesContainer::new(BytesContainerType::Input, Codec::Messagepack, input)?;

        assert_eq!(&container.raw[..12], b"\x81\xa9longitude\xcb");
        let decoded: BTreeMap<String, f64> = rmp_serde::from_slice(&container.raw)?;
        assert_eq!(decoded.get("longitude"), Some(&-0.9191460999999999));

        Ok(())
    }

    #[test]
    fn messagepack_input_preserves_object_order() -> Result<()> {
        let input = br#"{"b":1,"a":2}"#.to_vec();

        let container = BytesContainer::new(BytesContainerType::Input, Codec::Messagepack, input)?;

        assert_eq!(
            container.raw,
            vec![0x82, 0xa1, b'b', 0x01, 0xa1, b'a', 0x02]
        );

        Ok(())
    }
}
