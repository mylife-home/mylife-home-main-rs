use log::debug;
use plugin_runtime::{PluginRegistration, metadata::PluginMetadata, runtime::MylifeComponent};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, OnceLock},
};

const LOG_TARGET: &str = "mylife:home:core:modules";

#[derive(Debug)]
struct Module<'a> {
    name: String,
    version: String,
    plugins: HashMap<String, &'a Plugin<'a>>,
}

impl<'a> Module<'a> {
    fn new(plugin_metadata: &PluginMetadata) -> Pin<Box<Self>> {
        Box::pin(Self {
            name: String::from(plugin_metadata.module()),
            version: String::from(plugin_metadata.version()),
            plugins: HashMap::new(),
        })
    }

    fn add_plugin(&mut self, plugin: &'a Plugin) {
        self.plugins
            .insert(String::from(plugin.metadata().name()), &plugin);
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn plugin(&self, id: &str) -> Option<&'a Plugin> {
        self.plugins.get(id).copied()
    }

    pub fn plugins(&self) -> Vec<&'a Plugin> {
        self.plugins.values().copied().collect()
    }
}

#[derive(Debug)]
pub struct Plugin<'a> {
    runtime: Box<dyn plugin_runtime::runtime::MylifePluginRuntime>,
    module: &'a Module<'a>,
}

impl<'a> Plugin<'a> {
    fn new(
        runtime: Box<dyn plugin_runtime::runtime::MylifePluginRuntime>,
        module: &'a Module,
    ) -> Pin<Box<Self>> {
        Box::pin(Self { runtime, module })
    }

    pub fn id(&self) -> &str {
        &self.metadata().id()
    }

    pub fn version(&self) -> &str {
        &self.metadata().version()
    }

    pub fn metadata(&self) -> &PluginMetadata {
        self.runtime.metadata()
    }

    pub fn create_component(&self, id: &str) -> Box<dyn MylifeComponent> {
        self.runtime.create(id)
    }
}

#[derive(Debug)]
pub struct Registry<'a> {
    modules: HashMap<String, Pin<Box<Module<'a>>>>,
    plugins: HashMap<String, Pin<Box<Plugin<'a>>>>,
}

impl<'a> Registry<'a> {
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
            plugins: HashMap::new(),
        }
    }

    pub fn plugins(&self) -> Vec<&'a Plugin> {
        self.plugins.values().map(Self::mapper).collect()
    }

    pub fn plugin(&self, id: &str) -> Option<&'a Plugin> {
        self.plugins.get(id).map(Self::mapper)
    }

    pub fn modules(&self) -> Vec<&'a Module> {
        self.modules.values().map(Self::mapper).collect()
    }

    pub fn module(&self, id: &str) -> Option<&'a Module> {
        self.modules.get(id).map(Self::mapper)
    }

    fn mapper<T>(object: &'a Pin<Box<T>>) -> &'a T {
        // SAFETY: We are inside the Registry lifetime, so the object pointer remains valid.
        unsafe { Pin::into_inner_unchecked(object.as_ref()) }
    }
}

static REGISTRY: OnceLock<Registry<'static>> = OnceLock::new();

pub fn init() {
    let mut registry = Registry::new();

    for runtime in PluginRegistration::runtimes() {
        let metadata = runtime.metadata();
        let module = registry
            .modules
            .entry(String::from(metadata.module()))
            .or_insert_with(|| {
                let module = Module::new(metadata);
                debug!(
                    target: LOG_TARGET,
                    "Loading module '{}' v{}",
                    module.name(),
                    module.version()
                );

                module
            });

        // get unsafe refs to make links
        // SAFETY: Everything is valid inside the Registry lifetime.
        // Mutating module is OK because we are at build time and nothing else can access it.
        let plugin = Plugin::new(runtime, unsafe { unsafe_ref(module) });

        let plugin_entry = registry
            .plugins
            .entry(String::from(plugin.id()))
            .insert_entry(plugin);
        let plugin = plugin_entry.get();

        let plugin = unsafe { unsafe_ref(plugin) };

        unsafe {
            unsafe_mut_ref(module).add_plugin(plugin);
        }

        module.add_plugin(&plugin);

        debug!(
            target: LOG_TARGET,
            "Loaded plugin '{}'",
            plugin.id()
        );
    }

    REGISTRY
        .set(registry)
        .expect("Registry already initialized");
}

pub fn registry() -> &'static Registry<'static> {
    REGISTRY.get().expect("Registry not initialized")
}

unsafe fn unsafe_ref<T>(value: &Pin<Box<T>>) -> &'static T {
    let ptr = Pin::into_inner_unchecked(value.as_ref()) as *const T;
    &*ptr
}

unsafe fn unsafe_mut_ref<T>(value: &Pin<Box<T>>) -> &'static mut T {
    let ptr = Pin::into_inner_unchecked(value.as_ref()) as *const T as *mut T;
    #[allow(invalid_reference_casting)]
    &mut *ptr
}
