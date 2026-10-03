# function-runner

[About this repo](#about-this-repo) | [Usage](#usage) | [Development](#development)

## About this repo

**Introduction:**

This is a simple CLI (`function-runner`) which allows you to run Wasm
Functions intended for the Shopify Functions infrastructure. Functions will run using
the provided JSON input file and their output will be printed as JSON
upon completion.

By default, the Function is expected to be named `function.wasm` in the
current directory. This may be overriden using the `-f` option.

Example: `function-runner -f '../my-function-name.wasm' -i '../my-input.json'`

## Usage

If you wish to use `function-runner` without compiling it, the [Releases](https://github.com/Shopify/function-runner/releases) page
contains binaries that can be run on your computer.

To see the list of possible commands and arguments, run `function-runner --help`.

### Batch mode

Use `--batch` to run a Function against many inputs in one process. The
Function is loaded and compiled once, so each input costs only the run itself.

The input is [JSON Lines](https://jsonlines.org/): one JSON input per line,
from `--input` or stdin. Blank lines are skipped.

```sh
function-runner -f function.wasm --batch -i inputs.jsonl > results.jsonl
```

For each input, batch mode writes one JSON record on one line to stdout.
`line` is the 1-based line number of the input, so you can match each record
to its input.

- An input that ran:
  `{"line":1,"success":true,"instructions":5069,"memory_usage":1088,"logs":"","output":{...}}`.
  `success` is `false` if the Function failed. If the output is not valid
  JSON, `output` is `null` and `output_error` gives the reason.
- An input that could not run, for example invalid JSON:
  `{"line":2,"success":false,"error":"Invalid input JSON: ..."}`. If the
  Function itself cannot run, for example because it imports both WASI and a
  provider that does not allow WASI, each input gets an error record with that
  reason.

Each record is one complete JSON line. If a result cannot be serialized, the
record for that input is an error record, never partial output.

A summary goes to stderr, for example
`Batch complete: 3 inputs processed, 2 successful, 1 failed`.

Batch options:

- `--batch-continue-on-error`: run all inputs even if some fail. Without it,
  the batch stops after the first failed input.
- `--batch-full-output`: write the full result for each input, the same fields
  as `--json` plus `line`. `input` and `output` hold the JSON values as they
  are, so `output` is `null` when the output is not valid JSON, and
  `output_error` gives the reason.

`--json` cannot be used with `--batch`: batch records are already JSON. Use
`--batch-full-output` for the full result.

The exit code is `0` only if every input succeeds. `--schema-path` and
`--query-path` work in batch mode; the schema and query are parsed once.
Profiling is not available in batch mode.

## Library usage

To compute scale factors for many inputs, use
`bluejay_schema_analyzer::BluejaySchemaAnalyzer::with_analyzer`. It parses and
validates the schema and query once, then calls your closure with an `analyze`
function that returns the scale factor for one input:

```rust
use function_runner::bluejay_schema_analyzer::BluejaySchemaAnalyzer;

let scale_factors = BluejaySchemaAnalyzer::with_analyzer(
    &schema,
    Some("schema.graphql"),
    &query,
    Some("input.graphql"),
    |analyze| inputs.iter().map(|input| analyze(input)).collect::<anyhow::Result<Vec<f64>>>(),
)??;
```

The outer `Result` holds schema and query errors. The inner `Result` holds
analysis errors for an input. The `test_with_analyzer_analyzes_many_inputs`
test in `src/bluejay_schema_analyzer.rs` runs this pattern.

## Development

Building requires a rust toolchain of `1.66.0` to `1.67.0`. `cargo install --path . --locked` will build
and add the `function-runner` command to your path.

### Commands

- `cargo install --path . --locked` : Build and install the `function-runner` command.
- `function-runner` : Execute a Function.

## Releasing

1. Create and merge a PR incrementing the version in [Cargo.toml](https://github.com/Shopify/function-runner/blob/main/Cargo.toml#L11) in accordance with [SemVer](https://semver.org/) based on changes from the previous release
1. Create a new release in [Github](https://github.com/Shopify/function-runner/releases/new) with a name like `v3.2.3` where the version matches the Cargo.toml version
    - **:warning: Warning: If you create a draft release, the [GitHub action](https://github.com/Shopify/function-runner/actions/workflows/publish.yml) to generate the binary assets _will not_ be automatically run. You will need to manually run the action.**
