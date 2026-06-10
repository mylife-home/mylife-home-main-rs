use std::{any::Any, fmt};

use tokio::task::JoinHandle;

mod component;
pub mod metadata;
mod registry;
pub mod types;

pub use component::*;
pub use registry::*;

use crate::utils::mailbox::{Mailbox, MailboxHandle};

/// ComponentsMessage is a data-only message processed by the Components actor.
/// Implementors carry the payload; the behavior lives in the handlers.
pub trait ComponentsMessage: Send + fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

/// ComponentsHandler processes messages with mutable access to the registry.
/// Handlers are registered at init time and called in registration order.
pub trait ComponentsHandler: Send {
    /// Called once when the actor starts, before any message is processed.
    /// Use it to seed the registry or set up handler state.
    fn init(&mut self, data: &mut ComponentsData) {
        let _ = data;
    }

    /// Handles a single message, optionally mutating the registry.
    fn handle(&mut self, data: &mut ComponentsData, message: &dyn ComponentsMessage) {
        let _ = data;
        let _ = message;
    }
}

/// Components is the actor that owns the registry and processes incoming
/// messages sequentially, dispatching each to the registered handlers.
pub struct Components {
    data: ComponentsData,
    mailbox: Mailbox<Box<dyn ComponentsMessage>>,
    handlers: Vec<Box<dyn ComponentsHandler>>,
}

pub struct ComponentsData {
    shutdown: bool,
    registry: Registry,
}

impl ComponentsData {
    fn new() -> Self {
        Self {
            shutdown: false,
            registry: Registry::new(),
        }
    }

    pub fn set_shutdown(&mut self) {
        self.shutdown = true;
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
    }
}

impl Components {
    /// Creates a new Components actor reading from the given mailbox.
    pub fn new() -> Self {
        let handlers: Vec<Box<dyn ComponentsHandler>> = vec![Box::new(ShutdownHandler)];

        Self {
            data: ComponentsData::new(),
            mailbox: Mailbox::new(),
            handlers,
        }
    }

    /// Get a handle to the mailbox
    pub fn get_mailbox_handle(&self) -> MailboxHandle<Box<dyn ComponentsMessage>> {
        self.mailbox.handle()
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
            handler.init(&mut self.data);
        }

        log::trace!("Components started");

        loop {
            let message = self.mailbox.recv().await;
            log::trace!("Dispatching message {:?}", message);

            for handler in &mut self.handlers {
                handler.handle(&mut self.data, message.as_ref());
            }

            if self.data.is_shutdown() {
                break;
            }
        }

        log::trace!("Components terminated");
    }
}

#[derive(Debug)]
pub struct ShutdownMessage;

impl ComponentsMessage for ShutdownMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ShutdownHandler;

impl ComponentsHandler for ShutdownHandler {
    fn handle(&mut self, data: &mut ComponentsData, message: &dyn ComponentsMessage) {
        let Some(_message) = message.as_any().downcast_ref::<ShutdownMessage>() else {
            return;
        };

        data.set_shutdown();
    }
}
