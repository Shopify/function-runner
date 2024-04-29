use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
    process::Command,
};

use anyhow::{anyhow, bail, Result};
use clap::{Parser, ValueEnum};
use function_runner::engine::{run, FunctionRunParams, ProfileOpts};

use is_terminal::IsTerminal;

const PROFILE_DEFAULT_INTERVAL: u32 = 500_000; // every 5us

/// Supported input flavors
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Codec {
    /// JSON input, must be valid JSON
    Json,
    /// Raw input, no validation, passed as-is
    Raw,
    /// JSON input, will be converted to MessagePack, must be valid JSON
    JsonToMessagepack,
}

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser, Debug)]
#[clap(version)]
#[command(arg_required_else_help = true)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Path to json file containing Function input; if omitted, stdin is used
    #[clap(short, long)]
    input: Option<PathBuf>,

    /// Path to GraphQL schema file
    #[clap(long)]
    schema: PathBuf,

    /// Path to GraphQL query file
    #[clap(long)]
    query: PathBuf,

    /// Path to input generator file
    #[clap(long)]
    input_generator: PathBuf,

    /// Name of the export to invoke.
    #[clap(short, long, default_value = "_start")]
    export: String,

    /// Log the run result as a JSON object
    #[clap(short, long)]
    json: bool,

    /// Enable profiling. This will make your Function run slower.
    /// The resulting profile can be used in speedscope (https://www.speedscope.app/)
    /// Specifying --profile-* argument will also enable profiling.
    #[clap(short, long)]
    profile: bool,

    /// Where to save the profile information. Defaults to ./{wasm-filename}.perf.
    #[clap(long)]
    profile_out: Option<PathBuf>,

    /// How many samples per seconds. Defaults to 500_000 (every 5us).
    #[clap(long)]
    profile_frequency: Option<u32>,

    #[clap(short = 'c', long, value_enum, default_value = "json")]
    codec: Codec,
}

impl Opts {
    pub fn profile_opts(&self) -> Option<ProfileOpts> {
        if !self.profile && self.profile_out.is_none() && self.profile_frequency.is_none() {
            return None;
        }

        let interval = self.profile_frequency.unwrap_or(PROFILE_DEFAULT_INTERVAL);
        let out = self
            .profile_out
            .clone()
            .unwrap_or_else(|| self.default_profile_out());

        Some(ProfileOpts { interval, out })
    }

    fn default_profile_out(&self) -> PathBuf {
        let mut path = PathBuf::new();

        path.set_file_name(
            self.function
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("function")),
        );
        path.set_extension("perf");

        path
    }
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    // let mut input: Box<dyn Read + Sync + Send + 'static> = if let Some(ref input) = opts.input {
    //     Box::new(BufReader::new(File::open(input).map_err(|e| {
    //         anyhow!("Couldn't load input {:?}: {}", input, e)
    //     })?))
    // } else if !std::io::stdin().is_terminal() {
    //     Box::new(BufReader::new(stdin()))
    // } else {
    //     return Err(anyhow!(
    //         "You must provide input via the --input flag or piped via stdin."
    //     ));
    // };

    // let input_generator = opts.input.unwrap();
    let cart_sizes = [1, 10, 30, 40, 50, 100, 250, 500];
    for cart_size in cart_sizes {
        let cart_size_str = cart_size.to_string();
        let input_generation_output = Command::new("node")
            .args([
                "/Users/jeffcharles/src/github.com/Shopify/proj-functions-limit-scaling/sm-fn-fuzzer/generate-input/index.mjs",
                opts.schema.to_str().unwrap(),
                opts.query.to_str().unwrap(),
                opts.input_generator.to_str().unwrap(),
                &cart_size_str
            ])
            .output()?;
        if !input_generation_output.status.success() {
            eprintln!("Input generation failed");
            eprintln!(
                "{}",
                String::from_utf8_lossy(&input_generation_output.stderr),
            );
            bail!("No input generated");
        }
        let buffer = input_generation_output.stdout;

        let buffer = match opts.codec {
            Codec::Json => {
                let _ = serde_json::from_slice::<serde_json::Value>(&buffer)
                    .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;
                buffer
            }
            Codec::Raw => buffer,
            Codec::JsonToMessagepack => {
                let json: serde_json::Value = serde_json::from_slice(&buffer)
                    .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;
                rmp_serde::to_vec(&json)
                    .map_err(|e| anyhow!("Couldn't convert JSON to MessagePack: {}", e))?
            }
        };

        let profile_opts = opts.profile_opts();
        let function_run_result = run(FunctionRunParams {
            function_path: opts.function.clone(),
            input: buffer,
            export: opts.export.as_ref(),
            profile_opts: profile_opts.as_ref(),
        })?;

        if opts.json {
            println!("{}", function_run_result.to_json());
        } else {
            println!("Cart size: {cart_size}");
            println!("{function_run_result}");
        }
    }
    // if let Some(profile) = function_run_result.profile.as_ref() {
    //     std::fs::write(profile_opts.unwrap().out, profile)?;
    // }

    Ok(())
}
