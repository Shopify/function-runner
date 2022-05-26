use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use script_runner::engine::run;

/// Simple script runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version = "0.2.0")]
struct Opts {
    /// Path to wasm/wat script
    #[clap(short, long, default_value = "script.wasm")]
    script: PathBuf,

    /// Path to json file containing script input
    input: PathBuf,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let function_run_result = run(opts.script, opts.input)?;

    println!("{}", function_run_result);

    Ok(())
}
