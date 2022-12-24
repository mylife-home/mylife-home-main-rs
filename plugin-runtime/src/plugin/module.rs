use crate::runtime;

pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");
pub static MYLIFE_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ModuleDeclaration {
    pub rustc_version: &'static str,
    pub core_version: &'static str,
    pub mylife_runtime_version: &'static str,
    pub module_version: &'static str,
    pub init: unsafe extern "C" fn(params: &InitParams) -> Result<(), Box<dyn std::error::Error>>,
    pub register: unsafe extern "C" fn(registry: &mut dyn PluginRegistry),
}

pub struct InitParams {
    pub logger: &'static dyn log::Log,
    pub logger_max_level: log::LevelFilter
}

#[macro_export]
macro_rules! export_module {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static mylife_home_core_module_declaration: $crate::ModuleDeclaration =
            $crate::ModuleDeclaration {
                rustc_version: $crate::RUSTC_VERSION,
                core_version: $crate::CORE_VERSION,
                mylife_runtime_version: $crate::MYLIFE_RUNTIME_VERSION,
                module_version: env!("CARGO_PKG_VERSION"),
                init: mylife_home_core_module_init,
                register: $register,
            };

        #[doc(hidden)]
        extern "C" fn mylife_home_core_module_init(params: &$crate::InitParams) -> Result<(), Box<dyn std::error::Error>> {
            log::set_logger(params.logger)?;
            log::set_max_level(params.logger_max_level);
            Ok(())
        }
    };
}

pub trait PluginRegistry {
    fn register_plugin(&mut self, plugin: Box<dyn runtime::MylifePluginRuntime>);
}
