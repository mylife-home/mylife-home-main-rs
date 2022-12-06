use crate::runtime;

pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

pub struct PluginDeclaration {
    pub rustc_version: &'static str,
    pub core_version: &'static str,
    pub plugin_version: &'static str,
    pub register: unsafe extern "C" fn(registry: &mut dyn PluginRegistry),
}

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_declaration: $crate::PluginDeclaration = $crate::PluginDeclaration {
            rustc_version: $crate::RUSTC_VERSION,
            core_version: $crate::CORE_VERSION,
            plugin_version: env!("CARGO_PKG_VERSION"),
            register: $register,
        };
    };
}

pub trait PluginRegistry {
    fn register_plugin(&mut self, plugin: Box<dyn runtime::MyLifePluginRuntime>);
}
