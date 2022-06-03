#!/bin/bash

set -x
set -e

for d in ./*_function/
do 
  cd "$d"
  CARGO_TARGET_DIR=./target cargo build --profile "benchmark" --target "wasm32-wasi"
  echo $d
  cp -n ./target/wasm32-wasi/release/*.wasm ../build/ || true # avoids the error that -n throws if the file already exists
  cd ..
done

for filename in ./build/*.wasm; do
  wasm-opt -Oz --strip-debug "$filename" -o "$filename"
done
