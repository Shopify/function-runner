use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    io::Read,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use wasi_common::WasiCtx;
use wasmtime::{Config, Engine, Linker, Module, Store};

use crate::function_run_result::{
    FunctionOutput::{self, InvalidJsonOutput, JsonOutput},
    FunctionRunResult, InvalidOutput,
};
use crate::metering::InstrCounter;

pub fn run_with_count(
    name: &str,
    mut function: impl Read,
    input: impl Read,
) -> Result<(FunctionRunResult, HashMap<String, u64>)> {
    let mut module: Vec<u8> = Vec::new();
    function.read_to_end(&mut module)?;

    let instr_counter = Arc::new(Mutex::new(InstrCounter::new()));
    let module = { instr_counter.lock().unwrap().counterize(&module)? };

    let input: serde_json::Value =
        serde_json::from_reader(input).map_err(|e| anyhow!("Couldn't load input: {}", e))?;

    let input = serde_json::to_vec(&input)?;
    let instr_counter_copy = instr_counter.clone();
    let mut result = run_module(module, input, move |linker| {
        // IDK, Rust is being weird without this
        let instr_counter_copy = instr_counter_copy.clone();
        linker.func_wrap("instruction_counter", "inc", move |param: i32| {
            instr_counter_copy.lock().unwrap().inc(param);
        })?;
        Ok(())
    })?;

    result.name = name.to_string();

    let map = instr_counter.lock().unwrap();
    Ok((result, map.total_count().collect()))
}

pub fn run(name: &str, mut function: impl Read, input: impl Read) -> Result<FunctionRunResult> {
    let mut module: Vec<u8> = Vec::new();
    function.read_to_end(&mut module)?;
    let input: serde_json::Value =
        serde_json::from_reader(input).map_err(|e| anyhow!("Couldn't load input: {}", e))?;

    let input = serde_json::to_vec(&input)?;
    let mut result = run_module(module, input, |_| Ok(()))?;
    result.name = name.to_string();
    Ok(result)
}

pub fn run_module<F>(
    module_binary: Vec<u8>,
    input: Vec<u8>,
    mangle_func: F,
) -> Result<FunctionRunResult>
where
    F: Fn(&mut Linker<WasiCtx>) -> Result<()>,
{
    let engine = if cfg!(target_arch = "x86_64") {
        // enabling this on non-x86 architectures currently causes an error (as of wasmtime 0.37.0)
        Engine::new(Config::new().debug_info(true))?
    } else {
        Engine::default()
    };

    let module = Module::from_binary(&engine, &module_binary)
        .map_err(|e| anyhow!("Couldn't compile the Function: {}", e))?;

    let input_stream = wasi_common::pipe::ReadPipe::new(std::io::Cursor::new(input));
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

        mangle_func(&mut linker)?;

        linker.module(&mut store, "Function", &module)?;

        let instance = linker.instantiate(&mut store, &module)?;

        let start = Instant::now();

        let module_result = instance
            .get_typed_func::<(), (), _>(&mut store, "_start")?
            .call(&mut store, ());

        runtime = start.elapsed();

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

    let size: u64 = (module_binary.len() / 1024) as u64;

    let function_run_result = FunctionRunResult::new(
        "<anonymous>".to_string(),
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
    use std::path::Path;

    const LINEAR_MEMORY_USAGE: u64 = 159 * 64;

    #[test]
    fn test_linear_memory_usage_in_kb() {
        let function_run_result = run(
            "lol",
            std::fs::File::open(
                Path::new("benchmark/build/linear_memory_function.wasm").to_path_buf(),
            )
            .unwrap(),
            std::fs::File::open(Path::new("benchmark/build/product_discount.json").to_path_buf())
                .unwrap(),
        )
        .unwrap();

        assert_eq!(function_run_result.memory_usage, LINEAR_MEMORY_USAGE);
    }

    #[test]
    fn test_file_size_in_kb() {
        let function_run_result = run(
            "lol",
            std::fs::File::open(Path::new("benchmark/build/size_function.wasm").to_path_buf())
                .unwrap(),
            std::fs::File::open(Path::new("benchmark/build/product_discount.json").to_path_buf())
                .unwrap(),
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
