#!/bin/bash

set -x
set -e

for d in ./*_function/
do 
  cd "$d"
  CARGO_TARGET_DIR=./target cargo build --profile "benchmark" --target "wasm32-wasi"
  cp -n ./target/wasm32-wasi/benchmark/*.wasm ../build/ || true # avoids the error that -n throws if the file already exists
  cd ..
done

for filename in ./build/*.wasm
do
  wasm-opt -Oz --strip-debug "$filename" -o "$filename"
done

for filename in ./build/*.wat
do
  wat2wasm "$filename" -o "${filename%.*}".wasm
done