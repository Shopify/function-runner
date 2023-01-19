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
    pub logs: String,
    pub output: FunctionOutput,
}

impl FunctionRunResult {
    pub fn new(
        name: String,
        runtime: Duration,
        size: u64,
        memory_usage: u64,
        logs: String,
        output: FunctionOutput,
    ) -> Self {
        FunctionRunResult {
            name,
            runtime,
            size,
            memory_usage,
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
        write!(formatter, "{}\n\n", title)?;
        writeln!(formatter, "Name: {}", self.name)?;
        writeln!(formatter, "Runtime: {:?}", self.runtime)?;
        writeln!(formatter, "Linear Memory Usage: {}KB", self.memory_usage)?;
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
