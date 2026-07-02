use std::{
    collections::HashMap,
    fmt::{self},
    sync::Arc,
};

use crate::modules;
use common::{
    components::registry::{self, ComponentExecuteAction, RegistryHandle},
    utils::actors::CallError,
};
use kameo::{Actor, error::HookError, message, prelude::*};
use plugin_runtime::{
    metadata::{ConfigType, MemberType, PluginMetadata},
    runtime::{Config, ConfigError, ConfigValue, MylifeComponent, Value},
};
use thiserror::Error;

/// Untyped config.
/// It will be typed later when we know plugin definition
/// (we cannot determinate for instance if we need to deserialize a number value to a float or int)
pub type RawConfig = HashMap<String, serde_json::Value>;

#[derive(Debug, Clone)]
pub struct LocalComponentHandle {
    id: String,
    actor_ref: ActorRef<LocalComponent>,
}

#[derive(Debug, Error)]
#[error("failed to start component '{id}': {error}")]
pub struct ComponentStartError {
    id: String,
    error: Arc<LocalComponentActorError>,
}

impl ComponentStartError {
    pub fn new(id: String, error: Arc<LocalComponentActorError>) -> Self {
        Self { id, error }
    }
}

impl LocalComponentHandle {
    /// Start local component
    pub async fn start(config: LocalComponentConfig) -> Result<Self, ComponentStartError> {
        let id = config.id.clone();
        let actor_ref = LocalComponent::spawn(config);

        if let Err(e) = actor_ref.wait_for_startup_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("component '{}' panicked at startup: {}", id, p);
                }
                HookError::Error(e) => {
                    return Err(ComponentStartError::new(id, e));
                }
            }
        }

        Ok(Self { id, actor_ref })
    }

    /// Terminate local component
    pub async fn terminate(&self) {
        if let Err(error) = self.actor_ref.stop_gracefully().await {
            tracing::error!(
                ?error,
                component_id = self.id,
                "cannot stop component actor"
            );
            return;
        }

        if let Err(e) = self.actor_ref.wait_for_shutdown_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("component '{}' actor panicked at shutdown: {}", self.id, p);
                }
                HookError::Error(error) => {
                    tracing::error!(
                        ?error,
                        component_id = self.id,
                        "component failed to shutdown"
                    );
                }
            }
        }
    }
}

struct LocalComponent {
    id: String,
    component_impl: Box<dyn MylifeComponent>,
    registry: RegistryHandle,
}

impl fmt::Debug for LocalComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LocalComponent")
            .field("id", &self.id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct LocalComponentConfig {
    pub id: String,
    pub plugin: String,
    pub config: RawConfig,
    pub registry: RegistryHandle,
}

/// LocalComponentActorError occurs when something goes wrong in a local component actor.
#[derive(Debug, Error)]
pub enum LocalComponentActorError {
    #[error("plugin '{0}' not found")]
    PluginNotFound(String),

    #[error("component add error: {0}")]
    ComponentAddError(#[from] CallError<registry::ComponentAddError>),

    #[error("failed to translate component '{id}' config: {error}")]
    ConfigTranslationError {
        id: String,
        error: ConfigTranslationError,
    },

    #[error("failed to configure component '{id}': {error}")]
    ConfigureError {
        id: String,
        #[source]
        error: ConfigError,
    },

    #[error("failed to init component '{id}': {error}")]
    InitError {
        id: String,
        #[source]
        error: plugin_runtime::runtime::PluginError,
    },

    #[error("component remove error: {0}")]
    ComponentRemoveError(#[from] CallError<registry::ComponentRemoveError>),
}

impl LocalComponentActorError {
    pub fn plugin_not_found(plugin: impl Into<String>) -> Arc<Self> {
        Arc::new(LocalComponentActorError::PluginNotFound(plugin.into()))
    }

    pub fn component_add_error(error: CallError<registry::ComponentAddError>) -> Arc<Self> {
        Arc::new(LocalComponentActorError::ComponentAddError(error))
    }

    pub fn component_remove_error(error: CallError<registry::ComponentRemoveError>) -> Arc<Self> {
        Arc::new(LocalComponentActorError::ComponentRemoveError(error))
    }

    pub fn config_translation_error(
        id: impl Into<String>,
        error: ConfigTranslationError,
    ) -> Arc<Self> {
        Arc::new(LocalComponentActorError::ConfigTranslationError {
            id: id.into(),
            error,
        })
    }

    pub fn configure_error(id: impl Into<String>, error: ConfigError) -> Arc<Self> {
        Arc::new(LocalComponentActorError::ConfigureError {
            id: id.into(),
            error,
        })
    }

    pub fn init_error(
        id: impl Into<String>,
        error: plugin_runtime::runtime::PluginError,
    ) -> Arc<Self> {
        Arc::new(LocalComponentActorError::InitError {
            id: id.into(),
            error,
        })
    }
}

#[derive(Debug, Error)]
pub enum ConfigTranslationError {
    #[error("missing config key '{0}'")]
    KeyMissing(String),

    #[error("bad config value '{value:?}': cannot convert into type '{ty:?}' for key '{key}'")]
    BadValue {
        key: String,
        ty: ConfigType,
        value: serde_json::Value,
    },
}

impl ConfigTranslationError {
    pub fn key_missing(key: impl Into<String>) -> ConfigTranslationError {
        ConfigTranslationError::KeyMissing(key.into())
    }

    pub fn bad_value(
        key: impl Into<String>,
        ty: ConfigType,
        value: &serde_json::Value,
    ) -> ConfigTranslationError {
        ConfigTranslationError::BadValue {
            key: key.into(),
            ty,
            value: value.clone(),
        }
    }
}

impl LocalComponent {
    fn translate_config(
        metadata: &PluginMetadata,
        raw_config: RawConfig,
    ) -> Result<Config, ConfigTranslationError> {
        let mut config = Config::new();

        for (key, item) in metadata.config() {
            let Some(raw_value) = raw_config.get(key) else {
                return Err(ConfigTranslationError::key_missing(key));
            };

            let ty = item.value_type();

            let value = match ty {
                ConfigType::String => raw_value
                    .as_str()
                    .map(|v| ConfigValue::String(v.to_owned())),
                ConfigType::Bool => raw_value.as_bool().map(|v| ConfigValue::Bool(v)),
                ConfigType::Integer => raw_value.as_i64().map(|v| ConfigValue::Integer(v)),
                ConfigType::Float => raw_value.as_f64().map(|v| ConfigValue::Float(v)),
            };

            let value =
                value.ok_or_else(|| ConfigTranslationError::bad_value(key, ty, raw_value))?;
            config.insert(key.clone(), value);
        }

        Ok(config)
    }
}

impl Actor for LocalComponent {
    type Args = LocalComponentConfig;
    type Error = Arc<LocalComponentActorError>;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let LocalComponentConfig {
            id,
            plugin,
            config,
            registry,
        } = config;

        let plugin = modules::registry()
            .plugin(&plugin)
            .ok_or_else(|| LocalComponentActorError::plugin_not_found(plugin.clone()))?;

        let config = Self::translate_config(plugin.metadata(), config).map_err(|error| {
            LocalComponentActorError::config_translation_error(id.clone(), error)
        })?;

        // We cannot fail anymore, create component now on the registry to get its handle
        let handle = registry
            .component_add(
                None,
                plugin.metadata().id().to_owned(),
                id.clone(),
                actor_ref.clone().recipient(),
            )
            .await
            .map_err(|error| LocalComponentActorError::component_add_error(error))?;

        let waker = {
            let id = id.clone();
            let weak_ref_self = actor_ref.downgrade();

            move || {
                let Some(ref_self) = weak_ref_self.upgrade() else {
                    tracing::error!(
                        error = "cannot get actor ref",
                        component_id = id,
                        "cannot wake component"
                    );
                    return;
                };

                if let Err(error) = ref_self.tell(ComponentWakeMessage).try_send() {
                    tracing::error!(
                        ?error,
                        component_id = id,
                        "cannot wake component: cannot send message"
                    );
                }
            }
        };

        let state_change = {
            let handle = handle.clone();

            move |name: &str, value: &Value| {
                handle.state_changed(name.to_owned(), value.clone());
            }
        };

        let mut component_impl = plugin.create(&id, Box::new(waker), Box::new(state_change));

        if let Err(e) = component_impl.configure(&config) {
            if let Err(error) = registry.component_remove(id.clone()).await {
                tracing::error!(
                    ?error,
                    component_id = id,
                    "can not remove component that failed during configure",
                );
            }

            return Err(LocalComponentActorError::configure_error(id, e));
        }

        if let Err(e) = component_impl.init() {
            if let Err(error) = registry.component_remove(id.clone()).await {
                tracing::error!(
                    ?error,
                    component_id = id,
                    "can not remove component that failed during init",
                );
            }

            return Err(LocalComponentActorError::init_error(id, e));
        }

        // publish all state immediately
        for (name, member) in plugin.metadata().members() {
            if member.member_type() == MemberType::State {
                let value = component_impl.get_state(name);
                handle.state_changed(name.clone(), value);
            }
        }

        Ok(Self {
            id,
            component_impl,
            registry,
        })
    }

    async fn on_stop(
        &mut self,
        _actor_ref: WeakActorRef<Self>,
        _reason: ActorStopReason,
    ) -> Result<(), Self::Error> {
        self.registry
            .component_remove(self.id.clone())
            .await
            .map_err(|error| LocalComponentActorError::component_remove_error(error))?;

        Ok(())
    }
}

impl message::Message<ComponentWakeMessage> for LocalComponent {
    type Reply = ();

    async fn handle(
        &mut self,
        _msg: ComponentWakeMessage,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.component_impl.async_handler();
    }
}

impl message::Message<ComponentExecuteAction> for LocalComponent {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ComponentExecuteAction,
        _ctx: &mut message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Err(error) = self
            .component_impl
            .execute_action(msg.name(), msg.value().clone())
        {
            tracing::error!(
                ?error,
                component = self.id,
                action = msg.name(),
                value = ?msg.value(),
                "failed to execute action",
            );
        }
    }
}

#[derive(Debug)]
struct ComponentWakeMessage;
