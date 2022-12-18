use crate::runtime::{self, Value};

pub trait MylifePluginHooks {
    // called after config
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// Trait implemented by the plugin itself
pub trait MylifePlugin: Default + MylifePluginHooks {
    // used to export
    fn runtime() -> Box<dyn runtime::MyLifePluginRuntime>;

    // mark the plugin instance like it has failed
    // usually, only drop should be called after that
    fn fail(error: Box<dyn std::error::Error>);
}

// #[derive(Debug)]
pub struct State<T: Default> {
    value: T,
    on_change: Option<fn(value: &Value)>,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        State {
            value: T::default(),
            on_change: None,
        }
    }
}

impl<T: Default + Clone + Into<Value>> State<T> {
    pub fn set(&mut self, value: T) {
        let handler = self.on_change.as_ref().expect("Unbound state changed!");

        self.value = value;
        handler(&self.value.clone().into());
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn runtime_register(&mut self, listener: fn(value: &Value)) {
        self.on_change = Some(listener);
    }
}
