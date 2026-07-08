use std::{collections::HashMap, io};

use bytes::Bytes;
use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    instance_info::InstanceInfoPublisherHandle,
    utils::{
        actors::{ActorHandle, CallError, HandleLookupError, SpawnedActor, SpawnedActors},
        config,
    },
};
use kameo::{
    Actor,
    actor::{ActorRef, WeakActorRef},
    error::ActorStopReason,
    message,
};
use maplit::hashmap;
use serde::Deserialize;
use thiserror::Error;
use tokio::fs;

use crate::model::builder::ModelBuildError;

mod builder;
mod definition;
mod rpc_services;

const MODEL_NAME: &str = "model";

#[derive(Debug, Clone, Deserialize)]
struct ModelConfig {
    pub store_path: String,
}

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
        definition: definition::Definition,
    ) -> Result<(), CallError<SetDefinitionError>> {
        self.0.call(SetDefinition(definition)).await?;

        Ok(())
    }

    // TODO: get resource, get model hash, get model
}

pub async fn init_actor(actors: &mut SpawnedActors) {
    let config = config::section::<ModelConfig>("model");

    let (model, _) = SpawnedActor::start::<Model>(config).await;

    model.register(MODEL_NAME);

    actors.add(model);
}

#[derive(Debug)]
pub struct Resource {
    mime: String,
    data: Bytes,
}

impl Resource {
    pub fn mime(&self) -> &str {
        &self.mime
    }

    pub fn data(&self) -> &Bytes {
        &self.data
    }
}

#[derive(Debug)]
struct RequiredComponentState {
    id: String,
    state: String,
}

#[derive(Debug)]
struct Model {
    store_path: String,
    rpc: RpcHandle,
    model_hash: String,
    resources: HashMap<String, Resource>,
    required_component_states: Vec<RequiredComponentState>,
}

#[derive(Debug, Error)]
enum ModelActorError {
    #[error("Failed to lookup actor handle: {0}")]
    HandleLookupError(#[from] HandleLookupError),
    #[error("Failed to add rpc service: {0}")]
    RpcServiceAddError(#[from] CallError<RpcServiceAddError>),
    #[error("Failed to remove rpc service: {0}")]
    RpcServiceRemoveError(#[from] CallError<RpcServiceRemoveError>),
    #[error("Failed to set definition: {0}")]
    ModelBuildError(#[from] ModelBuildError),
}

impl Actor for Model {
    type Args = ModelConfig;
    type Error = ModelActorError;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let instance_info = InstanceInfoPublisherHandle::new();

        let mut _self = Self {
            store_path: config.store_path,
            rpc: RpcHandle::new()?,
            model_hash: "".to_owned(),
            resources: HashMap::new(),
            required_component_states: Vec::new(),
        };

        _self.load().await?;

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

#[derive(Debug, Error)]
pub enum LoadDefinitionError {
    #[error("got io error while loading store: {0}")]
    IoError(#[from] io::Error),
    #[error("got deserialization error while loading store: {0}")]
    Deserialization(#[from] serde_json::Error),
}

impl Model {
    async fn load(&mut self) -> Result<(), ModelBuildError> {
        let definition = match self.load_definition().await {
            Ok(definition) => definition,
            Err(error) => {
                tracing::warn!(
                    ?error,
                    store_path = self.store_path,
                    "could not load model store, using default model"
                );
                Self::default_definition()
            }
        };

        self.set_definition(definition)
    }

    async fn load_definition(&self) -> Result<definition::Definition, LoadDefinitionError> {
        let content = fs::read_to_string(&self.store_path).await?;
        let definition: definition::Definition = serde_json::from_str(&content)?;

        Ok(definition)
    }

    fn default_definition() -> definition::Definition {
        definition::Definition {
            resources: vec![],
            windows: vec![definition::Window {
                id: "default-window".to_owned(),
                style: vec![],
                width: 300,
                height: 100,
                background_resource: None,
                controls: vec![definition::Control {
                    id: "default-control".to_owned(),
                    style: vec![],
                    x: 0,
                    y: 0,
                    width: 300,
                    height: 100,
                    display: None,
                    text: Some(definition::ControlText {
                        format: "return 'No definition has been set';".to_owned(),
                        context: vec![],
                    }),
                    primary_action: None,
                    secondary_action: None,
                }],
            }],
            default_window: hashmap! {
                "desktop".to_owned() => "default-window".to_owned(),
                "mobile".to_owned() => "default-window".to_owned(),
            },
            styles: vec![],
        }
    }
}

#[derive(Clone, Debug)]
struct SetDefinition(definition::Definition);

#[derive(Debug, Error)]
pub enum SetDefinitionError {
    #[error("error building model: {0}")]
    ModelBuildError(#[from]ModelBuildError),
    #[error("got io error while saving store: {0}")]
    IoError(#[from] io::Error),
    #[error("got serialization error while saving store: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl message::Message<SetDefinition> for Model {
    type Reply = Result<(), SetDefinitionError>;

    async fn handle(
        &mut self,
        msg: SetDefinition,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // prepare definition's json (set_definition consumes it)
        let content = serde_json::to_string_pretty(&msg.0)?;

        self.set_definition(msg.0)?;

        fs::write(&self.store_path, content).await?;

        Ok(())
    }
}

impl Model {
    fn set_definition(
        &mut self,
        definition: definition::Definition,
    ) -> Result<(), ModelBuildError> {
        let mut builder = builder::ModelBuilder::default();
        builder.build(definition)?;

        self.model_hash = builder.model_hash;
        self.resources = builder.resources;
        self.required_component_states = builder.required_component_states;

        Ok(())
    }
}
