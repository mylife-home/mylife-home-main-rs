use std::{collections::HashMap, ops::Deref, sync::Arc};

use bytes::Bytes;
use kameo::{message, prelude::*};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    bus::{
        client::{self, ClientHandle, Subscription, Topic, TopicBuilder},
        encoding::{self, DecodingError},
        metadata::{self, MetadataHandle, RemoteUpdate},
    },
    components::{
        metadata::{MemberType, PluginMetadata, Type},
        registry::{self, ComponentExecuteAction, ComponentHandle, RegistryHandle},
        types::Value,
    },
    utils::actors::{HandleLookupError, SpawnedActor, SpawnedActors},
};

const DOMAIN: &str = "components";

const REMOTE_NAME: &str = "bus.remote";

// on connection, plugin metadata must be published before component metadata
const METADATA_PLUGIN_PRIORITY: i64 = 100;
const METADATA_COMPONENT_PRIORITY: i64 = 0;

#[derive(Debug)]
pub struct RemoteConfig {
    pub instance_name: Arc<String>,
}

pub async fn init_actor(actors: &mut SpawnedActors, config: RemoteConfig) {
    let (remote, _) = SpawnedActor::start::<Remote>(config).await;

    remote.register(REMOTE_NAME);

    actors.add(remote);
}

#[derive(Debug)]
struct Remote {
    instance_name: Arc<String>,
    client: ClientHandle,
    metadata: MetadataHandle,
    registry: RegistryHandle,
    weak_ref_self: WeakActorRef<Self>,

    remote_plugins: HashMap<RemotePluginKey, RemotePluginData>,
    remote_components: HashMap<String, RemoteComponentData>,
    remote_pending_components: Vec<RemotePendingComponent>,
    local_components: HashMap<String, LocalComponent>,
}

impl Actor for Remote {
    type Args = RemoteConfig;
    type Error = HandleLookupError;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let client = ClientHandle::new()?;
        let metadata = MetadataHandle::new()?;
        let registry = RegistryHandle::new()?;

        metadata.on_remote_update().subscribe(actor_ref.clone());
        registry.on_update().subscribe(actor_ref.clone());
        client.on_message().subscribe(actor_ref.clone());
        client.on_online().subscribe(actor_ref.clone());

        Ok(Self {
            instance_name: config.instance_name,
            client,
            metadata,
            registry,
            weak_ref_self: actor_ref.downgrade(),
            remote_plugins: HashMap::new(),
            remote_components: HashMap::new(),
            remote_pending_components: Vec::new(),
            local_components: HashMap::new(),
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.remote_plugins.clear();
        self.remote_pending_components.clear();

        Ok(())
    }
}

impl message::Message<metadata::RemoteUpdate> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: metadata::RemoteUpdate,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut parts = msg.path().splitn(2, '/');
        let Some(typ) = parts.next() else {
            return;
        };
        let Some(id) = parts.next() else {
            return;
        };

        match typ {
            "plugins" => {
                if msg.has_value() {
                    self.add_remote_plugin(id, &msg).await;
                } else {
                    self.remove_remote_plugin(id, &msg).await;
                }
            }

            "components" => {
                if msg.has_value() {
                    self.add_remote_component(id, &msg).await;
                } else {
                    self.remove_remote_component(id, &msg).await;
                }
            }

            _ => {}
        }
    }
}

impl message::Message<client::Message> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Message,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let Some(topic) = msg.parse_topic() else {
            return;
        };

        if topic.domain != DOMAIN {
            return;
        }

        let parts: Vec<_> = topic.remaining.split('/').collect();
        if parts.len() != 2 {
            tracing::warn!(topic = msg.topic(), "bad component message topic, ignored");
            return;
        }

        let component_id = parts[0];
        let member_name = parts[1];

        if topic.instance == *self.instance_name {
            if let Err(error) = self.execute_local_action(component_id, member_name, msg.payload())
            {
                tracing::error!(
                    ?error,
                    component_id,
                    action = member_name,
                    "cannot execute local component action"
                );
            }
        } else {
            if let Err(error) =
                self.handle_state_change(topic.instance, component_id, member_name, msg.payload())
            {
                tracing::error!(
                    ?error,
                    component_id,
                    state = member_name,
                    "cannot update local component state"
                );
            }
        }
    }
}

impl message::Message<registry::RegistryUpdated> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: registry::RegistryUpdated,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            registry::RegistryUpdated::PluginAdded(plugin_data) => {
                if plugin_data.instance().is_none() {
                    let plugin = plugin_data.plugin();

                    let path = format!("plugins/{}", plugin.id());
                    self.metadata
                        .set(&path, plugin.deref(), METADATA_PLUGIN_PRIORITY)
                        .await;
                }
            }

            registry::RegistryUpdated::PluginRemoved(plugin_data) => {
                if plugin_data.instance().is_none() {
                    let plugin = plugin_data.plugin();

                    let path = format!("plugins/{}", plugin.id());
                    self.metadata.clear(&path).await;
                }
            }

            registry::RegistryUpdated::ComponentAdded(component_data) => {
                if component_data.instance().is_none() {
                    let id = component_data.component_id();
                    let plugin = component_data.plugin();
                    let path = format!("components/{}", id);

                    let comp_meta = ComponentMetadata {
                        id: id.to_owned(),
                        plugin: plugin.id().to_owned(),
                    };

                    self.metadata
                        .set(&path, &comp_meta, METADATA_COMPONENT_PRIORITY)
                        .await;

                    self.local_components
                        .insert(id.to_owned(), LocalComponent::new(plugin.clone()));
                }
            }

            registry::RegistryUpdated::ComponentRemoved(component_data) => {
                if component_data.instance().is_none() {
                    let id = component_data.component_id();
                    let path = format!("components/{}", id);
                    self.metadata.clear(&path).await;

                    self.local_components.remove(id);

                    // remove all state
                    for (name, member) in component_data.plugin().members() {
                        if member.member_type() != MemberType::State {
                            continue;
                        }

                        let topic = self.component_topic(None, component_data.component_id(), name);
                        self.client.clear_retain(topic);
                    }
                }
            }

            registry::RegistryUpdated::ComponentStateChanged(state_data) => {
                if state_data.instance().is_none() {
                    let Some(member) = state_data.plugin().members().get(state_data.state()) else {
                        tracing::error!(
                            plugin_id = state_data.plugin().id(),
                            component_id = state_data.component_id(),
                            state = state_data.state(),
                            "got state change for non-existant state on local component",
                        );
                        return;
                    };

                    if member.member_type() != MemberType::State {
                        tracing::error!(
                            plugin_id = state_data.plugin().id(),
                            component_id = state_data.component_id(),
                            state = state_data.state(),
                            "got state change for invalid state on local component",
                        );
                        return;
                    }

                    let value = encoding::write_value(member.value_type(), state_data.value());
                    let topic =
                        self.component_topic(None, state_data.component_id(), state_data.state());
                    self.client.publish(topic, value.clone(), true);

                    // update local_component state
                    let Some(component) = self.local_components.get_mut(state_data.component_id())
                    else {
                        tracing::error!(
                            component_id = state_data.component_id(),
                            "local component state does not exist",
                        );
                        return;
                    };

                    component.state.insert(state_data.state().to_owned(), value);
                }
            }
        }
    }
}

impl message::Message<client::Online> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: client::Online,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !msg.is_online() {
            return;
        }

        // (re)publish all state
        for (id, component) in &self.local_components {
            for (name, value) in &component.state {
                let topic = self.component_topic(None, id, name);
                self.client.publish(topic, value.clone(), true);
            }
        }
    }
}

impl message::Message<registry::ComponentExecuteAction> for Remote {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: registry::ComponentExecuteAction,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(error) = self.execute_action(msg.component_id(), msg.name(), msg.value()) {
            tracing::error!(
                ?error,
                component_id = msg.component_id(),
                action = msg.name(),
                "cannot execute component action",
            );
        }
    }
}

#[derive(Debug, Error)]
enum ExecuteActionError {
    #[error("component not found")]
    ComponentNotFound,
    #[error("plugin '{instance}:{plugin_id}' not found")]
    PluginNotFound { instance: String, plugin_id: String },
    #[error("member '{member}' not found on plugin '{instance}:{plugin_id}'")]
    MemberNotFound {
        instance: String,
        plugin_id: String,
        member: String,
    },
}

#[derive(Debug, Error)]
enum HandleStateError {
    #[error("component not found")]
    ComponentNotFound,
    #[error(
        "component '{component_id}' is on instance '{expected_instance}', but is now updated from instance '{actual_instance}'"
    )]
    InstanceMismatch {
        component_id: String,
        expected_instance: String,
        actual_instance: String,
    },
    #[error("plugin '{instance}:{plugin_id}' not found")]
    PluginNotFound { instance: String, plugin_id: String },
    #[error("member '{member}' not found on plugin '{instance}:{plugin_id}'")]
    MemberNotFound {
        instance: String,
        plugin_id: String,
        member: String,
    },
    #[error("cannot read state '{state}' value '{value:?}' of type '{ty}': {error}")]
    ValueReadError {
        state: String,
        value: Bytes,
        ty: Type,
        #[source]
        error: DecodingError,
    },
}

#[derive(Debug, Error)]
enum ExecuteLocalActionError {
    #[error("component not found")]
    ComponentNotFound,
    #[error("member not found")]
    MemberNotFound,
    #[error("member is not an action")]
    MemberNotAction,
    #[error("cannot read value: {0}")]
    ValueReadError(#[from] DecodingError),
}

impl Remote {
    async fn add_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) {
        let plugin = match msg.read_value::<PluginMetadata>() {
            Ok(plugin) => Arc::new(plugin),
            Err(error) => {
                tracing::error!(
                    ?error,
                    instance = msg.instance(),
                    plugin_id = id,
                    "Cannot read plugin metadata"
                );
                return;
            }
        };
        if plugin.id() != id {
            tracing::error!(
                metadata_key = id,
                metadata_value = plugin.id(),
                "plugin id mismatch",
            );
            return;
        }

        let key = RemotePluginKey {
            instance: msg.instance().to_owned(),
            id: id.to_owned(),
        };

        if let Err(error) = self
            .registry
            .plugin_add(Some(key.instance.clone()), plugin.clone())
            .await
        {
            tracing::error!(
                ?error,
                instance = msg.instance(),
                plugin_id = id,
                "cannot add plugin to registry"
            );
            return;
        }

        // usage 1 = created
        self.remote_plugins.insert(
            key,
            RemotePluginData {
                metadata: plugin,
                usage: 1,
            },
        );

        // Check if we have pending components
        let pendings: Vec<_> = self
            .remote_pending_components
            .extract_if(.., |comp| {
                comp.instance == msg.instance() && comp.plugin_id == id
            })
            .collect();

        for pending in pendings {
            tracing::trace!(
                instance = pending.instance,
                component_id = pending.id,
                plugin_id = pending.plugin_id,
                "create pending component"
            );

            self.do_add_component(pending.instance, pending.plugin_id, pending.id)
                .await;
        }
    }

    async fn remove_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) {
        self.unref_plugin(msg.instance(), id).await;
    }

    async fn add_remote_component(&mut self, id: &str, msg: &RemoteUpdate) {
        let component = match msg.read_value::<ComponentMetadata>() {
            Ok(component) => component,
            Err(error) => {
                tracing::error!(
                    ?error,
                    instance = msg.instance(),
                    component_id = id,
                    "Cannot read component metadata",
                );
                return;
            }
        };
        if component.id != id {
            tracing::error!(
                metadata_key = id,
                metadata_value = component.id,
                "component id mismatch",
            );
            return;
        }

        let plugin_key = RemotePluginKey {
            instance: msg.instance().to_owned(),
            id: component.plugin,
        };

        if !self.remote_plugins.contains_key(&plugin_key) {
            let pending = RemotePendingComponent {
                instance: plugin_key.instance,
                id: component.id,
                plugin_id: plugin_key.id,
            };

            tracing::trace!(
                instance = pending.instance,
                component_id = pending.id,
                plugin_id = pending.plugin_id,
                "add pending component"
            );
            self.remote_pending_components.push(pending);
            return;
        };

        self.do_add_component(plugin_key.instance, plugin_key.id, component.id)
            .await;
    }

    async fn remove_remote_component(&mut self, id: &str, msg: &RemoteUpdate) {
        if let Err(error) = self.registry.component_remove(id.to_owned()).await {
            tracing::error!(
                ?error,
                instance = msg.instance(),
                component_id = id,
                "cannot remove component from registry"
            );
            return;
        }

        let Some(component) = self.remote_components.remove(id) else {
            tracing::error!(
                component_id = id,
                "data inconsistency: registry remove component but not found locally"
            );
            return;
        };

        self.client
            .unsubscribe(self.component_subscription(Some(&component.instance), id));

        tracing::trace!(
            instance = component.instance,
            component_id = id,
            plugin_id = component.plugin_id,
            "unref plugin"
        );
        self.unref_plugin(&component.instance, &component.plugin_id)
            .await;
    }

    async fn do_add_component(
        &mut self,
        instance: String,
        plugin_id: String,
        component_id: String,
    ) {
        let on_action = if let Some(self_ref) = self.weak_ref_self.upgrade() {
            self_ref.recipient::<ComponentExecuteAction>()
        } else {
            tracing::error!("cannot upgrade self ref");
            return;
        };

        let handle = match self
            .registry
            .component_add(
                Some(instance.clone()),
                plugin_id.clone(),
                component_id.clone(),
                on_action,
            )
            .await
        {
            Ok(handle) => handle,
            Err(error) => {
                tracing::error!(
                    ?error,
                    instance,
                    component_id,
                    "cannot add component to registry",
                );
                return;
            }
        };

        if let Some(plugin) = self.remote_plugins.get_mut(&RemotePluginKey {
            instance: instance.clone(),
            id: plugin_id.clone(),
        }) {
            plugin.usage += 1;

            tracing::trace!(
                instance,
                plugin_id,
                component_id,
                usage = plugin.usage,
                "ref plugin",
            );
        } else {
            tracing::error!(
                instance,
                plugin_id,
                component_id,
                "plugin not found for component",
            );
        }

        self.remote_components.insert(
            component_id.clone(),
            RemoteComponentData {
                instance: instance.clone(),
                plugin_id,
                handle,
            },
        );

        self.client
            .subscribe(self.component_subscription(Some(&instance), &component_id));
    }

    fn component_subscription(&self, instance: Option<&str>, component_id: &str) -> Subscription {
        let builder = if let Some(instance) = instance {
            TopicBuilder::remote(instance, DOMAIN)
        } else {
            TopicBuilder::local(&self.instance_name, DOMAIN)
        };

        builder.segment(component_id).any().build()
    }

    fn component_topic(
        &self,
        instance: Option<&str>,
        component_id: &str,
        member_name: &str,
    ) -> Topic {
        let builder = if let Some(instance) = instance {
            TopicBuilder::remote(instance, DOMAIN)
        } else {
            TopicBuilder::local(&self.instance_name, DOMAIN)
        };

        builder.segment(component_id).segment(member_name).build()
    }

    async fn unref_plugin(&mut self, instance: &str, id: &str) {
        let key = RemotePluginKey {
            instance: instance.to_owned(),
            id: id.to_owned(),
        };

        let Some(data) = self.remote_plugins.get_mut(&key) else {
            tracing::error!(
                instance = key.instance,
                plugin_id = key.id,
                "cannot remove non existant plugin",
            );
            return;
        };

        if data.usage == 0 {
            tracing::error!(
                instance = key.instance,
                plugin_id = key.id,
                "plugin usage is 0"
            );
            return;
        }

        data.usage -= 1;

        let Some(data) = self.remote_plugins.get_mut(&key) else {
            tracing::error!(
                instance = key.instance,
                plugin_id = key.id,
                "plugin not found"
            );
            return;
        };

        if data.usage == 0 {
            if let Err(error) = self
                .registry
                .plugin_remove(Some(key.instance.clone()), key.id.clone())
                .await
            {
                tracing::error!(
                    ?error,
                    instance,
                    plugin_id = id,
                    "cannot remove plugin from registry",
                );
                return;
            }

            self.remote_plugins.remove(&key);
        }
    }

    fn execute_action(
        &mut self,
        component_id: &str,
        action: &str,
        value: &Value,
    ) -> Result<(), ExecuteActionError> {
        let component = self
            .remote_components
            .get(component_id)
            .ok_or(ExecuteActionError::ComponentNotFound)?;

        let plugin_key = RemotePluginKey {
            instance: component.instance.clone(),
            id: component.plugin_id.clone(),
        };

        let plugin = self.remote_plugins.get(&plugin_key).ok_or_else(|| {
            ExecuteActionError::PluginNotFound {
                instance: plugin_key.instance.clone(),
                plugin_id: plugin_key.id.clone(),
            }
        })?;

        let member = plugin.metadata.members().get(action).ok_or_else(|| {
            ExecuteActionError::MemberNotFound {
                instance: plugin_key.instance.clone(),
                plugin_id: plugin_key.id.clone(),
                member: action.to_owned(),
            }
        })?;

        if member.member_type() != MemberType::Action {
            return Err(ExecuteActionError::MemberNotFound {
                instance: plugin_key.instance,
                plugin_id: plugin_key.id,
                member: action.to_owned(),
            });
        }

        let buffer = encoding::write_value(member.value_type(), value);
        let topic = self.component_topic(Some(&component.instance), component_id, action);
        self.client.publish(topic, buffer, false);

        Ok(())
    }

    fn handle_state_change(
        &mut self,
        instance_name: &str,
        component_id: &str,
        state: &str,
        value: &Bytes,
    ) -> Result<(), HandleStateError> {
        let component = self
            .remote_components
            .get(component_id)
            .ok_or(HandleStateError::ComponentNotFound)?;

        if component.instance != instance_name {
            return Err(HandleStateError::InstanceMismatch {
                component_id: component_id.to_owned(),
                expected_instance: component.instance.clone(),
                actual_instance: instance_name.to_owned(),
            });
        }

        let plugin_key = RemotePluginKey {
            instance: component.instance.clone(),
            id: component.plugin_id.clone(),
        };

        let plugin = self.remote_plugins.get(&plugin_key).ok_or_else(|| {
            HandleStateError::PluginNotFound {
                instance: plugin_key.instance.clone(),
                plugin_id: plugin_key.id.clone(),
            }
        })?;

        let member = plugin.metadata.members().get(state).ok_or_else(|| {
            HandleStateError::MemberNotFound {
                instance: plugin_key.instance.clone(),
                plugin_id: plugin_key.id.clone(),
                member: state.to_owned(),
            }
        })?;

        if member.member_type() != MemberType::State {
            return Err(HandleStateError::MemberNotFound {
                instance: plugin_key.instance,
                plugin_id: plugin_key.id,
                member: state.to_owned(),
            });
        }

        let value = encoding::read_value(member.value_type(), value).map_err(|e| {
            HandleStateError::ValueReadError {
                state: state.to_owned(),
                value: value.clone(),
                ty: member.value_type().clone(),
                error: e,
            }
        })?;

        component.handle.state_changed(state.to_owned(), value);

        Ok(())
    }

    fn execute_local_action(
        &mut self,
        component_id: &str,
        action: &str,
        value: &Bytes,
    ) -> Result<(), ExecuteLocalActionError> {
        let component = self
            .local_components
            .get(component_id)
            .ok_or(ExecuteLocalActionError::ComponentNotFound)?;
        let member = component
            .plugin
            .members()
            .get(action)
            .ok_or(ExecuteLocalActionError::MemberNotFound)?;
        if member.member_type() != MemberType::Action {
            return Err(ExecuteLocalActionError::MemberNotAction);
        }

        let value = encoding::read_value(member.value_type(), value)?;

        self.registry
            .component_execute_action(component_id.to_owned(), action.to_owned(), value);

        Ok(())
    }
}

#[derive(Debug)]
struct LocalComponent {
    plugin: Arc<PluginMetadata>,
    state: HashMap<String, Bytes>,
}

impl LocalComponent {
    pub fn new(plugin: Arc<PluginMetadata>) -> Self {
        Self {
            plugin,
            state: HashMap::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ComponentMetadata {
    pub id: String,
    pub plugin: String,
}

#[derive(Debug)]
struct RemotePendingComponent {
    instance: String,
    id: String,
    plugin_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RemotePluginKey {
    instance: String,
    id: String,
}

#[derive(Debug)]
struct RemotePluginData {
    metadata: Arc<PluginMetadata>,
    usage: usize,
}

#[derive(Debug)]
struct RemoteComponentData {
    instance: String,
    plugin_id: String,
    handle: ComponentHandle,
}
