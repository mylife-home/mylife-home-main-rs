use crate::modules;
use common::components::{ComponentsData, ComponentsHandler};

pub struct LocalPlugins {}

impl LocalPlugins {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsHandler for LocalPlugins {
    fn init(&mut self, data: &mut ComponentsData) {
        let registry = data.registry_mut();

        for plugin in modules::registry().plugins() {
            registry.add_plugin(None, plugin.metadata().clone());
        }
    }
}
