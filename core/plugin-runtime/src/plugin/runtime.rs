use std::fmt::Debug;

use common::components::{Component, metadata};

// re-exports for plugins
pub use common::components::types::*;

pub trait MylifePluginRuntime: Send + Sync + Debug {
    fn metadata(&self) -> &metadata::PluginMetadata;
    fn create(&self, id: &str) -> Box<dyn MylifeComponent>;
}

pub trait MylifeComponent: Component {
    fn configure(&mut self, config: &Config) -> anyhow::Result<()>;
    fn init(&mut self) -> anyhow::Result<()>;
}
