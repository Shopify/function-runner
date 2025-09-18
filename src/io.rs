use anyhow::{anyhow, Result};
use wasmtime::{AsContext, AsContextMut, Engine, Instance, Linker, Module, Store};
use wasmtime_wasi::{
    pipe::{MemoryInputPipe, MemoryOutputPipe},
    preview1::WasiP1Ctx,
    WasiCtxBuilder,
};

use crate::{
    function_run_result::FUNCTION_LOG_LIMIT, validated_module::ValidatedModule, BytesContainer,
};

pub(crate) struct OutputAndLogs {
    pub output: Vec<u8>,
    pub logs: Vec<u8>,
}

struct WasiIO {
    output: MemoryOutputPipe,
    logs: MemoryOutputPipe,
}

enum IOStrategy {
    Wasi(WasiIO),
    Memory(Option<Instance>),
}

pub(crate) struct IOHandler {
    strategy: IOStrategy,
    module: ValidatedModule,
    input: BytesContainer,
}

impl IOHandler {
    pub(crate) fn new(module: ValidatedModule, input: BytesContainer) -> Self {
        Self {
            strategy: if module.uses_mem_io() {
                IOStrategy::Memory(None)
            } else {
                IOStrategy::Wasi(WasiIO {
                    output: MemoryOutputPipe::new(usize::MAX),
                    logs: MemoryOutputPipe::new(usize::MAX),
                })
            },
            module,
            input,
        }
    }

    pub(crate) fn module(&self) -> &Module {
        self.module.inner()
    }

    pub(crate) fn wasi(&self) -> Option<WasiP1Ctx> {
        match &self.strategy {
            IOStrategy::Wasi(WasiIO { output, logs }) => {
                let input_stream = MemoryInputPipe::new(self.input.raw.clone());
                let mut wasi_builder = WasiCtxBuilder::new();
                wasi_builder.stdin(input_stream);
                wasi_builder.stdout(output.clone());
                wasi_builder.stderr(logs.clone());
                deterministic_wasi_ctx::add_determinism_to_wasi_ctx_builder(&mut wasi_builder);
                Some(wasi_builder.build_p1())
            }
            IOStrategy::Memory(_instance) => None,
        }
    }

    pub(crate) fn initialize<T>(
        &mut self,
        engine: &Engine,
        linker: &mut Linker<T>,
        store: &mut Store<T>,
    ) -> Result<()> {
        store.set_epoch_deadline(1); // Need to make sure we don't timeout during initialization.
        store.set_fuel(u64::MAX)?; // Make sure we have fuel for initialization.
        let mem_io_instance = instantiate_imports(&self.module, engine, linker, store);
        if let IOStrategy::Memory(ref mut instance) = self.strategy {
            *instance = mem_io_instance;
        }

        if let Some(instance) = mem_io_instance {
            let input_offset = instance
                .get_typed_func::<i32, i32>(store.as_context_mut(), "initialize")?
                .call(store.as_context_mut(), self.input.raw.len() as _)?;
            instance
                .get_memory(store.as_context_mut(), "memory")
                .ok_or_else(|| anyhow!("Missing memory export named memory"))?
                .write(store.as_context_mut(), input_offset as _, &self.input.raw)?;
        }
        Ok(())
    }

    pub(crate) fn finalize<T>(self, mut store: Store<T>) -> Result<OutputAndLogs> {
        match self.strategy {
            IOStrategy::Memory(instance) => {
                let instance = instance.expect("Should have been defined in initialize");
                store.set_epoch_deadline(1); // Make sure we don't timeout while finalizing.
                let old_fuel = store.get_fuel()?;
                store.set_fuel(u64::MAX)?; // Make sure we don't run out of fuel finalizing.
                let results_offset = instance
                    .get_typed_func::<(), i32>(store.as_context_mut(), "finalize")?
                    .call(store.as_context_mut(), ())?
                    as usize;
                store.set_fuel(old_fuel)?;

                let memory = instance
                    .get_memory(store.as_context_mut(), "memory")
                    .ok_or_else(|| anyhow!("Missing memory export named memory"))?;

                let mut buf = [0; 24];
                memory.read(store.as_context(), results_offset, &mut buf)?;

                let output_offset = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
                let output_len = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as usize;
                let log_offset1 = u32::from_le_bytes(buf[8..12].try_into().unwrap()) as usize;
                let log_len1 = u32::from_le_bytes(buf[12..16].try_into().unwrap()) as usize;
                let log_offset2 = u32::from_le_bytes(buf[16..20].try_into().unwrap()) as usize;
                let log_len2 = u32::from_le_bytes(buf[20..24].try_into().unwrap()) as usize;

                let mut output = vec![0; output_len];
                memory.read(store.as_context(), output_offset, &mut output)?;

                let mut logs = vec![0; log_len1];
                memory.read(store.as_context(), log_offset1, &mut logs)?;

                let mut logs2 = vec![0; log_len2];
                memory.read(store.as_context(), log_offset2, &mut logs2)?;

                logs.append(&mut logs2);

                if logs.len() > FUNCTION_LOG_LIMIT {
                    logs.splice(0..1, b"[TRUNCATED]...".iter().copied());
                }

                Ok(OutputAndLogs { output, logs })
            }
            IOStrategy::Wasi(WasiIO { output, logs }) => {
                // Need to drop store to have only one reference to output and error streams.
                drop(store);

                let output = output
                    .try_into_inner()
                    .expect("Should have only one reference to output stream at this point")
                    .to_vec();
                let logs = logs
                    .try_into_inner()
                    .expect("Should have only one reference to error stream at this point")
                    .to_vec();
                Ok(OutputAndLogs { output, logs })
            }
        }
    }
}

fn instantiate_imports<T>(
    module: &ValidatedModule,
    engine: &Engine,
    linker: &mut Linker<T>,
    mut store: &mut Store<T>,
) -> Option<Instance> {
    let mut mem_io_instance = None;

    if let Some(std_import) = module.std_import() {
        let imported_module = Module::from_binary(engine, &std_import.bytes)
            .unwrap_or_else(|_| panic!("Failed to load module {}", std_import.name));

        let imported_module_instance = linker
            .instantiate(&mut store, &imported_module)
            .expect("Failed to instantiate imported instance");

        if std_import.is_mem_io_provider() {
            mem_io_instance = Some(imported_module_instance);
        }

        linker
            .instance(&mut store, &std_import.name, imported_module_instance)
            .expect("Failed to import module");
    }

    mem_io_instance
}
