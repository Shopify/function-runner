use anyhow::{anyhow, Result};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use wasmtime::{Engine, Linker, Module, Store};

use wasmtime_wasi::sync::WasiCtxBuilder;

use crate::execution_result::ExecutionResult;

pub fn run(script_path: PathBuf, input_path: PathBuf) -> Result<ExecutionResult> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, &script_path)
        .map_err(|e| anyhow!("Couldn't load script {:?}: {}", &script_path, e))?;

    let input: serde_json::Value = serde_json::from_reader(
        std::fs::File::open(&input_path)
            .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &input_path, e))?,
    )
    .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &input_path, e))?;
    let input = serde_json::to_vec(&input)?;

    let input_stream = wasi_common::pipe::ReadPipe::new(std::io::Cursor::new(input));
    let output_stream = wasi_common::pipe::WritePipe::new_in_memory();
    let error_stream = wasi_common::pipe::WritePipe::new_in_memory();

    let runtime: Duration;
    let memory_usage: u64;

    {
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let wasi = WasiCtxBuilder::new()
            .stdin(Box::new(input_stream))
            .stdout(Box::new(output_stream.clone()))
            .stderr(Box::new(error_stream.clone()))
            .inherit_args()?
            .build();
        let mut store = Store::new(&engine, wasi);

        linker.module(&mut store, "", &module)?;

        let start = Instant::now();

        let instance = linker.instantiate(&mut store, &module)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or(anyhow::format_err!("failed to find `memory` export"))?;

        let module_result = instance
            .get_typed_func::<(), (), _>(&mut store, "_start")?
            .call(&mut store, ());

        runtime = start.elapsed();
        memory_usage = memory.size(&store);

        match module_result {
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

    let output = output_stream
        .try_into_inner()
        .expect("Output stream reference still exists")
        .into_inner();
    let output: serde_json::Value = serde_json::from_slice(output.as_slice())
        .map_err(|e| anyhow!("Couldn't decode Script Output: {}", e))?;

    let statistics = ExecutionResult::new(
        runtime,
        Duration::from_millis(5),
        memory_usage,
        output,
        logs.to_string(),
    );

    Ok(statistics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_runtime_under_threshold() {
        let statistics = run(
            Path::new("tests/benchmarks/hello_world.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_world.json").to_path_buf(),
        )
        .unwrap();

        assert!(statistics.runtime <= statistics.threshold);
    }

    #[test]
    fn test_runtime_over_threshold() {
        let statistics = run(
            Path::new("tests/benchmarks/sleeps.wasm").to_path_buf(),
            Path::new("tests/benchmarks/sleeps.json").to_path_buf(),
        )
        .unwrap();

        assert!(statistics.runtime > statistics.threshold);
    }
}
