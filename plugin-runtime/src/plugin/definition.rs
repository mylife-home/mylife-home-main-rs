use crate::runtime;

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
    on_change: Option<fn(value: &T)>,
}

impl<T: Default> Default for State<T> {
    fn default() -> Self {
        State {
            value: T::default(),
            on_change: None,
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
