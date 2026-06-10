use std::{any::Any, sync::Arc};

use crate::modules;
use anyhow::Context;
use common::{
    components::{
        Component, ComponentChange, ComponentChangeEventType, ComponentsData, ComponentsHandler,
        ComponentsMessage,
    },
    utils::{
        mailbox::MailboxHandle,
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
        id: &str,
        plugin: &PluginMetadata,
        config: &Config,
    ) -> anyhow::Result<Box<dyn Component>> {
        let plugin = modules::registry()
            .plugin(plugin.id())
            .unwrap_or_else(|| panic!("could not find plugin {}", plugin.id()));

        let component = LocalComponent::new(
            self.mailbox_sender
                .as_ref()
                .expect("no mailbox sender")
                .clone(),
            id,
            plugin,
            config,
        )?;

        Ok(Box::new(component))
    }
}

impl ComponentsHandler for LocalComponents {
    fn init(
        &mut self,
        data: &mut ComponentsData,
        mailbox_sender: &MailboxHandle<Box<dyn ComponentsMessage>>,
    ) {
        self.mailbox_sender = Some(mailbox_sender.clone());
        let registry = data.registry_mut();

        // TODO: load components
        let component = self
            .create_component(
                "comp-id",
                modules::registry()
                    .plugin("logic-base.value-binary")
                    .unwrap()
                    .metadata(),
                &Config::new(),
            )
            .expect("failed to create component");

        registry.add_component(None, component);
    }

    fn handle(&mut self, data: &mut ComponentsData, message: &dyn ComponentsMessage) {
        let Some(message) = message.as_any().downcast_ref::<ComponentWakeMessage>() else {
            return;
        };

        let Some(component) = data.registry_mut().get_component_mut(&message.component_id) else {
            warn!(
                "Got wake for non existant component '{}', ignored",
                message.component_id
            );
            return;
        };

        let Some(component) = component.as_any_mut().downcast_mut::<LocalComponent>() else {
            panic!("component '{}' is not local", component.id());
        };

        component.async_handler();
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
