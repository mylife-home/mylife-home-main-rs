use std::{any::Any, sync::Arc};

use crate::modules;
use anyhow::Context;
use async_trait::async_trait;
use common::{
    components::{
        Component, ComponentChange, ComponentChangeEventType, ComponentsData, ComponentsHandler,
        ComponentsMessage,
    },
    utils::{
        mailbox::{MailboxHandle, ReplySender},
        observable::{Observable, Observer, ObserverId, Subject},
    },
};
use log::{error, warn};
use plugin_runtime::{
    metadata::PluginMetadata,
    runtime::{Config, MylifeComponent, MylifePluginRuntime, Value},
};

#[derive(Debug)]
struct ComponentWakeMessage {
    component_id: String,
}

impl ComponentsMessage for ComponentWakeMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct ComponentCreateMessage {
    component_id: String,
    plugin_id: String,
    config: Config,
    reply: ReplySender<anyhow::Result<()>>,
}

impl ComponentsMessage for ComponentCreateMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Debug)]
struct ComponentRemoveMessage {
    component_id: String,
    reply: ReplySender<anyhow::Result<()>>,
}

impl ComponentsMessage for ComponentRemoveMessage {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
pub trait LocalComponentsMailboxHandleExt {
    async fn component_create(
        &self,
        component_id: String,
        plugin_id: String,
        config: Config,
    ) -> anyhow::Result<()>;
    async fn component_remove(&self, component_id: String) -> anyhow::Result<()>;
}

#[async_trait]
impl LocalComponentsMailboxHandleExt for MailboxHandle<Box<dyn ComponentsMessage>> {
    async fn component_create(
        &self,
        component_id: String,
        plugin_id: String,
        config: Config,
    ) -> anyhow::Result<()> {
        let (reply, receiver) = ReplySender::create_channel();

        self.send(Box::new(ComponentCreateMessage {
            component_id,
            plugin_id,
            config,
            reply,
        }));

        receiver.await?
    }

    async fn component_remove(&self, component_id: String) -> anyhow::Result<()> {
        let (reply, receiver) = ReplySender::create_channel();

        self.send(Box::new(ComponentRemoveMessage {
            component_id,
            reply,
        }));

        receiver.await?
    }
}

pub struct LocalComponents {
    mailbox_sender: Option<MailboxHandle<Box<dyn ComponentsMessage>>>,
}

impl LocalComponents {
    pub fn new() -> Self {
        Self {
            mailbox_sender: None,
        }
    }

    fn create_component(
        &self,
        data: &mut ComponentsData,
        id: &str,
        plugin: &str,
        config: &Config,
    ) -> anyhow::Result<()> {
        let registry = data.registry_mut();

        if registry.get_component(id).is_some() {
            anyhow::bail!("component '{}' already exists", id);
        }

        let plugin = modules::registry()
            .plugin(plugin)
            .unwrap_or_else(|| panic!("could not find plugin {}", plugin));

        let component = LocalComponent::new(
            self.mailbox_sender
                .as_ref()
                .expect("no mailbox sender")
                .clone(),
            id,
            plugin,
            config,
        )?;

        registry.add_component(None, Box::new(component));

        Ok(())
    }

    fn remove_component(&self, data: &mut ComponentsData, id: &str) -> anyhow::Result<()> {
        let registry = data.registry_mut();

        if let Some((instance_name, component)) = registry.get_component_data(id) {
            if instance_name.is_some()
                || component
                    .as_any()
                    .downcast_ref::<LocalComponent>()
                    .is_none()
            {
                anyhow::bail!("component '{}' is not local", id);
            }
        } else {
            anyhow::bail!("component '{}' does not exist", id);
        }

        registry.remove_component(None, id);

        Ok(())
    }

    fn wake_component(&self, data: &mut ComponentsData, id: &str) {
        let Some(component) = data.registry_mut().get_component_mut(id) else {
            warn!("Got wake for non existant component '{}', ignored", id);
            return;
        };

        let component = component
            .as_any_mut()
            .downcast_mut::<LocalComponent>()
            .unwrap_or_else(|| panic!("component '{}' is not local", id));

        component.async_handler();
    }
}

impl ComponentsHandler for LocalComponents {
    fn init(
        &mut self,
        _data: &mut ComponentsData,
        mailbox_sender: &MailboxHandle<Box<dyn ComponentsMessage>>,
    ) {
        self.mailbox_sender = Some(mailbox_sender.clone());
    }

    fn handle(&mut self, data: &mut ComponentsData, message: &dyn ComponentsMessage) {
        if let Some(message) = message.as_any().downcast_ref::<ComponentWakeMessage>() {
            self.wake_component(data, &message.component_id);
            return;
        }

        if let Some(message) = message.as_any().downcast_ref::<ComponentCreateMessage>() {
            message.reply.send(self.create_component(
                data,
                &message.component_id,
                &message.plugin_id,
                &message.config,
            ));
            return;
        }

        if let Some(message) = message.as_any().downcast_ref::<ComponentRemoveMessage>() {
            message
                .reply
                .send(self.remove_component(data, &message.component_id));
            return;
        }
    }
}

struct LocalComponent {
    subject: Arc<Subject<ComponentChangeEventType>>,
    component_impl: Box<dyn MylifeComponent>,
}

impl LocalComponent {
    pub fn new(
        mailbox_sender: MailboxHandle<Box<dyn ComponentsMessage + 'static>>,
        id: &str,
        plugin: &dyn MylifePluginRuntime,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let waker = {
            let id = id.to_owned();

            move || {
                mailbox_sender.send(Box::new(ComponentWakeMessage {
                    component_id: id.clone(),
                }));
            }
        };

        let subject = Arc::new(Subject::new());

        let state_change = {
            let subject = subject.clone();

            move |name: &str, value: &Value| {
                subject.notify(&ComponentChange::State { name, value });
            }
        };

        let mut component_impl = plugin.create(id, Box::new(waker), Box::new(state_change));

        component_impl
            .configure(config)
            .with_context(|| format!("configuration failed for component '{}'", id))?;

        component_impl
            .init()
            .with_context(|| format!("init failed for component '{}'", id))?;

        Ok(Self {
            subject,
            component_impl,
        })
    }

    pub fn async_handler(&mut self) {
        self.component_impl.async_handler();
    }
}

impl Component for LocalComponent {
    fn id(&self) -> &str {
        self.component_impl.id()
    }

    fn plugin(&self) -> Arc<PluginMetadata> {
        self.component_impl.plugin().clone()
    }

    fn get_state(&self, name: &str) -> Value {
        self.component_impl.get_state(name)
    }

    fn execute_action(&mut self, name: &str, value: Value) {
        if let Err(e) = self.component_impl.execute_action(name, value) {
            error!(
                "Could not execute action '{}' on component '{}': {}",
                name,
                self.id(),
                e
            );
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Observable<ComponentChangeEventType> for LocalComponent {
    fn observe(&self, observer: Box<Observer<ComponentChangeEventType>>) -> ObserverId {
        self.subject.observe(observer)
    }

    fn unobserve(&self, id: ObserverId) -> bool {
        self.subject.unobserve(id)
    }
}
