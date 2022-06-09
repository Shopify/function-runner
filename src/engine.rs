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
        std::str::from_utf8(&logs).map_err(|e| anyhow!("Couldn't print Script Logs: {}", e))?;

    let output = output_stream
        .try_into_inner()
        .expect("Output stream reference still exists")
        .into_inner();
    let output: serde_json::Value = serde_json::from_slice(output.as_slice())
        .map_err(|e| anyhow!("Couldn't decode Script Output: {}", e))?;

    // get the script file name
    let name = script_path.file_name().unwrap().to_str().unwrap();
    let size = script_path.metadata()?.len();

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

    const LINEAR_MEMORY_USAGE: u64 = 159;

    #[test]
    fn test_linear_memory_usage() {
        let function_run_result = run(
            Path::new("benchmark/build/linear_memory_function.wasm").to_path_buf(),
            Path::new("benchmark/build/hello_world.json").to_path_buf(),
        )
        .unwrap();

        assert_eq!(function_run_result.memory_usage, LINEAR_MEMORY_USAGE);
    }

    #[test]
    fn test_file_size() {
        let function_run_result = run(
            Path::new("benchmark/build/size_function.wasm").to_path_buf(),
            Path::new("benchmark/build/hello_world.json").to_path_buf(),
        )
        .unwrap();

        assert_eq!(
            function_run_result.size,
            Path::new("benchmark/build/size_function.wasm")
                .metadata()
                .unwrap()
                .len()
        );
    }
}
