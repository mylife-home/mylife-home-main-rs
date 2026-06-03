use std::sync::Arc;

use crate::{
    components::{metadata::PluginMetadata, types::Value},
    utils::observable::{EventType, Observable},
};

/// Component represents a component that can be registered to the registry.
pub trait Component: Observable<ComponentChangeEventType> + Send {
    /// Returns the unique identifier of the component.
    fn id(&self) -> &str;

    /// Returns the plugin metadata of the component.
    fn plugin(&self) -> Arc<PluginMetadata>;

    /// Gets the state of the component by its name.
    fn get_state(&self, name: &str) -> Option<Value>;

    /// Executes an action on the component.
    fn execute_action(&mut self, name: &str, action: Value) -> anyhow::Result<()>;
}

/// ComponentChange represents the changes that can occur on a component.
#[derive(Debug)]
pub enum ComponentChange<'a> {
    /// State is emitted when a state of the component changes, containing the state name and the new value.
    State { name: &'a str, value: &'a Value },
}

pub struct ComponentChangeEventType;

impl EventType for ComponentChangeEventType {
    type Event<'a> = ComponentChange<'a>;
}
