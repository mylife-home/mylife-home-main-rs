use properties::{StateDef, ConfigDef, ActionDef};

pub mod metadata;
pub mod properties;

pub trait Plugin {
    fn runtime_data(&self) -> (&[&dyn StateDef], &[&dyn ActionDef], &[&dyn ConfigDef]);
    fn init(&mut self) {}
    fn terminate(&mut self) {}
}

pub struct PluginData {
    metadata: metadata::PluginMetadata,
    factory: fn() -> Box<dyn Plugin>,
}

impl PluginData {
    pub fn new(metadata: metadata::PluginMetadata, factory: fn() -> Box<dyn Plugin>) -> PluginData {
        PluginData { metadata, factory }
    }

    pub fn metadata(&self) -> &metadata::PluginMetadata {
        &self.metadata
    }

    pub fn create(&self) -> Box<dyn Plugin> {
        (self.factory)()
    }
}
