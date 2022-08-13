use std::collections::HashMap;

#[derive(Debug)]
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
  pub fn from(builder: PluginMetadataBuilder) -> PluginMetadata {
    builder.metadata
  }

  pub fn get_name(&self) -> &str {
    &self.name
  }

  pub fn get_description(&self) -> Option<&str> {
    self.description.as_deref()
  }

  pub fn get_members(&self) -> &HashMap<String, Member> {
    &self.members
  }

  pub fn get_config(&self) -> &HashMap<String, ConfigItem> {
    &self.config
  }
}

#[derive(Debug)]
pub enum MemberType {
  Action,
  State,
}

#[derive(Debug)]
pub enum Type {
  Range(i64, i64),
  Text,
  Float,
  Bool,
  Enum(Vec<String>),
  Complex,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum ConfigType {
  String,
  Bool,
  Integer,
  Float,
}

#[derive(Debug)]
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

pub struct PluginMetadataBuilder {
  metadata: PluginMetadata,
}

impl PluginMetadataBuilder {
  pub fn set_name(&mut self, name: String) {
    self.metadata.name = name;
  }

  pub fn set_usage(&mut self, usage: PluginUsage) {
    self.metadata.usage = usage;
  }

  pub fn set_description(&mut self, description: Option<String>) {
    self.metadata.description = description;
  }

  pub fn add_member(&mut self, name: String, description: Option<String>, member_type: MemberType, value_type: Type) {
    let member = Member {
      description: description,
      member_type: member_type,
      value_type: value_type
    };

    self.metadata.members.insert(name, member);
  }

  pub fn add_config(&mut self, name: String, description: Option<String>, value_type: ConfigType) {
    let config_item = ConfigItem {
      description: description,
      value_type: value_type
    };

    self.metadata.config.insert(name, config_item);
  }
}

