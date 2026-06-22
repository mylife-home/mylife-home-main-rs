use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bytes::Bytes;
use kameo::{message, prelude::*};

use crate::{
    bus::client::{self, ClientHandle, TopicBuilder},
    utils::actors::{PublisherHandle, pubsub_subscribe},
};

const DOMAIN: &str = "metadata";

/// Name of the PubSub actor that delivers remote metadata update
pub const REMOTE_METADATA_SET_PUBSUB_NAME: &str = "bus.metadata.remote-update";

/// Name of the PubSub actor that delivers online changes
pub const ONLINE_PUBSUB_NAME: &str = "bus.client.online";

pub struct MetadataConfig {
    pub instance_name: Arc<String>,
    pub listen_remote: bool,
}

#[derive(Debug)]
pub struct Metadata {
    instance_name: Arc<String>,
    metadata: HashMap<String, Bytes>,
    remote: Option<Remote>,

    client: ClientHandle,
}

impl Actor for Metadata {
    type Args = MetadataConfig;
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let remote = if config.listen_remote {
            pubsub_subscribe::<client::InstanceOnline, _>(
                actor_ref.clone(),
                client::INSTANCE_ONLINE_PUBSUB_NAME,
            )?;
            pubsub_subscribe::<client::Message, _>(actor_ref, client::MESSAGE_PUBSUB_NAME)?;

            Some(Remote::new()?)
        } else {
            None
        };

        Ok(Self {
            instance_name: config.instance_name,
            metadata: HashMap::new(),
            remote,
            client: ClientHandle::new()?,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.metadata.clear();

        Ok(())
    }
}

impl message::Message<client::Message> for Metadata {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Message,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.remote
            .as_mut()
            .expect("remote not set")
            .handle_message(msg);
    }
}

impl message::Message<client::Online> for Metadata {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Online,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if msg.is_online() {
            for (path, value) in self.metadata.iter() {
                self.publish(path, Some(value.clone()));
            }
        }
    }
}

impl message::Message<client::InstanceOnline> for Metadata {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::InstanceOnline,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.remote
            .as_mut()
            .expect("remote not set")
            .handle_instance_online(msg);
    }
}

impl message::Message<LocalMetadataUpdate> for Metadata {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LocalMetadataUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(value) = msg.value {
            self.metadata.insert(msg.path.clone(), value.clone());
            self.publish(&msg.path, Some(value));
            log::trace!("set '{}'", msg.path);
        } else {
            if self.metadata.remove(&msg.path).is_some() {
                self.publish(&msg.path, None);
                log::trace!("clear '{}'", msg.path);
            }
        }
    }
}

impl Metadata {
    fn publish(&self, path: &str, value: Option<Bytes>) {
        let topic = TopicBuilder::local(&self.instance_name, DOMAIN)
            .segment(path)
            .build();

        if let Some(payload) = value {
            self.client.publish(topic, payload, true);
        } else {
            self.client.clear_retain(topic);
        }
    }
}

#[derive(Debug)]
struct Remote {
    metadata: HashMap<String, HashSet<String>>,
    on_update: PublisherHandle<RemoteMetadataUpdate>,
    client: ClientHandle,
}

impl Remote {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            metadata: HashMap::new(),
            on_update: PublisherHandle::from_name(REMOTE_METADATA_SET_PUBSUB_NAME)?,
            client: ClientHandle::new()?,
        })
    }

    pub fn handle_instance_online(&mut self, msg: client::InstanceOnline) {
        let subscription = self.subscription(msg.instance());
        if msg.is_online() {
            self.client.subscribe(subscription);
            self.metadata
                .insert(msg.instance().to_owned(), HashSet::new());
        } else {
            self.client.unsubscribe(subscription);

            // On offline clear all metadata
            if let Some(paths) = self.metadata.remove(msg.instance()) {
                for path in paths {
                    self.emit(msg.instance(), &path, None);
                }
            }
        }
    }

    pub fn handle_message(&mut self, msg: client::Message) {
        let Some(topic) = msg.parse_topic() else {
            return;
        };

        if topic.domain != DOMAIN {
            return;
        }

        if topic.remaining.len() == 0 {
            log::warn!("Malformed metadata topic: '{}', ignored", msg.topic());
            return;
        };

        let path = topic.remaining;

        let Some(paths) = self.metadata.get_mut(topic.instance) else {
            log::warn!(
                "Got metadata update for non-existant instance: '{}', ignored",
                msg.topic()
            );
            return;
        };

        if msg.payload().is_empty() {
            paths.remove(path);
            self.emit(topic.instance, path, None);
        } else {
            paths.insert(path.to_owned());
            self.emit(topic.instance, path, Some(msg.payload()));
        }
    }

    fn subscription(&self, instance_name: &str) -> client::Subscription {
        client::TopicBuilder::remote(instance_name, DOMAIN).rest()
    }

    fn emit(&self, instance: &str, path: &str, value: Option<&Bytes>) {
        self.on_update.publish(RemoteMetadataUpdate {
            instance: Arc::new(instance.to_owned()),
            path: Arc::new(path.to_owned()),
            value: value.map(|value| Arc::new(value.clone())),
        });

        log::trace!(
            "{} metadata {}:{}",
            if value.is_some() { "set" } else { "clear" },
            instance,
            path
        );
    }
}

#[derive(Debug, Clone)]
struct LocalMetadataUpdate {
    path: String,
    value: Option<Bytes>,
}

#[derive(Debug, Clone)]
pub struct RemoteMetadataUpdate {
    instance: Arc<String>,
    path: Arc<String>,
    value: Option<Arc<Bytes>>,
}

impl RemoteMetadataUpdate {
    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn value(&self) -> Option<&Bytes> {
        self.value.as_deref()
    }
}
