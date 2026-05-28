use std::{cell::RefCell, collections::HashMap, sync::Arc};

use crate::components::{
    Component,
    metadata::PluginMetadata,
    observable::{Observable, Observer, ObserverId, Subject},
};

/// Registry is responsible for managing the plugins and components of all instances, and providing an observable interface for other modules to subscribe to registry events.
pub struct Registry {
    plugins: HashMap<String, Arc<PluginMetadata>>,
    components: HashMap<String, ComponentData>,
    instances: HashMap<String, RefCell<InstanceData>>,
    subject: Subject<RegistryEvent>,
}

impl Registry {
    /// Creates a new Registry instance.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            components: HashMap::new(),
            instances: HashMap::new(),
            subject: Subject::new(),
        }
    }

    /// Add a plugin to the registry.
    pub fn add_plugin(&mut self, instance_name: Option<&str>, plugin: Arc<PluginMetadata>) {
        let id = Self::build_plugin_id(instance_name, plugin.id());
        let instance_name = Self::build_instance_name(instance_name);

        if self.plugins.contains_key(&id) {
            log::error!("plugin {} already added", id);
            return;
        }

        self.plugins.insert(id.clone(), plugin.clone());

        let instance_data = self
            .instances
            .entry(instance_name.clone())
            .or_insert_with(|| RefCell::new(InstanceData::new(instance_name.clone())));

        instance_data.borrow_mut().add_plugin(plugin.clone());

        log::debug!("plugin {} added", id);

        self.subject.notify(&RegistryEvent::PluginAdded {
            instance_name,
            plugin,
        });
    }

    /// Removes a plugin from the registry.
    pub fn remove_plugin(&mut self, instance_name: Option<&str>, plugin: Arc<PluginMetadata>) {
        let id = Self::build_plugin_id(instance_name, plugin.id());
        let instance_name = Self::build_instance_name(instance_name);

        let Some(plugin) = self.plugins.remove(&id) else {
            log::error!("plugin {} not found", id);
            return;
        };

        let is_empty = {
            let mut instance_data = self
                .instances
                .get(&instance_name)
                .expect("data inconsistency: instance data not found")
                .borrow_mut();
            instance_data.remove_plugin(&plugin);
            instance_data.is_empty()
        };

        if is_empty {
            self.instances.remove(&instance_name);
        }

        log::debug!("plugin {} removed", id);

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
    ) -> Option<Arc<PluginMetadata>> {
        let id = Self::build_plugin_id(instance_name, plugin_id);
        self.plugins.get(&id).cloned()
    }

    /// Gets all plugins of an instance.
    pub fn get_plugins(&self, instance_name: Option<&str>) -> Vec<Arc<PluginMetadata>> {
        let instance_name = Self::build_instance_name(instance_name);

        if let Some(instance_data) = self.instances.get(&instance_name) {
            instance_data.borrow().plugins.values().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Adds a component to the registry.
    pub fn add_component(
        &mut self,
        instance_name: Option<&str>,
        component: Arc<RefCell<dyn Component>>,
    ) {
        let component_id = component.borrow().id().to_owned();
        let instance_name = Self::build_instance_name(instance_name);

        if self.components.contains_key(&component_id) {
            log::error!("component {} already registered", component_id);
            return;
        }

        self.components.insert(
            component_id.clone(),
            ComponentData::new(instance_name.clone(), component.clone()),
        );

        let instance_data = self
            .instances
            .entry(instance_name.clone())
            .or_insert_with(|| RefCell::new(InstanceData::new(instance_name.clone())));

        instance_data.borrow_mut().add_component(component.clone());

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

        let is_empty = {
            let mut instance_data = self
                .instances
                .get(&instance_name)
                .expect("data inconsistency: instance data not found")
                .borrow_mut();
            instance_data.remove_component(&component_data.component());
            instance_data.is_empty()
        };

        if is_empty {
            self.instances.remove(&instance_name);
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
    pub fn get_component(&self, component_id: &str) -> Option<Arc<RefCell<dyn Component>>> {
        self.components
            .get(component_id)
            .map(|data| data.component().clone())
    }

    pub fn get_component_data(
        &self,
        component_id: &str,
    ) -> Option<(String, Arc<RefCell<dyn Component>>)> {
        self.components
            .get(component_id)
            .map(|data| (data.instance_name().to_owned(), data.component().clone()))
    }

    /// Gets all components.
    pub fn get_components(&self) -> Vec<Arc<RefCell<dyn Component>>> {
        self.components
            .values()
            .map(|data| data.component().clone())
            .collect()
    }

    /// Gets all instance names that have registered plugins or components.
    pub fn get_instance_names(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }

    fn build_plugin_id(instance_name: Option<&str>, plugin_id: &str) -> String {
        let instance_name = instance_name.unwrap_or("local");
        format!("{}:{}", instance_name, plugin_id)
    }

    fn build_instance_name(instance_name: Option<&str>) -> String {
        instance_name.unwrap_or("local").to_owned()
    }
}

impl Observable<RegistryEvent> for Registry {
    fn observe(&mut self, observer: Box<Observer<RegistryEvent>>) -> ObserverId {
        self.subject.observe(observer)
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.subject.unobserve(id)
    }
}

/// RegistryEvent represents the events that can occur in the registry, such as adding or removing a plugin or component.
pub enum RegistryEvent {
    /// PluginAdded is emitted when a plugin is added to the registry, containing the instance name and the plugin metadata.
    PluginAdded {
        instance_name: String,
        plugin: Arc<PluginMetadata>,
    },

    /// PluginRemoved is emitted when a plugin is removed from the registry, containing the instance name and the plugin metadata.
    PluginRemoved {
        instance_name: String,
        plugin: Arc<PluginMetadata>,
    },

    /// ComponentAdded is emitted when a component is added to the registry, containing the instance name and the component.
    ComponentAdded {
        instance_name: String,
        component: Arc<RefCell<dyn Component>>,
    },

    /// ComponentRemoved is emitted when a component is removed from the registry, containing the instance name and the component.
    ComponentRemoved {
        instance_name: String,
        component: Arc<RefCell<dyn Component>>,
    },
}
struct InstanceData {
    name: String,
    components: HashMap<String, Arc<RefCell<dyn Component>>>,
    plugins: HashMap<String, Arc<PluginMetadata>>,
}

impl InstanceData {
    pub fn new(name: String) -> Self {
        Self {
            name,
            components: HashMap::new(),
            plugins: HashMap::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_empty(&self) -> bool {
        self.components.is_empty() && self.plugins.is_empty()
    }

    pub fn add_component(&mut self, component: Arc<RefCell<dyn Component>>) {
        let id = component.borrow().id().to_owned();
        self.components.insert(id, component);
    }

    pub fn add_plugin(&mut self, plugin: Arc<PluginMetadata>) {
        self.plugins.insert(plugin.id().to_owned(), plugin);
    }

    pub fn remove_component(&mut self, component: &Arc<RefCell<dyn Component>>) {
        self.components.remove(component.borrow().id());
    }

    pub fn remove_plugin(&mut self, plugin: &Arc<PluginMetadata>) {
        self.plugins.remove(plugin.id());
    }
}

struct ComponentData {
    instance_name: String,
    component: Arc<RefCell<dyn Component>>,
}

impl ComponentData {
    pub fn new(instance_name: String, component: Arc<RefCell<dyn Component>>) -> Self {
        Self {
            instance_name,
            component,
        }
    }

    pub fn instance_name(&self) -> &str {
        &self.instance_name
    }

    pub fn component(&self) -> Arc<RefCell<dyn Component>> {
        self.component.clone()
    }
}
