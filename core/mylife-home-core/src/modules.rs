use plugin_runtime::{PluginRegistration, runtime::MylifePluginRuntime};
use std::{collections::HashMap, sync::OnceLock};

#[derive(Debug)]
pub struct Registry {
    plugins: HashMap<String, Box<dyn MylifePluginRuntime>>,
}

impl Registry {
    fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn plugins(&self) -> Vec<&dyn MylifePluginRuntime> {
        self.plugins.values().map(|v| &**v).collect()
    }

    pub fn plugin(&self, id: &str) -> Option<&dyn MylifePluginRuntime> {
        self.plugins.get(id).map(|v| &**v)
    }
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

pub fn init() {
    let mut registry = Registry::new();

    for runtime in PluginRegistration::runtimes() {
        let id = runtime.metadata().id().to_owned();

        registry.plugins.insert(id.clone(), runtime);

        tracing::debug!(plugin_id = id, "loaded plugin");
    }

    REGISTRY
        .set(registry)
        .expect("registry already initialized");
}

pub fn registry() -> &'static Registry {
    REGISTRY.get().expect("registry not initialized")
}
