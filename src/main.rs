use std::{
    fs::File,
    io::{stdin, BufReader, Read},
    path::PathBuf,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use function_runner::engine::run;

/// Simple Function runner which takes JSON as a convenience.
#[derive(Parser)]
#[clap(version)]
struct Opts {
    /// Path to wasm/wat Function
    #[clap(short, long, default_value = "function.wasm")]
    function: PathBuf,

    /// Path to json file containing Function input or piped input
    input: Option<PathBuf>,

    /// Log the run result as a JSON object
    #[clap(short, long)]
    json: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let mut input: Box<dyn Read + Sync + Send + 'static> = if let Some(ref input) = opts.input {
        Box::new(BufReader::new(File::open(input).map_err(|e| {
            anyhow!("Couldn't load input {:?}: {}", input, e)
        })?))
    } else {
        Box::new(BufReader::new(stdin()))
    };

    let mut buffer = Vec::new();
    input.read_to_end(&mut buffer)?;
    let _ = serde_json::from_slice::<serde_json::Value>(&buffer)
        .map_err(|e| anyhow!("Invalid input JSON: {}", e))?;

    let function_run_result = run(opts.function, buffer);

    if opts.json {
        println!("{}", function_run_result.as_ref().unwrap().to_json());
    } else {
        println!("{}", function_run_result.as_ref().unwrap());
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use assert_cmd::prelude::*;
    use function_runner::function_run_result::FunctionRunResult;
    use predicates::{prelude::*, str::contains};
    use std::{
        fs::File,
        process::{Command, Stdio},
    };

    #[test]
    fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.arg("--function")
            .arg("benchmark/build/runtime_function.wasm")
            .arg("benchmark/build/volume_discount.json");
        cmd.assert().success();

        Ok(())
    }

    #[test]
    fn invalid_json_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.arg("--function")
            .arg("benchmark/build/runtime_function.wasm")
            .arg("--json")
            .arg("benchmark/build/invalid_volume_discount.json");
        cmd.assert()
            .failure()
            .stderr("Error: Invalid input JSON: EOF while parsing an object at line 2 column 0\n");

        Ok(())
    }

    #[test]
    fn run_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open("benchmark/build/volume_discount.json")?;
        let mut cmd = Command::cargo_bin("function-runner")?;
        let output = cmd
            .arg("--function")
            .arg("benchmark/build/runtime_function.wasm")
            .arg("--json")
            .stdin(Stdio::from(file))
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process")
            .wait_with_output()
            .expect("Failed waiting for output");

        let _ = serde_json::from_slice::<FunctionRunResult>(&output.stdout)
            .expect("This shouldn't fail");

        Ok(())
    }

    #[test]
    fn run_json() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.arg("--function")
            .arg("benchmark/build/runtime_function.wasm")
            .arg("--json")
            .arg("benchmark/build/volume_discount.json");
        cmd.assert().success();
        let output = cmd.output().expect("Wasn't able to get output");
        let _ = serde_json::from_slice::<FunctionRunResult>(&output.stdout)
            .expect("This shouldn't fail");

        Ok(())
    }

    #[test]
    fn wasm_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.arg("--function")
            .arg("test/file/doesnt/exist")
            .arg("benchmark/build/volume_discount.json");
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Couldn't load the Function"));

        Ok(())
    }

    #[test]
    fn input_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.arg("--function")
            .arg("benchmark/build/runtime_function.wasm")
            .arg("test/file/doesnt/exist.json");
        cmd.assert()
            .failure()
            .stderr("Error: Couldn\'t load input \"test/file/doesnt/exist.json\": No such file or directory (os error 2)\n");

        Ok(())
    }

    #[test]
    fn incorrect_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.arg("--function")
            .arg("benchmark/build/runtime_function.wasm")
            .arg("benchmark/build/product_discount.json");
        cmd.assert()
            .success()
            .stdout(contains("missing field `discountNode`"))
            .stdout(contains("Invalid Output"))
            .stdout(contains("JSON Error"))
            .stderr(contains(""));

        Ok(())
    }
}
