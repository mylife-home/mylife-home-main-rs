use std::{collections::HashMap, io, sync::Arc};

use bytes::Bytes;
use common::{
    bus::rpc::{RpcHandle, RpcServiceAddError, RpcServiceRemoveError},
    instance_info::InstanceInfoPublisherHandle,
    utils::{
        actors::{
            ActorHandle, CallError, HandleLookupError, PublisherHandle, SpawnedActor,
            SpawnedActors, SubscriberHandle, spawn_pubsub,
        },
        config,
    },
};
use kameo::{
    Actor,
    actor::{ActorRef, WeakActorRef},
    error::{ActorStopReason, Infallible},
    message,
};
use kameo_actors::pubsub;
use maplit::hashmap;
use serde::Deserialize;
use thiserror::Error;
use tokio::fs;

use crate::model::builder::ModelBuildError;

mod builder;
mod definition;
mod rpc_services;

const MODEL_NAME: &str = "model";

/// Name of the PubSub actor that delivers model update
const MODEL_UPDATE_PUBSUB_NAME: &str = "model.update";

#[derive(Debug, Clone, Deserialize)]
struct ModelConfig {
    pub store_path: String,
}

/// Client access to the registry actor
#[derive(Debug, Clone)]
pub struct ModelHandle {
    actor: ActorHandle<Model>,
    on_update: SubscriberHandle<ModelUpdate>,
}

impl ModelHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self {
            actor: ActorHandle::from_name(MODEL_NAME)?,
            on_update: SubscriberHandle::from_name(MODEL_UPDATE_PUBSUB_NAME)?,
        })
    }

    fn from_actor_ref(
        actor_ref: ActorRef<Model>,
        on_update: ActorHandle<pubsub::PubSub<ModelUpdate>>,
    ) -> Self {
        Self {
            actor: ActorHandle::from_ref(actor_ref, MODEL_NAME),
            on_update: on_update.into(),
        }
    }

    /// Set the definition
    pub async fn set_definition(
        &self,
        definition: definition::Definition,
    ) -> Result<(), CallError<SetDefinitionError>> {
        self.actor.call(SetDefinition(definition)).await?;

        Ok(())
    }

    pub async fn get_resource(
        &self,
        hash: impl Into<String>,
    ) -> Result<Resource, CallError<GetResourceError>> {
        self.actor.call(GetResource(hash.into())).await
    }

    pub async fn get_model(&self) -> Result<ModelUpdate, CallError> {
        self.actor.call(GetModel).await
    }

    pub fn on_update(&self) -> &SubscriberHandle<ModelUpdate> {
        &self.on_update
    }
}

pub async fn init_pubsubs(actors: &mut SpawnedActors) {
    actors.add(spawn_pubsub::<ModelUpdate>(MODEL_UPDATE_PUBSUB_NAME).await);
}

pub async fn init_actor(actors: &mut SpawnedActors) {
    let config = config::section::<ModelConfig>("model");

    let (model, _) = SpawnedActor::start::<Model>(config).await;

    model.register(MODEL_NAME);

    actors.add(model);
}

#[derive(Debug, Clone)]
pub struct Resource {
    mime: Arc<String>,
    data: Arc<Bytes>,
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
    on_update: PublisherHandle<ModelUpdate>,

    model_hash: Arc<String>,
    required_component_states: Arc<[RequiredComponentState]>,
    resources: HashMap<String, Resource>,
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
            on_update: PublisherHandle::from_name(MODEL_UPDATE_PUBSUB_NAME)?,
            model_hash: Arc::new(String::new()),
            required_component_states: Vec::new().into_boxed_slice().into(),
            resources: HashMap::new(),
        };

        _self.load().await?;

        let self_handle = ModelHandle::from_actor_ref(actor_ref, _self.on_update.clone().into());

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
    ModelBuildError(#[from] ModelBuildError),
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

        self.model_hash = Arc::new(builder.model_hash);
        self.required_component_states =
            builder.required_component_states.into_boxed_slice().into();
        self.resources = builder.resources;

        self.on_update.publish(ModelUpdate::new(
            self.model_hash.clone(),
            self.required_component_states.clone(),
        ));

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct GetResource(String);

#[derive(Debug, Error)]
pub enum GetResourceError {
    #[error("resource not found '{0}")]
    NotFound(String),
}

impl message::Message<GetResource> for Model {
    type Reply = Result<Resource, GetResourceError>;

    async fn handle(
        &mut self,
        msg: GetResource,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(self
            .resources
            .get(&msg.0)
            .ok_or_else(|| GetResourceError::NotFound(msg.0.clone()))?
            .clone())
    }
}

#[derive(Clone, Debug)]
struct GetModel;

impl message::Message<GetModel> for Model {
    type Reply = Result<ModelUpdate, Infallible>;

    async fn handle(
        &mut self,
        _msg: GetModel,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        Ok(ModelUpdate::new(
            self.model_hash.clone(),
            self.required_component_states.clone(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct ModelUpdate {
    model_hash: Arc<String>,
    required_component_states: Arc<[RequiredComponentState]>,
}

impl ModelUpdate {
    pub fn new(
        model_hash: Arc<String>,
        required_component_states: Arc<[RequiredComponentState]>,
    ) -> Self {
        Self {
            model_hash,
            required_component_states,
        }
    }

    pub fn model_hash(&self) -> &str {
        &self.model_hash
    }

    pub fn required_component_states(&self) -> &[RequiredComponentState] {
        &self.required_component_states
    }
}
