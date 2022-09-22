use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use function_runner::engine::{run, run_with_count};

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Emit instruction count  
    #[clap(long)]
    instruction_count: bool,

    /// Path to json file containing Function input
    input: PathBuf,

    /// Log the run result as a JSON object
    #[clap(short, long, parse(from_flag))]
    json: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    if opts.instruction_count {
        let (result, count) = run_with_count(
            opts.function.to_str().unwrap(),
            std::fs::File::open(&opts.function)?,
            std::fs::File::open(&opts.input)?,
        )?;
        let list = count
            .into_iter()
            .map(|(instr, count)| format!("{}={}", instr, count))
            .collect::<Vec<String>>()
            .join(",");
        println!("{}", list);
        std::process::exit(0);
    }

    let function_run_result = run(
        opts.function.to_str().unwrap(),
        std::fs::File::open(&opts.function)?,
        std::fs::File::open(&opts.input)?,
    )?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{}", function_run_result);
    }

    Ok(())
}
