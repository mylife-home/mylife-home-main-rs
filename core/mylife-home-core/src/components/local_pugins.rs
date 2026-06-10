use crate::modules;
use common::{
    components::{ComponentsData, ComponentsHandler, ComponentsMessage},
    utils::mailbox::MailboxHandle,
};

pub struct LocalPlugins {}

impl LocalPlugins {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsHandler for LocalPlugins {
    fn init(&mut self, data: &mut ComponentsData, _: &MailboxHandle<Box<dyn ComponentsMessage>>) {
        let registry = data.registry_mut();

        for plugin in modules::registry().plugins() {
            registry.add_plugin(None, plugin.metadata().clone());
        }
    }
}
