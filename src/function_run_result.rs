use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt;

const FUNCTION_LOG_LIMIT: usize = 1_000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InvalidOutput {
    pub error: String,
    pub stdout: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum FunctionOutput {
    JsonOutput(serde_json::Value),
    InvalidJsonOutput(InvalidOutput),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionRunResult {
    pub name: String,
    pub size: u64,
    pub memory_usage: u64,
    pub instructions: u64,
    pub logs: String,
    pub input: serde_json::Value,
    pub output: FunctionOutput,
    #[serde(skip)]
    pub profile: Option<String>,
    #[serde(skip)]
    pub scale_factor: f64,
    pub success: bool,
}

const DEFAULT_INSTRUCTIONS_LIMIT: u64 = 11_000_000;
const DEFAULT_INPUT_SIZE_LIMIT: u64 = 128_000;
const DEFAULT_OUTPUT_SIZE_LIMIT: u64 = 20_000;

pub fn get_json_size_as_bytes(value: &serde_json::Value) -> usize {
    serde_json::to_vec(value).map(|v| v.len()).unwrap_or(0)
}

impl FunctionRunResult {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or_else(|error| error.to_string())
    }

    pub fn input_size(&self) -> usize {
        get_json_size_as_bytes(&self.input)
    }

    pub fn output_size(&self) -> usize {
        match &self.output {
            FunctionOutput::JsonOutput(value) => get_json_size_as_bytes(value),
            FunctionOutput::InvalidJsonOutput(_value) => 0,
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

impl fmt::Display for FunctionRunResult {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            formatter,
            "{}\n\n{}",
            "            Input            ".black().on_bright_yellow(),
            serde_json::to_string_pretty(&self.input)
                .expect("Input should be serializable to a string")
        )?;

        writeln!(
            formatter,
            "{}\n\n{}\n",
            "            Logs            ".black().on_bright_blue(),
            self.logs
        )?;

        let logs_length = self.logs.len();
        if logs_length > FUNCTION_LOG_LIMIT {
            writeln!(
                formatter,
                "{}\n\n",
                &format!(
                    "Logs would be truncated in production, length {logs_length} > {FUNCTION_LOG_LIMIT} limit",
                ).red()
            )?;
        }

        match &self.output {
            FunctionOutput::JsonOutput(json_output) => {
                writeln!(
                    formatter,
                    "{}\n\n{}",
                    "           Output           ".black().on_bright_green(),
                    serde_json::to_string_pretty(&json_output)
                        .expect("Output should be serializable to a string")
                )?;
            }
            FunctionOutput::InvalidJsonOutput(invalid_output) => {
                writeln!(
                    formatter,
                    "{}\n\n{}",
                    "        Invalid Output      ".black().on_bright_red(),
                    invalid_output.stdout
                )?;

                writeln!(
                    formatter,
                    "{}\n\n{}",
                    "         JSON Error         ".black().on_bright_red(),
                    invalid_output.error
                )?;
            }
        }

        let input_size_limit = self.scale_factor * DEFAULT_INPUT_SIZE_LIMIT as f64;
        let output_size_limit = self.scale_factor * DEFAULT_OUTPUT_SIZE_LIMIT as f64;
        let instructions_size_limit = self.scale_factor * DEFAULT_INSTRUCTIONS_LIMIT as f64;

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
        writeln!(formatter, "Name: {}", self.name)?;
        writeln!(formatter, "Linear Memory Usage: {}KB", self.memory_usage)?;
        writeln!(
            formatter,
            "{}",
            humanize_instructions(
                "Instructions",
                self.instructions,
                instructions_size_limit as u64
            )
        )?;
        writeln!(
            formatter,
            "{}",
            humanize_size(
                "Input Size",
                self.input_size() as u64,
                input_size_limit as u64,
            )
        )?;
        writeln!(
            formatter,
            "{}",
            humanize_size(
                "Output Size",
                self.output_size() as u64,
                output_size_limit as u64,
            )
        )?;

        writeln!(formatter, "Module Size: {}KB\n", self.size)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use predicates::prelude::*;

    use super::*;

    #[test]
    fn test_js_output() -> Result<()> {
        let mock_input_string = "{\"input_test\": \"input_value\"}".to_string();
        let mock_function_input = serde_json::from_str(&mock_input_string)?;
        let expected_input_display = serde_json::to_string_pretty(&mock_function_input)?;

        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1001,
            logs: "test".to_string(),
            input: mock_function_input,
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        let predicate = predicates::str::contains("Instructions: 1.001K")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(expected_input_display))
            .and(predicates::str::contains("Input Size: 28B"))
            .and(predicates::str::contains("Output Size: 15B"));
        assert!(predicate.eval(&function_run_result.to_string()));

        assert!(predicate.eval(&function_run_result.to_string()));
        Ok(())
    }

    #[test]
    fn test_js_output_1000() -> Result<()> {
        let mock_input_string = "{\"input_test\": \"input_value\"}".to_string();
        let mock_function_input = serde_json::from_str(&mock_input_string)?;
        let expected_input_display = serde_json::to_string_pretty(&mock_function_input)?;

        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1000,
            logs: "test".to_string(),
            input: mock_function_input,
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        let predicate = predicates::str::contains("Instructions: 1")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(expected_input_display));
        assert!(predicate.eval(&function_run_result.to_string()));
        Ok(())
    }

    #[test]
    fn test_instructions_less_than_1000() -> Result<()> {
        let mock_input_string = "{\"input_test\": \"input_value\"}".to_string();
        let mock_function_input = serde_json::from_str(&mock_input_string)?;
        let expected_input_display = serde_json::to_string_pretty(&mock_function_input)?;

        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 999,
            logs: "test".to_string(),
            input: mock_function_input,
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
            profile: None,
            scale_factor: 1.0,
            success: true,
        };

        let predicate = predicates::str::contains("Instructions: 999")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(expected_input_display));
        assert!(predicate.eval(&function_run_result.to_string()));
        Ok(())
    }
}
