use colored::Colorize;
use std::{fmt, time::Duration};

pub struct FunctionRunResult {
    pub runtime: Duration,
    pub threshold: Duration,
    pub memory_usage: u64,
    pub logs: String,
    pub output: serde_json::Value,
}

impl FunctionRunResult {
    pub fn new(
        runtime: Duration,
        threshold: Duration,
        memory_usage: u64,
        output: serde_json::Value,
        logs: String,
    ) -> Self {
        FunctionRunResult {
            runtime,
            threshold,
            memory_usage,
            output,
            logs,
        }
    }
}

impl fmt::Display for FunctionRunResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let title = "      Benchmark Results      ".black().on_bright_green();
        write!(f, "{}\n\n", title)?;

        let runtime_display: String = if self.runtime <= self.threshold {
            format!("{:?}", self.runtime).bright_green().to_string()
        } else {
            format!(
                "{:?} <- maximum allowed is {:?}",
                self.runtime, self.threshold
            )
            .red()
            .to_string()
        };

        writeln!(f, "Runtime: {}", runtime_display)?;
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
