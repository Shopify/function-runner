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

Example: `function-runner -f '../my-function-name.wasm' '../my-input.json'`

## Usage

If you wish to use `function-runner` without compiling it, the [Releases](https://github.com/Shopify/function-runner/releases) page
contains binaries that can be run on your computer.

To see the list of possible commands and arguments, run `function-runner --help`.

### Debugging

Wasm files with DWARF symbols can be debugged using `lldb` on x86 architectures (not Apple Silicon). To do this, install `lldb` and run:

```
lldb -- function-runner -f '../my-function-name.wasm' '../my-input.json'
```

## Development

Building requires a rust toolchain of at least `1.56.0` (older may work). `cargo install --path .` will build
and add the `function-runner` command to your path.

### Commands

- `cargo install --path .` : Build and install the `function-runner` command.
- `function-runner` : Execute a Function.
