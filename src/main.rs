use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
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

    // Needed to determine scale_factor
    #[clap(short = 's', long, default_value = "schema.graphql")]
    schema_path: PathBuf,

    // Needed to determine scale_factor
    #[clap(short = 'q', long, default_value = "src/run.graphql")]
    query_path: PathBuf,
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
    // ?? TODO: look into CLI generate schema and what that does
    // will they take in file path or string directly
    fn load_schema(&self) -> Result<String> {
        std::fs::read_to_string(&self.schema_path)
            .map_err(|e| anyhow!("Failed to load schema: {}", e))
    }

    // ?? TODO: we pass the query directly or do we read from file
    fn load_query(&self) -> Result<String> {
        std::fs::read_to_string(&self.schema_path)
            .map_err(|e| anyhow!("Failed to load query path: {}", e))
    }
}

struct ScaleLimits {
    input_size_bytes: u64,
    output_size_bytes: u64,
    instruction_count: u64,
}

const INPUT_SIZE_BYTES_LIMIT: u64 = 64_000;
const OUT_SIZE_BYTES_LIMIT: u64 = 20_000;
const INSTRUCTION_COUNT_LIMIT: u64 = 11_000_000;

fn analyze_query(schema: &str, query: &str) -> Result<ScaleLimits> {
    compute_scale_limits(&schema, &query)
}

fn compute_scale_limits(schema: &str, query: &str) -> Result<ScaleLimits> {
    let limits = ScaleLimits {
        input_size_bytes: INPUT_SIZE_BYTES_LIMIT,
        output_size_bytes: OUT_SIZE_BYTES_LIMIT,
        instruction_count: 11000000,
    };

    eprintln!("🔵bluejay analyzer");
    eprintln!("{:?}", schema);
    eprint!("{:?}", query);

    Ok(limits)
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    eprintln!("Opts {:?}", opts);

    let mut input: Box<dyn Read + Sync + Send + 'static> = if let Some(ref input) = opts.input {
        Box::new(BufReader::new(File::open(input).map_err(|e| {
            anyhow!("Couldn't load input {:?}: {}", input, e)
        })?))
    } else if !std::io::stdin().is_terminal() {
        Box::new(BufReader::new(stdin()))
    } else {
        return Err(anyhow!(
            "You must provide input via the --input flag or piped via stdin."
        ));
    };

    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;

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
        function_path: opts.function,
        input: buffer,
        export: opts.export.as_ref(),
        profile_opts: profile_opts.as_ref(),
    })?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{function_run_result}");
    }

    if let Some(profile) = function_run_result.profile.as_ref() {
        std::fs::write(profile_opts.unwrap().out, profile)?;
    }

    Ok(())
}
