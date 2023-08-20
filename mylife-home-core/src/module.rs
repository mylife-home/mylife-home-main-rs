use core_plugin_runtime::{
    metadata::PluginMetadata, runtime::MylifeComponent, ModuleDeclaration, PluginRegistry,
};
use libloading::Library;
use log::{debug, trace};
use regex::Regex;
use std::{collections::HashMap, fmt, fs::read_dir, path::PathBuf, sync::Arc};

const LOG_TARGET: &str = "mylife:home:core:module";

struct PluginRegistryImpl<'registry> {
    module: Arc<Module>,
    plugins: &'registry mut HashMap<String, Arc<Plugin>>,
}

impl<'registry> PluginRegistryImpl<'registry> {
    fn new(
        module: Arc<Module>,
        plugins: &'registry mut HashMap<String, Arc<Plugin>>,
    ) -> PluginRegistryImpl<'registry> {
        PluginRegistryImpl { module, plugins }
    }
}

impl PluginRegistry for PluginRegistryImpl<'_> {
    fn register_plugin(
        &mut self,
        plugin: Box<dyn core_plugin_runtime::runtime::MylifePluginRuntime>,
    ) {
        let plugin = Arc::new(Plugin::new(self.module.clone(), plugin));

        debug!(
            target: LOG_TARGET,
            "Plugin loaded: {} v{}",
            plugin.id(),
            plugin.version()
        );

        trace!(
            target: LOG_TARGET,
            "Plugin metadata: {:?}",
            plugin.metadata()
        );

        let id = String::from(plugin.id());

        if let Some(other) = self.plugins.insert(id, plugin) {
            panic!("Plugin id duplicate '{}'", other.id());
        }
    }
}

struct Module {
    _library: Library,
    name: String,
    version: String,
}

impl Module {
    fn new(library: Library, base_name: &str, version: &str) -> Arc<Self> {
        use convert_case::{Case, Casing};

        Arc::new(Module {
            _library: library,
            name: base_name.to_case(Case::Kebab),
            version: String::from(version),
        })
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
    runtime: Box<dyn core_plugin_runtime::runtime::MylifePluginRuntime>,
    module: Arc<Module>, // Note: keep it last so it is dropped last
}

impl Plugin {
    fn new(
        module: Arc<Module>,
        runtime: Box<dyn core_plugin_runtime::runtime::MylifePluginRuntime>,
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

    pub fn version(&self) -> &str {
        &self.module.version()
    }

    pub fn metadata(&self) -> &PluginMetadata {
        self.runtime.metadata()
    }

    pub fn create_component(&self, id: &str) -> Box<dyn MylifeComponent> {
        self.runtime.create(id)
    }
}

pub fn load_modules(
    module_path: &str,
) -> Result<HashMap<String, Arc<Plugin>>, Box<dyn std::error::Error>> {
    let mut plugins: HashMap<String, Arc<Plugin>> = HashMap::new();
    let name_match = Regex::new(&format!(
        "{}{}(.*){}",
        std::env::consts::DLL_PREFIX,
        "plugin_",
        std::env::consts::DLL_SUFFIX
    ))
    .unwrap();

    for path in read_dir(module_path)? {
        let entry = path?;
        let file_name = String::from(entry.file_name().to_string_lossy());
        if let Some(matchs) = name_match.captures(&file_name) {
            if matchs.len() == 2 {
                let name = &matchs[1];
                load_module(entry.path(), name, &mut plugins)?;
                continue;
            }
        }

        trace!(
            target: LOG_TARGET,
            "File ignored: {}",
            file_name
        );
    }

    Ok(plugins)
}

fn load_module(
    file_path: PathBuf,
    name: &str,
    plugins: &mut HashMap<String, Arc<Plugin>>,
) -> Result<(), Box<dyn std::error::Error>> {
    trace!(
        target: LOG_TARGET,
        "Opening module from path '{}'",
        file_path.display()
    );

    let library = unsafe { Library::new(file_path)? };

    let module_declaration = unsafe {
        library
            .get::<*const ModuleDeclaration>(b"mylife_home_core_module_declaration\0")?
            .read()
    };

    if module_declaration.rustc_version != core_plugin_runtime::RUSTC_VERSION {
        return Err(Box::new(ModuleLoadError::RustCompilerVersionMismatch(
            module_declaration.rustc_version.into(),
            core_plugin_runtime::RUSTC_VERSION.into(),
        )));
    } else if module_declaration.core_version != core_plugin_runtime::CORE_VERSION {
        return Err(Box::new(ModuleLoadError::CoreVersionMismatch(
            module_declaration.core_version.into(),
            core_plugin_runtime::CORE_VERSION.into(),
        )));
    } else if module_declaration.mylife_runtime_version
        != core_plugin_runtime::MYLIFE_RUNTIME_VERSION
    {
        return Err(Box::new(ModuleLoadError::MylifeRuntimeVersionMismatch(
            module_declaration.mylife_runtime_version.into(),
            core_plugin_runtime::MYLIFE_RUNTIME_VERSION.into(),
        )));
    }

    let module = Module::new(library, name, module_declaration.module_version);

    debug!(
        target: LOG_TARGET,
        "Loading module '{}' v{}",
        module.name(),
        module.version()
    );

    let register = module_declaration.register;

    let mut registry = PluginRegistryImpl::new(module, plugins);
    register(&mut registry);

    Ok(())
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
