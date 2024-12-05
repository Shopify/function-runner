fn main() {
    println!("cargo:rerun-if-changed=providers/javy_quickjs_provider_v1.wasm");
    println!("cargo:rerun-if-changed=providers/javy_quickjs_provider_v2.wasm");
    println!("cargo:rerun-if-changed=providers/shopify_functions_javy_v1.wasm");
}
