use std::collections::HashMap;

#[derive(Debug, Copy, Clone)]
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
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn get_usage(&self) -> PluginUsage {
        self.usage
    }

    pub fn get_members(&self) -> &HashMap<String, Member> {
        &self.members
    }

    pub fn get_config(&self) -> &HashMap<String, ConfigItem> {
        &self.config
    }
}

#[derive(Debug, Copy, Clone)]
pub enum MemberType {
    Action,
    State,
}

#[derive(Debug, Clone)]
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
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn get_member_type(&self) -> &MemberType {
        &self.member_type
    }

    pub fn get_value_type(&self) -> &Type {
        &self.value_type
    }
}

#[derive(Debug, Copy, Clone)]
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
    pub fn get_description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn get_value_type(&self) -> &ConfigType {
        &self.value_type
    }
}
