use std::{collections::HashMap, fmt};

use crate::{
    metadata::{ConfigItem, ConfigType, Member, MemberType, PluginMetadata, PluginUsage, Type},
    runtime::MylifePluginRuntime,
    MylifePlugin,
};

use super::{
    ActionRuntimeExecutor, ConfigRuntimeSetter, PluginRuntimeAccess, PluginRuntimeImpl,
    StateRuntimeRegister,
};

pub struct PluginRuntimeBuilder<PluginType: MylifePlugin + 'static> {
    name: Option<String>,
    usage: Option<PluginUsage>,
    description: Option<String>,
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
    config_runtime: HashMap<String, ConfigRuntimeSetter<PluginType>>,
    state_runtime: HashMap<String, StateRuntimeRegister<PluginType>>,
    action_runtime: HashMap<String, ActionRuntimeExecutor<PluginType>>,
}

impl<PluginType: MylifePlugin + 'static> PluginRuntimeBuilder<PluginType> {
    pub fn new() -> Self {
        PluginRuntimeBuilder {
            name: None,
            usage: None,
            description: None,
            members: HashMap::new(),
            config: HashMap::new(),
            config_runtime: HashMap::new(),
            state_runtime: HashMap::new(),
            action_runtime: HashMap::new(),
        }
    }

    pub fn build(self) -> Result<Box<dyn MylifePluginRuntime>, PluginRuntimeBuilderError> {
        Ok(PluginRuntimeImpl::<PluginType>::new(
            PluginMetadata::new(
                self.name.ok_or(PluginRuntimeBuilderError::NameNotSet)?,
                self.usage.ok_or(PluginRuntimeBuilderError::UsageNotSet)?,
                self.description,
                self.members,
                self.config,
            ),
            PluginRuntimeAccess::new(self.config_runtime, self.state_runtime, self.action_runtime),
        ))
    }

    pub fn set_plugin(&mut self, name: &str, description: Option<&str>, usage: PluginUsage) {
        self.name = Some(String::from(name));
        self.description = description.map(String::from);
        self.usage = Some(usage);
    }

    pub fn add_config(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: ConfigType,
        setter: ConfigRuntimeSetter<PluginType>,
    ) {
        let config_item = ConfigItem::new(description.map(String::from), value_type);
        self.config.insert(String::from(name), config_item);
        self.config_runtime.insert(String::from(name), setter);
    }

    pub fn add_state(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
        register: StateRuntimeRegister<PluginType>,
    ) {
        let member = Member::new(description.map(String::from), MemberType::State, value_type);
        self.members.insert(String::from(name), member);
        self.state_runtime.insert(String::from(name), register);
    }

    pub fn add_action(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
        executor: ActionRuntimeExecutor<PluginType>,
    ) {
        let member = Member::new(
            description.map(String::from),
            MemberType::Action,
            value_type,
        );
        self.members.insert(String::from(name), member);
        self.action_runtime.insert(String::from(name), executor);
    }
}

#[derive(Debug, Clone)]
pub enum PluginRuntimeBuilderError {
    NameNotSet,
    UsageNotSet,
}

impl fmt::Display for PluginRuntimeBuilderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PluginRuntimeBuilderError::NameNotSet => write!(fmt, "Name not set in metadata"),
            PluginRuntimeBuilderError::UsageNotSet => write!(fmt, "Usage not set in metadata"),
        }
    }
}
impl std::error::Error for PluginRuntimeBuilderError {
    
}

pub type BuilderPartCallback<PluginType> = fn(builder: &mut PluginRuntimeBuilder<PluginType>);
