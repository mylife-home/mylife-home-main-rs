use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    instance_info::InstanceInfoPublisherHandle,
    utils::actors::{ActorHandle, CallError, HandleLookupError, SpawnedActor, SpawnedActors},
};
use kameo::{
    Actor,
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message,
};
use thiserror::Error;

use crate::model::definition::Definition;

mod definition;
mod rpc_services;

const MODEL_NAME: &str = "model";

/// Client access to the registry actor
#[derive(Debug, Clone)]
pub struct ModelHandle(ActorHandle<Model>);

impl ModelHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self(ActorHandle::from_name(MODEL_NAME)?))
    }

    fn from_actor_ref(actor_ref: ActorRef<Model>) -> Self {
        Self(ActorHandle::from_ref(actor_ref, MODEL_NAME))
    }

    /// Set the definition
    pub async fn set_definition(
        &self,
        definition: Definition,
    ) -> Result<(), CallError<SetDefinitionError>> {
        self.0.call(SetDefinition(definition)).await?;

        Ok(())
    }

    // TODO: get resource, get model hash, get model
}

pub async fn init_actor(actors: &mut SpawnedActors) {
    let (model, _) = SpawnedActor::start::<Model>(()).await;

    model.register(MODEL_NAME);

    actors.add(model);
}

#[derive(Debug)]
struct Model {
    rpc: RpcHandle,
}

#[derive(Debug, Error)]
enum ModelActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to add rpc service: {0}")]
    RpcServiceAddError(#[from] CallError<RpcServiceAddError>),
    #[error("Failed to remove rpc service: {0}")]
    RpcServiceRemoveError(#[from] CallError<RpcServiceRemoveError>),
}

impl Actor for Model {
    type Args = ();
    type Error = ModelActorError;

    async fn on_start(_args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let instance_info = InstanceInfoPublisherHandle::new();

        let mut _self = Self {
            rpc: RpcHandle::new()?,
        };

        let self_handle = ModelHandle::from_actor_ref(actor_ref);

        _self
            .rpc
            .register_service(
                "definition.set",
                rpc_services::DefinitionSetRpcService::new(self_handle.clone()),
            )
            .await?;

        instance_info.add_capability("ui-api");

        Ok(_self)
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.rpc.unregister_service("definition.set").await?;

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct SetDefinition(Definition);

#[derive(Debug, Error)]
pub enum SetDefinitionError {}

impl message::Message<SetDefinition> for Model {
    type Reply = Result<(), SetDefinitionError>;

    async fn handle(
        &mut self,
        msg: SetDefinition,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.set_definition(msg.0).await
    }
}

impl Model {
    async fn set_definition(&mut self, definition: Definition) -> Result<(), SetDefinitionError> {
        todo!()
    }
}
