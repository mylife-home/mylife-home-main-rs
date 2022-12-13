use std::{collections::HashMap, fmt};

use crate::metadata::{
    ConfigItem, ConfigType, Member, MemberType, PluginMetadata, PluginUsage, Type,
};

#[derive(Debug)]
pub struct PluginRuntimeBuilder {
    name: Option<String>,
    usage: Option<PluginUsage>,
    description: Option<String>,
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl PluginRuntimeBuilder {
    pub fn new() -> Self {
        PluginRuntimeBuilder {
            name: None,
            usage: None,
            description: None,
            members: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn build(self) -> Result<PluginMetadata, PluginRuntimeBuilderError> {
        Ok(PluginMetadata::new(
            self.name.ok_or(PluginRuntimeBuilderError::NameNotSet)?,
            self.usage.ok_or(PluginRuntimeBuilderError::UsageNotSet)?,
            self.description,
            self.members,
            self.config,
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
        // TODO: setter
    ) {
        let config_item = ConfigItem::new(description.map(String::from), value_type);
        self.config.insert(String::from(name), config_item);
    }

    pub fn add_state(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
        // TODO: listener
    ) {
        let member = Member::new(description.map(String::from), MemberType::State, value_type);
        self.members.insert(String::from(name), member);
    }

    pub fn add_action(
        &mut self,
        name: &str,
        description: Option<&str>,
        value_type: Type,
        // TODO: setter
    ) {
        let member = Member::new(description.map(String::from), MemberType::Action, value_type);
        self.members.insert(String::from(name), member);
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

pub type BuilderPartCallback = fn(builder: &mut PluginRuntimeBuilder);