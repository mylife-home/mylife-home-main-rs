use log::trace;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt,
    sync::Arc,
};

use crate::{
    metadata::PluginMetadata,
    runtime::{Config, ConfigValue, MylifeComponent, MylifePluginRuntime, Value},
    MylifePlugin,
};

pub struct PluginRuntimeImpl<PluginType: MylifePlugin + 'static> {
    metadata: PluginMetadata,
    access: Arc<PluginRuntimeAccess<PluginType>>,
}

impl<PluginType: MylifePlugin + 'static> PluginRuntimeImpl<PluginType> {
    pub fn new(
        metadata: PluginMetadata,
        access: Arc<PluginRuntimeAccess<PluginType>>,
    ) -> Box<Self> {
        Box::new(PluginRuntimeImpl { metadata, access })
    }
}

impl<PluginType: MylifePlugin> MylifePluginRuntime for PluginRuntimeImpl<PluginType> {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn create(&self, id: &str) -> Box<dyn MylifeComponent> {
        ComponentImpl::<PluginType>::new(&self.access, id)
    }
}

pub struct StateRuntime<PluginType> {
    pub(crate) register: StateRuntimeRegister<PluginType>,
    pub(crate) getter: StateRuntimeGetter<PluginType>,
}

pub type ConfigRuntimeSetter<PluginType> =
    fn(target: &mut PluginType, config: ConfigValue) -> Result<(), Box<dyn std::error::Error>>;
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: Box<dyn Fn(Value)>) -> ();
pub type StateRuntimeGetter<PluginType> = fn(target: &PluginType) -> Value;
pub type ActionRuntimeExecutor<PluginType> =
    fn(target: &mut PluginType, action: Value) -> Result<(), Box<dyn std::error::Error>>;

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

struct ComponentImpl<PluginType: MylifePlugin> {
    access: Arc<PluginRuntimeAccess<PluginType>>,
    component: PluginType,
    id: String,
    state_handler: Arc<RefCell<Option<Box<dyn Fn(/*name:*/ &str, /*value:*/ Value)>>>>,
}

impl<PluginType: MylifePlugin> ComponentImpl<PluginType> {
    pub fn new(access: &Arc<PluginRuntimeAccess<PluginType>>, id: &str) -> Box<Self> {
        let mut component = Box::new(ComponentImpl {
            access: access.clone(),
            component: PluginType::new(id),
            id: String::from(id),
            state_handler: Arc::new(RefCell::new(None)),
        });

        component.register_state_handlers();

        component
    }

    fn register_state_handlers(&mut self) {
        for (name, state) in self.access.states.iter() {
            let id = self.id.clone();
            let name = name.clone();
            let state_handler = self.state_handler.clone();
            (state.register)(
                &mut self.component,
                Box::new(move |value: Value| {
                    trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{id}] state '{name}' changed to {value:?}");

                    if let Some(handler) = state_handler.borrow().as_ref() {
                        handler(&name, value);
                    }
                }),
            );
        }
    }
}

impl<PluginType: MylifePlugin> MylifeComponent for ComponentImpl<PluginType> {
    fn id(&self) -> &str {
        &self.id
    }

    fn set_on_state(&mut self, handler: Box<dyn Fn(/*name:*/ &str, /*value:*/ Value)>) {
        *self.state_handler.borrow_mut() = Some(handler);
    }

    fn get_state(&self, name: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let state = self.access.states.get(name).ok_or_else(|| {
            Box::new(NoSuchStateError {
                name: String::from(name),
            })
        })?;

        Ok((state.getter)(&self.component))
    }

    // TODO: better error type
    fn configure(&mut self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{}] configure with {config:?}", self.id);

        for (name, setter) in self.access.configs.iter() {
            let value = config
                .get(name)
                .ok_or_else(|| {
                    Box::new(ConfigNotSetError {
                        name: String::from(name),
                    })
                })?
                .clone();

            setter(&mut self.component, value)?;
        }

        Ok(())
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.component.init()
    }

    // TODO: better error type
    fn execute_action(&mut self, name: &str, action: Value) -> Result<(), Box<dyn std::error::Error>> {
        let handler = self.access.actions.get(name).ok_or_else(|| {
            Box::new(NoSuchActionError {
                name: String::from(name),
            })
        })?;

        trace!(target: "mylife:home:core:plugin-runtime:macros-backend:runtime", "[{}] execute action '{name}' with {action:?}", self.id);
        handler(&mut self.component, action)
    }
}

#[derive(Debug, Clone)]
pub struct ConfigNotSetError {
    name: String,
}

impl std::error::Error for ConfigNotSetError {}

impl fmt::Display for ConfigNotSetError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Config key not set: '{}'", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct NoSuchActionError {
    name: String,
}

impl std::error::Error for NoSuchActionError {}

impl fmt::Display for NoSuchActionError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "No such action: '{}'", self.name)
    }
}

#[derive(Debug, Clone)]
pub struct NoSuchStateError {
    name: String,
}

impl std::error::Error for NoSuchStateError {}

impl fmt::Display for NoSuchStateError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "No such state: '{}'", self.name)
    }
}
