use crate::{
    metadata,
    runtime::{self, TypedInto, Value},
};

pub trait MylifePluginHooks {
    // called after config
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// Trait implemented by the plugin itself
pub trait MylifePlugin: Default + MylifePluginHooks {
    // used to export
    fn runtime() -> Box<dyn runtime::MylifePluginRuntime>;

    // mark the plugin instance  (component) like it has failed
    // usually, only drop should be called after that
    fn fail(&mut self, error: Box<dyn std::error::Error>);
}

pub trait StateRuntimeListener {
    fn change(&self);
}

pub struct State<T: Default> {
    value: T,
    runtime_listener: Option<Box<dyn StateRuntimeListener>>,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        State {
            value: T::default(),
            runtime_listener: None,
        }
    }
}

impl<T: Default + Clone + TypedInto<Value>> State<T> {
    pub fn set(&mut self, value: T) {
        let listener = self
            .runtime_listener
            .as_ref()
            .expect("Unbound state changed!");

        self.value = value;
        listener.change();
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn runtime_register(&mut self, listener: Box<dyn StateRuntimeListener>) {
        self.runtime_listener = Some(listener);
    }
}
