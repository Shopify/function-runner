# function-runner

[About this repo](#about-this-repo) |  [Commands](#commands-optional) | [How to use this repo](#how-to-use-this-repo)

## About this repo
**Introduction:**

This is a simple CLI (`function-runner`) which allows you to run Wasm
Functions intended for the Shopify Functions infrastructure. Functions will run using
the provided JSON input file and their output will be printed as JSON
upon completion.

By default, the Function is expected to be named `function.wasm` in the
current directory. This may be overriden using the `-f` option.

Example: `function-runner -f '../my-function-name.wasm' '../my-input.json'`

## Commands (optional)

* `cargo install --path .` : Build and install the `function-runner` command.
* `function-runner` : Execute Functions.

## How to use this repo

Building requires a rust toolchain of at least `1.56.0` (older may work). `cargo install --path .` will build
and add the `function-runner` command to your path.

### Usage

```
$ function-runner --help
function-runner 0.2.3
Simple function runner which takes JSON as a convenience

USAGE:
    function-runner [OPTIONS] <INPUT>

ARGS:
    <INPUT>    Path to json file containing function input

OPTIONS:
    -h, --help               Print help information
    -j, --json               Log the run result as a JSON object
    -f, --function <FUNCTION>    Path to wasm/wat function [default: function.wasm]
    -V, --version            Print version information
```
