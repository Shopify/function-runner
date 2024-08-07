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
}

impl FunctionRunResult {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or_else(|error| error.to_string())
    }
}

fn humanize_instructions(instructions: u64) -> String {
    let instructions_humanized = match instructions {
        0..=999 => instructions.to_string(),
        1000..=999_999 => format!("{}K", instructions as f64 / 1000.0),
        1_000_000..=999_999_999 => format!("{}M", instructions as f64 / 1_000_000.0),
        1_000_000_000..=u64::MAX => format!("{}B", instructions as f64 / 1_000_000_000.0),
    };

    match instructions {
        0..=11_000_000 => format!("Instructions: {instructions_humanized}"),
        11_000_001..=u64::MAX => format!("Instructions: {instructions_humanized}")
            .red()
            .to_string(),
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

        let title = "     Benchmark Results      "
            .black()
            .on_truecolor(150, 191, 72);

        write!(formatter, "\n\n{title}\n\n")?;
        writeln!(formatter, "Name: {}", self.name)?;
        writeln!(formatter, "Linear Memory Usage: {}KB", self.memory_usage)?;
        writeln!(formatter, "{}", humanize_instructions(self.instructions))?;
        writeln!(formatter, "Size: {}KB\n", self.size)?;

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
        };

        let predicate = predicates::str::contains("Instructions: 1.001K")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(expected_input_display));

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
        };

        let predicate = predicates::str::contains("Instructions: 999")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"))
            .and(predicates::str::contains(expected_input_display));
        assert!(predicate.eval(&function_run_result.to_string()));
        Ok(())
    }
}
