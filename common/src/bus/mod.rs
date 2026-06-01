use std::fmt;

use tokio::{
    select,
    sync::{broadcast, mpsc::UnboundedReceiver},
    task::JoinHandle,
};

use crate::bus::{client::Client, mqtt::MqttEvent, presence::Presence};

pub mod client;
pub mod mqtt;
mod presence;

pub trait BusMessage: Send + fmt::Debug {}

pub trait BusHandler: Send {
    /// Called once when the actor starts, before any message is processed.
    /// Use it to seed the state or set up handler state.
    fn init(&mut self, data: &mut BusData) {
        let _ = data;
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
    mailbox: UnboundedReceiver<Box<dyn BusMessage>>,
    mqtt_events: broadcast::Receiver<MqttEvent>,
    handlers: Vec<Box<dyn BusHandler>>,
}

pub struct BusData {
    client: Client,
    presence: Presence,
}

impl BusData {
    fn new(client: Client) -> Self {
        Self {
            client,
            presence: Presence::new(),
        }
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
    pub fn new(
        mailbox: UnboundedReceiver<Box<dyn BusMessage>>,
        instance_name: String,
        server_address: String,
    ) -> anyhow::Result<Self> {
        let client = Client::create(instance_name, server_address)?;
        let mqtt_events = client.events();

        Ok(Self {
            data: BusData::new(client),
            mailbox,
            mqtt_events,
            handlers: Vec::new(),
        })
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
        log::trace!("Bus started");

        loop {
            select! {
                Ok(event) = self.mqtt_events.recv() => {
                    log::trace!("Received MQTT event {:?}", event);
                    for handler in &mut self.handlers {
                        handler.handle_mqtt(&mut self.data, &event);
                    }
                }

                Some(message) = self.mailbox.recv() => {
                    log::trace!("Received message {:?}", message);
                    for handler in &mut self.handlers {
                        handler.handle(&mut self.data, message.as_ref());
                    }
                }

                else => break,
            }
        }
    }
}
