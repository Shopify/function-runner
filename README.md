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

## Development

Building requires a rust toolchain of `1.66.0` to `1.67.0`. `cargo install --path . --locked` will build
and add the `function-runner` command to your path.

### Commands

- `cargo install --path . --locked` : Build and install the `function-runner` command.
- `function-runner` : Execute a Function.
