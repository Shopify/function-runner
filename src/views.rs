use std::fmt;

use colored::Colorize;
use serde::Serialize;

use crate::{
    function_run_result::{FunctionRunResult, FUNCTION_LOG_LIMIT},
    BytesContainer, Codec,
};

const DEFAULT_INSTRUCTIONS_LIMIT: u64 = 11_000_000;
const DEFAULT_INPUT_SIZE_LIMIT: u64 = 128_000;
const DEFAULT_OUTPUT_SIZE_LIMIT: u64 = 20_000;

pub struct ReadableBytes<'a>(&'a BytesContainer);

impl<'a> From<&'a BytesContainer> for ReadableBytes<'a> {
    fn from(bytes: &'a BytesContainer) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for ReadableBytes<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = self.0;

        if let Codec::Raw = bytes.codec {
            for (index, byte) in bytes.raw.iter().enumerate() {
                if index > 0 {
                    f.write_str(" ")?;
                }
                write!(f, "{byte:02x}")?;
            }
            return Ok(());
        }

        match &bytes.json_value {
            Some(json) => {
                let pretty = serde_json::to_string_pretty(json).map_err(|_| fmt::Error)?;
                f.write_str(&pretty)
            }
            None => f.write_str(&String::from_utf8_lossy(&bytes.raw)),
        }
    }
}

pub struct TextReport<'a>(&'a FunctionRunResult);

impl<'a> From<&'a FunctionRunResult> for TextReport<'a> {
    fn from(result: &'a FunctionRunResult) -> Self {
        Self(result)
    }
}

impl fmt::Display for TextReport<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let result = self.0;

        writeln!(
            formatter,
            "{}\n\n{}",
            "            Input            ".black().on_bright_yellow(),
            ReadableBytes::from(&result.input),
        )?;

        writeln!(
            formatter,
            "{}\n\n{}\n",
            "            Logs            ".black().on_bright_blue(),
            result.logs
        )?;

        let logs_length = result.logs.len();
        if logs_length > FUNCTION_LOG_LIMIT {
            writeln!(
                formatter,
                "{}\n\n",
                &format!(
                    "Logs would be truncated in production, length {logs_length} > {FUNCTION_LOG_LIMIT} limit",
                ).red()
            )?;
        }

        if let Some(e) = &result.output.encoding_error {
            writeln!(
                formatter,
                "{}\n\n{}",
                "        Invalid Output      ".black().on_bright_red(),
                ReadableBytes::from(&result.output),
            )?;

            writeln!(
                formatter,
                "{}\n\n{}",
                "         JSON Error         ".black().on_bright_red(),
                e
            )?;
        } else {
            writeln!(
                formatter,
                "{}\n\n{}",
                "           Output           ".black().on_bright_green(),
                ReadableBytes::from(&result.output),
            )?;
        }

        let input_size_limit = result.scale_factor * DEFAULT_INPUT_SIZE_LIMIT as f64;
        let output_size_limit = result.scale_factor * DEFAULT_OUTPUT_SIZE_LIMIT as f64;
        let instructions_size_limit = result.scale_factor * DEFAULT_INSTRUCTIONS_LIMIT as f64;

        writeln!(
            formatter,
            "\n{}\n\n",
            "        Resource Limits        "
                .black()
                .on_bright_magenta()
        )?;

        writeln!(
            formatter,
            "{}",
            humanize_size(
                "Input Size",
                input_size_limit as u64,
                input_size_limit as u64
            )
        )?;

        writeln!(
            formatter,
            "{}",
            humanize_size(
                "Output Size",
                output_size_limit as u64,
                output_size_limit as u64
            )
        )?;
        writeln!(
            formatter,
            "{}",
            humanize_instructions(
                "Instructions",
                instructions_size_limit as u64,
                instructions_size_limit as u64
            )
        )?;

        let title = "     Benchmark Results      "
            .black()
            .on_truecolor(150, 191, 72);

        write!(formatter, "\n\n{title}\n\n")?;
        writeln!(formatter, "Name: {}", result.name)?;
        writeln!(formatter, "Linear Memory Usage: {}KB", result.memory_usage)?;
        writeln!(
            formatter,
            "{}",
            humanize_instructions(
                "Instructions",
                result.instructions,
                instructions_size_limit as u64
            )
        )?;
        writeln!(
            formatter,
            "{}",
            humanize_size(
                "Input Size",
                result.input_size() as u64,
                input_size_limit as u64,
            )
        )?;
        writeln!(
            formatter,
            "{}",
            humanize_size(
                "Output Size",
                result.output_size() as u64,
                output_size_limit as u64,
            )
        )?;

        writeln!(formatter, "Module Size: {}KB\n", result.size)?;

        Ok(())
    }
}

#[derive(Serialize)]
#[serde(transparent)]
pub struct JsonReport<'a>(&'a FunctionRunResult);

impl<'a> From<&'a FunctionRunResult> for JsonReport<'a> {
    fn from(result: &'a FunctionRunResult) -> Self {
        Self(result)
    }
}

impl fmt::Display for JsonReport<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match serde_json::to_string_pretty(self) {
            Ok(json) => f.write_str(&json),
            Err(error) => write!(f, "{error}"),
        }
    }
}

fn humanize_size(title: &str, size_bytes: u64, size_limit: u64) -> String {
    let size_humanized = match size_bytes {
        0..=1023 => format!("{}B", size_bytes),
        1024..=1_048_575 => format!("{:.2}KB", size_bytes as f64 / 1024.0),
        1_048_576..=1_073_741_823 => format!("{:.2}MB", size_bytes as f64 / 1_048_576.0),
        _ => {
            format!("{:.2}GB", size_bytes as f64 / 1_073_741_824.0)
        }
    };

    if size_bytes > size_limit {
        format!("{}: {}", title, size_humanized).red().to_string()
    } else {
        format!("{}: {}", title, size_humanized)
    }
}

fn humanize_instructions(title: &str, instructions: u64, instructions_limit: u64) -> String {
    let instructions_humanized = match instructions {
        0..=999 => instructions.to_string(),
        1000..=999_999 => format!("{}K", instructions as f64 / 1000.0),
        1_000_000..=999_999_999 => format!("{}M", instructions as f64 / 1_000_000.0),
        1_000_000_000..=u64::MAX => format!("{}B", instructions as f64 / 1_000_000_000.0),
    };

    if instructions > instructions_limit {
        format!("{}: {}", title, instructions_humanized)
            .red()
            .to_string()
    } else {
        format!("{}: {}", title, instructions_humanized)
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use predicates::prelude::*;

    use crate::{
        function_run_result::FunctionRunResult, BytesContainer, BytesContainerType, Codec,
    };

    use super::*;

    fn json_output(raw: &[u8]) -> Result<BytesContainer> {
        BytesContainer::new(BytesContainerType::Output, Codec::Json, raw.to_vec())
    }

    fn mock_json_input() -> Result<BytesContainer> {
        let bytes = "{\"input_test\": \"input_value\"}".as_bytes();
        BytesContainer::new(BytesContainerType::Input, Codec::Json, bytes.to_vec())
    }

    #[test]
    fn test_js_output() -> Result<()> {
        let input = mock_json_input()?;

        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1001,
            logs: "test".to_string(),
            input: input.clone(),
            output: json_output(&serde_json::to_vec(&serde_json::json!({
                "test": "test"
            }))?)?,
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        let predicate = predicates::str::contains("Instructions: 1.001K")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(
                ReadableBytes::from(&input).to_string(),
            ))
            .and(predicates::str::contains("Input Size: 28B"))
            .and(predicates::str::contains("Output Size: 15B"));
        assert!(predicate.eval(&TextReport::from(&function_run_result).to_string()));
        Ok(())
    }

    #[test]
    fn test_js_output_1000() -> Result<()> {
        let input = mock_json_input()?;

        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1000,
            logs: "test".to_string(),
            input: input.clone(),
            output: json_output(&serde_json::to_vec(&serde_json::json!({
                "test": "test"
            }))?)?,
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        let predicate = predicates::str::contains("Instructions: 1")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(
                ReadableBytes::from(&input).to_string(),
            ));
        assert!(predicate.eval(&TextReport::from(&function_run_result).to_string()));
        Ok(())
    }

    #[test]
    fn test_instructions_less_than_1000() -> Result<()> {
        let input = mock_json_input()?;

        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 999,
            logs: "test".to_string(),
            input: input.clone(),
            output: json_output(&serde_json::to_vec(&serde_json::json!({
                "test": "test"
            }))?)?,
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        let predicate = predicates::str::contains("Instructions: 999")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(
                ReadableBytes::from(&input).to_string(),
            ));
        assert!(predicate.eval(&TextReport::from(&function_run_result).to_string()));
        Ok(())
    }

    #[test]
    fn readable_bytes_pretty_prints_json() -> Result<()> {
        let output = json_output(br#"{"a":{"b":1}}"#)?;

        assert_eq!(
            ReadableBytes::from(&output).to_string(),
            "{\n  \"a\": {\n    \"b\": 1\n  }\n}"
        );
        Ok(())
    }

    #[test]
    fn readable_bytes_shows_raw_codec_as_hex() -> Result<()> {
        let input = BytesContainer::new(
            BytesContainerType::Input,
            Codec::Raw,
            vec![0x0a, 0xff, 0x00],
        )?;

        assert_eq!(ReadableBytes::from(&input).to_string(), "0a ff 00");
        Ok(())
    }

    #[test]
    fn readable_bytes_shows_undecodable_output_as_text() -> Result<()> {
        let output = json_output(b"not json")?;

        assert!(output.encoding_error.is_some());
        assert_eq!(ReadableBytes::from(&output).to_string(), "not json");
        Ok(())
    }

    #[test]
    fn json_report_matches_serde_output() -> Result<()> {
        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1001,
            logs: "test".to_string(),
            input: mock_json_input()?,
            output: json_output(br#"{"test":"test"}"#)?,
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        assert_eq!(
            JsonReport::from(&function_run_result).to_string(),
            serde_json::to_string_pretty(&function_run_result)?
        );
        Ok(())
    }
}
