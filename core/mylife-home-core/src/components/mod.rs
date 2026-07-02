use std::collections::HashMap;

use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    components::registry::{self, RegistryHandle},
    instance_info::InstanceInfoPublisherHandle,
    utils::actors::{ActorHandle, CallError, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{Actor, message, prelude::*};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    components::local_component::{
        ComponentStartError, LocalComponentConfig, LocalComponentHandle, RawConfig,
    },
    modules,
    store::StoreHandle,
};

mod local_component;
mod rpc_services;

const LOCAL_COMPONENTS_NAME: &str = "components.local";

/// Configuration to setup one component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    pub id: String,
    pub plugin: String,
    pub config: RawConfig,
}

/// Client access to the registry actor
#[derive(Debug, Clone)]
pub struct LocalComponentsHandle(ActorHandle<LocalComponents>);

impl LocalComponentsHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(LOCAL_COMPONENTS_NAME)?))
    }

    fn from_actor_ref(actor_ref: ActorRef<LocalComponents>) -> Self {
        Self(ActorHandle::from_ref(actor_ref, LOCAL_COMPONENTS_NAME))
    }

    /// Add a component, waiting for the manager's reply
    pub async fn component_add(
        &self,
        component_id: String,
        plugin_id: String,
        config: RawConfig,
    ) -> Result<(), CallError<LocalComponentAddError>> {
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
    pub async fn component_remove(
        &self,
        component_id: String,
    ) -> Result<(), CallError<LocalComponentRemoveError>> {
        self.0.call(ComponentRemove { component_id }).await?;

        Ok(())
    }

    /// Get components list
    pub async fn component_list(&self) -> Result<Vec<ComponentConfig>, CallError<CallError>> {
        let list = self.0.call(ComponentList).await?;

        Ok(list)
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
    rpc: RpcHandle,
    store: StoreHandle,
    components: HashMap<String, LocalComponentHandle>,
}

#[derive(Debug, Error)]
enum LocalComponentsActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to add rpc service: {0}")]
    RpcServiceAddError(#[from] CallError<RpcServiceAddError>),
    #[error("Failed to remove rpc service: {0}")]
    RpcServiceRemoveError(#[from] CallError<RpcServiceRemoveError>),
}

impl Actor for LocalComponents {
    type Args = ();
    type Error = LocalComponentsActorError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let instance_info = InstanceInfoPublisherHandle::new();

        let _self = Self {
            registry: RegistryHandle::new()?,
            rpc: RpcHandle::new()?,
            store: StoreHandle::new()?,
            components: HashMap::new(),
        };

        // TODO: load components

        let self_handle = LocalComponentsHandle::from_actor_ref(actor_ref);

        _self
            .rpc
            .register_service(
                "components.add",
                rpc_services::ComponentAddRpcService::new(self_handle.clone()),
            )
            .await?;

        _self
            .rpc
            .register_service(
                "components.remove",
                rpc_services::ComponentRemoveRpcService::new(self_handle.clone()),
            )
            .await?;

        _self
            .rpc
            .register_service(
                "components.list",
                rpc_services::ComponentListRpcService::new(self_handle.clone()),
            )
            .await?;

        instance_info.add_capability("components-api");

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.components.clear();

        self.rpc.unregister_service("components.add").await?;
        self.rpc.unregister_service("components.remove").await?;
        self.rpc.unregister_service("components.list").await?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ComponentAdd {
    component_id: String,
    plugin_id: String,
    config: RawConfig,
}

#[derive(Debug, Error)]
pub enum LocalComponentAddError {
    #[error("component '{0}' already exists")]
    AlreadyExists(String),

    #[error(transparent)]
    ComponentStartError(#[from] ComponentStartError),
}

impl message::Message<ComponentAdd> for LocalComponents {
    type Reply = Result<(), LocalComponentAddError>;

    async fn handle(
        &mut self,
        msg: ComponentAdd,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let config = LocalComponentConfig {
            id: msg.component_id.clone(),
            plugin: msg.plugin_id.clone(),
            config: msg.config.clone(),
            registry: self.registry.clone(),
        };

        let id = config.id.clone();

        if self.components.contains_key(&id) {
            return Err(LocalComponentAddError::AlreadyExists(id));
        }

        let component = LocalComponentHandle::start(config).await?;
        self.components.insert(id, component);

        if let Err(error) = self
            .store
            .component_set(ComponentConfig {
                id: msg.component_id.clone(),
                plugin: msg.plugin_id,
                config: msg.config,
            })
            .await
        {
            tracing::error!(
                ?error,
                component_id = msg.component_id,
                "could not add component to store"
            );
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ComponentRemove {
    component_id: String,
}

#[derive(Debug, Error)]
pub enum LocalComponentRemoveError {
    #[error("component '{0}' not found")]
    NotFound(String),
}

impl message::Message<ComponentRemove> for LocalComponents {
    type Reply = Result<(), LocalComponentRemoveError>;

    async fn handle(
        &mut self,
        msg: ComponentRemove,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(component) = self.components.get(&msg.component_id) else {
            return Err(LocalComponentRemoveError::NotFound(msg.component_id));
        };

        component.terminate().await;
        self.components.remove(&msg.component_id);

        if let Err(error) = self.store.component_clear(&msg.component_id).await {
            tracing::error!(
                ?error,
                component_id = msg.component_id,
                "could not remove component from store"
            );
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ComponentList;

impl message::Message<ComponentList> for LocalComponents {
    type Reply = Result<Vec<ComponentConfig>, CallError>;

    async fn handle(
        &mut self,
        _msg: ComponentList,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // We rely on the store for the config + plugin instead of keeping a copy just for this call
        self.store.component_list().await
    }
}
