use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use script_runner::engine::run;

/// Simple script runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    /// Path to wasm/wat script
    #[clap(short, long, default_value = "script.wasm")]
    script: PathBuf,

    /// Path to json file containing script input
    input: PathBuf,

    /// Log the run result as a JSON object
    #[clap(short, long, parse(from_flag))]
    json: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let function_run_result = run(opts.script, opts.input)?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{}", function_run_result);
    }

    Ok(())
}
