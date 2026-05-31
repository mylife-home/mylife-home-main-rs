use std::{fmt, sync::Arc};

use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

use crate::components::{
    metadata::PluginMetadata,
    observable::{EventType, Observable},
    registry::Registry,
    types::Value,
};

pub mod metadata;
pub mod observable;
mod registry;
pub mod types;

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
pub enum ComponentChange {
    /// State is emitted when a state of the component changes, containing the state name and the new value.
    State { name: String, value: Value },
}

pub struct ComponentChangeEventType;

impl EventType for ComponentChangeEventType {
    type Event<'a> = ComponentChange;
}

//////

pub struct Components {
    registry: Registry,
    mailbox: UnboundedReceiver<Box<dyn ComponentsMessage>>,
    handlers: Vec<Box<dyn ComponentsHandler>>,
}

pub trait ComponentsMessage: Send + fmt::Debug {}

pub trait ComponentsHandler : Send {
    fn handle(&mut self, registry: &mut Registry, message: &dyn ComponentsMessage);
}

impl Components {
    pub fn new(mailbox: UnboundedReceiver<Box<dyn ComponentsMessage>>) -> Self {
        Self {
            registry: Registry::new(),
            mailbox,
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: impl ComponentsHandler + 'static) {
        self.handlers.push(Box::new(handler));
    }

    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(self.run())
    }

    async fn run(mut self) {
        while let Some(message) = self.mailbox.recv().await {
            log::trace!("Dispatching message {:?}", message);

            for handler in &mut self.handlers {
                handler.handle(&mut self.registry, message.as_ref());
            }
        }
    }
}
