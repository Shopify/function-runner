use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use colored::Colorize;
use function_runner::{
    bluejay_schema_analyzer::BluejaySchemaAnalyzer,
    engine::{run as engine_run, FunctionRunParams, ProfileOpts},
    function_run_result::FunctionOutput,
    test_report::TestReport,
};

use is_terminal::IsTerminal;

const PROFILE_DEFAULT_INTERVAL: u32 = 500_000; // every 5us
const DEFAULT_SCALE_FACTOR: f64 = 1.0;

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

#[derive(Parser, Debug)]
#[clap(version)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[clap(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run(RunOpts),
    Test(TestOpts),
}

#[derive(Args, Debug)]
struct CommonOpts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Name of the export to invoke.
    #[clap(short, long, default_value = "_start")]
    export: String,

    /// Log the result as a JSON object
    #[clap(short, long)]
    json: bool,

    #[clap(short = 'c', long, value_enum, default_value = "json")]
    codec: Codec,

    /// Path to graphql file containing Function schema; if omitted, defaults will be used to calculate limits.
    #[clap(short = 's', long)]
    schema_path: Option<PathBuf>,

    /// Path to graphql file containing Function input query; if omitted, defaults will be used to calculate limits.
    #[clap(short = 'q', long)]
    query_path: Option<PathBuf>,
}

impl CommonOpts {
    pub fn read_schema_to_string(&self) -> Option<Result<String>> {
        self.schema_path.as_ref().map(read_file_to_string)
    }

    pub fn read_query_to_string(&self) -> Option<Result<String>> {
        self.query_path.as_ref().map(read_file_to_string)
    }
}

/// Simple Function runner which takes JSON as a convenience.
#[derive(Args, Debug)]
struct RunOpts {
    #[clap(flatten)]
    common_opts: CommonOpts,

    /// Path to json file containing Function input; if omitted, stdin is used
    #[clap(short, long)]
    input: Option<PathBuf>,

    /// Enable profiling. This will make your Function run slower.
    /// The resulting profile can be used in speedscope (https://www.speedscope.app/)
    /// Specifying --profile-* argument will also enable profiling.
    #[clap(short, long)]
    profile: bool,

    /// Where to save the profile information. Defaults to ./{wasm-filename}.perf.
    #[clap(long)]
    profile_out: Option<PathBuf>,

    /// How many samples per second. Defaults to 500_000 (every 2us).
    #[clap(long)]
    profile_frequency: Option<u32>,
}

impl RunOpts {
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
            self.common_opts
                .function
                .file_name()
                .unwrap_or(std::ffi::OsStr::new("function")),
        );
        path.set_extension("perf");

        path
    }
}

#[derive(Args, Debug)]
struct TestOpts {
    #[clap(flatten)]
    common_opts: CommonOpts,

    /// Path to directory containing Function input test cases
    #[clap(short, long, default_value = "integration_tests/input/")]
    input: PathBuf,

    /// Path to directory containing Function test case expected outputs
    #[clap(short, long, default_value = "integration_tests/output/")]
    output: PathBuf,
}

fn read_file_to_string(file_path: &PathBuf) -> Result<String> {
    let mut file = File::open(file_path)
        .map_err(|e| anyhow!("Couldn't open file {}: {}", file_path.to_string_lossy(), e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| anyhow!("Couldn't read file {}: {}", file_path.to_string_lossy(), e))?;

    Ok(contents)
}

fn main() -> Result<()> {
    let Cli { cmd } = Cli::parse();

    match cmd {
        Command::Run(opts) => run(opts),
        Command::Test(opts) => test(opts),
    }
}

fn run(opts: RunOpts) -> Result<()> {
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

    let schema_string = opts.common_opts.read_schema_to_string().transpose()?;

    let query_string = opts.common_opts.read_query_to_string().transpose()?;

    let (json_value, buffer) = match opts.common_opts.codec {
        Codec::Json => {
            let json = serde_json::from_slice::<serde_json::Value>(&buffer)
                .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;
            let minified_buffer =
                serde_json::to_vec(&json).map_err(|e| anyhow!("Couldn't serialize JSON: {}", e))?;
            (Some(json), minified_buffer)
        }
        Codec::Raw => (None, buffer),
        Codec::JsonToMessagepack => {
            let json: serde_json::Value = serde_json::from_slice(&buffer)
                .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;
            let bytes = rmp_serde::to_vec(&json)
                .map_err(|e| anyhow!("Couldn't convert JSON to MessagePack: {}", e))?;
            (Some(json), bytes)
        }
    };

    let scale_factor = if let (Some(schema_string), Some(query_string), Some(json_value)) =
        (schema_string, query_string, json_value)
    {
        BluejaySchemaAnalyzer::analyze_schema_definition(
            &schema_string,
            opts.common_opts
                .schema_path
                .as_ref()
                .and_then(|p| p.to_str()),
            &query_string,
            opts.common_opts
                .query_path
                .as_ref()
                .and_then(|p| p.to_str()),
            &json_value,
        )?
    } else {
        DEFAULT_SCALE_FACTOR // Use default scale factor when schema or query is missing
    };

    let profile_opts = opts.profile_opts();

    let function_run_result = engine_run(FunctionRunParams {
        function_path: opts.common_opts.function,
        input: buffer,
        export: opts.common_opts.export.as_ref(),
        profile_opts: profile_opts.as_ref(),
        scale_factor,
    })?;

    if opts.common_opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{function_run_result}");
    }

    if let Some(profile) = function_run_result.profile.as_ref() {
        std::fs::write(profile_opts.unwrap().out, profile)?;
    }

    Ok(())
}

fn test(opts: TestOpts) -> Result<()> {
    if !opts.input.is_dir() {
        return Err(anyhow!("Input path must be a directory"));
    }
    if !opts.output.is_dir() {
        return Err(anyhow!("Output path must be a directory"));
    }

    if opts.common_opts.codec != Codec::Json {
        return Err(anyhow!("Only JSON codec is supported for testing"));
    }

    let mut report = TestReport::default();

    let file_name_and_input_and_output_paths = std::fs::read_dir(opts.input)?
        .map(|entry| {
            let entry = entry?;

            let input_path = entry.path();

            if input_path.is_dir() {
                Ok(None)
            } else if let Some(file_name) = input_path.file_name() {
                let file_name = file_name.to_string_lossy().into_owned();
                let output_path = opts.output.join(&file_name);
                Ok(Some((file_name, input_path, output_path)))
            } else {
                Err(anyhow!("Invalid file name"))
            }
        })
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>>>()?;

    match file_name_and_input_and_output_paths.len() {
        0 => anyhow::bail!("No test files found"),
        1 => println!("running 1 test"),
        n => println!("running {n} tests"),
    }

    file_name_and_input_and_output_paths
        .into_iter()
        .try_for_each(
            |(file_name, input_path, output_path)| -> anyhow::Result<()> {
                let input = read_file_to_string(&input_path)?;
                let output = read_file_to_string(&output_path)?;

                let output = serde_json::from_str::<serde_json::Value>(&output)
                    .map_err(|e| anyhow!("Invalid output JSON: {}", e))?;

                print!("test {} ...", &file_name);

                let function_run_result = engine_run(FunctionRunParams {
                    function_path: opts.common_opts.function.clone(),
                    input: input.into_bytes(),
                    export: opts.common_opts.export.as_ref(),
                    profile_opts: None,
                    scale_factor: 1.0,
                })?;

                match &function_run_result.output {
                    FunctionOutput::JsonOutput(json) => {
                        if json != &output {
                            report.add_failure(file_name, output, function_run_result);
                            println!(" {}", "FAILED".red());
                        } else {
                            report.add_success();
                            println!(" {}", "ok".green());
                        }
                    }
                    FunctionOutput::InvalidJsonOutput(_) => {
                        report.add_failure(file_name, output, function_run_result);
                        println!(" {}", "FAILED".red());
                    }
                }

                Ok(())
            },
        )?;

    report.into_result()
}
