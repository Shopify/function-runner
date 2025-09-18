use std::borrow::Cow;

use anyhow::{bail, Result};
use rust_embed::RustEmbed;
use wasmtime::Module;

#[derive(RustEmbed)]
#[folder = "providers/"]
struct StandardProviders;

#[derive(Debug)]
pub(crate) struct Provider {
    pub(crate) bytes: Cow<'static, [u8]>,
    pub(crate) name: String,
}

impl Provider {
    pub(crate) fn is_mem_io_provider(&self) -> bool {
        let javy_plugin_version = self
            .name
            .strip_prefix("shopify_functions_javy_v")
            .map(|s| s.parse::<usize>())
            .and_then(|result| result.ok());
        if javy_plugin_version.is_some_and(|version| version >= 3) {
            return true;
        }

        let functions_provider_version = self
            .name
            .strip_prefix("shopify_function_v")
            .map(|s| s.parse::<usize>())
            .and_then(|result| result.ok());
        if functions_provider_version.is_some_and(|version| version >= 2) {
            return true;
        }

        false
    }
}

#[derive(Debug)]
pub(crate) struct ValidatedModule {
    module: Module,
    std_import: Option<Provider>,
}

impl ValidatedModule {
    pub(crate) fn new(module: Module) -> Result<Self> {
        // Need to track with deterministic order so don't use a hash
        let mut imports = vec![];
        for import in module.imports().map(|i| i.module().to_string()) {
            if !imports.contains(&import) {
                imports.push(import);
            }
        }

        let uses_wasi = imports.contains(&"wasi_snapshot_preview1".to_string());

        let std_import = imports.iter().find_map(|import| {
            StandardProviders::get(&format!("{import}.wasm")).map(|file| Provider {
                bytes: file.data,
                name: import.into(),
            })
        });

        // If there are multiple standard imports or more than zero unknown imports,
        // the module will fail to instantiate because we only link the one
        // standard provider so the other imports will be unsatisfied.

        if let Some(import) = &std_import {
            if import.is_mem_io_provider() && uses_wasi {
                bail!("Invalid Function, cannot use `{}` and import WASI. If using Rust, change the build target to `wasm32-unknown-unknown`.", import.name);
            }
        }

        Ok(ValidatedModule { module, std_import })
    }

    pub(crate) fn inner(&self) -> &Module {
        &self.module
    }

    pub(crate) fn std_import(&self) -> Option<&Provider> {
        self.std_import.as_ref()
    }

    pub(crate) fn uses_mem_io(&self) -> bool {
        self.std_import
            .as_ref()
            .is_some_and(|i| i.is_mem_io_provider())
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use wasmtime::{Engine, Module};

    use crate::validated_module::ValidatedModule;

    #[test]
    fn test_module_with_just_wasi() -> Result<()> {
        let wat = r#"
        (module
          (import "wasi_snapshot_preview1" "fd_read" (func))
        )
        "#;
        let module = Module::new(&Engine::default(), &wat)?;
        ValidatedModule::new(module)?;
        Ok(())
    }

    #[test]
    fn test_module_with_wasi_and_old_provider() -> Result<()> {
        let wat = r#"
        (module
          (import "wasi_snapshot_preview1" "fd_read" (func))
          (import "shopify_function_v1" "shopify_function_input_get" (func))
        )
        "#;
        let module = Module::new(&Engine::default(), &wat)?;
        ValidatedModule::new(module)?;
        Ok(())
    }

    #[test]
    fn test_module_without_wasi_and_with_new_provider() -> Result<()> {
        let wat = r#"
        (module
          (import "shopify_function_v2" "shopify_function_input_get" (func))
        )
        "#;
        let module = Module::new(&Engine::default(), &wat)?;
        ValidatedModule::new(module)?;
        Ok(())
    }

    #[test]
    fn test_module_with_wasi_and_new_provider() -> Result<()> {
        let wat = r#"
        (module
          (import "wasi_snapshot_preview1" "fd_read" (func))
          (import "shopify_function_v2" "shopify_function_input_get" (func))
        )
        "#;
        let module = Module::new(&Engine::default(), &wat)?;
        ValidatedModule::new(module).unwrap_err();
        Ok(())
    }
}
