use std::collections::HashMap;

use crate::metadata;

pub trait MylifePluginRuntime {
    fn metadata(&self) -> &metadata::PluginMetadata;
    fn create(&self) -> Box<dyn MylifeComponent>;
}

pub trait MylifeComponent {
    fn set_on_fail(&mut self, handler: fn(error: Box<dyn std::error::Error>));
    fn set_on_state(&mut self, handler: fn(name: &str, state: &Value));
    fn configure(&mut self, config: &Config);
    fn execute_action(&mut self, name: &str, action: &Value);
}

pub type Config = HashMap<String, ConfigValue>;

#[derive(Debug, Clone)]
pub enum Value {
    RangeU8(u8),
    RangeI8(i8),
    RangeU32(u32),
    RangeI32(i32),
    Text(String),
    Float(f32),
    Bool(bool),
    Enum(String), // TODO: native enum binding?
    Complex,      // unsupported for now
}

#[derive(Debug, Clone)]
pub enum ConfigValue {
    String(String),
    Bool(bool),
    Integer(i64),
    Float(f64),
}
