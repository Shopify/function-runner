use function_runner::{BytesContainer, BytesContainerType, Codec};
use wasmtime::Module;

use std::{
    fs::File,
    io::{stdin, BufRead, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use function_runner::{
    bluejay_schema_analyzer::BluejaySchemaAnalyzer,
    engine::{run, FunctionRunParams, ProfileOpts},
};

use is_terminal::IsTerminal;

const PROFILE_DEFAULT_INTERVAL: u32 = 500_000; // every 5us
const DEFAULT_SCALE_FACTOR: f64 = 1.0;

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

    /// How many samples per second. Defaults to 500_000 (every 2us).
    #[clap(long)]
    profile_frequency: Option<u32>,

    /// Path to graphql file containing Function schema; if omitted, defaults will be used to calculate limits.
    #[clap(short = 's', long)]
    schema_path: Option<PathBuf>,

    /// Path to graphql file containing Function input query; if omitted, defaults will be used to calculate limits.
    #[clap(short = 'q', long)]
    query_path: Option<PathBuf>,

    /// Enable batch mode - read multiple JSON inputs (one per line) from stdin/file
    #[clap(short, long)]
    batch: bool,

    /// In batch mode, fail fast on individual input errors (default: false)
    #[clap(long)]
    batch_fail_on_error: bool,
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

    pub fn read_schema_to_string(&self) -> Option<Result<String>> {
        self.schema_path.as_ref().map(read_file_to_string)
    }

    pub fn read_query_to_string(&self) -> Option<Result<String>> {
        self.query_path.as_ref().map(read_file_to_string)
    }
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
    let opts: Opts = Opts::parse();

    // Create engine and module once (expensive operations - amortize across all inputs)
    let engine = function_runner::engine::new_engine()?;
    let module = Module::from_file(&engine, &opts.function)
        .map_err(|e| anyhow!("Couldn't load the Function {:?}: {}", &opts.function, e))?;

    // Infer codec from the module based on imported modules
    let codec = if function_runner::engine::uses_msgpack_provider(&module) {
        Codec::Messagepack
    } else {
        Codec::Json
    };

    if opts.batch {
        run_batch_mode(&opts, &engine, &module, codec)
    } else {
        run_single_mode(&opts, &engine, &module, codec)
    }
}

fn run_single_mode(
    opts: &Opts,
    engine: &wasmtime::Engine,
    module: &Module,
    codec: Codec,
) -> Result<()> {
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

    let schema_string = opts.read_schema_to_string().transpose()?;
    let query_string = opts.read_query_to_string().transpose()?;

    let input = BytesContainer::new(BytesContainerType::Input, codec, buffer)?;
    let scale_factor = if let (Some(schema_string), Some(query_string), Some(json_value)) =
        (schema_string, query_string, input.json_value.clone())
    {
        BluejaySchemaAnalyzer::analyze_schema_definition(
            &schema_string,
            opts.schema_path.as_ref().and_then(|p| p.to_str()),
            &query_string,
            opts.query_path.as_ref().and_then(|p| p.to_str()),
            &json_value,
        )?
    } else {
        DEFAULT_SCALE_FACTOR
    };

    let profile_opts = opts.profile_opts();

    let function_run_result = run(FunctionRunParams {
        function_path: opts.function.clone(),
        input,
        export: opts.export.as_ref(),
        profile_opts: profile_opts.as_ref(),
        scale_factor,
        module: module.clone(),
        engine: engine.clone(),
    })?;

    if opts.json {
        println!("{}", function_run_result.to_json());
    } else {
        println!("{function_run_result}");
    }

    if let Some(profile) = function_run_result.profile.as_ref() {
        std::fs::write(profile_opts.unwrap().out, profile)?;
    }

    if function_run_result.success {
        Ok(())
    } else {
        anyhow::bail!("The Function execution failed. Review the logs for more information.")
    }
}

fn run_batch_mode(
    opts: &Opts,
    engine: &wasmtime::Engine,
    module: &Module,
    codec: Codec,
) -> Result<()> {
    let input_reader: Box<dyn BufRead> = if let Some(ref input) = opts.input {
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

    // Load schema/query once; scale factor is computed per input.
    let schema_string = opts.read_schema_to_string().transpose()?;
    let query_string = opts.read_query_to_string().transpose()?;

    // Disable profiling in batch mode for performance
    let profile_opts = None;

    let mut line_num = 0;
    let mut processed_count = 0;
    let mut success_count = 0;
    let mut failed_count = 0;

    for line_result in input_reader.lines() {
        line_num += 1;

        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                failed_count += 1;
                if opts.batch_fail_on_error {
                    return Err(e.into());
                } else {
                    eprintln!("Error reading line {}: {}", line_num, e);
                    println!(
                        r#"{{"success":false,"error":"Error reading input: {}"}}"#,
                        e
                    );
                    continue;
                }
            }
        };

        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }

        processed_count += 1;

        // Parse input
        let input = match BytesContainer::new(BytesContainerType::Input, codec, line.into_bytes()) {
            Ok(i) => i,
            Err(e) => {
                failed_count += 1;
                if opts.batch_fail_on_error {
                    return Err(e);
                } else {
                    eprintln!("Error parsing line {}: {}", line_num, e);
                    println!(r#"{{"success":false,"error":"Invalid JSON input: {}"}}"#, e);
                    continue;
                }
            }
        };

        // Calculate scale factor for this input
        let scale_factor =
            if let (Some(ref schema_string), Some(ref query_string), Some(ref json_value)) =
                (&schema_string, &query_string, &input.json_value)
            {
                match BluejaySchemaAnalyzer::analyze_schema_definition(
                    schema_string,
                    opts.schema_path.as_ref().and_then(|p| p.to_str()),
                    query_string,
                    opts.query_path.as_ref().and_then(|p| p.to_str()),
                    json_value,
                ) {
                    Ok(sf) => sf,
                    Err(e) => {
                        failed_count += 1;
                        if opts.batch_fail_on_error {
                            return Err(e);
                        } else {
                            eprintln!("Error analyzing schema for line {}: {}", line_num, e);
                            println!(
                                r#"{{"success":false,"error":"Schema analysis failed: {}"}}"#,
                                e
                            );
                            continue;
                        }
                    }
                }
            } else {
                DEFAULT_SCALE_FACTOR
            };

        // Run function (reusing engine/module!)
        let result = run(FunctionRunParams {
            function_path: opts.function.clone(),
            input,
            export: opts.export.as_ref(),
            profile_opts,
            scale_factor,
            module: module.clone(),
            engine: engine.clone(),
        });

        // Output result immediately (streaming JSONL - compact format for line-by-line parsing)
        match result {
            Ok(function_result) => {
                let function_succeeded = function_result.success;
                if function_succeeded {
                    success_count += 1;
                } else {
                    failed_count += 1;
                }

                // Use compact JSON (not pretty-printed) for JSONL format
                let compact_json = serde_json::to_string(&function_result)
                    .unwrap_or_else(|error| error.to_string());
                println!("{}", compact_json);

                if !function_succeeded && opts.batch_fail_on_error {
                    anyhow::bail!(
                        "Function execution failed on line {}. Review the logs for more information.",
                        line_num
                    );
                }
            }
            Err(e) => {
                failed_count += 1;
                if opts.batch_fail_on_error {
                    return Err(e);
                } else {
                    eprintln!("Error executing line {}: {}", line_num, e);
                    println!(r#"{{"success":false,"error":"Execution failed: {}"}}"#, e);
                }
            }
        }
    }

    // Log summary to stderr (so it doesn't interfere with JSONL output on stdout)
    eprintln!(
        "Batch complete: {} inputs processed, {} successful, {} failed",
        processed_count, success_count, failed_count
    );

    Ok(())
}
