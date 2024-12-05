# Fixture Functions

Example Functions used as test fixtures.

## Recompiling

**Prereqs:**
- Cargo WASI: `cargo install cargo-wasi`
- wat2wasm from [WABT](https://github.com/WebAssembly/wabt)


**Rust examples:**
```
cargo wasi build --profile=wasm -p exit_code -p exports -p log_truncation_function &&
  cp target/wasm32-wasi/wasm/{exit_code.wasm,exports.wasm,log_truncation_function.wasm} tests/fixtures/build
```

**JS examples:**
js_functions_javy_v1.wasm:
```
javy build -C dynamic -C plugin=providers/shopify_functions_javy_v1.wasm -o tests/fixtures/build/js_functions_javy_v1.wasm tests/fixtures/js_function/src/functions.js
```

**`*.wat` examples:**
```
find tests/fixtures -maxdepth 1 -type f -name "*.wat" \
  | xargs -I {} sh -c 'name=$(basename {} .wat); wat2wasm {} -o "tests/fixtures/build/$name.wasm"'
```
