use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::Parser;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

/// Simple script runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version = "0.1.0")]
struct Opts {
    /// Path to wasm/wat script
    #[clap(short, long, default_value = "script.wasm")]
    script: PathBuf,

    /// Path to json file containing script input
    input: PathBuf,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let engine = Engine::default();
    let module = Module::from_file(&engine, &opts.script)
        .map_err(|e| anyhow!("Couldn't load script {:?}: {}", &opts.script, e))?;

    // Read input and translate to a msgpack stream
    let input: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(&opts.input)
            .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &opts.input, e))?,
    )
    .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &opts.input, e))?;
    let input = rmp_serde::encode::to_vec(&input)?;
    let input_stream = wasi_common::pipe::ReadPipe::new(std::io::Cursor::new(input));

    // Create a stream for capturing the output
    let output_stream = wasi_common::pipe::WritePipe::new_in_memory();

    {
        // Link WASI and construct the store.
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let wasi = WasiCtxBuilder::new()
            .stdin(Box::new(input_stream))
            .stdout(Box::new(output_stream.clone()))
            .inherit_args()?
            .build();
        let mut store = Store::new(&engine, wasi);

        linker.module(&mut store, "", &module)?;

        // Execute the module
        linker
            .get_default(&mut store, "")?
            .typed::<(), (), _>(&store)?
            .call(&mut store, ())?;
    };

    let output = output_stream
        .try_into_inner()
        .expect("Output stream reference still exists")
        .into_inner();

    // Translate msgpack output to JSON and write to STDOUT
    let output: serde_json::Value = rmp_serde::decode::from_read(output.as_slice())
        .map_err(|e| anyhow!("Couldn't decode Script Output: {}", e))?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
