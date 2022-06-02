#!/bin/bash

set -x
set -e

for d in ./*_function/
do 
  cd "$d"
  CARGO_TARGET_DIR=./target cargo build --release --target "wasm32-wasi"
  echo $d
  cp ./target/wasm32-wasi/release/*.wasm ../build/
  cd ..
done
