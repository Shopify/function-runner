use anyhow::{anyhow, Result};
use std::{
    io::Cursor,
    path::PathBuf,
    time::{Duration, Instant},
};
use wasmtime::{Engine, Linker, Module, Store};

use crate::function_run_result::{
    FunctionOutput::{self, InvalidJsonOutput, JsonOutput},
    FunctionRunResult, InvalidOutput,
};

pub fn run(function_path: PathBuf, input: Vec<u8>) -> Result<FunctionRunResult> {
    let engine = Engine::default();
    let module = Module::from_file(&engine, &function_path)
        .map_err(|e| anyhow!("Couldn't load the Function {:?}: {}", &function_path, e))?;

    let input_stream = wasi_common::pipe::ReadPipe::new(Cursor::new(input));
    let output_stream = wasi_common::pipe::WritePipe::new_in_memory();
    let error_stream = wasi_common::pipe::WritePipe::new_in_memory();

    let runtime: Duration;
    let memory_usage: u64;
    let mut error_logs: String = String::new();

    {
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let mut wasi = deterministic_wasi_ctx::build_wasi_ctx();
        wasi.set_stdin(Box::new(input_stream));
        wasi.set_stdout(Box::new(output_stream.clone()));
        wasi.set_stderr(Box::new(error_stream.clone()));
        let mut store = Store::new(&engine, wasi);

        linker.module(&mut store, "Function", &module)?;

        let instance = linker.instantiate(&mut store, &module)?;

        let start = Instant::now();

        let module_result = instance
            .get_typed_func::<(), ()>(&mut store, "_start")?
            .call(&mut store, ());

        runtime = start.elapsed();

        // This is a hack to get the memory usage. Wasmtime requires a mutable borrow to a store for caching.
        // We need this mutable borrow to fall out of scope so that we can measure memory usage.
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
                memory.data_size(&store) as u64
            })
            .sum::<u64>()
            / 1024;

        match module_result {
            Ok(_) => {}
            Err(e) => {
                error_logs = e.to_string();
            }
        }
    };

    let raw_logs = error_stream
        .try_into_inner()
        .expect("Error stream reference still exists")
        .into_inner();
    let mut logs = std::string::String::from_utf8(raw_logs)
        .map_err(|e| anyhow!("Couldn't print Function Logs: {}", e))?;

    logs.push_str(&error_logs);

    let raw_output = output_stream
        .try_into_inner()
        .expect("Output stream reference still exists")
        .into_inner();

    let output: FunctionOutput = match serde_json::from_slice(&raw_output) {
        Ok(json_output) => JsonOutput(json_output),
        Err(error) => InvalidJsonOutput(InvalidOutput {
            stdout: std::str::from_utf8(&raw_output)
                .map_err(|e| anyhow!("Couldn't print Function Output: {}", e))
                .unwrap()
                .to_owned(),
            error: error.to_string(),
        }),
    };

    let name = function_path.file_name().unwrap().to_str().unwrap();
    let size = function_path.metadata()?.len() / 1024;

    let function_run_result = FunctionRunResult::new(
        name.to_string(),
        runtime,
        size,
        memory_usage,
        logs.to_string(),
        output,
    );

    Ok(function_run_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::Path,
    };

    const LINEAR_MEMORY_USAGE: u64 = 159 * 64;

    #[test]
    fn test_linear_memory_usage_in_kb() {
        let mut input = vec![];
        let mut reader =
            BufReader::new(File::open("benchmark/build/product_discount.json").unwrap());
        reader.read_to_end(&mut input).expect("Should not fail");

        let function_run_result = run(
            Path::new("benchmark/build/linear_memory_function.wasm").to_path_buf(),
            input,
        )
        .unwrap();

        assert_eq!(function_run_result.memory_usage, LINEAR_MEMORY_USAGE);
    }

    #[test]
    fn test_file_size_in_kb() {
        let mut input = vec![];
        let mut reader =
            BufReader::new(File::open("benchmark/build/product_discount.json").unwrap());
        reader.read_to_end(&mut input).expect("Should not fail");

        let function_run_result = run(
            Path::new("benchmark/build/size_function.wasm").to_path_buf(),
            input,
        )
        .unwrap();

        assert_eq!(
            function_run_result.size,
            Path::new("benchmark/build/size_function.wasm")
                .metadata()
                .unwrap()
                .len()
                / 1024
        );
    }
}
