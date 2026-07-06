use std::{collections::HashMap, io};

use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    instance_info::InstanceInfoPublisherHandle,
    utils::actors::CallError,
};
use kameo::{message, prelude::*};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;

use crate::{
    bindings::{BindingConfig, BindingKey},
    components::ComponentConfig,
};

use common::utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors};

mod rpc_services;

const STORE_NAME: &str = "store";

#[derive(Debug)]
pub struct StoreConfig {
    pub path: String,
    pub mount_point: Option<String>,
}

/// Client access to the store actor
#[derive(Debug, Clone)]
pub struct StoreHandle(ActorHandle<Store>);

impl StoreHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(STORE_NAME)?))
    }

    fn from_actor_ref(actor_ref: ActorRef<Store>) -> Self {
        Self(ActorHandle::from_ref(actor_ref, STORE_NAME))
    }

    /// Set a component in the store
    pub async fn component_set(&self, component: ComponentConfig) -> Result<(), CallError> {
        self.0.call(ComponentSet(component)).await
    }

    /// Clear (remove) a component in the store
    pub async fn component_clear(&self, id: &str) -> Result<(), CallError> {
        self.0.call(ComponentClear(id.to_owned())).await
    }

    /// List the components in the store
    pub async fn component_list(&self) -> Result<Vec<ComponentConfig>, CallError> {
        self.0.call(ComponentList).await
    }

    /// Set a binding in the store
    pub async fn binding_set(&self, binding: BindingConfig) -> Result<(), CallError> {
        self.0.call(BindingSet(binding)).await
    }

    /// Clear (remove) a binding in the store
    pub async fn binding_clear(&self, binding: BindingConfig) -> Result<(), CallError> {
        self.0.call(BindingClear(binding)).await
    }

    /// List the bindings in the store
    pub async fn binding_list(&self) -> Result<Vec<BindingConfig>, CallError> {
        self.0.call(BindingList).await
    }

    /// Save the store
    pub async fn save(&self) -> Result<(), CallError<SaveError>> {
        self.0.call(Save).await
    }
}

pub async fn init_actor(actors: &mut SpawnedActors, config: StoreConfig) {
    let (store, _) = SpawnedActor::start::<Store>(config).await;

    store.register(STORE_NAME);

    actors.add(store);
}

#[derive(Debug)]
struct Store {
    path: String,
    mount_point: Option<String>,
    rpc: RpcHandle,
    components: HashMap<String, ComponentConfig>,
    bindings: HashMap<BindingKey, BindingConfig>,
}

#[derive(Debug, Error)]
enum StoreActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to add rpc service: {0}")]
    RpcServiceAddError(#[from] CallError<RpcServiceAddError>),
    #[error("Failed to remove rpc service: {0}")]
    RpcServiceRemoveError(#[from] CallError<RpcServiceRemoveError>),
    #[error("Failed to load store: {0}")]
    LoadError(#[from] LoadError),
}

impl Actor for Store {
    type Args = StoreConfig;
    type Error = StoreActorError;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let instance_info = InstanceInfoPublisherHandle::new();

        let mut _self = Self {
            path: config.path,
            mount_point: config.mount_point,
            rpc: RpcHandle::new()?,
            components: HashMap::new(),
            bindings: HashMap::new(),
        };

        _self.load().await?;

        let self_handle = StoreHandle::from_actor_ref(actor_ref);

        _self
            .rpc
            .register_service(
                "store.save",
                rpc_services::SaveRpcService::new(self_handle.clone()),
            )
            .await?;

        instance_info.add_capability("store-api");

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.components.clear();
        self.bindings.clear();

        self.rpc.unregister_service("store.save").await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct ComponentSet(ComponentConfig);

impl message::Message<ComponentSet> for Store {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ComponentSet,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_component(msg.0);
    }
}

impl Store {
    fn set_component(&mut self, config: ComponentConfig) {
        let id = config.id.clone();
        self.components.insert(id, config);
    }
}

#[derive(Debug)]
pub struct ComponentClear(String);

impl message::Message<ComponentClear> for Store {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ComponentClear,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.components.remove(&msg.0);
    }
}

#[derive(Debug)]
pub struct ComponentList;

impl message::Message<ComponentList> for Store {
    type Reply = Vec<ComponentConfig>;

    async fn handle(
        &mut self,
        _msg: ComponentList,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.components.values().cloned().collect()
    }
}

#[derive(Debug)]
pub struct BindingSet(BindingConfig);

impl message::Message<BindingSet> for Store {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: BindingSet,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_binding(msg.0);
    }
}

impl Store {
    fn set_binding(&mut self, config: BindingConfig) {
        let id = config.clone().into();
        self.bindings.insert(id, config);
    }
}

#[derive(Debug)]
pub struct BindingClear(BindingConfig);

impl message::Message<BindingClear> for Store {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: BindingClear,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.bindings.remove(&msg.0.into());
    }
}

#[derive(Debug)]
pub struct BindingList;

impl message::Message<BindingList> for Store {
    type Reply = Vec<BindingConfig>;

    async fn handle(
        &mut self,
        _msg: BindingList,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.bindings.values().cloned().collect()
    }
}

#[derive(Debug)]
pub struct Save;

impl message::Message<Save> for Store {
    type Reply = Result<(), SaveError>;

    async fn handle(&mut self, _msg: Save, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        self.save().await
    }
}

#[derive(Debug, Error)]
pub enum LoadError {
    #[error("got io error while loading store: {0}")]
    Io(#[from] io::Error),
    #[error("got deserialization error while loading store: {0}")]
    Deserialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("got io error while saving store: {0}")]
    Io(#[from] io::Error),
    #[error("got serialization error while saving store: {0}")]
    Deserialization(#[from] serde_json::Error),
}

impl Store {
    async fn load(&mut self) -> Result<(), LoadError> {
        let content = fs::read_to_string(&self.path).await?;
        let items: Vec<FileItem> = serde_json::from_str(&content)?;

        for item in items {
            match item {
                FileItem::Binding(config) => self.set_binding(config),
                FileItem::Component(config) => self.set_component(config),
            }
        }

        Ok(())
    }

    async fn save(&self) -> Result<(), SaveError> {
        let mut items = Vec::with_capacity(self.bindings.len() + self.components.len());

        for (_, config) in &self.bindings {
            items.push(FileItemRef::Binding(&config));
        }

        for (_, config) in &self.components {
            items.push(FileItemRef::Component(&config));
        }

        let content = serde_json::to_string_pretty(&items)?;
        fs::write(&self.path, content).await?;

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "config", rename_all = "kebab-case")]
enum FileItem {
    Binding(BindingConfig),
    Component(ComponentConfig),
}

/// Serialization version of FileItem which capture items by ref to avoid to copy
#[derive(Serialize)]
#[serde(tag = "type", content = "config", rename_all = "kebab-case")]
enum FileItemRef<'a> {
    Binding(&'a BindingConfig),
    Component(&'a ComponentConfig),
}
