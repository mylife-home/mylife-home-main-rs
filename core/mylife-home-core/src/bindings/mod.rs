use std::{collections::HashMap, fmt};

use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    components::registry::{self, RegistryHandle},
    instance_info::{self, InstanceInfoPublisherHandle},
    utils::actors::{ActorHandle, CallError, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{Actor, message, prelude::*};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{modules, store::StoreHandle};

mod rpc_services;

const BINDINGS_NAME: &str = "bindings";

/// Configuration to setup one binding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    pub source_id: String,
    pub source_state: String,
    pub target_id: String,
    pub target_action: String,
}

impl fmt::Display for BindingConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}.{} -> {}.{}",
            self.source_id, self.source_state, self.target_id, self.target_action
        ))
    }
}

/// Client access to the registry actor
#[derive(Debug, Clone)]
pub struct BindingsHandle(ActorHandle<Bindings>);

impl BindingsHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(BINDINGS_NAME)?))
    }

    fn from_actor_ref(actor_ref: ActorRef<Bindings>) -> Self {
        Self(ActorHandle::from_ref(actor_ref, BINDINGS_NAME))
    }

    /// Add a binding, waiting for the manager's reply
    pub async fn binding_add(
        &self,
        config: BindingConfig,
    ) -> Result<(), CallError<BindingAddError>> {
        self.0.call(BindingAdd(config)).await?;

        Ok(())
    }

    /// Remove a binding, waiting for the manager's reply
    pub async fn binding_remove(
        &self,
        config: BindingConfig,
    ) -> Result<(), CallError<BindingRemoveError>> {
        self.0.call(BindingRemove(config)).await?;

        Ok(())
    }

    /// Get bindings list
    pub async fn binding_list(&self) -> Result<Vec<BindingConfig>, CallError<CallError>> {
        let list = self.0.call(BindingList).await?;

        Ok(list)
    }
}

pub async fn init_actor(actors: &mut SpawnedActors) {
    let (local_bindings, _) = SpawnedActor::start::<Bindings>(()).await;

    local_bindings.register(BINDINGS_NAME);

    actors.add(local_bindings);
}

pub async fn init_plugins() {
    // plugin are here forever, we can just register them
    let registry = registry::RegistryHandle::new().expect("Cannot get registry access");
    let mut modules = HashMap::new();

    for plugin in modules::registry().plugins() {
        registry
            .plugin_add(None, plugin.metadata().clone())
            .await
            .expect("Could not registry plugin");

        let meta = plugin.metadata();
        modules.insert(meta.module(), meta.version());
    }

    let instance_info_handle = instance_info::InstanceInfoPublisherHandle::new();
    for (name, version) in modules {
        instance_info_handle.add_component(&format!("core-plugin.{}", name), version);
    }
}

struct Bindings {
    registry: RegistryHandle,
    rpc: RpcHandle,
    store: StoreHandle,
    bindings: HashMap<BindingKey, Binding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BindingKey {
    source_id: String,
    source_state: String,
    target_id: String,
    target_action: String,
}

impl From<BindingConfig> for BindingKey {
    fn from(value: BindingConfig) -> Self {
        Self {
            source_id: value.source_id,
            source_state: value.source_state,
            target_id: value.target_id,
            target_action: value.target_action,
        }
    }
}

impl fmt::Display for BindingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "{}.{} -> {}.{}",
            self.source_id, self.source_state, self.target_id, self.target_action
        ))
    }
}

#[derive(Debug)]
struct Binding {}

#[derive(Debug, Error)]
enum BindingsActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to add rpc service: {0}")]
    RpcServiceAddError(#[from] CallError<RpcServiceAddError>),
    #[error("Failed to remove rpc service: {0}")]
    RpcServiceRemoveError(#[from] CallError<RpcServiceRemoveError>),
    #[error("Failed to call store: {0}")]
    StoreError(#[source] CallError),
    #[error("failed to add component: {0}")]
    BindingAddError(#[from] BindingAddError),
}

impl Actor for Bindings {
    type Args = ();
    type Error = BindingsActorError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let instance_info = InstanceInfoPublisherHandle::new();

        let mut _self = Self {
            registry: RegistryHandle::new()?,
            rpc: RpcHandle::new()?,
            store: StoreHandle::new()?,
            bindings: HashMap::new(),
        };

        for config in _self
            .store
            .binding_list()
            .await
            .map_err(|e| BindingsActorError::StoreError(e))?
        {
            _self.add_binding(config).await?;
        }

        let self_handle = BindingsHandle::from_actor_ref(actor_ref);

        _self
            .rpc
            .register_service(
                "bindings.add",
                rpc_services::BindingAddRpcService::new(self_handle.clone()),
            )
            .await?;

        _self
            .rpc
            .register_service(
                "bindings.remove",
                rpc_services::BindingRemoveRpcService::new(self_handle.clone()),
            )
            .await?;

        _self
            .rpc
            .register_service(
                "bindings.list",
                rpc_services::BindingListRpcService::new(self_handle.clone()),
            )
            .await?;

        instance_info.add_capability("bindings-api");

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.bindings.clear();

        self.rpc.unregister_service("bindings.add").await?;
        self.rpc.unregister_service("bindings.remove").await?;
        self.rpc.unregister_service("bindings.list").await?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct BindingAdd(BindingConfig);

#[derive(Debug, Error)]
pub enum BindingAddError {
    #[error("binding '{0}' already exists")]
    AlreadyExists(String),
}

impl message::Message<BindingAdd> for Bindings {
    type Reply = Result<(), BindingAddError>;

    async fn handle(
        &mut self,
        msg: BindingAdd,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.add_binding(msg.0).await
    }
}

impl Bindings {
    async fn add_binding(&mut self, config: BindingConfig) -> Result<(), BindingAddError> {
        todo!();

        // create binding
        // initial link if components exist
        // save to store

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct BindingRemove(BindingConfig);

#[derive(Debug, Error)]
pub enum BindingRemoveError {
    #[error("binding '{0}' not found")]
    NotFound(BindingConfig),
}

impl message::Message<BindingRemove> for Bindings {
    type Reply = Result<(), BindingRemoveError>;

    async fn handle(
        &mut self,
        msg: BindingRemove,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if self.bindings.remove(&msg.0.clone().into()).is_none() {
            return Err(BindingRemoveError::NotFound(msg.0));
        };

        if let Err(error) = self.store.binding_clear(msg.0.clone()).await {
            tracing::error!(
                ?error,
                binding = %msg.0,
                "could not remove binding from store"
            );
        }

        tracing::debug!(binding = %msg.0, "binding removed");

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct BindingList;

impl message::Message<BindingList> for Bindings {
    type Reply = Result<Vec<BindingConfig>, CallError>;

    async fn handle(
        &mut self,
        _msg: BindingList,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // We rely on the store for the config + plugin instead of keeping a copy just for this call
        self.store.binding_list().await
    }
}
