use std::fmt::Debug;

use common::components::{Component, metadata};

// re-exports for plugins
pub use common::components::types::*;

/// MylifePluginRuntime represents a plugin type: it carries the plugin
/// metadata and acts as a factory for component instances.
pub trait MylifePluginRuntime: Send + Sync + Debug {
    /// Returns the metadata describing this plugin (its members, config, ...).
    fn metadata(&self) -> &metadata::PluginMetadata;

    /// Creates a new component instance of this plugin with the given id.
    fn create(&self, id: &str) -> Box<dyn MylifeComponent>;
}

/// MylifeComponent is a component instance produced by a plugin, with the
/// lifecycle hooks the actor calls to configure, start, and drive it.
pub trait MylifeComponent: Component {
    /// Applies the instance configuration. Called once before init.
    fn configure(&mut self, config: &Config) -> anyhow::Result<()>;

    /// Starts the component once configured. Called before any action.
    fn init(&mut self) -> anyhow::Result<()>;

    /// Hook invoked by the actor to let the component drive its asynchronous
    /// work (network, timers, ...) outside of synchronous action handling.
    fn async_handler(&mut self);
}
