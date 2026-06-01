use std::{fmt, sync::Arc};

use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};

use crate::{components::{
    metadata::PluginMetadata,
    types::Value,
}, utils::observable::{EventType, Observable}};

pub mod metadata;
mod registry;
pub mod types;

pub use registry::Registry;
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

/// Components is the actor that owns the registry and processes incoming
/// messages sequentially, dispatching each to the registered handlers.
pub struct Components {
    registry: Registry,
    mailbox: UnboundedReceiver<Box<dyn ComponentsMessage>>,
    handlers: Vec<Box<dyn ComponentsHandler>>,
}

/// ComponentsMessage is a data-only message processed by the Components actor.
/// Implementors carry the payload; the behavior lives in the handlers.
pub trait ComponentsMessage: Send + fmt::Debug {}

/// ComponentsHandler processes messages with mutable access to the registry.
/// Handlers are registered at init time and called in registration order.
pub trait ComponentsHandler: Send {
    /// Called once when the actor starts, before any message is processed.
    /// Use it to seed the registry or set up handler state.
    fn init(&mut self, registry: &mut Registry) {
        let _ = registry;
    }

    /// Handles a single message, optionally mutating the registry.
    fn handle(&mut self, registry: &mut Registry, message: &dyn ComponentsMessage) {
        let _ = registry;
        let _ = message;
    }
}

impl Components {
    /// Creates a new Components actor reading from the given mailbox.
    pub fn new(mailbox: UnboundedReceiver<Box<dyn ComponentsMessage>>) -> Self {
        Self {
            registry: Registry::new(),
            mailbox,
            handlers: Vec::new(),
        }
    }

    /// Registers a handler. Must be called before the actor is started.
    pub fn add_handler(&mut self, handler: impl ComponentsHandler + 'static) {
        self.handlers.push(Box::new(handler));
    }

    /// Spawns the actor on the current runtime, consuming it.
    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(self.run())
    }

    /// Runs the message loop until the mailbox is closed, dispatching each
    /// message to every handler in turn.
    async fn run(mut self) {
        log::trace!("Starting components");

        for handler in &mut self.handlers {
            handler.init(&mut self.registry);
        }

        log::trace!("Components started");

        while let Some(message) = self.mailbox.recv().await {
            log::trace!("Dispatching message {:?}", message);

            for handler in &mut self.handlers {
                handler.handle(&mut self.registry, message.as_ref());
            }
        }
    }
}
