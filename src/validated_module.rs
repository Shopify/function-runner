use std::{
    borrow::Cow,
    collections::HashMap,
    sync::{Mutex, PoisonError},
};

use anyhow::{bail, Result};
use rust_embed::RustEmbed;
use wasmtime::{Engine, Module};

#[derive(RustEmbed)]
#[folder = "providers/"]
struct StandardProviders;

static COMPILED_PROVIDERS: Mutex<ProviderCache> = Mutex::new(ProviderCache::new());

struct CompiledProviders {
    engine: Engine,
    modules: HashMap<String, Module>,
}

struct ProviderCache(Option<CompiledProviders>);

impl ProviderCache {
    const fn new() -> Self {
        Self(None)
    }

    fn module(&mut self, engine: &Engine, provider: &Provider) -> Result<Module> {
        let compiled = match &mut self.0 {
            Some(compiled) if Engine::same(&compiled.engine, engine) => compiled,
            cache => cache.insert(CompiledProviders {
                engine: engine.clone(),
                modules: HashMap::new(),
            }),
        };

        if let Some(module) = compiled.modules.get(&provider.name) {
            return Ok(module.clone());
        }

        let module = Module::from_binary(engine, &provider.bytes)?;
        compiled
            .modules
            .insert(provider.name.clone(), module.clone());
        Ok(module)
    }
}

#[derive(Debug)]
pub(crate) struct Provider {
    pub(crate) bytes: Cow<'static, [u8]>,
    pub(crate) name: String,
}

impl Provider {
    pub(crate) fn module(&self, engine: &Engine) -> Result<Module> {
        COMPILED_PROVIDERS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .module(engine, self)
    }

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

    use crate::validated_module::{Provider, ProviderCache, StandardProviders, ValidatedModule};

    fn provider(name: &str) -> Provider {
        Provider {
            bytes: StandardProviders::get(&format!("{name}.wasm"))
                .unwrap()
                .data,
            name: name.into(),
        }
    }

    #[test]
    fn test_provider_cache_reuses_module_for_same_engine() -> Result<()> {
        let engine = Engine::default();
        let provider = provider("shopify_function_v2");
        let mut cache = ProviderCache::new();

        let first = cache.module(&engine, &provider)?;
        let second = cache.module(&engine.clone(), &provider)?;

        assert_eq!(first.image_range(), second.image_range());
        Ok(())
    }

    #[test]
    fn test_provider_cache_keeps_each_provider() -> Result<()> {
        let engine = Engine::default();
        let v1 = provider("shopify_function_v1");
        let v2 = provider("shopify_function_v2");
        let mut cache = ProviderCache::new();

        let first_v1 = cache.module(&engine, &v1)?;
        let first_v2 = cache.module(&engine, &v2)?;

        assert_ne!(first_v1.image_range(), first_v2.image_range());
        assert_eq!(
            first_v1.image_range(),
            cache.module(&engine, &v1)?.image_range()
        );
        assert_eq!(
            first_v2.image_range(),
            cache.module(&engine, &v2)?.image_range()
        );
        Ok(())
    }

    #[test]
    fn test_provider_cache_holds_only_most_recent_engine() -> Result<()> {
        let first_engine = Engine::default();
        let second_engine = Engine::default();
        let provider = provider("shopify_function_v2");
        let mut cache = ProviderCache::new();

        let first = cache.module(&first_engine, &provider)?;
        let second = cache.module(&second_engine, &provider)?;
        assert!(Engine::same(second.engine(), &second_engine));
        assert_ne!(first.image_range(), second.image_range());

        let first_again = cache.module(&first_engine, &provider)?;
        assert!(Engine::same(first_again.engine(), &first_engine));
        assert_ne!(first.image_range(), first_again.image_range());
        Ok(())
    }

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
