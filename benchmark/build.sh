#!/bin/bash

set -x
set -e

mkdir -p build
cargo build --release --target "wasm32-wasi" 
cp ../target/wasm32-wasi/release/*.wasm build/script.wasm 
cd build 
wasm-opt -Oz --strip-debug script.wasm -o script.wasm