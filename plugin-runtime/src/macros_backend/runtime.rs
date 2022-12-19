use std::{collections::HashMap, sync::Arc};

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

    fn create(&self) -> Box<dyn MylifeComponent> {
        ComponentImpl::<PluginType>::new(&self.access)
    }
}

pub type ConfigRuntimeSetter<PluginType> =
    fn(target: &mut PluginType, config: ConfigValue) -> Result<(), Box<dyn std::error::Error>>;
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: Box<dyn Fn(/*state:*/ Value)>) -> ();
pub type ActionRuntimeExecutor<PluginType> =
    fn(target: &mut PluginType, action: Value) -> Result<(), Box<dyn std::error::Error>>;

pub struct PluginRuntimeAccess<PluginType: MylifePlugin> {
    configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
    states: HashMap<String, StateRuntimeRegister<PluginType>>,
    actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
}

impl<PluginType: MylifePlugin> PluginRuntimeAccess<PluginType> {
    pub fn new(
        configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
        states: HashMap<String, StateRuntimeRegister<PluginType>>,
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
    fail_handler: Option<fn(error: Box<dyn std::error::Error>)>,
    state_handler: Option<fn(name: &str, state: Value)>,
}

impl<PluginType: MylifePlugin> ComponentImpl<PluginType> {
    pub fn new(access: &Arc<PluginRuntimeAccess<PluginType>>) -> Box<Self> {
        let mut component = Box::new(ComponentImpl {
            access: access.clone(),
            component: PluginType::default(),
            fail_handler: None,
            state_handler: None,
        });

        for (name, register) in access.states.iter() {
            register(&mut component.component, Box::new(|value: Value| {
                if let Some(handler) = &component.state_handler {
                    handler(name, value);
                }
            }));
        }

        component
    }
}

impl<PluginType: MylifePlugin> MylifeComponent for ComponentImpl<PluginType> {
    fn set_on_fail(&mut self, handler: Box<dyn Fn(/*error:*/ Box<dyn std::error::Error>)>) {}

    fn set_on_state(&mut self, handler: Box<dyn Fn(/*name:*/ &str, /*state:*/ Value)>) {}

    fn configure(&mut self, config: &Config) {}

    fn execute_action(&mut self, name: &str, action: &Value) {}
}
