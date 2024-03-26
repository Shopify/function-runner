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
    pub output: FunctionOutput,
    #[serde(skip)]
    pub profile: Option<String>,
}

impl FunctionRunResult {
    pub fn new(
        name: String,
        size: u64,
        memory_usage: u64,
        instructions: u64,
        logs: String,
        output: FunctionOutput,
        profile: Option<String>,
    ) -> Self {
        FunctionRunResult {
            name,
            size,
            memory_usage,
            instructions,
            output,
            logs,
            profile,
        }
    }

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
        let title = "      Benchmark Results      "
            .black()
            .on_truecolor(150, 191, 72);

        write!(formatter, "{title}\n\n")?;
        writeln!(formatter, "Name: {}", self.name)?;
        writeln!(formatter, "Linear Memory Usage: {}KB", self.memory_usage)?;
        writeln!(formatter, "{}", humanize_instructions(self.instructions))?;
        writeln!(formatter, "Size: {}KB\n", self.size)?;

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
                        .unwrap_or_else(|error| error.to_string())
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use predicates::prelude::*;

    use super::*;

    #[test]
    fn test_js_output() {
        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1001,
            logs: "test".to_string(),
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
            profile: None,
        };

        let predicate = predicates::str::contains("Instructions: 1.001K")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"));
        assert!(predicate.eval(&function_run_result.to_string()));
    }

    #[test]
    fn test_js_output_1000() {
        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 1000,
            logs: "test".to_string(),
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
            profile: None,
        };

        let predicate = predicates::str::contains("Instructions: 1")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"));
        assert!(predicate.eval(&function_run_result.to_string()));
    }

    #[test]
    fn test_instructions_less_than_1000() {
        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            size: 100,
            memory_usage: 1000,
            instructions: 999,
            logs: "test".to_string(),
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
            profile: None,
        };

        let predicate = predicates::str::contains("Instructions: 999")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"));
        assert!(predicate.eval(&function_run_result.to_string()));
    }
}
