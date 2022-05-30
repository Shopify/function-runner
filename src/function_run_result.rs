use colored::Colorize;
use serde::Serialize;
use std::{fmt, time::Duration};

#[derive(Serialize)]
pub struct FunctionRunResult {
    pub runtime: Duration,
    pub memory_usage: u64,
    pub logs: String,
    pub output: serde_json::Value,
}

impl FunctionRunResult {
    pub fn new(
        runtime: Duration,
        memory_usage: u64,
        output: serde_json::Value,
        logs: String,
    ) -> Self {
        FunctionRunResult {
            runtime,
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
        let title = "      Benchmark Results      ".black().on_bright_green();
        write!(f, "{}\n\n", title)?;

        writeln!(f, "Runtime: {:?}", self.runtime)?;
        writeln!(f, "Memory Usage: {}KB\n", self.memory_usage * 64)?;

        writeln!(
            f,
            "{}\n\n{}",
            "            Logs             ".black().on_bright_blue(),
            self.logs
        )?;

        writeln!(
            f,
            "Output:\n{}",
            serde_json::to_string_pretty(&self.output).unwrap_or_else(|error| error.to_string())
        )?;

        Ok(())
    }
}
