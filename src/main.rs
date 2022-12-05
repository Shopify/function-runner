use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use function_runner::engine::run;

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Path to json file containing Function input
    input: Option<PathBuf>,

    /// Log the run result as a JSON object
    #[clap(short, long)]
    json: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let input: Box<dyn Read + Sync + Send + 'static> = match opts.input {
        Some(input) => Box::new(BufReader::new(File::open(input)?)),
        None => Box::new(stdin()),
    };

    let function_run_result = run(opts.function, input)?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{}", function_run_result);
    }

    Ok(())
}
