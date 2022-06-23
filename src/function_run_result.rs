use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

#[derive(Serialize, Deserialize)]
pub enum FunctionOutput {
    JsonOutput(serde_json::Value),
    InvalidOutput(String),
}

#[derive(Serialize)]
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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let title = "      Benchmark Results      "
            .black()
            .on_truecolor(150, 191, 72);
        write!(f, "{}\n\n", title)?;
        writeln!(f, "Name: {}", self.name)?;
        writeln!(f, "Runtime: {:?}", self.runtime)?;
        writeln!(f, "Memory Usage: {}KB", self.memory_usage * 64)?;
        writeln!(f, "Size: {}KB\n", self.size / 1024)?;

        writeln!(
            f,
            "{}\n\n{}",
            "            Logs            ".black().on_bright_blue(),
            self.logs
        )?;

        match &self.output {
            FunctionOutput::JsonOutput(json_output) => {
                writeln!(
                    f,
                    "{}\n\n{}",
                    "           Output           ".black().on_bright_green(),
                    serde_json::to_string_pretty(&json_output)
                        .unwrap_or_else(|error| error.to_string())
                )?;
            }
            FunctionOutput::InvalidOutput(output) => {
                writeln!(
                    f,
                    "{}\n\n{}",
                    "        Invalid Output      ".black().on_bright_red(),
                    output
                )?;
            }
        }

        Ok(())
    }
}
