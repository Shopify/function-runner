use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use function_runner::engine::{run, run_with_gas};

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Path to json file containing Function input
    input: PathBuf,

    /// Log the run result as a JSON object
    #[clap(short, long, parse(from_flag))]
    json: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let function_run_result = run_with_gas(opts.function, opts.input)?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{}", function_run_result);
    }

    Ok(())
}
