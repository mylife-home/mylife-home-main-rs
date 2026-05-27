use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::components::{metadata::PluginMetadata, observable::{Observable, ObserverId, Subject}};

/// Component represents a component that can be registered to the registry.
pub trait Component {
    fn id(&self) -> &str;
}

struct InstanceData {
    name: String,
    components: HashMap<String, Rc<dyn Component>>,
    plugins: HashMap<String, Rc<PluginMetadata>>,
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

    pub fn add_component(&mut self, component: Rc<dyn Component>) {
        self.components.insert(component.id().to_owned(), component);
    }

    pub fn add_plugin(&mut self, plugin: Rc<PluginMetadata>) {
        self.plugins.insert(plugin.id().to_owned(), plugin);
    }

    pub fn remove_component(&mut self, component: &Rc<dyn Component>) {
        self.components.remove(component.id());
    }

    pub fn remove_plugin(&mut self, plugin: &Rc<PluginMetadata>) {
        self.plugins.remove(plugin.id());
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct InstancePluginKey(String, String);

impl InstancePluginKey {
    pub fn new(instance_name: &str, plugin_id: &str) -> Self {
        Self(instance_name.to_owned(), plugin_id.to_owned())
    }
}

/// Registry is responsible for managing the plugins and components of all instances, and providing an observable interface for other modules to subscribe to registry events.
pub struct Registry {
    plugins_per_instance: HashMap<InstancePluginKey, Rc<PluginMetadata>>,
    components: HashMap<String, Rc<RefCell<dyn Component>>>,
    instances: HashMap<String, Rc<RefCell<InstanceData>>>,
    subject: Subject<RegistryEvent>,
}

impl Registry {
    /// Creates a new Registry instance.
    pub fn new() -> Self {
        Self {
            plugins_per_instance: HashMap::new(),
            components: HashMap::new(),
            instances: HashMap::new(),
            subject: Subject::new(),
        }
    }

    /// Registers a plugin to the registry.
    pub fn registry_plugin(&mut self, instance_name: &str, plugin: Rc<PluginMetadata>) {
        let key = InstancePluginKey::new(instance_name, plugin.id());
        if self.plugins_per_instance.contains_key(&key) {
            log::error!(
                "plugin {} already registered for instance {}",
                plugin.id(),
                instance_name
            );
            return;
        }

        self.plugins_per_instance.insert(key, plugin.clone());

        let instance_data = self
            .instances
            .entry(instance_name.to_owned())
            .or_insert_with(|| Rc::new(RefCell::new(InstanceData::new(instance_name.to_owned()))));

        instance_data.borrow_mut().add_plugin(plugin.clone());

        log::debug!(
            "plugin {} registered for instance {}",
            plugin.id(),
            instance_name
        );
        self.subject.notify(&RegistryEvent::PluginAdded {
            instance_name: instance_name.to_owned(),
            plugin,
        });
    }

    /// Unregisters a plugin from the registry.
    pub fn unregistry_plugin(&mut self, instance_name: &str, plugin_id: &str) {
        // TODO
    }
}

impl Observable<RegistryEvent> for Registry {
    fn observe(&mut self, observer: impl Fn(&RegistryEvent) + 'static) -> ObserverId {
        self.subject.observe(observer)
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.subject.unobserve(id)
    }
}

/// RegistryEvent represents the events that can occur in the registry, such as adding or removing a plugin or component.
pub enum RegistryEvent {
    /// PluginAdded is emitted when a plugin is added to the registry, containing the instance name and the plugin metadata.
    PluginAdded{ instance_name: String, plugin: Rc<PluginMetadata> },

    /// PluginRemoved is emitted when a plugin is removed from the registry, containing the instance name and the plugin metadata.
    PluginRemoved{ instance_name: String, plugin: Rc<PluginMetadata> },

    /// ComponentAdded is emitted when a component is added to the registry, containing the instance name and the component.
    ComponentAdded{ instance_name: String, component: Rc<RefCell<dyn Component>> },

    /// ComponentRemoved is emitted when a component is removed from the registry, containing the instance name and the component.
    ComponentRemoved{ instance_name: String, component: Rc<RefCell<dyn Component>> },
}