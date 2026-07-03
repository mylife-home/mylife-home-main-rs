use std::collections::HashMap;

use common::utils::actors::CallError;
use kameo::{error::Infallible, message, prelude::*};
use thiserror::Error;

use crate::{
    bindings::{BindingConfig, BindingKey},
    components::ComponentConfig,
};

use common::utils::actors::{ActorHandle, HandleLookupError, SpawnedActor, SpawnedActors};

const STORE_NAME: &str = "store";

#[derive(Debug)]
pub struct StoreConfig {}

/// Client access to the store actor
#[derive(Debug, Clone)]
pub struct StoreHandle(ActorHandle<Store>);

impl StoreHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(STORE_NAME)?))
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
}

pub async fn init_actor(actors: &mut SpawnedActors, config: StoreConfig) {
    let (store, _) = SpawnedActor::start::<Store>(config).await;

    store.register(STORE_NAME);

    actors.add(store);
}

#[derive(Debug)]
struct Store {
    components: HashMap<String, ComponentConfig>,
    bindings: HashMap<BindingKey, BindingConfig>,
}

impl Actor for Store {
    type Args = StoreConfig;
    type Error = Infallible;

    async fn on_start(config: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let components = HashMap::new();
        let bindings = HashMap::new();

        // TODO: load

        Ok(Self {
            components,
            bindings,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
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
        let id = msg.0.id.clone();
        self.components.insert(id, msg.0);
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
        let id = msg.0.clone().into();
        self.bindings.insert(id, msg.0);
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

#[derive(Debug, Error)]
pub enum SaveError {}

impl message::Message<Save> for Store {
    type Reply = Result<(), SaveError>;

    async fn handle(&mut self, msg: Save, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        todo!()
    }
}

// TODO: store.load + rpc service for store.save + capability
