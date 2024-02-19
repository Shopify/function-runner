#[cfg(test)]
mod tests {

    use assert_cmd::prelude::*;
    use assert_fs::prelude::*;
    use function_runner::function_run_result::FunctionRunResult;
    use predicates::prelude::*;
    use predicates::{prelude::predicate, str::contains};
    use std::{
        fs::File,
        process::{Command, Stdio},
    };

    #[test]
    fn run() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "tests/fixtures/build/runtime_function.wasm"])
            .args(["--input", "tests/fixtures/build/volume_discount.json"]);
        cmd.assert().success();

        Ok(())
    }

    #[test]
    fn invalid_json_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "tests/fixtures/build/runtime_function.wasm"])
            .arg("--json")
            .args(["--input", "tests/fixtures/build/invalid_json.json"]);
        cmd.assert()
            .failure()
            .stderr("Error: Invalid input JSON: EOF while parsing an object at line 2 column 0\n");

        Ok(())
    }

    #[test]
    fn run_stdin() -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open("tests/fixtures/build/volume_discount.json")?;
        let mut cmd = Command::cargo_bin("function-runner")?;
        let output = cmd
            .args(["--function", "tests/fixtures/build/runtime_function.wasm"])
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
    fn run_no_opts() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;
        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn child process")
            .wait_with_output()
            .expect("Failed waiting for output");

        let actual = String::from_utf8(output.stderr).unwrap();
        let predicate = predicate::str::contains(
            "Simple Function runner which takes JSON as a convenience\n\nUsage: function-runner",
        )
        .count(1);

        assert!(predicate.eval(&actual));

        Ok(())
    }

    #[test]
    #[ignore = "This test hangs on CI but runs locally, is_terminal is likely returning false in CI"]
    fn run_function_no_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "tests/fixtures/build/runtime_function.wasm"]);
        cmd.assert()
            .failure()
            .stderr("Error: You must provide input via the --input flag or piped via stdin.\n");

        Ok(())
    }

    #[test]
    fn run_json() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "tests/fixtures/build/runtime_function.wasm"])
            .arg("--json")
            .args(["--input", "tests/fixtures/build/volume_discount.json"]);
        cmd.assert().success();
        let output = cmd.output().expect("Wasn't able to get output");
        let _ = serde_json::from_slice::<FunctionRunResult>(&output.stdout)
            .expect("This shouldn't fail");

        Ok(())
    }

    #[test]
    fn wasm_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "test/file/doesnt/exist"])
            .args(["--input", "tests/fixtures/build/volume_discount.json"]);
        cmd.assert()
            .failure()
            .stderr("Error: Couldn\'t load the Function \"test/file/doesnt/exist\": failed to read input file: test/file/doesnt/exist\n");

        Ok(())
    }

    #[test]
    fn input_file_doesnt_exist() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "tests/fixtures/build/runtime_function.wasm"])
            .args(["--input", "test/file/doesnt/exist.json"]);
        cmd.assert()
            .failure()
            .stderr("Error: Couldn\'t load input \"test/file/doesnt/exist.json\": No such file or directory (os error 2)\n");

        Ok(())
    }

    #[test]
    fn profile_writes_file() -> Result<(), Box<dyn std::error::Error>> {
        let (mut cmd, temp) = profile_base_cmd_in_temp_dir()?;
        cmd.arg("--profile").assert().success();
        temp.child("runtime_function.perf")
            .assert(predicate::path::exists());

        Ok(())
    }

    #[test]
    fn profile_writes_specified_file_name() -> Result<(), Box<dyn std::error::Error>> {
        let (mut cmd, temp) = profile_base_cmd_in_temp_dir()?;
        cmd.args(["--profile-out", "foo.perf"]).assert().success();
        temp.child("foo.perf").assert(predicate::path::exists());

        Ok(())
    }

    #[test]
    fn profile_frequency_triggers_profiling() -> Result<(), Box<dyn std::error::Error>> {
        let (mut cmd, temp) = profile_base_cmd_in_temp_dir()?;
        cmd.args(["--profile-frequency", "80000"])
            .assert()
            .success();
        temp.child("runtime_function.perf")
            .assert(predicate::path::exists());

        Ok(())
    }

    #[test]
    fn incorrect_input() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;

        cmd.args(["--function", "tests/fixtures/build/runtime_function.wasm"])
            .args(["--input", "tests/fixtures/build/product_discount.json"]);
        cmd.assert()
            .success()
            .stdout(contains("missing field `discountNode`"))
            .stdout(contains("Invalid Output"))
            .stdout(contains("JSON Error"))
            .stderr(contains(""));

        Ok(())
    }

    #[test]
    fn exports() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;
        cmd.args(["--function", "tests/fixtures/build/exports.wasm"])
            .args(["--export", "export1"])
            .args(["--input", "tests/fixtures/build/product_discount.json"]);

        cmd.assert().success().stdout(contains("export1"));

        Ok(())
    }

    #[test]
    fn missing_export() -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;
        cmd.args(["--function", "tests/fixtures/build/exports.wasm"])
            .args(["--input", "tests/fixtures/build/product_discount.json"]);

        cmd.assert()
            .failure()
            .stderr(contains(" failed to find function export `_start`"));

        Ok(())
    }

    fn profile_base_cmd_in_temp_dir(
    ) -> Result<(Command, assert_fs::TempDir), Box<dyn std::error::Error>> {
        let mut cmd = Command::cargo_bin("function-runner")?;
        let cwd = std::env::current_dir()?;
        let temp = assert_fs::TempDir::new()?;

        cmd.current_dir(temp.path())
            .arg("--function")
            .arg(cwd.join("tests/fixtures/build/runtime_function.wasm"))
            .arg("--input")
            .arg(cwd.join("tests/fixtures/build/volume_discount.json"));

        Ok((cmd, temp))
    }
}
