# script-runner

[About this repo](#about-this-repo) |  [Commands](#commands-optional) | [How to use this repo](#how-to-use-this-repo)

## About this repo
**Introduction:**

This is a simple CLI (`script-runner`) which allows you to run Wasm
scripts intended for the ScriptsV2 platform. Scripts will run using
the provided JSON input file and their output will be printed as JSON
upon completion.

By default, the script is expected to be named `script.wasm` in the
current directory. This may be overriden using the `-s` option.

Example: `script-runner -s '../my-script-name.wasm' '../my-input.json'`


|                |                                                                                                 |
|----------------|-------------------------------------------------------------------------------------------------|
| Current status | Prototype                                                                                       |
| Owner          | [Scripts Platform](https://vault.shopify.io/pages/7064-Scripts-Platform-Team)                   |
| Help           | [#scripts](https://shopify.slack.com/app_redirect?channel=scripts)                              |


## Commands (optional)

* `cargo install --path .` : Build and install the `script-runner` command.
* `script-runner` : Execute scripts.


## How to use this repo

Building requires a rust toolchain of at least `1.56.0` (older may work). `cargo install --path.` will build
and add the `script-runner` command to your path.

### Usage
```
$ script-runner --help
script-runner 0.1.0

Steven MacLeod <steven.macleod@shopify.com>

Simple script runner which takes JSON as a convenience

USAGE:
    script-runner [OPTIONS] <INPUT>

ARGS:
    <INPUT>    Path to json file containing script input

OPTIONS:
    -h, --help               Print help information
    -s, --script <SCRIPT>    Path to wasm/wat script [default: script.wasm]
    -V, --version            Print version information
```
