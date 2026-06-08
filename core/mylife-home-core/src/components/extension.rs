use crate::modules;
use common::components::{ComponentsData, ComponentsHandler};

pub struct Extension {}

impl Extension {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsHandler for Extension {
    fn init(&mut self, data: &mut ComponentsData) {
        let registry = data.registry_mut();

        for plugin in modules::registry().plugins() {
            registry.add_plugin(None, plugin.metadata().clone());
        }

        // TODO: load components
        let component = modules::registry()
            .plugin("logic-base.value-binary")
            .unwrap()
            .create(
                "comp-id",
                Box::new(|| {
                    println!("WAKE ASKED");
                }),
            );

        registry.add_component(None, component);
    }
}
