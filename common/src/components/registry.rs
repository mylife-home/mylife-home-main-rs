use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

use kameo::{message, prelude::*};
use thiserror::Error;

use crate::{
    components::{
        metadata::{MemberType, PluginMetadata},
        types::Value,
    },
    utils::actors::{
        ActorHandle, CallError, HandleLookupError, PublisherHandle, SpawnedActor, SpawnedActors,
        SubscriberHandle, spawn_pubsub,
    },
};

const REGISTRY_NAME: &str = "components.registry";
const UPDATE_PUBSUB_NAME: &str = "components.registry.update";

/// Client access to the registry actor
#[derive(Debug, Clone)]
pub struct RegistryHandle {
    actor: ActorHandle<Registry>,
    on_update: SubscriberHandle<RegistryUpdated>,
}

impl RegistryHandle {
    /// Create a new access
    pub fn new() -> Result<Self, HandleLookupError> {
        Ok(Self {
            actor: ActorHandle::from_name(REGISTRY_NAME)?,
            on_update: SubscriberHandle::from_name(UPDATE_PUBSUB_NAME)?,
        })
    }

    /// Add a plugin, waiting for the registry reply
    pub async fn plugin_add(
        &self,
        instance: Option<String>,
        plugin: Arc<PluginMetadata>,
    ) -> Result<(), CallError<PluginAddError>> {
        self.actor.call(PluginAdd { instance, plugin }).await?;

        Ok(())
    }

    /// Remove a plugin, waiting for the registry reply
    pub async fn plugin_remove(
        &self,
        instance: Option<String>,
        plugin_id: String,
    ) -> Result<(), CallError<PluginRemoveError>> {
        self.actor
            .call(PluginRemove {
                instance,
                plugin_id,
            })
            .await?;

        Ok(())
    }

    /// Add a component, waiting for the registry reply
    pub async fn component_add(
        &self,
        instance: Option<String>,
        plugin_id: String,
        component_id: String,
        on_action: Recipient<ComponentExecuteAction>,
    ) -> Result<ComponentHandle, CallError<ComponentAddError>> {
        self.actor
            .call(ComponentAdd {
                instance,
                plugin_id,
                component_id: component_id.clone(),
                on_action,
            })
            .await?;

        Ok(ComponentHandle::new(self.actor.clone(), component_id))
    }

    /// Remove a component, waiting for the registry reply
    pub async fn component_remove(
        &self,
        component_id: String,
    ) -> Result<(), CallError<ComponentRemoveError>> {
        self.actor.call(ComponentRemove { component_id }).await?;

        Ok(())
    }

    /// Get info on a component
    pub async fn get_component(
        &self,
        component_id: String,
    ) -> Result<ComponentInfo, CallError<ComponentGetError>> {
        self.actor.call(ComponentGet { component_id }).await
    }

    /// Execute an action on a component
    pub fn component_execute_action(&self, component_id: String, action: String, value: Value) {
        self.actor.send(ComponentAction {
            component_id,
            action,
            value,
        });
    }

    /// Get the PubSub for registry update
    pub fn on_update(&self) -> &SubscriberHandle<RegistryUpdated> {
        &self.on_update
    }
}

#[derive(Debug)]
pub struct ComponentInfo {
    pub instance: Option<String>,
    pub plugin: Arc<PluginMetadata>,
    pub component_id: String,
    pub state: HashMap<String, Option<Value>>,
}

/// Specific registry access part for a component
#[derive(Debug, Clone)]
pub struct ComponentHandle {
    registry: ActorHandle<Registry>,
    component_id: Arc<String>,
}

impl ComponentHandle {
    fn new(registry: ActorHandle<Registry>, component_id: String) -> Self {
        Self {
            registry,
            component_id: Arc::new(component_id),
        }
    }

    pub fn state_changed(&self, state: String, value: Value) {
        self.registry.send(ComponentEmitState {
            component_id: self.component_id.clone(),
            state,
            value,
        });
    }
}

pub async fn init_pubsubs(actors: &mut SpawnedActors) {
    actors.add(spawn_pubsub::<RegistryUpdated>(UPDATE_PUBSUB_NAME).await);
}

pub async fn init_actor(actors: &mut SpawnedActors) {
    let (registry, _) = SpawnedActor::start::<Registry>(()).await;

    registry.register(REGISTRY_NAME);

    actors.add(registry);
}

/// Registry is responsible for managing the plugins and components of all instances, and providing an observable interface for other modules to subscribe to registry events.
struct Registry {
    components: HashMap<Arc<String>, ComponentData>,
    instances: HashMap<InstanceName, InstanceData>,
    on_update: PublisherHandle<RegistryUpdated>,
}

impl Registry {
    fn add_plugin(
        &mut self,
        instance_name: Option<String>,
        plugin: Arc<PluginMetadata>,
    ) -> Result<(), PluginAddError> {
        let instance_name = InstanceName::from(instance_name);

        let instance_data = self
            .instances
            .entry(instance_name.clone())
            .or_insert_with(|| InstanceData::new(instance_name.clone()));

        instance_data.add_plugin(plugin.clone())?;

        tracing::debug!(
            instance = %instance_name,
            plugin_id = plugin.id(),
            "plugin added"
        );

        self.on_update
            .publish(RegistryUpdated::PluginAdded(PluginAdded {
                instance: instance_name.into(),
                plugin,
            }));

        Ok(())
    }

    fn remove_plugin(
        &mut self,
        instance_name: Option<String>,
        plugin_id: &str,
    ) -> Result<(), PluginRemoveError> {
        let instance_name = InstanceName::from(instance_name);

        let Some(instance_data) = self.instances.get_mut(&instance_name) else {
            return Err(PluginRemoveError::not_found(
                instance_name.to_string(),
                plugin_id.to_owned(),
            ));
        };

        let plugin = instance_data.remove_plugin(plugin_id)?;

        if instance_data.is_empty() {
            self.instances.remove(&instance_name);
        }

        tracing::debug!(instance = %instance_name, plugin_id, "plugin removed");

        self.on_update
            .publish(RegistryUpdated::PluginRemoved(PluginRemoved {
                instance: instance_name.into(),
                plugin,
            }));

        Ok(())
    }

    fn add_component(
        &mut self,
        instance_name: Option<String>,
        plugin_id: String,
        component_id: String,
        on_action: Recipient<ComponentExecuteAction>,
    ) -> Result<(), ComponentAddError> {
        let instance_name = InstanceName::from(instance_name);
        let component_id = Arc::new(component_id);

        if self.components.contains_key(&component_id) {
            return Err(ComponentAddError::already_exists(component_id.to_string()));
        }

        let Some(instance_data) = self.instances.get_mut(&instance_name) else {
            return Err(ComponentAddError::plugin_not_found(
                component_id.to_string(),
                instance_name.to_string(),
                plugin_id,
            ));
        };

        let plugin = instance_data.add_component(&plugin_id, component_id.clone())?;

        self.components.insert(
            component_id.clone(),
            ComponentData::new(
                component_id.clone(),
                instance_name.clone(),
                plugin.clone(),
                on_action,
                self.on_update.clone(),
            ),
        );

        tracing::debug!(
            instance = %instance_name,
            %component_id,
            "component registered"
        );

        self.on_update
            .publish(RegistryUpdated::ComponentAdded(ComponentAdded {
                instance: instance_name.into(),
                plugin,
                component_id,
            }));

        Ok(())
    }

    fn remove_component(&mut self, component_id: String) -> Result<(), ComponentRemoveError> {
        let component_id = Arc::new(component_id);

        let Some(component_data) = self.components.get(&component_id) else {
            return Err(ComponentRemoveError::not_found(component_id.to_string()));
        };

        let plugin = component_data.plugin().clone();
        let instance_name = component_data.instance_name().clone();

        self.components.remove(&component_id);

        let instance_data = self
            .instances
            .get_mut(&instance_name)
            .expect("data inconsistency: instance data not found");

        instance_data.remove_component(plugin.id(), &component_id);

        if instance_data.is_empty() {
            self.instances.remove(&instance_name);
        }

        tracing::debug!(
            instance = %instance_name,
            %component_id,
            "component unregistered"
        );

        self.on_update
            .publish(RegistryUpdated::ComponentRemoved(ComponentRemoved {
                instance: instance_name.into(),
                plugin,
                component_id,
            }));

        Ok(())
    }

    fn get_component(&mut self, component_id: String) -> Result<ComponentInfo, ComponentGetError> {
        let component_id = Arc::new(component_id);

        let Some(component_data) = self.components.get(&component_id) else {
            return Err(ComponentGetError::not_found(component_id.to_string()));
        };

        Ok(ComponentInfo {
            instance: component_data.instance_name().into(),
            plugin: component_data.plugin().clone(),
            component_id: component_id.to_string(),
            state: component_data.state().clone(),
        })
    }

    fn execute_action(&mut self, component_id: String, action: &str, value: Value) {
        let component_id = Arc::new(component_id);

        let Some(component_data) = self.components.get_mut(&component_id) else {
            tracing::error!(%component_id, "component not found");
            return;
        };

        component_data.execute_action(action, value);
    }

    fn handle_state_change(&mut self, component_id: Arc<String>, state: &str, value: Value) {
        let Some(component_data) = self.components.get_mut(&component_id) else {
            tracing::error!(%component_id, "component not found");
            return;
        };

        component_data.handle_state_change(state, value);
    }
}

impl Actor for Registry {
    type Args = ();

    type Error = HandleLookupError;

    async fn on_start(_args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            components: HashMap::new(),
            instances: HashMap::new(),
            on_update: PublisherHandle::from_name(UPDATE_PUBSUB_NAME)?,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.components.clear();
        self.instances.clear();

        Ok(())
    }
}

#[derive(Debug, Error)]
#[error("failed to add plugin '{instance}:{plugin_id}': {kind}")]
pub struct PluginAddError {
    instance: String,
    plugin_id: String,
    #[source]
    kind: PluginAddErrorKind,
}

impl PluginAddError {
    fn already_exists(instance: impl Into<String>, plugin_id: impl Into<String>) -> Self {
        Self {
            instance: instance.into(),
            plugin_id: plugin_id.into(),
            kind: PluginAddErrorKind::AlreadyExists,
        }
    }
}

#[derive(Debug, Error)]
enum PluginAddErrorKind {
    #[error("plugin already exists")]
    AlreadyExists,
}

impl message::Message<PluginAdd> for Registry {
    type Reply = Result<(), PluginAddError>;

    async fn handle(
        &mut self,
        msg: PluginAdd,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.add_plugin(msg.instance, msg.plugin)
    }
}

#[derive(Debug, Error)]
#[error("failed to remove plugin '{instance}:{plugin_id}': {kind}")]
pub struct PluginRemoveError {
    instance: String,
    plugin_id: String,
    #[source]
    kind: PluginRemoveErrorKind,
}

impl PluginRemoveError {
    fn not_found(instance: impl Into<String>, plugin_id: impl Into<String>) -> Self {
        Self {
            instance: instance.into(),
            plugin_id: plugin_id.into(),
            kind: PluginRemoveErrorKind::NotFound,
        }
    }

    fn used(instance: impl Into<String>, plugin_id: impl Into<String>) -> Self {
        Self {
            instance: instance.into(),
            plugin_id: plugin_id.into(),
            kind: PluginRemoveErrorKind::Used,
        }
    }
}

#[derive(Debug, Error)]
enum PluginRemoveErrorKind {
    #[error("plugin not found")]
    NotFound,
    #[error("plugin is used by components")]
    Used,
}

impl message::Message<PluginRemove> for Registry {
    type Reply = Result<(), PluginRemoveError>;

    async fn handle(
        &mut self,
        msg: PluginRemove,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.remove_plugin(msg.instance, &msg.plugin_id)
    }
}

#[derive(Debug, Error)]
#[error("failed to add component '{component_id}': {kind}")]
pub struct ComponentAddError {
    component_id: String,
    #[source]
    kind: ComponentAddErrorKind,
}

impl ComponentAddError {
    fn already_exists(component_id: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            kind: ComponentAddErrorKind::AlreadyExists,
        }
    }

    fn plugin_not_found(
        component_id: impl Into<String>,
        instance: impl Into<String>,
        plugin_id: impl Into<String>,
    ) -> Self {
        Self {
            component_id: component_id.into(),
            kind: ComponentAddErrorKind::PluginNotFound {
                instance: instance.into(),
                plugin_id: plugin_id.into(),
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum ComponentAddErrorKind {
    #[error("component already exists")]
    AlreadyExists,
    #[error("plugin '{instance}:{plugin_id}' does not exist")]
    PluginNotFound { instance: String, plugin_id: String },
}

impl message::Message<ComponentAdd> for Registry {
    type Reply = Result<(), ComponentAddError>;

    async fn handle(
        &mut self,
        msg: ComponentAdd,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.add_component(msg.instance, msg.plugin_id, msg.component_id, msg.on_action)
    }
}

#[derive(Debug, Error)]
#[error("failed to remove component '{component_id}': {kind}")]
pub struct ComponentRemoveError {
    component_id: String,
    #[source]
    kind: ComponentRemoveErrorKind,
}

impl ComponentRemoveError {
    fn not_found(component_id: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            kind: ComponentRemoveErrorKind::NotFound,
        }
    }
}

#[derive(Debug, Error)]
pub enum ComponentRemoveErrorKind {
    #[error("component not found")]
    NotFound,
}

impl message::Message<ComponentRemove> for Registry {
    type Reply = Result<(), ComponentRemoveError>;

    async fn handle(
        &mut self,
        msg: ComponentRemove,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.remove_component(msg.component_id)
    }
}

#[derive(Debug, Error)]
#[error("failed to get component '{component_id}': {kind}")]
pub struct ComponentGetError {
    component_id: String,
    #[source]
    kind: ComponentGetErrorKind,
}

impl ComponentGetError {
    fn not_found(component_id: impl Into<String>) -> Self {
        Self {
            component_id: component_id.into(),
            kind: ComponentGetErrorKind::NotFound,
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn kind(&self) -> &ComponentGetErrorKind {
        &self.kind
    }
}

#[derive(Debug, Error)]
pub enum ComponentGetErrorKind {
    #[error("component not found")]
    NotFound,
}

impl message::Message<ComponentGet> for Registry {
    type Reply = Result<ComponentInfo, ComponentGetError>;

    async fn handle(
        &mut self,
        msg: ComponentGet,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.get_component(msg.component_id)
    }
}

impl message::Message<ComponentAction> for Registry {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ComponentAction,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.execute_action(msg.component_id, &msg.action, msg.value);
    }
}

impl message::Message<ComponentEmitState> for Registry {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ComponentEmitState,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.handle_state_change(msg.component_id, &msg.state, msg.value);
    }
}

/// Registry command: add a plugin
#[derive(Debug, Clone)]
struct PluginAdd {
    instance: Option<String>,
    plugin: Arc<PluginMetadata>,
}

/// Registry command: remove a plugin
#[derive(Debug, Clone)]
struct PluginRemove {
    instance: Option<String>,
    plugin_id: String,
}

/// Registry command: add a component
#[derive(Debug, Clone)]
struct ComponentAdd {
    instance: Option<String>,
    plugin_id: String,
    component_id: String,
    on_action: Recipient<ComponentExecuteAction>,
}

/// Registry command: remove a component
#[derive(Debug, Clone)]
struct ComponentRemove {
    component_id: String,
}

/// Registry command: get a component
#[derive(Debug, Clone)]
struct ComponentGet {
    component_id: String,
}

/// Registry command: execute action on component
#[derive(Debug, Clone)]
struct ComponentAction {
    component_id: String,
    action: String,
    value: Value,
}

/// Message to be implemented by a component so that registry can dispatch actions to it
#[derive(Debug, Clone)]
pub struct ComponentExecuteAction {
    // Still provide component_id here to allow one actor to handle multiple components
    component_id: Arc<String>,
    name: String,
    value: Value,
}

impl ComponentExecuteAction {
    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, Clone)]
struct ComponentEmitState {
    component_id: Arc<String>,
    state: String,
    value: Value,
}

/// Registry updates
#[derive(Debug, Clone)]
pub enum RegistryUpdated {
    PluginAdded(PluginAdded),
    PluginRemoved(PluginRemoved),
    ComponentAdded(ComponentAdded),
    ComponentRemoved(ComponentRemoved),
    ComponentStateChanged(ComponentStateChanged),
}

#[derive(Debug, Clone)]
pub struct PluginAdded {
    instance: Option<Arc<String>>,
    plugin: Arc<PluginMetadata>,
}

impl PluginAdded {
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_ref().map(|v| v.as_str())
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }
}

#[derive(Debug, Clone)]
pub struct PluginRemoved {
    instance: Option<Arc<String>>,
    plugin: Arc<PluginMetadata>,
}

impl PluginRemoved {
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_ref().map(|v| v.as_str())
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }
}
#[derive(Debug, Clone)]
pub struct ComponentAdded {
    instance: Option<Arc<String>>,
    plugin: Arc<PluginMetadata>,
    component_id: Arc<String>,
}

impl ComponentAdded {
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_ref().map(|v| v.as_str())
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }
}

#[derive(Debug, Clone)]
pub struct ComponentRemoved {
    instance: Option<Arc<String>>,
    plugin: Arc<PluginMetadata>,
    component_id: Arc<String>,
}

impl ComponentRemoved {
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_ref().map(|v| v.as_str())
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStateChanged {
    instance: Option<Arc<String>>,
    plugin: Arc<PluginMetadata>,
    component_id: Arc<String>,
    state: Arc<String>,
    value: Arc<Value>,
}

impl ComponentStateChanged {
    pub fn instance(&self) -> Option<&str> {
        self.instance.as_ref().map(|v| v.as_str())
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn value(&self) -> &Value {
        &self.value
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct InstanceName(Option<Arc<String>>);

impl From<Option<Arc<String>>> for InstanceName {
    fn from(value: Option<Arc<String>>) -> Self {
        Self(value)
    }
}

impl From<Option<String>> for InstanceName {
    fn from(value: Option<String>) -> Self {
        Self(value.map(|s| Arc::new(s)))
    }
}

impl From<InstanceName> for Option<Arc<String>> {
    fn from(value: InstanceName) -> Self {
        value.0
    }
}

impl From<&InstanceName> for Option<String> {
    fn from(value: &InstanceName) -> Self {
        value.0.as_ref().map(ToString::to_string)
    }
}

impl fmt::Display for InstanceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(value) = &self.0 {
            f.write_str(value)
        } else {
            f.write_str("local")
        }
    }
}

#[derive(Debug)]
struct InstanceData {
    instance_name: InstanceName,
    components: HashSet<Arc<String>>,
    plugins: HashMap<String, PluginData>,
}

impl InstanceData {
    pub fn new(instance_name: InstanceName) -> Self {
        Self {
            instance_name,
            components: HashSet::new(),
            plugins: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty() && self.plugins.is_empty()
    }

    pub fn add_component(
        &mut self,
        plugin_id: &str,
        component_id: Arc<String>,
    ) -> Result<Arc<PluginMetadata>, ComponentAddError> {
        let Some(plugin) = self.plugins.get_mut(plugin_id) else {
            return Err(ComponentAddError::plugin_not_found(
                component_id.to_string(),
                self.instance_name.to_string(),
                plugin_id.to_owned(),
            ));
        };

        plugin.add_component();

        self.components.insert(component_id);

        Ok(plugin.metadata().clone())
    }

    pub fn remove_component(&mut self, plugin_id: &str, component_id: &Arc<String>) {
        self.plugins
            .get_mut(plugin_id)
            .expect("data inconsistency")
            .remove_component();
        self.components.remove(component_id);
    }

    pub fn add_plugin(&mut self, plugin: Arc<PluginMetadata>) -> Result<(), PluginAddError> {
        let id = plugin.id().to_owned();

        if self.plugins.contains_key(&id) {
            return Err(PluginAddError::already_exists(
                self.instance_name.to_string(),
                id,
            ));
        }

        self.plugins
            .insert(plugin.id().to_owned(), PluginData::new(plugin));
        Ok(())
    }

    pub fn remove_plugin(
        &mut self,
        plugin_id: &str,
    ) -> Result<Arc<PluginMetadata>, PluginRemoveError> {
        let Some(plugin) = self.plugins.get(plugin_id) else {
            return Err(PluginRemoveError::not_found(
                self.instance_name.to_string(),
                plugin_id.to_owned(),
            ));
        };

        if plugin.used() {
            return Err(PluginRemoveError::used(
                self.instance_name.to_string(),
                plugin_id.to_owned(),
            ));
        }

        let plugin = plugin.metadata().clone();

        self.plugins.remove(plugin_id);

        Ok(plugin)
    }

    // pub fn get_plugin(&self, id: &str) -> Option<&PluginData> {
    //     self.plugins.get(id)
    // }
}

#[derive(Debug)]

struct PluginData {
    metadata: Arc<PluginMetadata>,
    components: usize,
}

impl PluginData {
    pub fn new(metadata: Arc<PluginMetadata>) -> Self {
        Self {
            metadata,
            components: 0,
        }
    }

    pub fn metadata(&self) -> &Arc<PluginMetadata> {
        &self.metadata
    }

    pub fn add_component(&mut self) {
        self.components += 1;
    }

    pub fn remove_component(&mut self) {
        assert!(self.components > 0);
        self.components -= 1;
    }

    pub fn used(&self) -> bool {
        self.components > 0
    }
}

#[derive(Debug)]
struct ComponentData {
    instance_name: InstanceName,
    component_id: Arc<String>,
    plugin: Arc<PluginMetadata>,
    state: HashMap<String, Option<Value>>,
    on_action: Recipient<ComponentExecuteAction>,
    on_update: PublisherHandle<RegistryUpdated>,
}

impl ComponentData {
    pub fn new(
        component_id: Arc<String>,
        instance_name: InstanceName,
        plugin: Arc<PluginMetadata>,
        on_action: Recipient<ComponentExecuteAction>,
        on_update: PublisherHandle<RegistryUpdated>,
    ) -> Self {
        let mut state = HashMap::new();

        for (name, member) in plugin.members() {
            if member.member_type() == MemberType::State {
                state.insert(name.clone(), None);
            }
        }

        Self {
            component_id,
            instance_name,
            plugin,
            state,
            on_action,
            on_update,
        }
    }

    // pub fn component_id(&self) -> &str {
    //     &self.component_id
    // }

    pub fn instance_name(&self) -> &InstanceName {
        &self.instance_name
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }

    pub fn state(&self) -> &HashMap<String, Option<Value>> {
        &self.state
    }

    pub fn execute_action(&mut self, name: &str, value: Value) {
        let Some(member) = self.plugin.members().get(name) else {
            tracing::error!(component_id = %self.component_id, action = name, "action does not exist on component");
            return;
        };

        if member.member_type() != MemberType::Action {
            tracing::error!(component_id = %self.component_id, action = name, "action does not exist on component");
            return;
        }

        if !value.is_valid(member.value_type()) {
            tracing::error!(component_id = %self.component_id, action = name, r#type = %member.value_type(), ?value, "action does not accept value");
            return;
        }

        tracing::trace!(component_id = %self.component_id, action = name, ?value, "execute component action");

        if let Err(error) = self
            .on_action
            .tell(ComponentExecuteAction {
                component_id: self.component_id.clone(),
                name: name.to_owned(),
                value,
            })
            .try_send()
        {
            tracing::error!(?error, component_id = %self.component_id, "could not send action to actor component");
        }
    }

    pub fn handle_state_change(&mut self, name: &str, value: Value) {
        let Some(member) = self.plugin.members().get(name) else {
            tracing::error!(component_id = %self.component_id, state = name, "state does not exist on component");
            return;
        };

        if member.member_type() != MemberType::State {
            tracing::error!(component_id = %self.component_id, state = name, "state does not exist on component");
            return;
        }

        if !value.is_valid(member.value_type()) {
            tracing::error!(component_id = %self.component_id, state = name, r#type = %member.value_type(), ?value, "state does not accept value");
            return;
        }

        *self
            .state
            .get_mut(name)
            .expect("data inconsistency: state missing") = Some(value.clone());

        if tracing::enabled!(tracing::Level::TRACE) {
            let state_complete = self.state.iter().all(|(_, v)| v.is_some());

            tracing::trace!(component_id = %self.component_id, state = name, ?value, state_complete, "component state changed");
        }

        self.on_update
            .publish(RegistryUpdated::ComponentStateChanged(
                ComponentStateChanged {
                    instance: self.instance_name.clone().into(),
                    plugin: self.plugin.clone(),
                    component_id: self.component_id.clone(),
                    state: Arc::new(name.to_owned()),
                    value: Arc::new(value),
                },
            ));
    }
}
