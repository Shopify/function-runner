use anyhow::{anyhow, Result};
use clap::error;
use rust_embed::RustEmbed;
use wasmtime::AsContextMut;
use wasmtime::component::*;
use wasmtime_wasi::preview2::command::sync::add_to_linker;
use wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView};
use std::{collections::HashSet, io::Cursor, path::PathBuf};
use wasi_common::{I32Exit};
//use wasmtime::{AsContextMut, Config, Engine, Module, Store, Caller, component::Component};
use wasmtime::{Store, Caller, Module, Engine, Config};
use crate::local_storage::runner::local_storage::sql_ops;

use crate::{
    function_run_result::{
        FunctionOutput::{self, InvalidJsonOutput, JsonOutput},
        FunctionRunResult, InvalidOutput,
    },
    logs::LogStream,
    local_storage::*,
};

#[derive(Clone)]
pub struct ProfileOpts {
    pub interval: u32,
    pub out: PathBuf,
}

#[derive(RustEmbed)]
#[folder = "providers/"]
struct StandardProviders;

pub struct Ctx {
    table: Table,
    sql: SQLStorage,
    wasi: WasiCtx,
}

impl Ctx {
    fn new(sql: SQLStorage, wasi: WasiCtx) -> Self {
        let table = Table::new();

        Self { table, sql, wasi }
    }
}

impl WasiView for Ctx {
    fn table(&self) -> &Table {
        &self.table
    }

    fn table_mut(&mut self) -> &mut Table {
        &mut self.table
    }

    fn ctx(&self) -> &WasiCtx {
        &self.wasi
    }

    fn ctx_mut(&mut self) -> &mut WasiCtx {
        &mut self.wasi
    }
}

// fn import_modules(
//     module: &Component,
//     engine: &Engine,
//     linker: &mut Linker<Ctx>,
//     mut store: &mut Store<Ctx>,
// ) {
//     let imported_modules: HashSet<String> =
//         module.imports().map(|i| i.module().to_string()).collect();
//     imported_modules.iter().for_each(|imported_module| {
//         let imported_module_bytes = StandardProviders::get(&format!("{imported_module}.wasm"));

//         if let Some(bytes) = imported_module_bytes {
//             let imported_module = Component::from_binary(engine, &bytes.data)
//                 .unwrap_or_else(|_| panic!("Failed to load module {imported_module}"));
//             let imported_module_instance = linker
//                 .instantiate(&mut store, &imported_module)
//                 .expect("Failed to instantiate imported instance");
//             // linker
//             //     .instance(
//             //         &mut store,
//             //         "javy_quickjs_provider_v1",
//             //         imported_module_instance,
//             //     )
//             //     .expect("Failed to import module");
//         }
//     });
// }

pub fn run(
    function_path: PathBuf,
    input: Vec<u8>,
    export: &str,
    profile_opts: Option<&ProfileOpts>,
) -> Result<FunctionRunResult> {
    let mut config = Config::new();
    config.wasm_multi_memory(true)
    .consume_fuel(true)
    .epoch_interruption(true)
    .wasm_component_model(true);

    let engine = Engine::new(
        &config)?;

    let module = Component::from_file(&engine, &function_path)
        .map_err(|e| anyhow!("Couldn't load the Function {:?}: {}", &function_path, e))?;

    // let input_stream = wasi_common::pipe::ReadPipe::new(Cursor::new(input));
    // let output_stream = wasi_common::pipe::WritePipe::new_in_memory();
    // let error_stream = wasi_common::pipe::WritePipe::new(LogStream::default());

    let memory_usage: u64;
    let instructions: u64;
    let mut error_logs: String = String::new();
    let profile_data: Option<String>;

    {
        let sql = SQLStorage::new("demo.sqlite");
        let mut linker = Linker::new(&engine);
        //let wasi = deterministic_wasi_ctx::build_wasi_ctx();
        let mut wasi = WasiCtxBuilder::new()
        .inherit_stdio()
        .build();
        // wasi.set_stdin(Box::new(input_stream));
        // wasi.set_stdout(Box::new(output_stream.clone()));
        // wasi.set_stderr(Box::new(error_stream.clone()));
        let mut store = Store::new(&engine, Ctx::new(sql, wasi));
       //store.add_fuel(u64::MAX)?;
        store.set_epoch_deadline(1);

        //import_modules(&module, &engine, &mut linker, &mut store);
        sql_ops::add_to_linker(&mut linker, |ctx: &mut Ctx| &mut ctx.sql)?;
        add_to_linker(&mut linker)?;


        //linker.module(&mut store, "Function", &module)?;
        let (_, instance) = Sql::instantiate(&mut store, &module, &linker)?;
        //let instance = linker.instantiate(&mut store, &module)?;

        let func = instance.get_typed_func::<(), ()>(store.as_context_mut(), export)?;

        //let module_result;
        // (module_result, profile_data) = if let Some(profile_opts) = profile_opts {
        //     let (result, profile_data) = wasmprof::ProfilerBuilder::new(&mut store)
        //         .frequency(profile_opts.interval)
        //         .weight_unit(wasmprof::WeightUnit::Fuel)
        //         .profile(|store| func.call(store.as_context_mut(), ()));

        //     (
        //         result,
        //         Some(profile_data.into_collapsed_stacks().to_string()),
        //     )
        // } else {
            let module_result = func.call(store.as_context_mut(), ());
        //};

        // modules may exit with a specific exit code, an exit code of 0 is considered success but is reported as
        // a GuestFault by wasmtime, so we need to map it to a success result. Any other exit code is considered
        // a failure.
        let module_result =
            module_result.or_else(|error| match error.downcast_ref::<wasi_common::I32Exit>() {
                Some(I32Exit(0)) => Ok(()),
                Some(I32Exit(code)) => Err(anyhow!("module exited with code: {}", code)),
                None => Err(error),
            });

        // This is a hack to get the memory usage. Wasmtime requires a mutable borrow to a store for caching.
        // We need this mutable borrow to fall out of scope so that we can measure memory usage.
        // https://docs.rs/wasmtime/0.37.0/wasmtime/struct.Instance.html#why-does-get_export-take-a-mutable-context
        // let memory_names: Vec<String> = instance
        //     .exports(&mut store)
        //     .filter(|export| export.clone().into_memory().is_some())
        //     .map(|export| export.name().to_string())
        //     .collect();

        // memory_usage = memory_names
        //     .iter()
        //     .map(|name| {
        //         let memory = instance.get_memory(&mut store, name).unwrap();
        //         memory.data_size(&store) as u64
        //     })
        //     .sum::<u64>()
        //     / 1024;
        memory_usage = 0;
        instructions = 0;
        //store.fuel_consumed().unwrap_or_default();

        match module_result {
            Ok(_) => {}
            Err(e) => {
                error_logs = e.to_string();
            }
        }
    };

    let error_stream = wasi_common::pipe::WritePipe::new(LogStream::default());

    let mut logs = error_stream
        .try_into_inner()
        .expect("Log stream reference still exists");

    logs.append(error_logs.as_bytes())
        .expect("Couldn't append error logs");

    let output_stream = wasi_common::pipe::WritePipe::new_in_memory();

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
        size,
        memory_usage,
        instructions,
        logs.to_string(),
        output,
        None,
    );

    Ok(function_run_result)
}

#[cfg(test)]
mod tests {
    use colored::Colorize;

    use super::*;
    use std::path::Path;

    const LINEAR_MEMORY_USAGE: u64 = 159 * 64;
    const DEFAULT_EXPORT: &str = "_start";

    #[test]
    fn test_js_function() {
        let input = include_bytes!("../benchmark/build/js_function_input.json").to_vec();
        let function_run_result = run(
            Path::new("benchmark/build/js_function.wasm").to_path_buf(),
            input,
            DEFAULT_EXPORT,
            None,
        );

        assert!(function_run_result.is_ok());
    }

    #[test]
    fn test_exit_code_zero() {
        let input = include_bytes!("../benchmark/build/product_discount.json").to_vec();
        let function_run_result = run(
            Path::new("benchmark/build/exit_code_function_zero.wasm").to_path_buf(),
            input,
            DEFAULT_EXPORT,
            None,
        )
        .unwrap();

        assert_eq!(function_run_result.logs, "");
    }

    #[test]
    fn test_exit_code_one() {
        let input = include_bytes!("../benchmark/build/product_discount.json").to_vec();
        let function_run_result = run(
            Path::new("benchmark/build/exit_code_function_one.wasm").to_path_buf(),
            input,
            DEFAULT_EXPORT,
            None,
        )
        .unwrap();

        assert_eq!(function_run_result.logs, "module exited with code: 1");
    }

    #[test]
    fn test_linear_memory_usage_in_kb() {
        let input = include_bytes!("../benchmark/build/product_discount.json").to_vec();
        let function_run_result = run(
            Path::new("benchmark/build/linear_memory_function.wasm").to_path_buf(),
            input,
            DEFAULT_EXPORT,
            None,
        )
        .unwrap();

        assert_eq!(function_run_result.memory_usage, LINEAR_MEMORY_USAGE);
    }

    #[test]
    fn test_logs_truncation() {
        let input = "{}".as_bytes().to_vec();
        let function_run_result = run(
            Path::new("benchmark/build/log_truncation_function.wasm").to_path_buf(),
            input,
            DEFAULT_EXPORT,
            None,
        )
        .unwrap();

        assert!(function_run_result
            .logs
            .contains(&"...[TRUNCATED]".red().to_string()));
    }

    #[test]
    fn test_file_size_in_kb() {
        let input = include_bytes!("../benchmark/build/product_discount.json").to_vec();
        let function_run_result = run(
            Path::new("benchmark/build/size_function.wasm").to_path_buf(),
            input,
            DEFAULT_EXPORT,
            None,
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
