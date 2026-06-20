use std::{collections::HashMap, sync::{Arc, Mutex, MutexGuard}};

use bytes::Bytes;
use log::error;

use crate::{
    bus::{
        BusData, BusHandler, BusMessage,
        client::{ClientHandle, MqttEvent, Topic, TopicBuilder},
    },
    utils::mailbox::MailboxHandle,
};

const DOMAIN: &str = "metadata";

pub struct Metadata {
    ops: Arc<MetadataOps>,
}

impl Metadata {
    pub fn new(client: ClientHandle) -> Self {
        let ops = Arc::new(MetadataOps {
            metadata: Mutex::new(HashMap::new()),
            client: client.clone(),
        });

        client.online().observe({
            let ops = ops.clone();

            Box::new(move |&online| {
                if online {
                    ops.publish_all();
                }
            })
        });

        Self {
            ops,
        }
    }

    pub fn set<T: serde::Serialize>(&mut self, path: &str, value: &T) {
        self.ops.set(path, value);
    }

    pub fn clear(&mut self, path: &str) {
        self.ops.clear(path);
    }
}

struct MetadataOps {
    metadata: Mutex<HashMap<String, Bytes>>,
    client: ClientHandle,
}

impl MetadataOps {
    fn metadata(&self) -> MutexGuard<'_, HashMap<String, Bytes>> {
        self.metadata.lock().expect("could not acquire mutex")
    }

    pub fn set<T: serde::Serialize>(&self, path: &str, value: &T) {
        let value = match serde_json::to_vec(value) {
            Err(err) => {
                error!("could not convert object: {}", err);
                return;
            }
            Ok(value) => Bytes::from(value),
        };

        self.metadata().insert(path.to_owned(), value.clone());
        self.client.publish(self.topic(path), value, true);
    }

    pub fn clear(&self, path: &str) {
        self.metadata().remove(path);
        self.client.publish(self.topic(path), Bytes::new(), true);
    }

    fn topic(&self, path: &str) -> Topic {
        TopicBuilder::local(self.client.instance_name(), DOMAIN)
            .segment(path)
            .build()
    }

    pub fn publish_all(&self) {
        for (path, value) in self.metadata().iter() {
            self.client.publish(self.topic(path), value.clone(), true);
        }
    }
}

pub struct MetadataHandler;

impl BusHandler for MetadataHandler {
    fn handle_mqtt(&mut self, data: &mut BusData, event: &MqttEvent) {
        let _ = data;
        let _ = event;
    }
}
