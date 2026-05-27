use std::{cell::RefCell, collections::{HashMap, HashSet}, rc::Rc};

pub trait Component {

}

struct InstanceData {
    name: String,
    components: HashSet<Rc<dyn Component>>,
    plugins: HashSet<Rc<Plugin>>,
}

pub struct Plugin {
    id: String,
    name: String,
    module: String,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone)]
struct InstancePluginKey(String, String);

impl InstancePluginKey {
    pub fn new(instance_name: String, plugin_id: String) -> Self {
        Self(instance_name, plugin_id)
    }
}


pub struct Registry {
    plugins_per_instance: HashMap<InstancePluginKey, Rc<Plugin>>,
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

    pub fn registry_plugin(&mut self, instance_name: &str, plugin: Plugin) {
        let key = InstancePluginKey::new(instance_name.to_owned(), plugin.id.clone());
    }

    pub fn unregistry_plugin(&mut self, instance_name: &str, plugin_id: &str) {

    }
}