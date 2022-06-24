use anyhow::{anyhow, Result};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use wasmtime::{Config, Engine, Linker, Module, Store};

use crate::function_run_result::FunctionRunResult;

pub fn run(function_path: PathBuf, input_path: PathBuf) -> Result<FunctionRunResult> {
    let engine = if cfg!(target_arch = "x86_64") {
        // enabling this on non-x86 architectures currently causes an error (as of wasmtime 0.37.0)
        Engine::new(Config::new().debug_info(true))?
    } else {
        Engine::default()
    };
    let module = Module::from_file(&engine, &function_path)
        .map_err(|e| anyhow!("Couldn't load the Function {:?}: {}", &function_path, e))?;

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
        let mut wasi = deterministic_wasi_ctx::build_wasi_ctx();
        wasi.set_stdin(Box::new(input_stream));
        wasi.set_stdout(Box::new(output_stream.clone()));
        wasi.set_stderr(Box::new(error_stream.clone()));
        let mut store = Store::new(&engine, wasi);

        linker.module(&mut store, "Function", &module)?;

        let start = Instant::now();

        let instance = linker.instantiate(&mut store, &module)?;

        // This is a hack to get the memory usage. Wasmtime requires a mutable borrow to a store for caching.
        // We need this mutable borrow to fall out of scope so that we can mesure memory usage.
        // https://docs.rs/wasmtime/0.37.0/wasmtime/struct.Instance.html#why-does-get_export-take-a-mutable-context
        let memory_names: Vec<String> = instance
            .exports(&mut store)
            .into_iter()
            .filter(|export| export.clone().into_memory().is_some())
            .map(|export| export.name().to_string())
            .collect();

        memory_usage = memory_names
            .iter()
            .map(|name| {
                let memory = instance.get_memory(&mut store, name).unwrap();
                memory.size(&store)
            })
            .sum();

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
        std::str::from_utf8(&logs).map_err(|e| anyhow!("Couldn't print Function Logs: {}", e))?;

    let output = output_stream
        .try_into_inner()
        .expect("Output stream reference still exists")
        .into_inner();
    let output: serde_json::Value = serde_json::from_slice(output.as_slice())
        .map_err(|e| anyhow!("Couldn't decode Function Output: {}", e))?;

    let name = function_path.file_name().unwrap().to_str().unwrap();
    let size = function_path.metadata()?.len();

    let function_run_result = FunctionRunResult::new(
        name.to_string(),
        runtime,
        size,
        memory_usage,
        output,
        logs.to_string(),
    );

    Ok(function_run_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Arbitrary, used to verify that the runner works as expected.
    const HELLO_WORLD_MEMORY_USAGE: u64 = 17;
    const MODIFIED_HELLO_WORLD_MEMORY_USAGE: u64 = 42;

    #[test]
    fn test_memory_usage_under_threshold() {
        let function_run_result = run(
            Path::new("tests/benchmarks/hello_world.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_world.json").to_path_buf(),
        )
        .unwrap();

        assert_eq!(function_run_result.memory_usage, HELLO_WORLD_MEMORY_USAGE);
    }

    #[test]
    fn test_memory_usage_over_threshold() {
        let function_run_result = run(
            Path::new("tests/benchmarks/hello_42_pages.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_42_pages.json").to_path_buf(),
        )
        .unwrap();

        assert_eq!(
            function_run_result.memory_usage,
            MODIFIED_HELLO_WORLD_MEMORY_USAGE
        );
    }

    #[test]
    fn test_stack_overflow() {
        let function_run_result = run(
            Path::new("tests/benchmarks/stack_overflow.wasm").to_path_buf(),
            Path::new("tests/benchmarks/stack_overflow.json").to_path_buf(),
        );

        assert!(function_run_result.is_err());
    }

    #[test]
    fn test_file_size() {
        let function_run_result = run(
            Path::new("tests/benchmarks/hello_world.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_world.json").to_path_buf(),
        )
        .unwrap();

        assert_eq!(
            function_run_result.size,
            Path::new("tests/benchmarks/hello_world.wasm")
                .metadata()
                .unwrap()
                .len()
        );
    }
}
