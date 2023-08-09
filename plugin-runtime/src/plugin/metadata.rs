use std::collections::HashMap;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum PluginUsage {
    Sensor,
    Actuator,
    Logic,
    Ui,
}

#[derive(Debug)]
pub struct PluginMetadata {
    // id
    name: String,
    // module
    usage: PluginUsage,
    // version
    description: Option<String>,

    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl PluginMetadata {
    pub(crate) fn new(
        name: String,
        usage: PluginUsage,
        description: Option<String>,
        members: HashMap<String, Member>,
        config: HashMap<String, ConfigItem>,
    ) -> PluginMetadata {
        PluginMetadata {
            name,
            usage,
            description,
            members,
            config,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn usage(&self) -> PluginUsage {
        self.usage
    }

    pub fn members(&self) -> &HashMap<String, Member> {
        &self.members
    }

    pub fn config(&self) -> &HashMap<String, ConfigItem> {
        &self.config
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MemberType {
    Action,
    State,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Range(i64, i64),
    Text,
    Float,
    Bool,
    Enum(Vec<String>),
    Complex,
}

#[derive(Debug, Clone)]
pub struct Member {
    description: Option<String>,
    member_type: MemberType,
    value_type: Type,
}

impl Member {
    pub(crate) fn new(
        description: Option<String>,
        member_type: MemberType,
        value_type: Type,
    ) -> Member {
        Member {
            description,
            member_type,
            value_type,
        }
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn member_type(&self) -> &MemberType {
        &self.member_type
    }

    pub fn value_type(&self) -> &Type {
        &self.value_type
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConfigType {
    String,
    Bool,
    Integer,
    Float,
}

#[derive(Debug, Clone)]
pub struct ConfigItem {
    description: Option<String>,
    value_type: ConfigType,
}

impl ConfigItem {
    pub(crate) fn new(description: Option<String>, value_type: ConfigType) -> ConfigItem {
        ConfigItem {
            description,
            value_type,
        }
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn value_type(&self) -> &ConfigType {
        &self.value_type
    }
}
