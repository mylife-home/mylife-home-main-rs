use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::components::metadata::PluginMetadata;

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

pub struct Registry {
    plugins_per_instance: HashMap<InstancePluginKey, Rc<PluginMetadata>>,
    components: HashMap<String, Rc<RefCell<dyn Component>>>,
    instances: HashMap<String, Rc<RefCell<InstanceData>>>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            plugins_per_instance: HashMap::new(),
            components: HashMap::new(),
            instances: HashMap::new(),
        }
    }

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
        // TODO: emit event: plugin.add
    }

    pub fn unregistry_plugin(&mut self, instance_name: &str, plugin_id: &str) {
        // TODO
    }
}
