use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};

use clap::{Parser, ValueEnum};
use function_runner::{
    bluejay_schema_analyzer::BluejaySchemaAnalyzer,
    engine::{run, FunctionRunParams, ProfileOpts},
};

use is_terminal::IsTerminal;
use serde::Deserialize;
use serde::Serialize;

const PROFILE_DEFAULT_INTERVAL: u32 = 500_000; // every 5us

// todo: get rid of these
#[derive(Serialize, Deserialize, Debug)]
struct Input {
    cart: Cart,
}

#[derive(Serialize, Deserialize, Debug)]
struct Cart {
    lines: Vec<CartLine>,
}

#[derive(Serialize, Deserialize, Debug)]
struct CartLine {
    quantity: u32,
    merchandise: Merchandise,
}

#[derive(Serialize, Deserialize, Debug)]
struct Merchandise {
    id: String,
}

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

    // Also takes in schema string, CLI can generate this via 'generate schema'
    /// Path to json file containing Function input; if omitted, stdin is used
    #[clap(short = 's', long, default_value = "schema.graphql")]
    schema_path: Option<PathBuf>,

    // Also takes in schema string, CLI can generate this via 'generate schema'
    /// Path to json file containing Function input; if omitted, stdin is used
    #[clap(short = 'q', long, default_value = "input.graphql")]
    query_path: Option<PathBuf>,
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

    // Reads the schema file and returns its contents as a String.
    pub fn read_schema_to_string(&self) -> Result<String> {
        match &self.schema_path {
            Some(schema_path) => {
                let mut file = File::open(schema_path)
                    .map_err(|e| anyhow!("Couldn't open schema file {:?}: {}", schema_path, e))?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|e| anyhow!("Couldn't read schema file {:?}: {}", schema_path, e))?;
                Ok(contents)
            }
            None => Err(anyhow!("Schema file path is not provided")),
        }
    }

    pub fn read_query_to_string(&self) -> Result<String> {
        match &self.query_path {
            Some(query_path) => {
                let mut file = File::open(query_path)
                    .map_err(|e| anyhow!("Couldn't open schema file {:?}: {}", query_path, e))?;
                let mut contents = String::new();
                file.read_to_string(&mut contents)
                    .map_err(|e| anyhow!("Couldn't read schema file {:?}: {}", query_path, e))?;
                Ok(contents)
            }
            None => Err(anyhow!("Schema file path is not provided")),
        }
    }
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let schema_string = opts.read_schema_to_string()?;
    let query_string = opts.read_query_to_string()?;

    let document_definition =
        BluejaySchemaAnalyzer::create_definition_document(&schema_string).unwrap();

    // Properly handle the Result from create_schema_definition
    let schema_result = BluejaySchemaAnalyzer::create_schema_definition(&document_definition);

    let analyze_result = match schema_result {
        Ok(schema) => BluejaySchemaAnalyzer::analyze_schema_definition(schema, &query_string),
        Err(errors) => {
            for error in errors {
                eprintln!("Error creating schema definition: {:?}", error);
            }
            panic!("opps got an error!")
        }
    };

    eprintln!("analyze result => {:?}", analyze_result);

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

    let scaling_factor = match analyze_result {
        Ok(rate) => {
            let input: Input = serde_json::from_slice(&buffer)?;
            let num_cart_lines = input.cart.lines.len();

            match num_cart_lines < 200 {
                true => 1.0,
                false => rate,
            }
        }
        Err(_) => {
            panic!("an error occured");
        }
    };

    eprintln!("scaling_factor {:?}", scaling_factor);

    // ** Determine Cart Lines for Scaling **
    // based on 'num_cart_lines' and the result of 'analyze_result
    // use 1.0 for scaling rate if rate lines are less than
    // the scaling factor is 1.0 if cart lines is less than 200
    // the scaling factor is the rate if lines are over 200

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
