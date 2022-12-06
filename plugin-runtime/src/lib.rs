pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

pub struct PluginDeclaration {
  pub rustc_version: &'static str,
  pub core_version: &'static str,
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
            register: $register,
        };
    };
}

pub trait PluginRegistry {
  fn register_plugin(&mut self, plugin: Box<dyn MyLifePluginRuntime>);
}

pub trait MyLifePluginRuntime {
}

// Trait implemented by the plugin itself
pub trait MylifePlugin {
  fn runtime() -> Box<dyn MyLifePluginRuntime>;
}

pub struct State<T: Default> {
  value: T,
  on_change: Option<fn(value: &T)>
}

impl<T: Default> Default for State<T> {
  fn default() -> Self {
    State {
      value: T::default(),
      on_change: None
    }
  }
}

impl<T: Default> State<T> {
  pub fn set(&mut self, value: T) {
    let handler = self.on_change.as_ref().expect("Unbound state changed!");

    self.value = value;
    handler(&self.value);
  }

  pub fn get(&self) -> &T {
    &self.value
  }
}