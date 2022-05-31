#!/bin/bash

set -x
set -e

source build.sh
cd ../..
cargo run --release --package "script-runner" -- "./benchmark/build/hello_world.json" -s "./benchmark/build/script.wasm"