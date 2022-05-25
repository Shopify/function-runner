use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use script_runner::run_engine::run_engine;

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

    let statistics = run_engine(opts.script, opts.input)?;

    println!("{}", statistics);

    Ok(())
}
