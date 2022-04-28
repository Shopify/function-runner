use std::path::PathBuf;
use anyhow::{anyhow, Result};
use clap::Parser;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

mod memory_limiter;
use memory_limiter::MemoryLimiter;

/// Simple script runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version = "0.2.0")]
struct Opts {
    /// Path to wasm/wat script
    #[clap(short, long, default_value = "script.wasm")]
    script: PathBuf,

    /// Path to json file containing script input
    input: PathBuf,
}

const DEFAULT_LINEAR_MEMORY_LIMIT: usize = 1;

struct StoreData {
    wasi: wasmtime_wasi::WasiCtx,
    limiter: MemoryLimiter,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let engine = Engine::default();
    let module = Module::from_file(&engine, &opts.script)
        .map_err(|e| anyhow!("Couldn't load script {:?}: {}", &opts.script, e))?;

    let input: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(&opts.input)
            .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &opts.input, e))?,
    )
    .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &opts.input, e))?;
    let input = serde_json::to_vec(&input)?;

    let input_stream = wasi_common::pipe::ReadPipe::new(std::io::Cursor::new(input));
    let output_stream = wasi_common::pipe::WritePipe::new_in_memory();
    let error_stream = wasi_common::pipe::WritePipe::new_in_memory();

    {
        // Link WASI and construct the store.
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut StoreData| &mut ctx.wasi)?;
        let wasi = WasiCtxBuilder::new()
            .stdin(Box::new(input_stream))
            .stdout(Box::new(output_stream.clone()))
            .stderr(Box::new(error_stream.clone()))
            .inherit_args()?
            .build();
        let limiter =  MemoryLimiter::new(DEFAULT_LINEAR_MEMORY_LIMIT);
        let store_data = StoreData {
            wasi,
            limiter,
        };
        let mut store = Store::new(&engine, store_data);
        store.limiter(|d| &mut d.limiter);

        linker.module(&mut store, "", &module)?;

        // Execute the module
        let result = linker
            .get_default(&mut store, "")?
            .typed::<(), (), _>(&store)?
            .call(&mut store, ());

        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error:\n{}", e);
            }
        }
    };

    let logs = error_stream
        .try_into_inner()
        .expect("Error stream reference still exists")
        .into_inner();
    let logs =
        std::str::from_utf8(&logs).map_err(|e| anyhow!("Couldn't print Script Logs: {}", e))?;
    println!("Logs:\n{}", logs);

    let output = output_stream
        .try_into_inner()
        .expect("Output stream reference still exists")
        .into_inner();
    let output: serde_json::Value = serde_json::from_slice(output.as_slice())
        .map_err(|e| anyhow!("Couldn't decode Script Output: {}", e))?;
    println!("Output:\n{}", serde_json::to_string_pretty(&output)?);

    Ok(())
}
