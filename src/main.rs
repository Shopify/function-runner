use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use function_runner::engine::run;

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Path to json file containing Function input; if omitted, stdin is used
    input: Option<PathBuf>,

    /// Log the run result as a JSON object
    #[clap(short, long)]
    json: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let mut input: Box<dyn Read + Sync + Send + 'static> = if let Some(ref input) = opts.input {
        Box::new(BufReader::new(File::open(input).map_err(|e| {
            anyhow!("Couldn't load input {:?}: {}", input, e)
        })?))
    } else {
        Box::new(BufReader::new(stdin()))
    };

    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;
    let _ = serde_json::from_slice::<serde_json::Value>(&buffer)
        .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;

    let function_run_result = run(opts.function, buffer)?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{}", function_run_result);
    }

    Ok(())
}
