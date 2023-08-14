use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use function_runner::engine::{run, ProfileOpts};

use is_terminal::IsTerminal;

const PROFILE_DEFAULT_INTERVAL: u32 = 500_000; // every 5us

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

    #[clap(long)]
    /// How many samples per seconds. Defaults to 500_000 (every 5us).
    profile_frequency: Option<u32>,
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
    let _ = serde_json::from_slice::<serde_json::Value>(&buffer)
        .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;

    let profile_opts = opts.profile_opts();
    let function_run_result = run(
        opts.function,
        buffer,
        opts.export.as_ref(),
        profile_opts.as_ref(),
    )?;

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
