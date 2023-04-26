use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::{CommandFactory, Parser};
use function_runner::engine::run;

use is_terminal::IsTerminal;

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser, Debug)]
#[clap(version)]
#[command(arg_required_else_help = true)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Path to json file containing Function input; if omitted, stdin is used
    input: Option<PathBuf>,

    /// Path to GraphQL file containing Function API schema; if omitted, output is not validated
    #[clap(short, long)]
    schema: Option<PathBuf>,

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
    } else if !std::io::stdin().is_terminal() {
        Box::new(BufReader::new(stdin()))
    } else {
        Opts::command()
            .print_help()
            .expect("Printing help should not fail");
        return Ok(());
    };

    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;
    let _ = serde_json::from_slice::<serde_json::Value>(&buffer)
        .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;

    let function_run_result = run(opts.function, buffer, opts.schema)?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{function_run_result}");
    }

    Ok(())
}
