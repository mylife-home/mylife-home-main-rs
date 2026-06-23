use std::{
    collections::{HashMap, HashSet},
    fmt,
    sync::Arc,
};

use kameo::{message, prelude::*};

use crate::{
    components::metadata::PluginMetadata,
    utils::actors::{
        ActorHandle, PublisherHandle, SpawnedActor, SpawnedActors, SubscriberHandle, spawn_pubsub,
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
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            actor: ActorHandle::from_name(REGISTRY_NAME)?,
            on_update: SubscriberHandle::from_name(UPDATE_PUBSUB_NAME)?,
        })
    }

    /// Add a plugin, waiting for the registry reply
    pub async fn plugin_add(&self, instance: Option<String>, plugin: Arc<PluginMetadata>) -> anyhow::Result<()> {
        self.actor.call(PluginAdd {
            instance,
            plugin,
        }).await?;

        Ok(())
    }

    /// Remove a plugin, waiting for the registry reply
    pub async fn plugin_remove(&self, instance: Option<String>, plugin_id: String) -> anyhow::Result<()> {
        self.actor.call(PluginRemove {
            instance,
            plugin_id,
        }).await?;

        Ok(())
    }

    /// Add a component, waiting for the registry reply
    pub async fn component_add(&self, instance: Option<String>, plugin_id: String, component_id: String) -> anyhow::Result<()> {
        self.actor.call(ComponentAdd {
            instance,
            plugin_id,
            component_id,
        }).await?;

        Ok(())
    }

    /// Remove a component, waiting for the registry reply
    pub async fn component_remove(&self, instance: Option<String>, component_id: String) -> anyhow::Result<()> {
        self.actor.call(ComponentRemove {
            instance,
            component_id,
        }).await?;

        Ok(())
    }

    /// Get the PubSub for registry update
    pub fn on_update(&self) -> &SubscriberHandle<RegistryUpdated> {
        &self.on_update
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
    ) -> anyhow::Result<()> {
        let instance_name = InstanceName::from(instance_name);

        let instance_data = self
            .instances
            .entry(instance_name.clone())
            .or_insert_with(|| InstanceData::new(instance_name.clone()));

        instance_data.add_plugin(plugin.clone())?;

        log::debug!("plugin '{}:{}' added", instance_name, plugin.id());

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
    ) -> anyhow::Result<()> {
        let instance_name = InstanceName::from(instance_name);

        let Some(instance_data) = self.instances.get_mut(&instance_name) else {
            anyhow::bail!("plugin '{}:{}' not found", instance_name, plugin_id);
        };

        let plugin = instance_data.remove_plugin(plugin_id)?;

        if instance_data.is_empty() {
            self.instances.remove(&instance_name);
        }

        log::debug!("plugin '{}:{}' removed", instance_name, plugin_id);

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
    ) -> anyhow::Result<()> {
        let instance_name = InstanceName::from(instance_name);
        let component_id = Arc::new(component_id);

        if self.components.contains_key(&component_id) {
            anyhow::bail!("component '{}' already registered", component_id);
        }

        let Some(instance_data) = self.instances.get_mut(&instance_name) else {
            anyhow::bail!("plugin '{}:{}' does not exist", instance_name, component_id);
        };

        let plugin = instance_data.add_component(&plugin_id, component_id.clone())?;

        self.components.insert(
            component_id.clone(),
            ComponentData::new(component_id.clone(), instance_name.clone(), plugin.clone()),
        );

        log::debug!(
            "component '{}' registered for instance '{}'",
            component_id,
            instance_name
        );

        self.on_update
            .publish(RegistryUpdated::ComponentAdded(ComponentAdded {
                instance: instance_name.into(),
                plugin,
                component_id,
            }));

        Ok(())
    }

    fn remove_component(
        &mut self,
        instance_name: Option<String>,
        component_id: String,
    ) -> anyhow::Result<()> {
        let instance_name = InstanceName::from(instance_name);
        let component_id = Arc::new(component_id);

        let Some(component_data) = self.components.get(&component_id) else {
            anyhow::bail!("component '{}' not found", component_id)
        };

        if component_data.instance_name != instance_name {
            anyhow::bail!(
                "component '{}' not found on instance '{}'",
                component_id,
                instance_name
            );
        }

        let plugin = component_data.plugin().clone();

        self.components.remove(&component_id);

        let instance_data = self
            .instances
            .get_mut(&instance_name)
            .expect("data inconsistency: instance data not found");

        instance_data.remove_component(&component_id);

        if instance_data.is_empty() {
            self.instances.remove(&instance_name);
        }

        log::debug!(
            "component '{}' unregistered for instance '{}'",
            component_id,
            instance_name
        );

        self.on_update
            .publish(RegistryUpdated::ComponentRemoved(ComponentRemoved {
                instance: instance_name.into(),
                plugin,
                component_id,
            }));

        Ok(())
    }
}

impl Actor for Registry {
    type Args = ();

    type Error = anyhow::Error;

    async fn on_start(_args: Self::Args, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        Ok(Self {
            components: HashMap::new(),
            instances: HashMap::new(),
            on_update: PublisherHandle::from_name(UPDATE_PUBSUB_NAME)?,
        })
    }
}

impl message::Message<PluginAdd> for Registry {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: PluginAdd,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.add_plugin(msg.instance, msg.plugin)
    }
}

impl message::Message<PluginRemove> for Registry {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: PluginRemove,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.remove_plugin(msg.instance, &msg.plugin_id)
    }
}

impl message::Message<ComponentAdd> for Registry {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: ComponentAdd,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.add_component(msg.instance, msg.plugin_id, msg.component_id)
    }
}

impl message::Message<ComponentRemove> for Registry {
    type Reply = anyhow::Result<()>;

    async fn handle(
        &mut self,
        msg: ComponentRemove,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.remove_component(msg.instance, msg.component_id)
    }
}

#[derive(Debug, Clone)]
struct PluginAdd {
    instance: Option<String>,
    plugin: Arc<PluginMetadata>,
}

#[derive(Debug, Clone)]
struct PluginRemove {
    instance: Option<String>,
    plugin_id: String,
}

#[derive(Debug, Clone)]
struct ComponentAdd {
    instance: Option<String>,
    plugin_id: String,
    component_id: String,
}

#[derive(Debug, Clone)]
struct ComponentRemove {
    instance: Option<String>,
    component_id: String,
}

#[derive(Debug, Clone)]
pub enum RegistryUpdated {
    PluginAdded(PluginAdded),
    PluginRemoved(PluginRemoved),
    ComponentAdded(ComponentAdded),
    ComponentRemoved(ComponentRemoved),
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

impl<'a> From<InstanceName> for Option<Arc<String>> {
    fn from(value: InstanceName) -> Self {
        value.0
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
    ) -> anyhow::Result<Arc<PluginMetadata>> {
        let Some(plugin) = self.plugins.get_mut(plugin_id) else {
            anyhow::bail!(
                "plugin '{}:{}' does not exist",
                self.instance_name,
                plugin_id
            );
        };

        plugin.add_component();

        self.components.insert(component_id);

        Ok(plugin.metadata().clone())
    }

    pub fn remove_component(&mut self, component_id: &Arc<String>) {
        self.components.remove(component_id);
    }

    pub fn add_plugin(&mut self, plugin: Arc<PluginMetadata>) -> anyhow::Result<()> {
        let id = plugin.id().to_owned();

        if self.plugins.contains_key(&id) {
            anyhow::bail!("plugin '{}:{}' does already exist", self.instance_name, id);
        }

        self.plugins
            .insert(plugin.id().to_owned(), PluginData::new(plugin));
        Ok(())
    }

    pub fn remove_plugin(&mut self, plugin_id: &str) -> anyhow::Result<Arc<PluginMetadata>> {
        let Some(plugin) = self.plugins.get(plugin_id) else {
            anyhow::bail!(
                "plugin '{}:{}' does not exist",
                self.instance_name,
                plugin_id
            );
        };

        if plugin.used() {
            anyhow::bail!("plugin '{}:{}' is used", self.instance_name, plugin_id);
        }

        let plugin = plugin.metadata().clone();

        self.plugins.remove(plugin_id);

        Ok(plugin)
    }

    pub fn get_plugin(&self, id: &str) -> Option<&PluginData> {
        self.plugins.get(id)
    }
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
}

impl ComponentData {
    pub fn new(
        component_id: Arc<String>,
        instance_name: InstanceName,
        plugin: Arc<PluginMetadata>,
    ) -> Self {
        Self {
            component_id,
            instance_name,
            plugin,
        }
    }

    pub fn component_id(&self) -> &str {
        &self.component_id
    }

    pub fn instance_name(&self) -> Option<&str> {
        self.instance_name.0.as_ref().map(|s| s.as_str())
    }

    pub fn plugin(&self) -> &Arc<PluginMetadata> {
        &self.plugin
    }
}
