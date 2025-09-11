(module
    (import "shopify_function_v2" "_shopify_function_input_get" (func (result i64)))
    (import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32)))
    (func $start)
    (export "_start" (func $start))
)