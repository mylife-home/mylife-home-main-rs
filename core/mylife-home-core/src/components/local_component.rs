use std::{
    fmt::{self},
    sync::Arc,
};

use crate::modules;
use anyhow::Context;
use common::components::registry::{ComponentExecuteAction, RegistryHandle};
use kameo::{Actor, error::HookError, message, prelude::*};
use plugin_runtime::{
    metadata::MemberType,
    runtime::{Config, MylifeComponent, Value},
};

#[derive(Debug, Clone)]
pub struct LocalComponentHandle {
    id: String,
    actor_ref: ActorRef<LocalComponent>,
}

impl LocalComponentHandle {
    /// Start local component
    pub async fn start(config: LocalComponentConfig) -> anyhow::Result<Self> {
        let id = config.id.clone();
        let actor_ref = LocalComponent::spawn(config);

        if let Err(e) = actor_ref.wait_for_startup_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("component '{}' panicked at startup: {}", id, p);
                }
                HookError::Error(e) => {
                    // cannot reuse Arc<anyhow::Error>
                    anyhow::bail!("component '{}' failed to start: {}", id, e);
                }
            }
        }

        Ok(Self { id, actor_ref })
    }

    /// Terminate local component
    pub async fn terminate(&self) {
        if let Err(e) = self.actor_ref.stop_gracefully().await {
            log::error!("cannot stop component actor '{}': {}", self.id, e);
            return;
        }

        if let Err(e) = self.actor_ref.wait_for_shutdown_result().await {
            match e {
                HookError::Panicked(p) => {
                    panic!("component '{}' actor panicked at shutdown: {}", self.id, p);
                }
                HookError::Error(e) => {
                    // cannot reuse Arc<anyhow::Error>
                    log::error!("component '{}' failed to shutdown: {}", self.id, e);
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
    pub config: Config,
    pub registry: RegistryHandle,
}

impl Actor for LocalComponent {
    type Args = LocalComponentConfig;
    type Error = Arc<anyhow::Error>;

    async fn on_start(config: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        let LocalComponentConfig {
            id,
            plugin,
            config,
            registry,
        } = config;

        let plugin = modules::registry()
            .plugin(&plugin)
            .with_context(|| format!("plugin '{}' not found", plugin))?;

        // We cannot fail anymore, create component now on the registry to get its handle
        let handle = registry
            .component_add(
                None,
                plugin.metadata().id().to_owned(),
                id.clone(),
                actor_ref.clone().recipient(),
            )
            .await?;

        let waker = {
            let id = id.clone();
            let weak_ref_self = actor_ref.downgrade();

            move || {
                let Some(ref_self) = weak_ref_self.upgrade() else {
                    log::error!("cannot wake component '{}': cannot get actor ref", id);
                    return;
                };

                if let Err(e) = ref_self.tell(ComponentWakeMessage).try_send() {
                    log::error!("cannot wake component '{}': cannot send message: {}", id, e);
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
            if let Err(e) = registry.component_remove(id.clone()).await {
                log::error!(
                    "could not remove component '{}' that failed during configure: {}",
                    id,
                    e
                );
            }

            Err(e).with_context(|| format!("failed to configure component '{}'", id))?;
        }

        if let Err(e) = component_impl.init() {
            if let Err(e) = registry.component_remove(id.clone()).await {
                log::error!(
                    "could not remove component '{}' that failed during init: {}",
                    id,
                    e
                );
            }

            Err(e).with_context(|| format!("failed to init component '{}'", id))?;
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
        self.registry.component_remove(self.id.clone()).await?;

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
        if let Err(e) = self
            .component_impl
            .execute_action(msg.name(), msg.value().clone())
        {
            log::error!(
                "failed to execute action '{}' on component '{}' with value '{:?}': {}",
                msg.name(),
                self.id,
                msg.value(),
                e
            );
        }
    }
}

#[derive(Debug)]
struct ComponentWakeMessage;
