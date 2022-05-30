use anyhow::{anyhow, Result};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use wasmtime::{Engine, Linker, Module, Store};

use crate::function_run_result::FunctionRunResult;

pub fn run(script_path: PathBuf, input_path: PathBuf) -> Result<FunctionRunResult> {
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

    {
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let mut wasi = deterministic_wasi_ctx::build_wasi_ctx();
        wasi.set_stdin(Box::new(input_stream));
        wasi.set_stdout(Box::new(output_stream.clone()));
        wasi.set_stderr(Box::new(error_stream.clone()));
        let mut store = Store::new(&engine, wasi);

        linker.module(&mut store, "", &module)?;

        let start = Instant::now();

        let instance = linker.instantiate(&mut store, &module)?;
        let module_result = instance
            .get_typed_func::<(), (), _>(&mut store, "_start")?
            .call(&mut store, ());

        runtime = start.elapsed();

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

    let function_run_result = FunctionRunResult::new(runtime, output, logs.to_string());

    Ok(function_run_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use more_asserts::*;
    use std::path::Path;

    #[test]
    fn test_runtime_duration() {
        // This is using the https://github.com/Shopify/shopify-vm-prototype-script script
        let function_run_result = run(
            Path::new("tests/benchmarks/hello_world.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_world.json").to_path_buf(),
        )
        .unwrap();

        assert_ge!(function_run_result.runtime, Duration::from_millis(0));
        assert_le!(function_run_result.runtime, Duration::from_millis(5));
    }
}
