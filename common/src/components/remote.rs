use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use bytes::Bytes;
use kameo::{message, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    bus::{
        client::{self, ClientHandle, TopicBuilder},
        metadata::{self, MetadataHandle, RemoteUpdate},
    },
    components::{
        metadata::PluginMetadata,
        registry::{self, RegistryHandle},
    },
    utils::actors::{PublisherHandle, SpawnedActor, SpawnedActors},
};

const REMOTE_NAME: &str = "bus.remote";

pub async fn init_actor(actors: &mut SpawnedActors) {
    let (remote, _) = SpawnedActor::start::<Remote>(()).await;

    remote.register(REMOTE_NAME);

    actors.add(remote);
}

#[derive(Debug)]
struct Remote {
    metadata: MetadataHandle,
    registry: RegistryHandle,
}

impl Actor for Remote {
    type Args = ();
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;
        let registry = RegistryHandle::new()?;

        metadata.on_remote_update().subscribe(actor_ref.clone());
        registry.on_update().subscribe(actor_ref);

        Ok(Self { metadata, registry })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl message::Message<metadata::RemoteUpdate> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: metadata::RemoteUpdate,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut parts = msg.path().splitn(2, '/');
        let Some(typ) = parts.next() else {
            return;
        };
        let Some(id) = parts.next() else {
            return;
        };

        match typ {
            "plugins" => {
                if msg.has_value() {
                    if let Err(e) = self.add_remote_plugin(id, &msg) {
                        log::error!("add plugin '{}:{}' failed: {}", msg.instance(), id, e);
                    }
                } else {
                    if let Err(e) = self.remove_remote_plugin(id, &msg) {
                        log::error!("remove plugin '{}:{}' failed: {}", msg.instance(), id, e);
                    }
                }
            }

            "components" => {
                if msg.has_value() {
                    if let Err(e) = self.add_remote_component(id, &msg) {
                        log::error!("add component '{}:{}' failed: {}", msg.instance(), id, e);
                    }
                } else {
                    if let Err(e) = self.remove_remote_component(id, &msg) {
                        log::error!("remove component '{}:{}' failed: {}", msg.instance(), id, e);
                    }
                }
            }

            _ => {}
        }
    }
}

impl message::Message<registry::RegistryUpdated> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: registry::RegistryUpdated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // TODO
    }
}

impl Remote {
    fn add_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) -> anyhow::Result<()> {
        let plugin = Arc::new(msg.read_value::<PluginMetadata>()?);
        log::debug!("NEW PLUGIN: {}:{} -> {:?}", msg.instance(), id, plugin);

        Ok(())
    }

    fn remove_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) -> anyhow::Result<()> {
        Ok(())
    }

    fn add_remote_component(&mut self, id: &str, msg: &RemoteUpdate) -> anyhow::Result<()> {
        let component = msg.read_value::<ComponentMetadata>()?;
        log::debug!(
            "NEW COMPONENT: {}:{} -> {:?}",
            msg.instance(),
            id,
            component
        );

        Ok(())
    }

    fn remove_remote_component(&mut self, id: &str, msg: &RemoteUpdate) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentMetadata {
    pub id: String,
    pub plugin: String,
}

