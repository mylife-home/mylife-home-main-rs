use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::broadcast;

use crate::bus::mqtt::MqttEvent;

use super::mqtt;

#[derive(Debug)]
pub struct Client {
    mqtt_client: mqtt::MqttClient,
    online: AtomicBool,
    // Keep subscription list
    // clear resident state
}

impl Client {
    pub fn create(instance_name: String, server_address: String) -> anyhow::Result<Self> {
        let mqtt_client = mqtt::MqttClient::create(instance_name, server_address)?;

        Ok(Self {
            mqtt_client,
            online: AtomicBool::new(false),
        })
    }

    pub fn online(&self) -> bool {
        self.online.load(Ordering::Relaxed)
    }

    pub fn events(&self) -> broadcast::Receiver<MqttEvent> {
        self.mqtt_client.events()
    }
}
