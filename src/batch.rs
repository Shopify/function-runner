use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use function_runner::{
    bluejay_schema_analyzer::BluejaySchemaAnalyzer,
    engine::{run, FunctionRunParams},
    function_run_result::FunctionRunResult,
    BytesContainer, BytesContainerType, Codec,
};
use serde::Serialize;
use wasmtime::Module;

type ScaleFactorFn<'a> = dyn Fn(&serde_json::Value) -> Result<f64> + 'a;

pub(crate) struct ScaleLimitsSource<'a> {
    pub schema: &'a str,
    pub schema_path: Option<&'a str>,
    pub query: &'a str,
    pub query_path: Option<&'a str>,
}

pub(crate) struct BatchOptions<'a> {
    pub function_path: &'a Path,
    pub export: &'a str,
    pub codec: Codec,
    pub default_scale_factor: f64,
    pub continue_on_error: bool,
    pub full_output: bool,
}

#[derive(Serialize)]
struct MinimalRecord<'a> {
    line: usize,
    success: bool,
    instructions: u64,
    memory_usage: u64,
    logs: &'a str,
    output: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_error: Option<&'a str>,
}

#[derive(Serialize)]
struct FullRecord<'a> {
    line: usize,
    #[serde(flatten)]
    result: &'a FunctionRunResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_error: Option<&'a str>,
}

#[derive(Serialize)]
struct ErrorRecord<'a> {
    line: usize,
    success: bool,
    error: &'a str,
}

#[derive(Default)]
struct Summary {
    processed: usize,
    successful: usize,
    failed: usize,
}

pub(crate) fn run_batch(
    input: impl BufRead,
    output: impl Write,
    module: &Module,
    scale_limits: Option<ScaleLimitsSource>,
    options: &BatchOptions,
) -> Result<()> {
    match scale_limits {
        Some(source) => BluejaySchemaAnalyzer::with_analyzer(
            source.schema,
            source.schema_path,
            source.query,
            source.query_path,
            |analyze| run_lines(input, output, module, Some(analyze), options),
        )?,
        None => run_lines(input, output, module, None, options),
    }
}

fn run_lines(
    mut input: impl BufRead,
    mut output: impl Write,
    module: &Module,
    analyze: Option<&ScaleFactorFn<'_>>,
    options: &BatchOptions,
) -> Result<()> {
    let mut summary = Summary::default();
    let mut line_bytes = Vec::new();
    let mut record = Vec::new();
    let mut line = 0;

    let outcome = loop {
        line_bytes.clear();
        match input.read_until(b'\n', &mut line_bytes) {
            Ok(0) => break Ok(()),
            Ok(_) => {}
            Err(e) => break Err(anyhow!("Couldn't read input line {}: {}", line + 1, e)),
        }
        line += 1;

        if line_bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        summary.processed += 1;

        let failure = match run_line(std::mem::take(&mut line_bytes), module, analyze, options) {
            Ok(result) => match write_result_record(&mut record, line, &result, options.full_output) {
                Err(error) => Some(format!("Line {line}: {error}")),
                Ok(()) if result.success => None,
                Ok(()) => Some(format!(
                    "The Function execution failed on line {line}. Review the logs for more information."
                )),
            },
            Err(error) => {
                let error = format!("{error:#}");
                write_error_record(&mut record, line, &error);
                Some(format!("Line {line}: {error}"))
            }
        };

        if let Err(e) = output.write_all(&record) {
            break Err(e.into());
        }

        match failure {
            None => summary.successful += 1,
            Some(reason) => {
                summary.failed += 1;
                if !options.continue_on_error {
                    break Err(anyhow!(reason));
                }
            }
        }
    };

    output.flush()?;

    let status = if outcome.is_ok() {
        "complete"
    } else {
        "stopped"
    };
    eprintln!(
        "Batch {status}: {} inputs processed, {} successful, {} failed",
        summary.processed, summary.successful, summary.failed
    );

    outcome?;

    if summary.failed > 0 {
        bail!("{} of {} inputs failed", summary.failed, summary.processed);
    }

    Ok(())
}

fn run_line(
    line_bytes: Vec<u8>,
    module: &Module,
    analyze: Option<&ScaleFactorFn<'_>>,
    options: &BatchOptions,
) -> Result<FunctionRunResult> {
    let input = BytesContainer::new(BytesContainerType::Input, options.codec, line_bytes)?;

    let scale_factor = match (analyze, input.json_value.as_ref()) {
        (Some(analyze), Some(json_value)) => analyze(json_value)?,
        _ => options.default_scale_factor,
    };

    run(FunctionRunParams {
        function_path: options.function_path.to_path_buf(),
        input,
        export: options.export,
        profile_opts: None,
        scale_factor,
        module: module.clone(),
        engine: module.engine().clone(),
    })
}

fn write_result_record(
    record: &mut Vec<u8>,
    line: usize,
    result: &FunctionRunResult,
    full_output: bool,
) -> Result<(), String> {
    record.clear();
    let output_error = result.output.encoding_error.as_deref();
    let serialized = if full_output {
        serde_json::to_writer(
            &mut *record,
            &FullRecord {
                line,
                result,
                output_error,
            },
        )
    } else {
        serde_json::to_writer(
            &mut *record,
            &MinimalRecord {
                line,
                success: result.success,
                instructions: result.instructions,
                memory_usage: result.memory_usage,
                logs: &result.logs,
                output: result.output.json_value.as_ref(),
                output_error,
            },
        )
    };

    match serialized {
        Ok(()) => {
            record.push(b'\n');
            Ok(())
        }
        Err(e) => {
            let error = format!("Couldn't serialize result: {e}");
            write_error_record(record, line, &error);
            Err(error)
        }
    }
}

fn write_error_record(record: &mut Vec<u8>, line: usize, error: &str) {
    record.clear();
    serde_json::to_writer(
        &mut *record,
        &ErrorRecord {
            line,
            success: false,
            error,
        },
    )
    .expect("An error record contains only a number, a bool, and a string");
    record.push(b'\n');
}
