use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use crate::{
    metadata::{ConfigType, PluginMetadata, Type},
    runtime::{Config, MyLifePluginRuntime, MylifeComponent, Value},
};

pub struct MyLifePluginRuntimeImpl<Component: MylifeComponent + Default + 'static> {
    metadata: PluginMetadata,
    _marker: PhantomData<Component>, // only to keep Component type info
}

impl<Component: MylifeComponent + Default> MyLifePluginRuntimeImpl<Component> {
    pub fn new(metadata: PluginMetadata) -> Box<Self> {
        Box::new(MyLifePluginRuntimeImpl {
            metadata,
            _marker: PhantomData,
        })
    }
}

impl<Component: MylifeComponent + Default> MyLifePluginRuntime
    for MyLifePluginRuntimeImpl<Component>
{
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn create(&self) -> Box<dyn MylifeComponent> {
        Box::new(Component::default())
    }
}

pub type ConfigRuntimeSetter<PluginType> = fn(target: &mut PluginType, config: &Value) -> ();
pub type StateRuntimeRegister<PluginType> =
    fn(target: &mut PluginType, listener: fn(state: &Value)) -> ();
pub type ActionRuntimeExecutor<PluginType> =
    fn(target: &mut PluginType, action: &Value) -> Result<(), Box<dyn std::error::Error>>;

pub struct PluginRuntimeAccess<PluginType> {
    configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
    states: HashMap<String, StateRuntimeRegister<PluginType>>,
    actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
}

impl<PluginType> PluginRuntimeAccess<PluginType> {
    pub fn new(
        configs: HashMap<String, ConfigRuntimeSetter<PluginType>>,
        states: HashMap<String, StateRuntimeRegister<PluginType>>,
        actions: HashMap<String, ActionRuntimeExecutor<PluginType>>,
    ) -> Self {
        PluginRuntimeAccess {
            configs,
            states,
            actions,
        }
    }
}

impl<PluginType> PluginRuntimeAccess<PluginType> {
    pub fn add_config(&mut self, name: &str, setter: ConfigRuntimeSetter<PluginType>) {
        self.configs.insert(String::from(name), setter);
    }

    pub fn add_state(&mut self, name: &str, register: StateRuntimeRegister<PluginType>) {
        self.states.insert(String::from(name), register);
    }

    pub fn add_action(&mut self, name: &str, executor: ActionRuntimeExecutor<PluginType>) {
        self.actions.insert(String::from(name), executor);
    }
}

struct ComponentImpl<PluginType> {
    access: Arc<PluginRuntimeAccess<PluginType>>,
    component: PluginType,
}

impl<PluginType> MylifeComponent for ComponentImpl<PluginType> {
    fn set_on_fail(&mut self, handler: fn(error: Box<dyn std::error::Error>)) {}

    fn set_on_state(&mut self, handler: fn(name: &str, state: &Value)) {}

    fn configure(&mut self, config: &Config) {}

    fn execute_action(&mut self, name: &str, action: &Value) {}
}

/*
//-----

trait ActionHandler {
    fn execute_action(&self, action: &Value) -> Result<(), Box<dyn std::error::Error>>;
}

struct ActionHandlerWithoutResult<PluginT, ArgType> {
    owner: Arc<PluginT>,
    target: fn(_self: &mut PluginT, arg: ArgType) -> (),
}

impl<PluginT, ArgType> ActionHandlerWithoutResult<PluginT, ArgType> {
    pub fn new(owner: Arc<PluginT>, target: fn(_self: &mut PluginT, arg: ArgType) -> ()) -> Self {
        ActionHandlerWithoutResult { owner, target }
    }
}

impl<PluginT, ArgType> ActionHandler for ActionHandlerWithoutResult<PluginT, ArgType> {
    fn execute_action(&self, action: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let arg: ArgType = action.try_into()?;
        (self.target)(&mut self.owner, arg);

        Ok(())
    }
}

struct ActionHandlerWithResult<PluginT, ArgType> {
    owner: Arc<PluginT>,
    target: fn(_self: &mut PluginT, arg: ArgType) -> Result<(), Box<dyn std::error::Error>>,
}

impl<PluginT, ArgType> ActionHandlerWithResult<PluginT, ArgType> {
    pub fn new(owner: Arc<PluginT>, target: fn(_self: &mut PluginT, arg: ArgType) -> Result<(), Box<dyn std::error::Error>>) -> Self {
        ActionHandlerWithResult { owner, target }
    }
}

impl<PluginT, ArgType> ActionHandler for ActionHandlerWithResult<PluginT, ArgType> {
    fn execute_action(&self, action: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let arg: ArgType = action.try_into()?;
        (self.target)(&mut self.owner, arg)?;

        Ok(())
    }
}

//-----

impl Definition {
    pub const fn new_config(
        name: &'static str,
        description: Option<&'static str>,
        r#type: ConfigType,
        setter: fn(arg: &Value),
    ) -> Self {
        Definition::Config(ConfigDefinition {
            name,
            description,
            r#type,
            setter,
        })
    }

    pub const fn new_state(
        name: &'static str,
        description: Option<&'static str>,
        r#type: Type,
    ) -> Self {
        Definition::State(StateDefinition {
            name,
            description,
            r#type,
        })
    }

    pub const fn new_action(
        name: &'static str,
        description: Option<&'static str>,
        r#type: Type,
    ) -> Self {
        Definition::Action(ActionDefinition {
            name,
            description,
            r#type,
        })
    }
}

pub enum Definition {
    Config(ConfigDefinition),
    State(StateDefinition),
    Action(ActionDefinition),
}

pub struct ConfigDefinition {
    name: &'static str,
    description: Option<&'static str>,
    r#type: ConfigType,
    setter: fn(arg: &Value),
}

pub struct StateDefinition {
    name: &'static str,
    description: Option<&'static str>,
    r#type: Type,
}

pub struct ActionDefinition {
    name: &'static str,
    description: Option<&'static str>,
    r#type: Type,
}
*/
