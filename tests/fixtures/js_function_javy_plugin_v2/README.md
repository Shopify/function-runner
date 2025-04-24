#### Compile

Use this command to recompile the `js_function_javy_plugin_v2.wasm`

```
javy build -C wit=index.wit -C wit-world=index-world -C dynamic -C plugin='../../../providers/shopify_functions_javy_v2.wasm' -o './js_function_javy_plugin_v2.wasm' './functions.js'
```
