use colored::Colorize;
use std::{
    fmt,
    time::{Duration, Instant},
};

const RUNTIME_THRESHOLD: Duration = Duration::from_millis(5);

pub struct FunctionBenchmark {
    pub runtime: Option<Duration>,
    memory_size: Option<u32>,
    stack_size: Option<u32>,
    start: Option<Instant>,
}

impl FunctionBenchmark {
    /// Create a new `FunctionBenchmark`
    pub fn new() -> Self {
        FunctionBenchmark {
            runtime: None,
            memory_size: None,
            stack_size: None,
            start: None,
        }
    }

    /// Start the benchmark
    pub fn start(&mut self) {
        self.start = Some(Instant::now());
    }

    /// Stop the benchmark
    pub fn stop(&mut self) {
        self.runtime = Some(self.start.unwrap().elapsed());
    }
}

impl fmt::Display for FunctionBenchmark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let title = "      Benchmark Results      ".black().on_bright_green();
        write!(f, "{}\n\n", title)?;

        let runtime_display: String;

        if let Some(runtime) = self.runtime {
            if runtime <= RUNTIME_THRESHOLD {
                runtime_display = format!("{:?}", runtime).bright_green().to_string();
            } else {
                runtime_display = format!(
                    "{:?} <- maximum allowed is {:?}",
                    runtime, RUNTIME_THRESHOLD
                )
                .red()
                .to_string();
            }
        } else {
            runtime_display = "N/A".blue().to_string();
        }

        writeln!(f, "Runtime: {}", runtime_display)?;
        writeln!(f, "Memory: {:#?} bytes", self.memory_size)?;
        writeln!(f, "Stack: {:#?} bytes", self.stack_size)?;
        Ok(())
    }
}

impl Default for FunctionBenchmark {
    fn default() -> Self {
        FunctionBenchmark::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::path::{Path, PathBuf};
    use wasmtime::*;
    use wasmtime_wasi::WasiCtxBuilder;

    #[test]
    fn test_benchmark_display_no_stop() {
        let benchmark = FunctionBenchmark::new();
        assert_eq!(format!("{}", benchmark), "\u{1b}[102;30m      Benchmark Results      \u{1b}[0m\n\nRuntime: \u{1b}[34mN/A\u{1b}[0m\nMemory: None bytes\nStack: None bytes\n");
    }

    #[test]
    fn test_benchmark_runtime_allowed() {
        let mut benchmark = FunctionBenchmark::new();
        run_function(
            Path::new("tests/benchmarks/hello_world.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_world.json").to_path_buf(),
            &mut benchmark,
        );

        assert!(benchmark.runtime.unwrap() <= RUNTIME_THRESHOLD);
    }

    #[test]
    fn test_benchmark_runtime_not_allowed() {
        let mut benchmark = FunctionBenchmark::new();
        run_function(
            Path::new("tests/benchmarks/sleeps.wasm").to_path_buf(),
            Path::new("tests/benchmarks/sleeps.json").to_path_buf(),
            &mut benchmark,
        );

        assert!(benchmark.runtime.unwrap() > RUNTIME_THRESHOLD);
    }

    /// Executes a given script and runs the benchmark
    fn run_function(script_path: PathBuf, input_path: PathBuf, benchmark: &mut FunctionBenchmark) {
        let engine = Engine::default();
        let module = Module::from_file(&engine, &script_path)
            .map_err(|e| anyhow!("Couldn't load script {:?}: {}", &script_path, e))
            .unwrap();

        let input: serde_json::Value = serde_json::from_reader(
            std::fs::File::open(&input_path)
                .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &input_path, e))
                .unwrap(),
        )
        .map_err(|e| anyhow!("Couldn't load input {:?}: {}", &input_path, e))
        .unwrap();
        let input = serde_json::to_vec(&input).unwrap();

        let input_stream = wasi_common::pipe::ReadPipe::new(std::io::Cursor::new(input));
        let output_stream = wasi_common::pipe::WritePipe::new_in_memory();
        let error_stream = wasi_common::pipe::WritePipe::new_in_memory();

        {
            // Link WASI and construct the store.
            let mut linker = Linker::new(&engine);
            wasmtime_wasi::add_to_linker(&mut linker, |s| s).unwrap();
            let wasi = WasiCtxBuilder::new()
                .stdin(Box::new(input_stream))
                .stdout(Box::new(output_stream.clone()))
                .stderr(Box::new(error_stream.clone()))
                .inherit_args()
                .unwrap()
                .build();
            let mut store = Store::new(&engine, wasi);

            linker.module(&mut store, "", &module).unwrap();

            benchmark.start();

            // Execute the module
            let result = linker
                .get_default(&mut store, "")
                .unwrap()
                .typed::<(), (), _>(&store)
                .unwrap()
                .call(&mut store, ());

            benchmark.stop();

            match result {
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
        let _logs = std::str::from_utf8(&logs)
            .map_err(|e| anyhow!("Couldn't print Script Logs: {}", e))
            .unwrap();

        let output = output_stream
            .try_into_inner()
            .expect("Output stream reference still exists")
            .into_inner();
        let _output: serde_json::Value = serde_json::from_slice(output.as_slice())
            .map_err(|e| anyhow!("Couldn't decode Script Output: {}", e))
            .unwrap();
    }
}
