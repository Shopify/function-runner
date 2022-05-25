use colored::Colorize;
use std::{fmt, time::Duration};

const RUNTIME_THRESHOLD: Duration = Duration::from_millis(5);

pub struct FunctionBenchmark {
    pub runtime: Duration,
}

impl FunctionBenchmark {
    pub fn new(runtime: Duration) -> Self {
        FunctionBenchmark { runtime }
    }
}

impl fmt::Display for FunctionBenchmark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let title = "      Benchmark Results      ".black().on_bright_green();
        write!(f, "{}\n\n", title)?;

        let runtime_display: String = if self.runtime <= RUNTIME_THRESHOLD {
            format!("{:?}", self.runtime).bright_green().to_string()
        } else {
            format!(
                "{:?} <- maximum allowed is {:?}",
                self.runtime, RUNTIME_THRESHOLD
            )
            .red()
            .to_string()
        };

        writeln!(f, "Runtime: {}", runtime_display)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::anyhow;
    use std::{
        path::{Path, PathBuf},
        time::Instant,
    };
    use wasmtime::*;
    use wasmtime_wasi::WasiCtxBuilder;

    #[test]
    fn test_benchmark_runtime_allowed() {
        let benchmark = run_function(
            Path::new("tests/benchmarks/hello_world.wasm").to_path_buf(),
            Path::new("tests/benchmarks/hello_world.json").to_path_buf(),
        );

        assert!(benchmark.runtime <= RUNTIME_THRESHOLD);
    }

    #[test]
    fn test_benchmark_runtime_not_allowed() {
        let benchmark = run_function(
            Path::new("tests/benchmarks/sleeps.wasm").to_path_buf(),
            Path::new("tests/benchmarks/sleeps.json").to_path_buf(),
        );

        assert!(benchmark.runtime > RUNTIME_THRESHOLD);
    }

    /// Executes a given script and runs the benchmark
    fn run_function(script_path: PathBuf, input_path: PathBuf) -> FunctionBenchmark {
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

        let benchmark;
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

            let start = Instant::now();

            // Execute the module
            let result = linker
                .get_default(&mut store, "")
                .unwrap()
                .typed::<(), (), _>(&store)
                .unwrap()
                .call(&mut store, ());

            let elapsed = start.elapsed();

            benchmark = FunctionBenchmark::new(elapsed);

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

        benchmark
    }
}
