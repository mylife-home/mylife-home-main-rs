use crate::{
    metadata,
    runtime::{self, TypedInto, Value},
};

use std::fmt;

pub trait MylifePluginHooks: Sized {
    fn new(id: &str) -> Self;

    // called after config
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

// Trait implemented by the plugin itself
pub trait MylifePlugin: MylifePluginHooks + fmt::Debug {
    // used to export
    fn runtime() -> Box<dyn runtime::MylifePluginRuntime>;
}

struct StateRuntimeData {
    listener: Box<dyn Fn(Value)>,
    r#type: metadata::Type,
}

impl fmt::Debug for StateRuntimeData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StateRuntimeData")
            .field("listener", &"Box<dyn Fn(Value)>")
            .field("type", &self.r#type)
            .finish()
    }
}

#[derive(Debug)]
pub struct State<T: Default> {
    value: T,
    runtime: Option<StateRuntimeData>,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        State {
            value: T::default(),
            runtime: None,
        }
    }
}

impl<T: Default + Clone + TypedInto<Value>> State<T> {
    pub fn set(&mut self, value: T) {
        let StateRuntimeData { listener, r#type } =
            self.runtime.as_ref().expect("Unbound state changed!");

        self.value = value;
        let value = self.value.clone().typed_into(r#type);
        listener(value);
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn runtime_register(&mut self, listener: Box<dyn Fn(Value)>, r#type: metadata::Type) {
        self.runtime = Some(StateRuntimeData { listener, r#type });
    }
}
