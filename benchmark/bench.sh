#!/bin/bash

set -x
set -e

source ./build.sh

cargo run --release --package "function-runner" -- "./build/volume_discount.json" -f "./build/runtime_function.wasm"

cargo run --release --package "function-runner" -- "./build/product_discount.json" -f "./build/linear_memory_function.wasm"

cargo run --release --package "function-runner" -- "./build/product_discount.json" -f "./build/size_function.wasm"

cargo run --release --package "function-runner" -- "./build/volume_discount.json" -f "./build/stack_memory_function.wasm"