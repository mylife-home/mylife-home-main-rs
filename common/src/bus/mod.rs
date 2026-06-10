use std::{any::Any, fmt};

use tokio::{select, task::JoinHandle};

use crate::{
    bus::{
        client::{Client, MqttEvent},
        presence::{Presence, PresenceHandler},
    },
    utils::mailbox::{Mailbox, MailboxHandle},
};

pub mod client;
mod encoding;
pub mod mqtt;
mod presence;

pub trait BusMessage: Any + Send + fmt::Debug {
    fn as_any(&self) -> &dyn Any;
}

pub trait BusHandler: Send {
    /// Called once when the actor starts, before any message is processed.
    /// Use it to seed the state or set up handler state.
    fn init(&mut self, data: &mut BusData, mailbox_sender: &MailboxHandle<Box<dyn BusMessage>>) {
        let _ = data;
        let _ = mailbox_sender;
    }

    /// Handles a single message, optionally mutating the state.
    fn handle(&mut self, data: &mut BusData, message: &dyn BusMessage) {
        let _ = data;
        let _ = message;
    }

    /// Handles an MQTT event, optionally mutating the state.
    fn handle_mqtt(&mut self, data: &mut BusData, event: &MqttEvent) {
        let _ = data;
        let _ = event;
    }
}

pub struct Transport {
    data: BusData,
    mailbox: Mailbox<Box<dyn BusMessage>>,
    handlers: Vec<Box<dyn BusHandler>>,
}

pub struct BusData {
    shutdown: bool,
    client: Client,
    presence: Presence,
}

impl BusData {
    fn new(client: Client) -> Self {
        Self {
            shutdown: false,
            client,
            presence: Presence::new(),
        }
    }

    pub fn set_shutdown(&mut self) {
        self.shutdown = true;
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    pub fn presence(&self) -> &Presence {
        &self.presence
    }

    pub fn presence_mut(&mut self) -> &mut Presence {
        &mut self.presence
    }
}

impl Transport {
    /// Creates a new Transport actor reading from the given mailbox.
    pub fn new(instance_name: String, server_address: String) -> anyhow::Result<Self> {
        let handlers: Vec<Box<dyn BusHandler>> =
            vec![Box::new(ShutdownHandler), Box::new(PresenceHandler)];

        Ok(Self {
            data: BusData::new(Client::create(instance_name, server_address)?),
            mailbox: Mailbox::new(),
            handlers,
        })
    }

    /// Get a handle to the mailbox
    pub fn get_mailbox_handle(&self) -> MailboxHandle<Box<dyn BusMessage>> {
        self.mailbox.handle()
    }

    /// Registers a handler. Must be called before the actor is started.
    pub fn add_handler(&mut self, handler: impl BusHandler + 'static) {
        self.handlers.push(Box::new(handler));
    }

    /// Spawns the actor on the current runtime, consuming it.
    pub fn start(self) -> JoinHandle<()> {
        tokio::spawn(self.run())
    }

    /// Runs the message loop until the mailbox is closed, dispatching each
    /// message to every handler in turn.
    async fn run(mut self) {
        log::trace!("Starting bus");

        let mailbox_sender = self.mailbox.handle();
        for handler in &mut self.handlers {
            handler.init(&mut self.data, &mailbox_sender);
        }

        log::trace!("Bus started");

        loop {
            select! {
                Some(event) = self.data.client.next_event() => {
                    log::trace!("Dispatching MQTT event {:?}", event);

                    for handler in &mut self.handlers {
                        handler.handle_mqtt(&mut self.data, &event);
                    }
                }

                message = self.mailbox.recv() => {
                    log::trace!("Dispatching message {:?}", message);

                    for handler in &mut self.handlers {
                        handler.handle(&mut self.data, message.as_ref());
                    }
                }
            }

            if self.data.is_shutdown() {
                break;
            }
        }

        self.data.client.shutdown().await;

        log::trace!("Bus terminated");
    }
}

#[derive(Debug)]
pub struct ShutdownMessage;

impl BusMessage for ShutdownMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

struct ShutdownHandler;

impl BusHandler for ShutdownHandler {
    fn handle(&mut self, data: &mut BusData, message: &dyn BusMessage) {
        let Some(_message) = message.as_any().downcast_ref::<ShutdownMessage>() else {
            return;
        };

        data.set_shutdown();
    }
}
