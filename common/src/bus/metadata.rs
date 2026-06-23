use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bytes::Bytes;
use kameo::{message, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    bus::client::{self, ClientHandle, TopicBuilder},
    utils::actors::{
        ActorHandle, PublisherHandle, SpawnedActor, SpawnedActors, SubscriberHandle, spawn_pubsub,
    },
};

const DOMAIN: &str = "metadata";

const METADATA_NAME: &str = "bus.metadata";

/// Name of the PubSub actor that delivers remote metadata update
const REMOTE_UPDATE_PUBSUB_NAME: &str = "bus.metadata.remote-update";

#[derive(Debug)]
pub struct MetadataConfig {
    pub instance_name: Arc<String>,
    pub listen_remote: bool,
}

/// Client access to the metadata actor
#[derive(Debug, Clone)]
pub struct MetadataHandle {
    actor: ActorHandle<Metadata>,
    on_remote_update: SubscriberHandle<RemoteUpdate>,
}

impl MetadataHandle {
    /// Create a new access
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            actor: ActorHandle::from_name(METADATA_NAME)?,
            on_remote_update: SubscriberHandle::from_name(REMOTE_UPDATE_PUBSUB_NAME)?,
        })
    }

    /// Set metadata on the local instance
    pub fn set<T: Serialize>(&self, path: &str, value: &T) -> anyhow::Result<()> {
        let buff = serde_json::to_vec(value)?;

        self.actor.send(LocalUpdate {
            path: path.to_owned(),
            value: Some(Bytes::from_owner(buff)),
        });

        Ok(())
    }

    /// Clear metadata on the local instance
    pub fn clear(&self, path: &str) {
        self.actor.send(LocalUpdate {
            path: path.to_owned(),
            value: None,
        });
    }

    /// Get the PubSub for remote metadata update
    pub fn on_remote_update(&self) -> &SubscriberHandle<RemoteUpdate> {
        &self.on_remote_update
    }
}

pub async fn init_pubsubs(actors: &mut SpawnedActors) {
    actors.add(spawn_pubsub::<RemoteUpdate>(REMOTE_UPDATE_PUBSUB_NAME).await);
}

pub async fn init_actor(actors: &mut SpawnedActors, config: MetadataConfig) {
    let (metadata, _) = SpawnedActor::start::<Metadata>(config).await;

    metadata.register(METADATA_NAME);

    actors.add(metadata);
}

#[derive(Debug)]
struct Metadata {
    instance_name: Arc<String>,
    metadata: HashMap<String, Bytes>,
    remote: Option<Remote>,

    client: ClientHandle,
}

impl Actor for Metadata {
    type Args = MetadataConfig;
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let client = ClientHandle::new()?;

        let remote = if config.listen_remote {
            let remote = Remote::new(client.clone())?;

            client.on_instance_online().subscribe(actor_ref.clone());
            client.on_message().subscribe(actor_ref.clone());

            Some(remote)
        } else {
            None
        };

        Ok(Self {
            instance_name: config.instance_name,
            metadata: HashMap::new(),
            remote,
            client,
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

impl message::Message<LocalUpdate> for Metadata {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LocalUpdate,
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
    on_update: PublisherHandle<RemoteUpdate>,
    client: ClientHandle,
}

impl Remote {
    pub fn new(client: ClientHandle) -> anyhow::Result<Self> {
        Ok(Self {
            metadata: HashMap::new(),
            on_update: PublisherHandle::from_name(REMOTE_UPDATE_PUBSUB_NAME)?,
            client,
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
        self.on_update.publish(RemoteUpdate {
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
struct LocalUpdate {
    path: String,
    value: Option<Bytes>,
}

#[derive(Debug, Clone)]
pub struct RemoteUpdate {
    instance: Arc<String>,
    path: Arc<String>,
    value: Option<Arc<Bytes>>,
}

impl RemoteUpdate {
    pub fn instance(&self) -> &str {
        &self.instance
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub fn read_value<T: for<'a> Deserialize<'a>>(&self) -> anyhow::Result<T> {
        let Some(raw) = &self.value else {
            anyhow::bail!("no value");
        };

        let value = serde_json::from_slice::<T>(raw)?;

        Ok(value)
    }
}
