use crate::{
    metadata,
    runtime::{self, Value, TypedInto},
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

struct StateRuntimeData {
    listener: Box<dyn Fn(/*value:*/ Value)>,
    ty: metadata::Type,
}

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
        let StateRuntimeData { listener, ty } =
            self.runtime.as_ref().expect("Unbound state changed!");

        self.value = value;
        listener(self.value.clone().typed_into(ty));
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn runtime_register(&mut self, listener: Box<dyn Fn(/*value:*/ Value)>, ty: metadata::Type) {
        self.runtime = Some(StateRuntimeData { listener, ty });
    }
}
