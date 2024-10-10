use crate::function_run_result::{FunctionOutput, FunctionRunResult};
use colored::Colorize;
use serde_json::Value;
use similar::TextDiff;

#[derive(Default)]
pub struct TestReport {
    successes: usize,
    failures: Vec<TestFailure>,
}

impl TestReport {
    pub fn add_success(&mut self) {
        self.successes += 1;
    }

    pub fn add_failure(
        &mut self,
        filename: String,
        expected_output: Value,
        run_result: FunctionRunResult,
    ) {
        self.failures.push(TestFailure {
            filename,
            expected_output,
            run_result,
        });
    }

    pub fn into_result(self) -> anyhow::Result<()> {
        println!();

        if !self.failures.is_empty() {
            println!("failures:\n");

            self.failures.iter().for_each(|failure| {
                println!("{:-^40}", format!(" {} logs ", failure.filename));
                println!("{}\n", failure.run_result.logs);
                println!("{:-^40}", format!(" {} output ", failure.filename));
                let output: std::borrow::Cow<str> = match &failure.run_result.output {
                    FunctionOutput::JsonOutput(json) => serde_json::to_string_pretty(json)
                        .expect("failed to serialize JSON")
                        .into(),
                    FunctionOutput::InvalidJsonOutput(output) => (&output.stdout).into(),
                };
                println!("{}\n", output.as_ref());

                println!("{:-^40}", format!(" {} output diff ", failure.filename));

                let expected = serde_json::to_string_pretty(&failure.expected_output)
                    .expect("failed to serialize JSON");

                let diff = TextDiff::from_lines(expected.as_str(), output.as_ref());

                println!("{}", diff.unified_diff().missing_newline_hint(false));

                println!();
            });
        }

        println!(
            "test result: {}. {} passed; {} failed",
            if self.failures.is_empty() {
                "ok".green()
            } else {
                "FAILED".red()
            },
            self.successes,
            self.failures.len()
        );

        if self.failures.is_empty() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("test failed"))
        }
    }
}

pub struct TestFailure {
    filename: String,
    expected_output: Value,
    run_result: FunctionRunResult,
}
