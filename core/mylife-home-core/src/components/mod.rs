use std::collections::HashMap;

use common::{
    components::registry::{self, RegistryHandle},
    utils::actors::{ActorHandle, SpawnedActor, SpawnedActors},
};
use kameo::{Actor, message};
use plugin_runtime::runtime::Config;

use crate::{
    components::local_component::{LocalComponentConfig, LocalComponentHandle},
    modules,
};

mod local_component;

const LOCAL_COMPONENTS_NAME: &str = "components.local";

/// Client access to the registry actor
#[derive(Debug, Clone)]
pub struct LocalComponentsHandle(ActorHandle<LocalComponents>);

impl LocalComponentsHandle {
    /// Create a new access
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self(ActorHandle::from_name(LOCAL_COMPONENTS_NAME)?))
    }

    /// Add a component, waiting for the manager's reply
    pub async fn component_add(
        &self,
        component_id: String,
        plugin_id: String,
        config: Config,
    ) -> anyhow::Result<()> {
        self.0
            .call(ComponentAdd {
                component_id,
                plugin_id,
                config,
            })
            .await?;

        Ok(())
    }

    /// Remove a component, waiting for the manager's reply
    pub async fn component_remove(&self, component_id: String) -> anyhow::Result<()> {
        self.0.call(ComponentRemove { component_id }).await?;

        Ok(())
    }
}

pub async fn init_actor(actors: &mut SpawnedActors) {
    let (local_components, _) = SpawnedActor::start::<LocalComponents>(()).await;

    local_components.register(LOCAL_COMPONENTS_NAME);

    actors.add(local_components);
}

pub async fn init_plugins() {
    // plugin are here forever, we can just register them
    let registry = registry::RegistryHandle::new().expect("Cannot get registry access");

    for plugin in modules::registry().plugins() {
        registry
            .plugin_add(None, plugin.metadata().clone())
            .await
            .expect("Could not registry plugin")
    }
}

struct LocalComponents {
    registry: RegistryHandle,
    components: HashMap<String, LocalComponentHandle>,
}

impl Actor for LocalComponents {
    type Args = ();

    type Error = anyhow::Error;

    async fn on_start(
        _args: Self::Args,
        _actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            registry: RegistryHandle::new()?,
            components: HashMap::new(),
        })
    }
}

impl message::Message<ComponentAdd> for LocalComponents {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: ComponentAdd,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let config = LocalComponentConfig {
            id: msg.component_id,
            plugin: msg.plugin_id,
            config: msg.config,
            registry: self.registry.clone(),
        };

        let id = config.id.clone();

        let component = LocalComponentHandle::start(config).await?;
        self.components.insert(id, component);

        Ok(())
    }
}

impl message::Message<ComponentRemove> for LocalComponents {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: ComponentRemove,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(component) = self.components.get(&msg.component_id) else {
            anyhow::bail!("component '{}' not found", msg.component_id);
        };

        component.terminate().await;
        self.components.remove(&msg.component_id);

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ComponentAdd {
    component_id: String,
    plugin_id: String,
    config: Config,
}

#[derive(Clone, Debug)]
struct ComponentRemove {
    component_id: String,
}
