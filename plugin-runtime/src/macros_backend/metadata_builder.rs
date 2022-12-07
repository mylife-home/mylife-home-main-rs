use std::{collections::HashMap, fmt};

use crate::metadata::{
    ConfigItem, ConfigType, Member, MemberType, PluginMetadata, PluginUsage, Type,
};

#[derive(Debug)]
pub struct PluginMetadataBuilder {
    name: Option<String>,
    usage: Option<PluginUsage>,
    description: Option<String>,
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl PluginMetadataBuilder {
    pub fn new() -> Self {
        PluginMetadataBuilder {
            name: None,
            usage: None,
            description: None,
            members: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn build(self) -> Result<PluginMetadata, PluginMetadataBuilderError> {
        Ok(PluginMetadata::new(
            self.name.ok_or(PluginMetadataBuilderError::NameNotSet)?,
            self.usage.ok_or(PluginMetadataBuilderError::UsageNotSet)?,
            self.description,
            self.members,
            self.config,
        ))
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn set_usage(&mut self, usage: PluginUsage) {
        self.usage = Some(usage);
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    pub fn add_member(
        &mut self,
        name: String,
        description: Option<String>,
        member_type: MemberType,
        value_type: Type,
    ) {
        let member = Member::new(description, member_type, value_type);
        self.members.insert(name, member);
    }

    pub fn add_config(
        &mut self,
        name: String,
        description: Option<String>,
        value_type: ConfigType,
    ) {
        let config_item = ConfigItem::new(description, value_type);
        self.config.insert(name, config_item);
    }
}

#[derive(Debug, Clone)]
pub enum PluginMetadataBuilderError {
    NameNotSet,
    UsageNotSet,
}

impl fmt::Display for PluginMetadataBuilderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PluginMetadataBuilderError::NameNotSet => write!(fmt, "Name not set in metadata"),
            PluginMetadataBuilderError::UsageNotSet => write!(fmt, "Usage not set in metadata"),
        }
    }
}
