use std::{collections::HashMap, sync::Arc};

use bytes::Bytes;
use kameo::{message, prelude::*};

use crate::bus::client::{self, ClientHandle, TopicBuilder};

const DOMAIN: &str = "metadata";

pub struct MetadataConfig {
    pub instance_name: Arc<String>,
}

#[derive(Debug)]
pub struct Metadata {
    instance_name: Arc<String>,
    metadata: HashMap<String, Bytes>,

    client: ClientHandle,
}

impl Actor for Metadata {
    type Args = MetadataConfig;
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            instance_name: config.instance_name,
            metadata: HashMap::new(),
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
        // TODO
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

impl message::Message<Set> for Metadata {
    type Reply = ();

    async fn handle(&mut self, msg: Set, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.metadata.insert(msg.path.clone(), msg.value.clone());
        self.publish(&msg.path, Some(msg.value));
    }
}

impl message::Message<Clear> for Metadata {
    type Reply = ();

    async fn handle(&mut self, msg: Clear, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        if self.metadata.remove(&msg.path).is_some() {
            self.publish(&msg.path, None);
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

#[derive(Debug, Clone)]
struct Set {
    path: String,
    value: Bytes,
}

#[derive(Debug, Clone)]
struct Clear {
    path: String,
}
