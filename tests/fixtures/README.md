# Fiture Functions

Example Functions used as test fixtures.

To recompile rust examples, from the project root:
```
rustup target add wasm32-wasi
cargo wasi build --release -p exit_code -p exports -p log_truncation_function
cp target/wasm32-wasi/release/{exit_code.wasm,exports.wasm,log_truncation_function.wasm} tests/fixtures/build
```
