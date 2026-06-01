use crate::modules;
use common::components::{Components, ComponentsHandler, Registry};

pub struct Extension {}

impl Extension {
    pub fn new() -> Self {
        Self {}
    }
}

impl ComponentsHandler for Extension {
    fn init(&mut self, registry: &mut Registry) {
        for plugin in modules::registry().plugins() {
            registry.add_plugin(None, plugin.metadata().clone());
        }

        // TODO: load components
        let component = modules::registry()
            .plugin("logic-base.value-binary")
            .unwrap()
            .create_component(
                "comp-id",
                Box::new(|| {
                    println!("WAKE ASKED");
                }),
            );

        registry.add_component(None, component);
    }
}
