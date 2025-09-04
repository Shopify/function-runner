use anyhow::{anyhow, Result};
use std::path::PathBuf;
use std::string::String;
use wasmtime::{AsContextMut, Config, Engine, Linker, Module, ResourceLimiter, Store};
use wasmtime_wasi::preview1::WasiP1Ctx;
use wasmtime_wasi::I32Exit;

use crate::function_run_result::FunctionRunResult;
use crate::io::{IOHandler, OutputAndLogs};
use crate::{BytesContainer, BytesContainerType};

#[derive(Clone)]
pub struct ProfileOpts {
    pub interval: u32,
    pub out: PathBuf,
}

pub fn uses_msgpack_provider(module: &Module) -> bool {
    module.imports().map(|i| i.module()).any(|module| {
        module.starts_with("shopify_function_v")
            || module
                .strip_prefix("shopify_functions_javy_v")
                .is_some_and(|v| v.parse::<usize>().is_ok_and(|v| v >= 2))
    })
}

pub struct FunctionRunParams<'a> {
    pub function_path: PathBuf,
    pub input: BytesContainer,
    pub export: &'a str,
    pub profile_opts: Option<&'a ProfileOpts>,
    pub scale_factor: f64,
    pub module: Module,
    pub engine: Engine,
}

const STARTING_FUEL: u64 = u64::MAX;
const MAXIMUM_MEMORIES: usize = 2; // 1 for the module, 1 for Javy's provider

struct FunctionContext {
    wasi: Option<WasiP1Ctx>,
    limiter: MemoryLimiter,
}

impl FunctionContext {
    fn new(wasi: Option<WasiP1Ctx>) -> Self {
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
        engine,
        module,
    } = params;

    let mut io_handler = IOHandler::new(module, input.clone());

    let mut error_logs: String = String::new();

    let mut linker = Linker::new(&engine);
    let wasi = io_handler.wasi();
    if wasi.is_some() {
        wasmtime_wasi::preview1::add_to_linker_sync(&mut linker, |ctx: &mut FunctionContext| {
            ctx.wasi.as_mut().expect("Should have WASI context")
        })?;
        deterministic_wasi_ctx::replace_scheduling_functions(&mut linker)?;
    }

    let function_context = FunctionContext::new(wasi);
    let mut store = Store::new(&engine, function_context);
    store.limiter(|s| &mut s.limiter);

    io_handler.initialize(&engine, &mut linker, &mut store)?;

    store.set_fuel(STARTING_FUEL)?;
    store.set_epoch_deadline(1);

    let instance = linker.instantiate(&mut store, io_handler.module())?;

    let func = instance.get_typed_func::<(), ()>(store.as_context_mut(), export)?;

    let (mut module_result, profile_data) = if let Some(profile_opts) = profile_opts {
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
    module_result = module_result.or_else(|error| match error.downcast_ref::<I32Exit>() {
        Some(I32Exit(0)) => Ok(()),
        Some(I32Exit(code)) => Err(anyhow!("module exited with code: {}", code)),
        None => Err(error),
    });

    let memory_usage = store.data().max_memory_bytes() as u64 / 1024;
    let instructions = STARTING_FUEL.saturating_sub(store.get_fuel().unwrap_or_default());

    match module_result {
        Ok(_) => {}
        Err(ref e) => {
            error_logs = e.to_string();
        }
    }

    let OutputAndLogs {
        output: raw_output,
        mut logs,
    } = io_handler.finalize(store)?;

    logs.extend_from_slice(error_logs.as_bytes());

    let output_codec = input.codec;
    let output = BytesContainer::new(
        BytesContainerType::Output,
        output_codec,
        raw_output.to_vec(),
    )?;

    let name = function_path.file_name().unwrap().to_str().unwrap();
    let size = function_path.metadata()?.len() / 1024;

    let function_run_result = FunctionRunResult {
        name: name.to_string(),
        size,
        memory_usage,
        instructions,
        logs: String::from_utf8_lossy(&logs).into(),
        input,
        output,
        profile: profile_data,
        scale_factor,
        success: module_result.is_ok(),
    };

    Ok(function_run_result)
}

/// Creates a new Engine with our standard configuration.
/// We use a dedicated function instead of making this the default configuration because:
/// 1. It's more explicit about what configuration we're using
/// 2. It keeps the door open for different configurations in the future without breaking changes
/// 3. It makes it easier to find all places where we create an Engine
pub fn new_engine() -> Result<Engine> {
    let mut config = Config::new();
    config
        .wasm_multi_memory(true)
        .wasm_threads(false)
        .consume_fuel(true)
        .epoch_interruption(true);
    config.cache_config_load_default()?;
    Engine::new(&config)
}

#[cfg(test)]
mod tests {
    use colored::Colorize;
    use serde_json::json;

    use super::*;
    use crate::Codec;
    use anyhow::Result;
    use std::path::Path;

    const DEFAULT_EXPORT: &str = "_start";

    fn json_input(raw: &[u8]) -> Result<BytesContainer> {
        BytesContainer::new(BytesContainerType::Input, Codec::Json, raw.to_vec())
    }

    #[test]
    fn test_js_function() -> Result<()> {
        let engine = new_engine()?;
        let module =
            Module::from_file(&engine, Path::new("tests/fixtures/build/js_function.wasm"))?;
        let input = json_input(include_bytes!(
            "../tests/fixtures/input/js_function_input.json"
        ))?;

        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_function.wasm").to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.memory_usage, 1280);

        Ok(())
    }

    #[test]
    fn test_js_v2_function() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(
            &engine,
            Path::new("tests/fixtures/build/js_function_v2.wasm"),
        )?;
        let input = json_input(include_bytes!(
            "../tests/fixtures/input/js_function_input.json"
        ))?;
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_function_v2.wasm").to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.memory_usage, 1344);
        Ok(())
    }

    #[test]
    fn test_js_v3_function() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(
            &engine,
            Path::new("tests/fixtures/build/js_function_v3.wasm"),
        )?;
        let input = json_input(include_bytes!(
            "../tests/fixtures/input/js_function_input.json"
        ))?;

        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_function_v3.wasm").to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.memory_usage, 1344);
        Ok(())
    }

    #[test]
    fn test_js_functions_javy_v1() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(
            &engine,
            Path::new("tests/fixtures/build/js_functions_javy_v1.wasm"),
        )?;
        let input = json_input(include_bytes!(
            "../tests/fixtures/input/js_function_input.json"
        ))?;

        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/js_functions_javy_v1.wasm")
                .to_path_buf(),
            input,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.memory_usage, 1344);
        Ok(())
    }

    #[test]
    fn test_exit_code_zero() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(&engine, Path::new("tests/fixtures/build/exit_code.wasm"))?;
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/exit_code.wasm").to_path_buf(),
            input: json_input(&serde_json::to_vec(&json!({ "code": 0 }))?)?,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.logs, "");
        Ok(())
    }

    #[test]
    fn test_exit_code_one() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(&engine, Path::new("tests/fixtures/build/exit_code.wasm"))?;
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/exit_code.wasm").to_path_buf(),
            input: json_input(&serde_json::to_vec(&json!({ "code": 1 }))?)?,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.logs, "module exited with code: 1");
        Ok(())
    }

    #[test]
    fn test_linear_memory_usage_in_kb() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(
            &engine,
            Path::new("tests/fixtures/build/linear_memory.wasm"),
        )?;
        let function_run_result = run(FunctionRunParams {
            function_path: Path::new("tests/fixtures/build/linear_memory.wasm").to_path_buf(),
            input: json_input(&serde_json::to_vec(&json!({}))?)?,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(function_run_result.memory_usage, 12800); // 200 * 64KiB pages
        Ok(())
    }

    #[test]
    fn test_logs_truncation() -> Result<()> {
        let engine = new_engine()?;
        let module = Module::from_file(
            &engine,
            Path::new("tests/fixtures/build/log_truncation_function.wasm"),
        )?;
        let function_run_result = run(FunctionRunParams {
            input: json_input("{}".as_bytes())?,
            function_path: Path::new("tests/fixtures/build/log_truncation_function.wasm")
                .to_path_buf(),
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert!(
            function_run_result.to_string().contains(
                &"Logs would be truncated in production, length 6000 > 1000 limit"
                    .red()
                    .to_string()
            ),
            "Expected logs to be truncated, but were: {function_run_result}"
        );
        Ok(())
    }

    #[test]
    fn test_file_size_in_kb() -> Result<()> {
        let file_path = Path::new("tests/fixtures/build/exit_code.wasm");
        let engine = new_engine()?;
        let module = Module::from_file(&engine, file_path)?;
        let function_run_result = run(FunctionRunParams {
            function_path: file_path.to_path_buf(),
            input: json_input(&serde_json::to_vec(&json!({ "code": 0 }))?)?,
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        })?;

        assert_eq!(
            function_run_result.size,
            file_path.metadata().unwrap().len() / 1024
        );
        Ok(())
    }

    #[test]
    fn test_wasm_api_v1_function() -> Result<()> {
        let trampolined_path = assert_fs::NamedTempFile::new("wasm_api_v1.trampolined.wasm")?;
        test_utils::process_with_v1_trampoline(
            "tests/fixtures/build/wasm_api_v1.wasm",
            &trampolined_path,
        )?;
        let engine = new_engine()?;
        let module = Module::from_file(&engine, &trampolined_path)?;
        let expected_input_value = json!({"foo": "echo", "bar": "test"});
        let input = serde_json::to_vec(&expected_input_value).unwrap();
        let input_bytes = BytesContainer::new(BytesContainerType::Input, Codec::Json, input);
        let function_run_result = run(FunctionRunParams {
            function_path: trampolined_path.to_path_buf(),
            input: input_bytes.unwrap(),
            export: DEFAULT_EXPORT,
            module,
            engine,
            scale_factor: 1.0,
            profile_opts: None,
        });

        assert!(function_run_result.is_ok());
        let result = function_run_result.unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&result.input.raw).unwrap(),
            expected_input_value
        );
        Ok(())
    }
}
