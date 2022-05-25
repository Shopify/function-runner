use colored::Colorize;
use std::{fmt, time::Duration};

pub struct RunStatistics {
    pub runtime: Duration,
    pub threshold: Duration,
    pub logs: String,
    pub output: serde_json::Value,
}

impl RunStatistics {
    pub fn new(
        runtime: Duration,
        threshold: Duration,
        output: serde_json::Value,
        logs: String,
    ) -> Self {
        RunStatistics {
            runtime,
            threshold,
            output,
            logs,
        }
    }
}

impl fmt::Display for RunStatistics {
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

        writeln!(f, "Runtime: {}\n", runtime_display)?;

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
