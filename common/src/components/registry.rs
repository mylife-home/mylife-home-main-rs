use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::components::{
    Component,
    metadata::PluginMetadata,
    observable::{EventType, Observable, Observer, ObserverId, Subject},
};

/// Registry is responsible for managing the plugins and components of all instances, and providing an observable interface for other modules to subscribe to registry events.
pub struct Registry {
    components: HashMap<String, ComponentData>,
    instances: HashMap<String, InstanceData>,
    subject: Subject<RegistryEventType>,
}

impl Registry {
    /// Creates a new Registry instance.
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            instances: HashMap::new(),
            subject: Subject::new(),
        }
    }

    /// Add a plugin to the registry.
    pub fn add_plugin(&mut self, instance_name: Option<&str>, plugin: Arc<PluginMetadata>) {
        let instance_name = Self::build_instance_name(instance_name);

        let instance_data = self
            .instances
            .entry(instance_name.to_owned())
            .or_insert_with(|| InstanceData::new(instance_name.to_owned()));

        if instance_data.get_plugin(plugin.id()).is_some() {
            log::error!("plugin {}:{} already added", instance_name, plugin.id());
            return;
        }

        instance_data.add_plugin(plugin.to_owned());

        log::debug!("plugin {}:{} added", instance_name, plugin.id());

        self.subject.notify(&RegistryEvent::PluginAdded {
            instance_name,
            plugin: &plugin,
        });
    }

    /// Removes a plugin from the registry.
    pub fn remove_plugin(&mut self, instance_name: Option<&str>, plugin: &Arc<PluginMetadata>) {
        let instance_name = Self::build_instance_name(instance_name);

        let Some(instance_data) = self.instances.get_mut(instance_name) else {
            log::error!("plugin {}:{} not found", instance_name, plugin.id());
            return;
        };

        instance_data.remove_plugin(plugin);

        if instance_data.is_empty() {
            self.instances.remove(instance_name);
        }

        self.subject.notify(&RegistryEvent::PluginRemoved {
            instance_name,
            plugin,
        });
    }

    /// Gets a plugin by its unique identifier, which is a combination of the instance name and the plugin id.
    pub fn get_plugin(
        &self,
        instance_name: Option<&str>,
        plugin_id: &str,
    ) -> Option<&Arc<PluginMetadata>> {
        let instance_name = Self::build_instance_name(instance_name);

        let Some(instance_data) = self.instances.get(instance_name) else {
            return None;
        };

        instance_data.get_plugin(plugin_id)
    }

    /// Gets all plugins of an instance.
    pub fn get_plugins(&self, instance_name: Option<&str>) -> Vec<Arc<PluginMetadata>> {
        let instance_name = Self::build_instance_name(instance_name);

        if let Some(instance_data) = self.instances.get(instance_name) {
            instance_data.plugins.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Adds a component to the registry.
    pub fn add_component(
        &mut self,
        instance_name: Option<&str>,
        component: impl Component + 'static,
    ) {
        let component_id = component.id().to_owned();
        let instance_name = Self::build_instance_name(instance_name);

        if self.components.contains_key(&component_id) {
            log::error!("component {} already registered", component_id);
            return;
        }

        self.components.insert(
            component_id.clone(),
            ComponentData::new(instance_name.to_owned(), Box::new(component)),
        );

        let instance_data = self
            .instances
            .entry(instance_name.to_owned())
            .or_insert_with(|| InstanceData::new(instance_name.to_owned()));

        let component = self
            .components
            .get(&component_id)
            .expect("data inconsistency: could not get component")
            .component();

        instance_data.add_component(component);

        log::debug!(
            "component {} registered for instance {}",
            component_id,
            instance_name
        );
        self.subject.notify(&RegistryEvent::ComponentAdded {
            instance_name,
            component,
        });
    }

    /// Removes a component from the registry.
    pub fn remove_component(
        &mut self,
        instance_name: Option<&str>,
        component: Arc<RefCell<dyn Component>>,
    ) {
        let component_id = component.borrow().id().to_owned();
        let instance_name = Self::build_instance_name(instance_name);

        let component_data = match self.components.remove(&component_id) {
            Some(component_data) => component_data,
            None => {
                log::error!("component {} not found", component_id);
                return;
            }
        };

        let component = component_data.component();

        let instance_data = self
            .instances
            .get_mut(instance_name)
            .expect("data inconsistency: instance data not found");

        instance_data.remove_component(component);

        if instance_data.is_empty() {
            self.instances.remove(instance_name);
        }

        log::debug!(
            "component {} unregistered for instance {}",
            component_id,
            instance_name
        );

        self.subject.notify(&RegistryEvent::ComponentRemoved {
            instance_name,
            component,
        });
    }

    /// Gets a component by its unique identifier.
    pub fn get_component(&self, component_id: &str) -> Option<&dyn Component> {
        self.components
            .get(component_id)
            .map(|data| data.component())
    }

    pub fn get_component_mut(&mut self, component_id: &str) -> Option<&mut dyn Component> {
        self.components
            .get_mut(component_id)
            .map(|data| data.component_mut())
    }

    pub fn get_component_data(&self, component_id: &str) -> Option<(&str, &dyn Component)> {
        self.components.get(component_id).map(|data| data.data())
    }

    pub fn get_component_data_mut(
        &mut self,
        component_id: &str,
    ) -> Option<(&str, &mut dyn Component)> {
        self.components
            .get_mut(component_id)
            .map(|data| data.data_mut())
    }

    /// Gets all components.
    pub fn get_components(&self) -> Vec<&dyn Component> {
        self.components
            .values()
            .map(|data| data.component())
            .collect()
    }

    /// Gets all instance names that have registered plugins or components.
    pub fn get_instance_names(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }

    fn build_instance_name(instance_name: Option<&str>) -> &str {
        instance_name.unwrap_or("local")
    }
}

impl Observable<RegistryEventType> for Registry {
    fn observe(&mut self, observer: Box<Observer<RegistryEventType>>) -> ObserverId {
        self.subject.observe(observer)
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.subject.unobserve(id)
    }
}

#[derive(Debug)]
pub struct RegistryEventType;

impl EventType for RegistryEventType {
    type Event<'a> = RegistryEvent<'a>;
}

/// RegistryEvent represents the events that can occur in the registry, such as adding or removing a plugin or component.
pub enum RegistryEvent<'a> {
    /// PluginAdded is emitted when a plugin is added to the registry, containing the instance name and the plugin metadata.
    PluginAdded {
        instance_name: &'a str,
        plugin: &'a Arc<PluginMetadata>,
    },

    /// PluginRemoved is emitted when a plugin is removed from the registry, containing the instance name and the plugin metadata.
    PluginRemoved {
        instance_name: &'a str,
        plugin: &'a Arc<PluginMetadata>,
    },

    /// ComponentAdded is emitted when a component is added to the registry, containing the instance name and the component.
    ComponentAdded {
        instance_name: &'a str,
        component: &'a dyn Component,
    },

    /// ComponentRemoved is emitted when a component is removed from the registry, containing the instance name and the component.
    ComponentRemoved {
        instance_name: &'a str,
        component: &'a dyn Component,
    },
}
struct InstanceData {
    name: String,
    components: HashSet<String>,
    plugins: HashMap<String, Arc<PluginMetadata>>,
}

impl InstanceData {
    pub fn new(name: String) -> Self {
        Self {
            name,
            components: HashSet::new(),
            plugins: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty() && self.plugins.is_empty()
    }

    pub fn add_component(&mut self, component: &dyn Component) {
        self.components.insert(component.id().to_owned());
    }

    pub fn remove_component(&mut self, component: &dyn Component) {
        self.components.remove(component.id());
    }

    pub fn add_plugin(&mut self, plugin: Arc<PluginMetadata>) {
        self.plugins.insert(plugin.id().to_owned(), plugin);
    }

    pub fn remove_plugin(&mut self, plugin: &Arc<PluginMetadata>) {
        self.plugins.remove(plugin.id());
    }

    pub fn get_plugin(&self, id: &str) -> Option<&Arc<PluginMetadata>> {
        self.plugins.get(id)
    }
}

struct ComponentData {
    instance_name: String,
    component: Box<dyn Component>,
}

impl ComponentData {
    pub fn new(instance_name: String, component: Box<dyn Component>) -> Self {
        Self {
            instance_name,
            component,
        }
    }

    pub fn component(&self) -> &dyn Component {
        self.component.as_ref()
    }

    pub fn component_mut(&mut self) -> &mut dyn Component {
        self.component.as_mut()
    }

    pub fn data(&self) -> (&str, &dyn Component) {
        (&self.instance_name, self.component.as_ref())
    }

    pub fn data_mut(&mut self) -> (&str, &mut dyn Component) {
        (&self.instance_name, self.component.as_mut())
    }
}
