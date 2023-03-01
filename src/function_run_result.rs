use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

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
    pub runtime: Duration,
    pub size: u64,
    pub memory_usage: u64,
    pub fuel_consumed: u64,
    pub logs: String,
    pub output: FunctionOutput,
}

impl FunctionRunResult {
    pub fn new(
        name: String,
        runtime: Duration,
        size: u64,
        memory_usage: u64,
        fuel_consumed: u64,
        logs: String,
        output: FunctionOutput,
    ) -> Self {
        FunctionRunResult {
            name,
            runtime,
            size,
            memory_usage,
            fuel_consumed,
            output,
            logs,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap_or_else(|error| error.to_string())
    }
}

impl fmt::Display for FunctionRunResult {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let title = "      Benchmark Results      "
            .black()
            .on_truecolor(150, 191, 72);

        let fuel_message: String = if self.fuel_consumed >= 1000 {
            format!("Instructions: {}K", self.fuel_consumed as f64 / 1000.0)
        } else {
            format!("Instructions: {}", self.fuel_consumed)
        };
        write!(formatter, "{title}\n\n")?;
        writeln!(formatter, "Name: {}", self.name)?;
        writeln!(formatter, "Runtime: {:?}", self.runtime)?;
        writeln!(formatter, "Linear Memory Usage: {}KB", self.memory_usage)?;

        writeln!(formatter, "{fuel_message}")?;

        writeln!(formatter, "Size: {}KB\n", self.size)?;

        writeln!(
            formatter,
            "{}\n\n{}",
            "            Logs            ".black().on_bright_blue(),
            self.logs
        )?;

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
            runtime: Duration::from_millis(100),
            size: 100,
            memory_usage: 1000,
            fuel_consumed: 1001,
            logs: "test".to_string(),
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
        };

        let predicate = predicates::str::contains("Instructions: 1.001K")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"));
        assert!(predicate.eval(&function_run_result.to_string()));
    }

    #[test]
    fn test_js_output_1000() {
        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            runtime: Duration::from_millis(100),
            size: 100,
            memory_usage: 1000,
            fuel_consumed: 1000,
            logs: "test".to_string(),
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
        };

        let predicate = predicates::str::contains("Instructions: 1")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"));
        assert!(predicate.eval(&function_run_result.to_string()));
    }

    #[test]
    fn test_instructions_less_than_1000() {
        let function_run_result = FunctionRunResult {
            name: "test".to_string(),
            runtime: Duration::from_millis(100),
            size: 100,
            memory_usage: 1000,
            fuel_consumed: 999,
            logs: "test".to_string(),
            output: FunctionOutput::JsonOutput(serde_json::json!({
                "test": "test"
            })),
        };

        let predicate = predicates::str::contains("Instructions: 999")
            .and(predicates::str::contains("Linear Memory Usage: 1000KB"));
        assert!(predicate.eval(&function_run_result.to_string()));
    }
}
