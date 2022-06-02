#!/bin/bash

set -x
set -e

for filename in ./build/*.wasm; do
  cargo run --release --package "script-runner" -- "./build/hello_world.json" -s "$filename"
done