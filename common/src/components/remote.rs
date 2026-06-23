use std::{collections::HashMap, sync::Arc};

use kameo::{message, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    bus::metadata::{self, MetadataHandle, RemoteUpdate},
    components::{
        metadata::PluginMetadata,
        registry::{self, RegistryHandle},
    },
    utils::actors::{SpawnedActor, SpawnedActors},
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

    remote_plugins: HashMap<RemoteKey, RemotePluginData>,
    remote_components: HashMap<RemoteKey, RemoteComponentData>,
    remote_pending_components: Vec<RemotePendingComponent>,
}

impl Actor for Remote {
    type Args = ();
    type Error = anyhow::Error;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let metadata = MetadataHandle::new()?;
        let registry = RegistryHandle::new()?;

        metadata.on_remote_update().subscribe(actor_ref.clone());
        registry.on_update().subscribe(actor_ref);

        Ok(Self {
            metadata,
            registry,
            remote_plugins: HashMap::new(),
            remote_components: HashMap::new(),
            remote_pending_components: Vec::new(),
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.remote_plugins.clear();
        self.remote_pending_components.clear();

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
                    self.add_remote_plugin(id, &msg).await;
                } else {
                    self.remove_remote_plugin(id, &msg).await;
                }
            }

            "components" => {
                if msg.has_value() {
                    self.add_remote_component(id, &msg).await;
                } else {
                    self.remove_remote_component(id, &msg).await;
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
    async fn add_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) {
        let plugin = match msg.read_value::<PluginMetadata>() {
            Ok(plugin) => Arc::new(plugin),
            Err(e) => {
                log::error!(
                    "Cannot read plugin metadata: '{}:{}': {}",
                    msg.instance(),
                    id,
                    e
                );
                return;
            }
        };
        if plugin.id() != id {
            log::error!(
                "plugin id mismatch: (metadata key = '{}', metadata value = '{}'",
                id,
                plugin.id()
            );
            return;
        }

        let key = RemoteKey {
            instance: msg.instance().to_owned(),
            id: id.to_owned(),
        };

        if let Err(e) = self
            .registry
            .plugin_add(Some(key.instance.clone()), plugin.clone())
            .await
        {
            log::error!(
                "cannot add plugin to registry '{}:{}': {}",
                msg.instance(),
                id,
                e
            );
            return;
        }

        // usage 1 = created
        self.remote_plugins
            .insert(key, RemotePluginData { usage: 1 });

        // Check if we have pending components
        let pendings: Vec<_> = self
            .remote_pending_components
            .extract_if(.., |comp| {
                comp.instance == msg.instance() && comp.plugin_id == id
            })
            .collect();

        for pending in pendings {
            log::trace!("create pending component {:?}", pending);

            self.do_add_component(pending.instance, pending.plugin_id, pending.id)
                .await;
        }
    }

    async fn remove_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) {
        self.unref_plugin(msg.instance(), id).await;
    }

    async fn add_remote_component(&mut self, id: &str, msg: &RemoteUpdate) {
        let component = match msg.read_value::<ComponentMetadata>() {
            Ok(component) => component,
            Err(e) => {
                log::error!(
                    "Cannot read component metadata: '{}:{}': {}",
                    msg.instance(),
                    id,
                    e
                );
                return;
            }
        };
        if component.id != id {
            log::error!(
                "component id mismatch: (metadata key = '{}', metadata value = '{}'",
                id,
                component.id
            );
            return;
        }

        let plugin_key = RemoteKey {
            instance: msg.instance().to_owned(),
            id: component.plugin,
        };

        if !self.remote_plugins.contains_key(&plugin_key) {
            let pending = RemotePendingComponent {
                instance: plugin_key.instance,
                id: component.id,
                plugin_id: plugin_key.id,
            };

            log::trace!("add pending component: {:?}", pending);
            self.remote_pending_components.push(pending);
            return;
        };

        self.do_add_component(plugin_key.instance, plugin_key.id, component.id)
            .await;
    }

    async fn remove_remote_component(&mut self, id: &str, msg: &RemoteUpdate) {
        let key = RemoteKey {
            instance: msg.instance().to_owned(),
            id: id.to_owned(),
        };

        if let Err(e) = self
            .registry
            .component_remove(Some(key.instance.clone()), key.id.clone())
            .await
        {
            log::error!(
                "cannot remove component from registry '{}:{}': {}",
                msg.instance(),
                id,
                e
            );
            return;
        }

        let Some(component) = self.remote_components.remove(&key) else {
            log::error!(
                "data inconsistency: registry remove component but not found locally: '{}:{}'",
                key.instance,
                key.id
            );
            return;
        };

        self.unref_plugin(&key.instance, &component.plugin_id).await;
    }

    async fn do_add_component(
        &mut self,
        instance: String,
        plugin_id: String,
        component_id: String,
    ) {
        if let Err(e) = self
            .registry
            .component_add(
                Some(instance.clone()),
                plugin_id.clone(),
                component_id.clone(),
            )
            .await
        {
            log::error!(
                "cannot add component to registry '{}:{}': {}",
                instance,
                component_id,
                e
            );
            return;
        }

        self.remote_components.insert(
            RemoteKey {
                instance,
                id: component_id,
            },
            RemoteComponentData { plugin_id },
        );
    }

    async fn unref_plugin(&mut self, instance: &str, id: &str) {
        let key = RemoteKey {
            instance: instance.to_owned(),
            id: id.to_owned(),
        };

        let Some(data) = self.remote_plugins.get_mut(&key) else {
            log::error!(
                "cannot remove non existant plugin '{}:{}'",
                key.instance,
                key.id,
            );
            return;
        };

        if data.usage == 0 {
            log::error!("plugin '{}:{}' usage is 0", key.instance, key.id);
            return;
        }

        data.usage -= 1;

        let Some(data) = self.remote_plugins.get_mut(&key) else {
            log::error!("plugin not found '{}:{}'", key.instance, key.id);
            return;
        };

        if data.usage == 0 {
            if let Err(e) = self
                .registry
                .plugin_remove(Some(key.instance.clone()), key.id.clone())
                .await
            {
                log::error!(
                    "cannot remove plugin from registry '{}:{}': {}",
                    instance,
                    id,
                    e
                );
                return;
            }

            self.remote_plugins.remove(&key);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentMetadata {
    pub id: String,
    pub plugin: String,
}

#[derive(Debug)]
struct RemotePendingComponent {
    instance: String,
    id: String,
    plugin_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RemoteKey {
    instance: String,
    id: String,
}

#[derive(Debug)]
struct RemotePluginData {
    usage: usize,
}

#[derive(Debug)]
struct RemoteComponentData {
    plugin_id: String,
}
