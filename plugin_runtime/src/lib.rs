use properties::{state, config, action};

pub mod metadata;
pub mod properties;

#[derive(Debug)]
pub enum NetValue {
    Int8(i8),
    UInt8(i8),
    Int32(i32),
    UInt32(i32),
    String(String),
    Float(f64),
    Bool(bool),
    Complex(),
}

#[derive(Debug)]
pub enum ConfigValue {
    String(String),
    Bool(bool),
    Integer(i64),
    Float(f64),
}

pub trait Plugin {
    fn runtime_data(&self) -> (&[&dyn state::Definition], &[&dyn action::Definition], &[&dyn config::Definition]);
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
