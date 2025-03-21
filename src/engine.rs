use anyhow::{anyhow, Result};
use rust_embed::RustEmbed;
use std::string::String;
use std::{collections::HashSet, io::Cursor, path::PathBuf};
use wasi_common::{I32Exit, WasiCtx};
use wasmtime::{AsContextMut, Config, Engine, Linker, Module, ResourceLimiter, Store};

use crate::function_run_result::{
    FunctionOutput::{self, InvalidJsonOutput, JsonOutput},
    FunctionRunResult, InvalidOutput,
};

#[derive(Clone)]
pub struct ProfileOpts {
    pub interval: u32,
    pub out: PathBuf,
}

#[derive(RustEmbed)]
#[folder = "providers/"]
struct StandardProviders;

fn import_modules<T>(
    module: &Module,
    engine: &Engine,
    linker: &mut Linker<T>,
    mut store: &mut Store<T>,
) {
    let imported_modules: HashSet<String> =
        module.imports().map(|i| i.module().to_string()).collect();
    imported_modules.iter().for_each(|module_name| {
        let imported_module_bytes = StandardProviders::get(&format!("{module_name}.wasm"));

        if let Some(bytes) = imported_module_bytes {
            let imported_module = Module::from_binary(engine, &bytes.data)
                .unwrap_or_else(|_| panic!("Failed to load module {module_name}"));

            let imported_module_instance = linker
                .instantiate(&mut store, &imported_module)
                .expect("Failed to instantiate imported instance");
            linker
                .instance(&mut store, module_name, imported_module_instance)
                .expect("Failed to import module");
        }
    });
}

#[derive(Default)]
pub struct FunctionRunParams<'a> {
    pub function_path: PathBuf,
    pub input: Vec<u8>,
    pub export: &'a str,
    pub profile_opts: Option<&'a ProfileOpts>,
    pub scale_factor: f64,
}

const STARTING_FUEL: u64 = u64::MAX;
const MAXIMUM_MEMORIES: usize = 2; // 1 for the module, 1 for Javy's provider

struct FunctionContext {
    wasi: WasiCtx,
    limiter: MemoryLimiter,
}

impl FunctionContext {
    fn new(wasi: WasiCtx) -> Self {
        Self {
            wasi,
            limiter: Default::default(),
        }
    }

    fn max_memory_bytes(&self) -> usize {
        self.limiter.max_memory_bytes
    }
}

#[derive(Default)]
pub struct MemoryLimiter {
    max_memory_bytes: usize,
}

impl ResourceLimiter for MemoryLimiter {
    /// See [`wasmtime::ResourceLimiter::memory_growing`].
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        self.max_memory_bytes = std::cmp::max(self.max_memory_bytes, desired);
        Ok(true)
    }

    /// See [`wasmtime::ResourceLimiter::table_growing`].
    fn table_growing(
        &mut self,
        _current: usize,
        _desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn memories(&self) -> usize {
        MAXIMUM_MEMORIES
    }
}

pub fn run(params: FunctionRunParams) -> Result<FunctionRunResult> {
    let FunctionRunParams {
        function_path,
        input,
        export,
        profile_opts,
        scale_factor,
    } = params;

    let engine = Engine::new(
        Config::new()
            .wasm_multi_memory(true)
            .wasm_threads(false)
            .consume_fuel(true)
            .epoch_interruption(true),
    )?;
    let module = Module::from_file(&engine, &function_path)
        .map_err(|e| anyhow!("Couldn't load the Function {:?}: {}", &function_path, e))?;

    let input_stream = wasi_common::pipe::ReadPipe::new(Cursor::new(input.clone()));
    let output_stream = wasi_common::pipe::WritePipe::new_in_memory();
    let error_stream = wasi_common::pipe::WritePipe::new_in_memory();

    let memory_usage: u64;
    let instructions: u64;
    let mut error_logs: String = String::new();
    let mut module_result: Result<(), anyhow::Error>;
    let profile_data: Option<String>;

    {
        let mut linker = Linker::new(&engine);
        wasi_common::sync::add_to_linker(&mut linker, |ctx: &mut FunctionContext| &mut ctx.wasi)?;
        let wasi = deterministic_wasi_ctx::build_wasi_ctx();
        wasi.set_stdin(Box::new(input_stream));
        wasi.set_stdout(Box::new(output_stream.clone()));
        wasi.set_stderr(Box::new(error_stream.clone()));
        let function_context = FunctionContext::new(wasi);
        let mut store = Store::new(&engine, function_context);
        store.limiter(|s| &mut s.limiter);
        store.set_fuel(STARTING_FUEL)?;
        store.set_epoch_deadline(1);

        import_modules(&module, &engine, &mut linker, &mut store);

        linker.module(&mut store, "Function", &module)?;
        let instance = linker.instantiate(&mut store, &module)?;

        let func = instance.get_typed_func::<(), ()>(store.as_context_mut(), export)?;

        (module_result, profile_data) = if let Some(profile_opts) = profile_opts {
            let (result, profile_data) = wasmprof::ProfilerBuilder::new(&mut store)
                .frequency(profile_opts.interval)
                .weight_unit(wasmprof::WeightUnit::Fuel)
                .profile(|store| func.call(store.as_context_mut(), ()));

            (
                result,
                Some(profile_data.into_collapsed_stacks().to_string()),
            )
        } else {
            (func.call(store.as_context_mut(), ()), None)
        };

        // modules may exit with a specific exit code, an exit code of 0 is considered success but is reported as
        // a GuestFault by wasmtime, so we need to map it to a success result. Any other exit code is considered
        // a failure.
        module_result =
            module_result.or_else(|error| match error.downcast_ref::<wasi_common::I32Exit>() {
                Some(I32Exit(0)) => Ok(()),
                Some(I32Exit(code)) => Err(anyhow!("module exited with code: {}", code)),
                None => Err(error),
            });

        memory_usage = store.data().max_memory_bytes() as u64 / 1024;
        instructions = STARTING_FUEL.saturating_sub(store.get_fuel().unwrap_or_default());

        match module_result {
            Ok(_) => {}
            Err(ref e) => {
                error_logs = e.to_string();
            }
        }
    };

    let mut logs = error_stream
        .try_into_inner()
        .expect("Log stream reference still exists")
        .into_inner();

    logs.extend_from_slice(error_logs.as_bytes());

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

    let parsed_input =
        String::from_utf8(input).map_err(|e| anyhow!("Couldn't parse input: {}", e))?;

    let function_run_input = serde_json::from_str(&parsed_input)?;

    let function_run_result = FunctionRunResult {
        name: name.to_string(),
        size,
        memory_usage,
        instructions,
        logs: String::from_utf8_lossy(&logs).into(),
        input: function_run_input,
        output,
        profile: profile_data,
        scale_factor,
        success: module_result.is_ok(),
    };

    Ok(function_run_result)
}

#[cfg(test)]
mod tests {
    use colored::Colorize;
    use serde_json::json;

    use super::*;
    use std::path::Path;

    const DEFAULT_EXPORT: &str = "_start";

    #[test]
    fn test_js_function() {
        let input = include_bytes!("../tests/fixtures/input/js_function_input.json").to_vec();
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_function.wasm").to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            ..Default::default()
        });

        assert!(function_run_result.is_ok());
        assert_eq!(function_run_result.unwrap().memory_usage, 1280);
    }

    #[test]
    fn test_js_v2_function() {
        let input = include_bytes!("../tests/fixtures/input/js_function_input.json").to_vec();
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_function_v2.wasm").to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            ..Default::default()
        });

        assert!(function_run_result.is_ok());
        assert_eq!(function_run_result.unwrap().memory_usage, 1344);
    }

    #[test]
    fn test_js_v3_function() {
        let input = include_bytes!("../tests/fixtures/input/js_function_input.json").to_vec();
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_function_v3.wasm").to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            ..Default::default()
        });

        assert!(function_run_result.is_ok());
        assert_eq!(function_run_result.unwrap().memory_usage, 1344);
    }

    #[test]
    fn test_js_functions_javy_v1() {
        let input = include_bytes!("../tests/fixtures/input/js_function_input.json").to_vec();
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_functions_javy_v1.wasm")
                .to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            ..Default::default()
        });

        assert!(function_run_result.is_ok());
        assert_eq!(function_run_result.unwrap().memory_usage, 1344);
    }

    #[test]
    fn test_exit_code_zero() {
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/exit_code.wasm").to_path_buf(),
            input: json!({ "code": 0 }).to_string().into(),
            export: DEFAULT_EXPORT,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(function_run_result.logs, "");
    }

    #[test]
    fn test_exit_code_one() {
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/exit_code.wasm").to_path_buf(),
            input: json!({ "code": 1 }).to_string().into(),
            export: DEFAULT_EXPORT,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(function_run_result.logs, "module exited with code: 1");
    }

    #[test]
    fn test_linear_memory_usage_in_kb() {
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/linear_memory.wasm").to_path_buf(),
            input: "{}".as_bytes().to_vec(),
            export: DEFAULT_EXPORT,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(function_run_result.memory_usage, 12800); // 200 * 64KiB pages
    }

    #[test]
    fn test_logs_truncation() {
        let input = "{}".as_bytes().to_vec();
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/log_truncation_function.wasm")
                .to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            ..Default::default()
        })
        .unwrap();

        assert!(
            function_run_result.to_string().contains(
                &"Logs would be truncated in production, length 6000 > 1000 limit"
                    .red()
                    .to_string()
            ),
            "Expected logs to be truncated, but were: {function_run_result}"
        );
    }

    #[test]
    fn test_file_size_in_kb() {
        let file_path = Path::new("tests/fixtures/build/exit_code.wasm");

        let function_run_result = run(FunctionRunParams {
            function_path: file_path.to_path_buf(),
            input: json!({ "code": 0 }).to_string().into(),
            export: DEFAULT_EXPORT,
            ..Default::default()
        })
        .unwrap();

        assert_eq!(
            function_run_result.size,
            file_path.metadata().unwrap().len() / 1024
        );
    }
}
