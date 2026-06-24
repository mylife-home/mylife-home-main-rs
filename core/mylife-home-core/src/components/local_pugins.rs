use crate::modules;
use common::components::registry;

pub async fn init() {
    // plugin are here forever, we can just register them
    let registry = registry::RegistryHandle::new().expect("Cannot get registry access");

    for plugin in modules::registry().plugins() {
        registry
            .plugin_add(None, plugin.metadata().clone())
            .await
            .expect("Could not registry plugin")
    }
}
