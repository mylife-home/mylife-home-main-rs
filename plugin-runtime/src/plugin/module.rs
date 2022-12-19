use crate::runtime;

pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");
pub static MYLIFE_RUNTIME_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct ModuleDeclaration {
    pub rustc_version: &'static str,
    pub core_version: &'static str,
    pub mylife_runtime_version: &'static str,
    pub module_version: &'static str,
    pub register: unsafe extern "C" fn(registry: &mut dyn PluginRegistry),
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
                register: $register,
            };
    };
}

pub trait PluginRegistry {
    fn register_plugin(&mut self, plugin: Box<dyn runtime::MylifePluginRuntime>);
}
