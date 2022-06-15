#!/bin/bash

set -x
set -e

source ./build.sh
for filename in ./build/*.wasm; do
  cargo run --release --package "function-runner" -- "./build/hello_world.json" -f "$filename"
done