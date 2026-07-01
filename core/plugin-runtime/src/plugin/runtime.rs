use std::{convert::Infallible, fmt::Debug, sync::Arc};

use common::components::metadata;

// re-exports for plugins
pub use common::components::types::*;
use thiserror::Error;

/// ConfigError occurs when a config value fails to apply to a plugin instance.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config key not found: '{0}'")]
    KeyNotFound(String),

    #[error("config value type mismatch for key '{key}': {error}")]
    TypeMismatch {
        key: String,
        #[source]
        error: ConfigValueConversionError,
    },
}

impl ConfigError {
    pub fn key_not_found(key: impl Into<String>) -> Self {
        ConfigError::KeyNotFound(key.into())
    }

    pub fn type_mismatch(key: impl Into<String>, error: ConfigValueConversionError) -> Self {
        ConfigError::TypeMismatch {
            key: key.into(),
            error,
        }
    }
}

/// PluginError occurs when something goes wrong in a plugin instance.
#[derive(Error, Debug)]
#[error(transparent)]
pub struct PluginError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl PluginError {
    /// Creates a new PluginError from a boxed error.
    pub fn new<E: std::error::Error + Send + Sync + 'static>(error: E) -> Self {
        PluginError(Box::new(error))
    }
}

impl From<Infallible> for PluginError {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

/// MylifePluginRuntime represents a plugin type: it carries the plugin
/// metadata and acts as a factory for component instances.
pub trait MylifePluginRuntime: Send + Sync + Debug {
    /// Returns the metadata describing this plugin (its members, config, ...).
    fn metadata(&self) -> &Arc<metadata::PluginMetadata>;

    /// Creates a new component instance of this plugin with the given id.
    fn create(
        &self,
        id: &str,
        waker: Box<dyn Fn() + Send + Sync>,
        state_change: Box<dyn Fn(/*name:*/ &str, /*value:*/ &Value) + Send + Sync>,
    ) -> Box<dyn MylifeComponent>;
}

/// MylifeComponent is a component instance produced by a plugin, with the
/// lifecycle hooks the actor calls to configure, start, and drive it.
pub trait MylifeComponent: Send {
    /// Returns the unique identifier of the component.
    fn id(&self) -> &str;

    /// Returns the plugin metadata of the component.
    fn plugin(&self) -> &Arc<metadata::PluginMetadata>;

    /// Applies the instance configuration. Called once before init.
    fn configure(&mut self, config: &Config) -> Result<(), ConfigError>;

    /// Starts the component once configured. Called before any action.
    fn init(&mut self) -> Result<(), PluginError>;

    /// Hook invoked by the actor to let the component drive its asynchronous
    /// work (network, timers, ...) outside of synchronous action handling.
    fn async_handler(&mut self);

    /// Gets the state of the component by its name.
    fn get_state(&self, name: &str) -> Value;

    /// Executes an action on the component.
    fn execute_action(&mut self, name: &str, value: Value) -> Result<(), PluginError>;
}
