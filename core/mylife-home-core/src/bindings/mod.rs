use std::{collections::HashMap, fmt};

use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    components::registry::{
        self, ComponentGetError, ComponentGetErrorKind, ComponentInfo, RegistryHandle,
    },
    instance_info::InstanceInfoPublisherHandle,
    utils::actors::{
        ActorHandle,
        CallError::{self, HandlerError},
        HandleLookupError, SpawnedActor, SpawnedActors,
    },
};
use kameo::{Actor, message, prelude::*};
use plugin_runtime::runtime::Value;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::store::StoreHandle;

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
    // pub fn new() -> Result<Self, HandleLookupError> {
    //     Ok(Self(ActorHandle::from_name(BINDINGS_NAME)?))
    // }

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
    let (bindings, _) = SpawnedActor::start::<Bindings>(()).await;

    bindings.register(BINDINGS_NAME);

    actors.add(bindings);
}

struct Bindings {
    registry: RegistryHandle,
    rpc: RpcHandle,
    store: StoreHandle,
    bindings: HashMap<BindingKey, Binding>,
}

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

impl message::Message<registry::RegistryUpdated> for Bindings {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: registry::RegistryUpdated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            registry::RegistryUpdated::ComponentStateChanged(msg) => {
                for binding in self.bindings.values_mut() {
                    binding.process_state_change(&msg);
                }
            }

            registry::RegistryUpdated::ComponentAdded(msg) => {
                for binding in self.bindings.values_mut() {
                    binding.component_added(msg.component_id());
                }
            }

            registry::RegistryUpdated::ComponentRemoved(msg) => {
                for binding in self.bindings.values_mut() {
                    binding.component_removed(msg.component_id());
                }
            }

            _ => {}
        }
    }
}

#[derive(Clone, Debug)]
struct BindingAdd(BindingConfig);

#[derive(Debug, Error)]
pub enum BindingAddError {
    #[error("binding '{0}' already exists")]
    AlreadyExists(BindingConfig),
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
        let key = config.clone().into();

        if self.bindings.contains_key(&key) {
            return Err(BindingAddError::AlreadyExists(config));
        }

        let mut entry = self
            .bindings
            .entry(key)
            .insert_entry(Binding::new(self.registry.clone(), &config));
        let binding = entry.get_mut();

        binding.init().await;

        if let Err(error) = self.store.binding_set(config.clone()).await {
            tracing::error!(
                ?error,
                binding = %config,
                "could not add binding to store"
            );
        }
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
struct Binding {
    registry: RegistryHandle,
    source_id: String,
    source_state: String,
    target_id: String,
    target_action: String,
    source_value: Option<Value>, // only if online
    target_online: bool,
}

impl Binding {
    pub fn new(registry: RegistryHandle, config: &BindingConfig) -> Self {
        Self {
            registry,
            source_id: config.source_id.clone(),
            source_state: config.source_state.clone(),
            target_id: config.target_id.clone(),
            target_action: config.target_action.clone(),
            source_value: None,
            target_online: false,
        }
    }

    /// Run the initial linking
    pub async fn init(&mut self) {
        self.init_source_value().await;
        self.init_target_online().await;
        self.apply_binding();
    }

    async fn init_source_value(&mut self) {
        let source_component = match self.find_component(&self.source_id).await {
            Ok(comp) => comp,
            Err(error) => {
                tracing::error!(
                    ?error,
                    component_id = self.source_id,
                    "failed to lookup for source component"
                );
                return;
            }
        };

        if let Some(source_component) = source_component {
            let Some(value) = source_component.state.get(&self.source_state) else {
                tracing::error!(
                    component_id = self.source_id,
                    state_name = self.source_state,
                    "no such state on component"
                );
                return;
            };

            // here value = None is OK: the component state has not been set for now
            self.source_value = value.clone();
        } else {
            self.source_value = None;
        }
    }

    async fn init_target_online(&mut self) {
        let target_component = match self.find_component(&self.target_id).await {
            Ok(comp) => comp,
            Err(error) => {
                tracing::error!(
                    ?error,
                    component_id = self.target_id,
                    "failed to lookup for target component"
                );
                return;
            }
        };

        self.target_online = target_component.is_some();
    }

    async fn find_component(
        &self,
        id: &String,
    ) -> Result<Option<ComponentInfo>, CallError<ComponentGetError>> {
        match self.registry.get_component(id.clone()).await {
            Ok(component) => Ok(Some(component)),
            Err(err) => {
                if let HandlerError(err) = &err
                    && matches!(err.kind(), ComponentGetErrorKind::NotFound)
                {
                    return Ok(None);
                }

                return Err(err);
            }
        }
    }

    pub fn process_state_change(&mut self, msg: &registry::ComponentStateChanged) {
        if msg.component_id() == self.source_id && msg.state() == self.source_state {
            self.source_value = Some(msg.value().clone());
            self.apply_binding();
        }
    }

    pub fn component_added(&mut self, id: &str) {
        if id == self.target_id {
            self.target_online = true;
            self.apply_binding();
        }

        // source_value set from process_state_change
    }

    pub fn component_removed(&mut self, id: &str) {
        if id == self.source_id {
            self.source_value = None;
        }

        if id == self.target_id {
            self.target_online = false
        }
    }

    fn apply_binding(&self) {
        if self.target_online
            && let Some(value) = &self.source_value
        {
            self.registry.component_execute_action(
                self.target_id.clone(),
                self.target_action.clone(),
                value.clone(),
            );
        }
    }
}
