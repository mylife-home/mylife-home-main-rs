use std::{fmt, path::Path, sync::Arc};

use libloading::Library;
use plugin_runtime::{
    metadata::PluginMetadata, runtime::MylifeComponent, ModuleDeclaration, PluginRegistry,
};

struct PluginRegistryImpl {
    module: Arc<Module>,
    plugins: Vec<Arc<Plugin>>,
}

impl PluginRegistryImpl {
    fn new(module: Arc<Module>) -> PluginRegistryImpl {
        PluginRegistryImpl {
            module,
            plugins: Vec::new(),
        }
    }
}

impl PluginRegistry for PluginRegistryImpl {
    fn register_plugin(&mut self, plugin: Box<dyn plugin_runtime::runtime::MyLifePluginRuntime>) {
        self.plugins
            .push(Arc::new(Plugin::new(self.module.clone(), plugin)));
    }
}

pub struct Module {
    _library: Library,
    name: String,
    version: String,
}

impl Module {
    pub fn load(
        module_path: &str,
        name: &str,
    ) -> Result<Vec<Arc<Plugin>>, Box<dyn std::error::Error>> {
        let path = Path::new(module_path).join(format!("lib{}.so", name));

        let library = unsafe { Library::new(path)? };

        let module_declaration = unsafe {
            library
                .get::<*const ModuleDeclaration>(b"mylife_home_core_module_declaration\0")?
                .read()
        };

        if module_declaration.rustc_version != plugin_runtime::RUSTC_VERSION {
            return Err(Box::new(ModuleLoadError::RustCompilerVersionMismatch(
                module_declaration.rustc_version.into(),
                plugin_runtime::RUSTC_VERSION.into(),
            )));
        } else if module_declaration.core_version != plugin_runtime::CORE_VERSION {
            return Err(Box::new(ModuleLoadError::CoreVersionMismatch(
                module_declaration.core_version.into(),
                plugin_runtime::CORE_VERSION.into(),
            )));
        } else if module_declaration.mylife_runtime_version
            != plugin_runtime::MYLIFE_RUNTIME_VERSION
        {
            return Err(Box::new(ModuleLoadError::MylifeRuntimeVersionMismatch(
                module_declaration.mylife_runtime_version.into(),
                plugin_runtime::MYLIFE_RUNTIME_VERSION.into(),
            )));
        }

        let module = Arc::new(Module {
            _library: library,
            name: String::from(name),
            version: String::from(module_declaration.module_version),
        });

        let mut registry = PluginRegistryImpl::new(module.clone());
        let register = module_declaration.register;
        unsafe { register(&mut registry) };

        Ok(registry.plugins)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }
}

pub struct Plugin {
    id: String,
    module: Arc<Module>,
    runtime: Box<dyn plugin_runtime::runtime::MyLifePluginRuntime>,
}

impl Plugin {
    fn new(
        module: Arc<Module>,
        runtime: Box<dyn plugin_runtime::runtime::MyLifePluginRuntime>,
    ) -> Plugin {
        let id = format!("{}.{}", module.name(), runtime.metadata().name());

        Plugin {
            module,
            runtime,
            id,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn module(&self) -> &str {
        self.module.name()
    }

    pub fn version(&self) -> &str {
        self.module.version()
    }

    pub fn metadata(&self) -> &PluginMetadata {
        self.runtime.metadata()
    }

    pub fn create_component(&self) -> Box<dyn MylifeComponent> {
        self.runtime.create()
    }
}

#[derive(Debug, Clone)]
pub enum ModuleLoadError {
    RustCompilerVersionMismatch(String, String),
    CoreVersionMismatch(String, String),
    MylifeRuntimeVersionMismatch(String, String),
}

impl std::error::Error for ModuleLoadError {}

impl fmt::Display for ModuleLoadError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ModuleLoadError::RustCompilerVersionMismatch(module_version, core_version) => write!(
                fmt,
                "Rust compiler version mismatch: module='{}', core='{}'",
                module_version, core_version
            ),
            ModuleLoadError::CoreVersionMismatch(module_version, core_version) => write!(
                fmt,
                "Rust core version mismatch: module='{}', core='{}'",
                module_version, core_version
            ),
            ModuleLoadError::MylifeRuntimeVersionMismatch(module_version, core_version) => write!(
                fmt,
                "Mylife runtime version mismatch: module='{}', core='{}'",
                module_version, core_version
            ),
        }
    }
}
