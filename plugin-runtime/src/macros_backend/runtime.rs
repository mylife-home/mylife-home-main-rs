use std::{cell::RefCell, collections::HashMap, fmt, sync::Arc};

use crate::{
    metadata::PluginMetadata,
    runtime::{Config, ConfigValue, MylifeComponent, MylifePluginRuntime, Value},
    MylifePlugin, StateRuntimeListener,
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

    fn create(&self) -> Box<dyn MylifeComponent> {
        ComponentImpl::<PluginType>::new(&self.access)
    }
}

pub struct StateRuntime<PluginType> {
    pub(crate) register: StateRuntimeRegister<PluginType>,
    pub(crate) getter: StateRuntimeGetter<PluginType>,
}

pub type ConfigRuntimeSetter<PluginType> =
    fn(target: &mut PluginType, config: ConfigValue) -> Result<(), Box<dyn std::error::Error>>;
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: Box<dyn StateRuntimeListener>) -> ();
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
    fail_handler: Option<Box<dyn Fn(/*error:*/ Box<dyn std::error::Error>)>>,
    state_handler: Arc<RefCell<Option<Box<dyn Fn(/*name:*/ &str, /*value:*/ Value)>>>>,
}

struct StateRuntimeListenerImpl {
    name: String,
    state_handler: Arc<RefCell<Option<Box<dyn Fn(/*name:*/ &str, /*value:*/ Value)>>>>,
}

impl StateRuntimeListener for StateRuntimeListenerImpl {
    fn change(&self, value: Value) {
        if let Some(handler) = self.state_handler.borrow().as_ref() {
            handler(&self.name, value);
        }
    }
}

impl<PluginType: MylifePlugin> ComponentImpl<PluginType> {
    pub fn new(access: &Arc<PluginRuntimeAccess<PluginType>>) -> Box<Self> {
        let mut component = Box::new(ComponentImpl {
            access: access.clone(),
            component: PluginType::default(),
            fail_handler: None,
            state_handler: Arc::new(RefCell::new(None)),
        });

        component.register_state_handlers();

        component
    }

    fn register_state_handlers(&mut self) {
        for (name, state) in self.access.states.iter() {
            (state.register)(
                &mut self.component,
                Box::new(StateRuntimeListenerImpl {
                    name: String::from(name),
                    state_handler: self.state_handler.clone(),
                }),
            );
        }
    }

    fn configure_with_res(&mut self, config: &Config) -> Result<(), Box<dyn std::error::Error>> {
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

    fn execute_action_with_res(
        &mut self,
        name: &str,
        action: Value,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let handler = self.access.actions.get(name).ok_or_else(|| {
            Box::new(NoSuchActionError {
                name: String::from(name),
            })
        })?;

        handler(&mut self.component, action)
    }

    fn res_to_fail<T>(&self, result: Result<T, Box<dyn std::error::Error>>) -> Option<T> {
        let fail_handler = self
            .fail_handler
            .as_ref()
            .expect("Cannot report error without registered fail handler");

        match result {
            Ok(value) => Some(value),
            Err(error) => {
                fail_handler(error);
                None
            }
        }
    }
}

impl<PluginType: MylifePlugin> MylifeComponent for ComponentImpl<PluginType> {
    fn set_on_fail(&mut self, handler: Box<dyn Fn(/*error:*/ Box<dyn std::error::Error>)>) {
        self.fail_handler = Some(handler);
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

    fn configure(&mut self, config: &Config) {
        let result = self.configure_with_res(config);
        self.res_to_fail(result);
    }

    fn init(&mut self) {
        let result = self.component.init();
        self.res_to_fail(result);
    }

    fn execute_action(&mut self, name: &str, action: Value) {
        let result = self.execute_action_with_res(name, action);
        self.res_to_fail(result);
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
