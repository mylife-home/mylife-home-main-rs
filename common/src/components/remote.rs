use std::{collections::HashMap, ops::Deref, sync::Arc};

use anyhow::Context;
use bytes::Bytes;
use kameo::{message, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    bus::{
        client::{self, ClientHandle, Subscription, Topic, TopicBuilder},
        encoding,
        metadata::{self, MetadataHandle, RemoteUpdate},
    },
    components::{
        metadata::{MemberType, PluginMetadata},
        registry::{self, ComponentExecuteAction, ComponentHandle, RegistryHandle},
        types::Value,
    },
    utils::actors::{SpawnedActor, SpawnedActors},
};

const DOMAIN: &str = "components";

const REMOTE_NAME: &str = "bus.remote";

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
    type Error = anyhow::Error;

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
            if let Err(e) = self.execute_local_action(component_id, member_name, msg.payload()) {
                tracing::error!(
                    "Cannot execute local component '{}' action '{}': {}",
                    component_id,
                    member_name,
                    e
                );
            }
        } else {
            if let Err(e) =
                self.handle_state_change(topic.instance, component_id, member_name, msg.payload())
            {
                tracing::error!(
                    "Cannot update component '{}' state '{}': {}",
                    component_id,
                    member_name,
                    e
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
                    if let Err(e) = self.metadata.set(&path, plugin.deref()) {
                        tracing::error!(
                            "could not set metadata for plugin '{}': {}",
                            plugin.id(),
                            e
                        );
                    }
                }
            }

            registry::RegistryUpdated::PluginRemoved(plugin_data) => {
                if plugin_data.instance().is_none() {
                    let plugin = plugin_data.plugin();

                    let path = format!("plugins/{}", plugin.id());
                    self.metadata.clear(&path);
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

                    if let Err(e) = self.metadata.set(&path, &comp_meta) {
                        tracing::error!("could not set metadata for component '{}': {}", id, e);
                    }

                    self.local_components
                        .insert(id.to_owned(), LocalComponent::new(plugin.clone()));
                }
            }

            registry::RegistryUpdated::ComponentRemoved(component_data) => {
                if component_data.instance().is_none() {
                    let id = component_data.component_id();
                    let path = format!("components/{}", id);
                    self.metadata.clear(&path);

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
                            "got state change for non-existant state '{}' on local component '{}' with plugin '{}'",
                            state_data.state(),
                            state_data.component_id(),
                            state_data.plugin().id()
                        );
                        return;
                    };

                    if member.member_type() != MemberType::State {
                        tracing::error!(
                            "got state change for invalid state '{}' on local component '{}' with plugin '{}'",
                            state_data.state(),
                            state_data.component_id(),
                            state_data.plugin().id()
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
                            "local component '{}' state does not exist",
                            state_data.component_id()
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
        if let Err(e) = self.execute_action(msg.component_id(), msg.name(), msg.value()) {
            tracing::error!(
                "Cannot execute component '{}' action '{}': {}",
                msg.component_id(),
                msg.name(),
                e
            );
        }
    }
}

impl Remote {
    async fn add_remote_plugin(&mut self, id: &str, msg: &RemoteUpdate) {
        let plugin = match msg.read_value::<PluginMetadata>() {
            Ok(plugin) => Arc::new(plugin),
            Err(e) => {
                tracing::error!(
                    "Cannot read plugin metadata: '{}:{}': {}",
                    msg.instance(),
                    id,
                    e
                );
                return;
            }
        };
        if plugin.id() != id {
            tracing::error!(
                "plugin id mismatch: (metadata key = '{}', metadata value = '{}'",
                id,
                plugin.id()
            );
            return;
        }

        let key = RemotePluginKey {
            instance: msg.instance().to_owned(),
            id: id.to_owned(),
        };

        if let Err(e) = self
            .registry
            .plugin_add(Some(key.instance.clone()), plugin.clone())
            .await
        {
            tracing::error!(
                "cannot add plugin to registry '{}:{}': {}",
                msg.instance(),
                id,
                e
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
            tracing::trace!("create pending component {:?}", pending);

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
            Err(e) => {
                tracing::error!(
                    "Cannot read component metadata: '{}:{}': {}",
                    msg.instance(),
                    id,
                    e
                );
                return;
            }
        };
        if component.id != id {
            tracing::error!(
                "component id mismatch: (metadata key = '{}', metadata value = '{}'",
                id,
                component.id
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

            tracing::trace!("add pending component: {:?}", pending);
            self.remote_pending_components.push(pending);
            return;
        };

        self.do_add_component(plugin_key.instance, plugin_key.id, component.id)
            .await;
    }

    async fn remove_remote_component(&mut self, id: &str, msg: &RemoteUpdate) {
        if let Err(e) = self.registry.component_remove(id.to_owned()).await {
            tracing::error!(
                "cannot remove component from registry '{}:{}': {}",
                msg.instance(),
                id,
                e
            );
            return;
        }

        let Some(component) = self.remote_components.remove(id) else {
            tracing::error!(
                "data inconsistency: registry remove component but not found locally: '{}'",
                id
            );
            return;
        };

        self.client
            .unsubscribe(self.component_subscription(Some(&component.instance), id));

        tracing::trace!(
            "unref plugin '{}:{}' from component '{}'",
            &component.instance,
            &component.plugin_id,
            id
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
            Err(e) => {
                tracing::error!(
                    "cannot add component to registry '{}:{}': {}",
                    instance,
                    component_id,
                    e
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
                "ref plugin '{}:{}' from component '{}:{}' -> {}",
                instance,
                plugin_id,
                instance,
                component_id,
                plugin.usage
            );
        } else {
            tracing::error!(
                "plugin '{}' not found for component '{}:{}'",
                plugin_id,
                instance,
                component_id
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
                "cannot remove non existant plugin '{}:{}'",
                key.instance,
                key.id,
            );
            return;
        };

        if data.usage == 0 {
            tracing::error!("plugin '{}:{}' usage is 0", key.instance, key.id);
            return;
        }

        data.usage -= 1;

        let Some(data) = self.remote_plugins.get_mut(&key) else {
            tracing::error!("plugin not found '{}:{}'", key.instance, key.id);
            return;
        };

        if data.usage == 0 {
            if let Err(e) = self
                .registry
                .plugin_remove(Some(key.instance.clone()), key.id.clone())
                .await
            {
                tracing::error!(
                    "cannot remove plugin from registry '{}:{}': {}",
                    instance,
                    id,
                    e
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
    ) -> anyhow::Result<()> {
        let component = self
            .remote_components
            .get(component_id)
            .context("component does not exist")?;

        let plugin_key = RemotePluginKey {
            instance: component.instance.clone(),
            id: component.plugin_id.clone(),
        };

        let plugin = self.remote_plugins.get(&plugin_key).with_context(|| {
            format!(
                "plugin not found '{}:{}'",
                plugin_key.instance, plugin_key.id
            )
        })?;

        let member = plugin.metadata.members().get(action).with_context(|| {
            format!(
                "member '{}' not found on plugin '{}:{}'",
                action, plugin_key.instance, plugin_key.id
            )
        })?;

        if member.member_type() != MemberType::Action {
            anyhow::bail!(
                "member '{}' not found on plugin '{}:{}'",
                action,
                plugin_key.instance,
                plugin_key.id
            );
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
    ) -> anyhow::Result<()> {
        let component = self
            .remote_components
            .get(component_id)
            .context("component does not exist")?;

        if component.instance != instance_name {
            anyhow::bail!(
                "component '{}' is on instance '{}', but is now updated from instance '{}'",
                component_id,
                component.instance,
                instance_name
            );
        }

        let plugin_key = RemotePluginKey {
            instance: component.instance.clone(),
            id: component.plugin_id.clone(),
        };

        let plugin = self.remote_plugins.get(&plugin_key).with_context(|| {
            format!(
                "plugin not found '{}:{}'",
                plugin_key.instance, plugin_key.id
            )
        })?;

        let member = plugin.metadata.members().get(state).with_context(|| {
            format!(
                "member '{}' not found on plugin '{}:{}'",
                state, plugin_key.instance, plugin_key.id
            )
        })?;

        if member.member_type() != MemberType::State {
            anyhow::bail!(
                "member '{}' not found on plugin '{}:{}'",
                state,
                plugin_key.instance,
                plugin_key.id
            );
        }

        let value = encoding::read_value(member.value_type(), value).with_context(|| {
            format!(
                "cannot read state '{}' value '{:?}' of type '{}'",
                state,
                value,
                member.value_type()
            )
        })?;

        component.handle.state_changed(state.to_owned(), value);

        Ok(())
    }

    fn execute_local_action(
        &mut self,
        component_id: &str,
        action: &str,
        value: &Bytes,
    ) -> anyhow::Result<()> {
        let component = self
            .local_components
            .get(component_id)
            .context("component not found")?;
        let member = component
            .plugin
            .members()
            .get(action)
            .context("member not found")?;
        if member.member_type() != MemberType::Action {
            anyhow::bail!("member is not an action");
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
