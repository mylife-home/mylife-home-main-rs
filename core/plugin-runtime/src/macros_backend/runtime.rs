use anyhow::Context;
use log::trace;
use std::{collections::HashMap, fmt, sync::{Arc, Mutex}};

use crate::{
    MylifePlugin,
    runtime::{MylifeComponent, MylifePluginRuntime},
};
use common::components::{
    Component, ComponentChange, ComponentChangeEventType, metadata::PluginMetadata, observable::{Observable, Observer, ObserverId, Subject}, types::{Config, ConfigValue, Value}
};

#[derive(Debug)]
pub struct PluginRuntimeImpl<PluginType: MylifePlugin + 'static> {
    metadata: Arc<PluginMetadata>,
    access: Arc<PluginRuntimeAccess<PluginType>>,
}

impl<PluginType: MylifePlugin + 'static> PluginRuntimeImpl<PluginType> {
    pub fn new(
        metadata: PluginMetadata,
        access: Arc<PluginRuntimeAccess<PluginType>>,
    ) -> Box<Self> {
        Box::new(PluginRuntimeImpl {
            metadata: Arc::new(metadata),
            access,
        })
    }
}

impl<PluginType: MylifePlugin> MylifePluginRuntime for PluginRuntimeImpl<PluginType> {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn create(&self, id: &str) -> Box<dyn MylifeComponent> {
        ComponentImpl::<PluginType>::new(&self.access, id, &self.metadata)
    }
}

#[derive(Debug)]
pub struct StateRuntime<PluginType> {
    pub(crate) register: StateRuntimeRegister<PluginType>,
    pub(crate) getter: StateRuntimeGetter<PluginType>,
}

pub type ConfigRuntimeSetter<PluginType> =
    fn(target: &mut PluginType, config: ConfigValue) -> anyhow::Result<()>;
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: Box<dyn Fn(Value) + Send + Sync>) -> ();
pub type StateRuntimeGetter<PluginType> = fn(target: &PluginType) -> Value;
pub type ActionRuntimeExecutor<PluginType> =
    fn(target: &mut PluginType, action: Value) -> anyhow::Result<()>;

#[derive(Debug)]
pub struct PluginRuntimeAccess<PluginType: MylifePlugin> {
    configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
    states: HashMap<String, StateRuntime<PluginType>>,
    actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
}

impl<PluginType: MylifePlugin> PluginRuntimeAccess<PluginType> {
    pub fn new(
        configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
        states: HashMap<String, StateRuntime<PluginType>>,
        actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
    ) -> Arc<Self> {
        Arc::new(PluginRuntimeAccess {
            configs,
            states,
            actions,
        })
    }
}

// FIXME: remove Arc<Mutex<>>

struct ComponentImpl<PluginType: MylifePlugin> {
    access: Arc<PluginRuntimeAccess<PluginType>>,
    component: PluginType,
    id: String,
    plugin_metadata: Arc<PluginMetadata>,
    subject: Arc<Mutex<Subject<ComponentChangeEventType>>>,
}

impl<PluginType: MylifePlugin> fmt::Debug for ComponentImpl<PluginType> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("ComponentImpl")
            .field("access", &self.access)
            .field("component", &self.component)
            .field("id", &self.id)
            .finish()
    }
}

impl<PluginType: MylifePlugin> ComponentImpl<PluginType> {
    pub fn new(
        access: &Arc<PluginRuntimeAccess<PluginType>>,
        id: &str,
        plugin_metadata: &Arc<PluginMetadata>,
    ) -> Box<Self> {
        let mut component = Box::new(ComponentImpl {
            access: access.clone(),
            component: PluginType::new(id),
            id: String::from(id),
            plugin_metadata: plugin_metadata.clone(),
            subject: Arc::new(Mutex::new(Subject::new())),
        });

        component.register_state_handlers();

        component
    }

    fn register_state_handlers(&mut self) {
        for (name, state) in self.access.states.iter() {
            let id = self.id.clone();
            let name = name.clone();
            let subject = self.subject.clone();
            (state.register)(
                &mut self.component,
                Box::new(move |value: Value| {
                    trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{id}] state '{name}' changed to {value:?}");
                    subject.lock().expect("cannot lock mutex").notify(&ComponentChange::State {
                        name: &name,
                        value: &value,
                    });
                }),
            );
        }
    }
}

impl<PluginType: MylifePlugin> Component for ComponentImpl<PluginType> {
    fn id(&self) -> &str {
        &self.id
    }

    fn plugin(&self) -> Arc<PluginMetadata> {
        self.plugin_metadata.clone()
    }

    fn get_state(&self, name: &str) -> Option<Value> {
        let state = self.access.states.get(name)?;
        Some((state.getter)(&self.component))
    }

    fn execute_action(&mut self, name: &str, action: Value) -> anyhow::Result<()> {
        let handler = self
            .access
            .actions
            .get(name)
            .with_context(|| format!("Action not found: {}", name))?;

        trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{}] execute action '{name}' with {action:?}", self.id);
        handler(&mut self.component, action)
    }
}

impl<PluginType: MylifePlugin> Observable<ComponentChangeEventType> for ComponentImpl<PluginType> {
    fn observe(&mut self, observer: Box<Observer<ComponentChangeEventType>>) -> ObserverId {
        self.subject.lock().expect("cannot lock mutex").observe(observer)
    }

    fn unobserve(&mut self, id: ObserverId) -> bool {
        self.subject.lock().expect("cannot lock mutex").unobserve(id)
    }
}

impl<PluginType: MylifePlugin> MylifeComponent for ComponentImpl<PluginType> {
    fn configure(&mut self, config: &Config) -> anyhow::Result<()> {
        trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{}] configure with {config:?}", self.id);

        for (name, setter) in self.access.configs.iter() {
            let value = config
                .get(name)
                .context(format!("Config '{name}' not found"))?
                .clone();

            setter(&mut self.component, value)?;
        }

        Ok(())
    }

    fn init(&mut self) -> anyhow::Result<()> {
        self.component.init()
    }

    fn async_handler(&mut self) {
        self.component.async_handler();
    }
}
