pub enum PluginUsage {
  Sensor,
  Actuator,
  Logic,
  Ui
}

pub struct PluginMetadata {
  // id
  name: String;
  // module
  usage: PluginUsage;
  // version
  description: Option<String>;

  members: HashMap<str, Member>;
  config: HashMap<str, ConfigItem>;
}

impl PluginMetadata {
  pub fn get_name(&self) -> &str {
    &self.name
  }

  pub fn get_description(&self) -> Option<&str> {
    &self.description
  }

  pub fn get_members(&self) -> &HashMap<str, Member> {
    &self.members
  }

  pub fn get_config(&self) -> &HashMap<str, ConfigItem> {
    &self.config
  }
}

pub enum MemberType {
  Action,
  State
}

pub enum Type {
  Range(i64, i64),
  Text,
  Float,
  Bool,
  Enum(&[str]),
  Complex
}

pub struct Member {
  name: String;
  description: Option<String>;
  member_type: MemberType;
  value_type: Type;
}

impl Member {
  pub fn get_name(&self) -> &str {
    &self.name
  }

  pub fn get_description(&self) -> Option<&str> {
    &self.description
  }

  pub fn get_member_type(&self) -> MemberType {
    &self.member_type
  }

  pub fn get_value_type(&self) -> &Type {
    &self.value_type
  }
}

pub enum ConfigType {
  String,
  Bool,
  Integer,
  Float
}

pub struct ConfigItem {
  name: String;
  description: Option<String>;
  valueType: ConfigType;
}

impl ConfigItem {
  pub fn get_name(&self) -> &str {
    &self.name
  }

  pub fn get_description(&self) -> Option<&str> {
    &self.description
  }

  pub fn get_value_type(&self) -> ConfigType {
    &self.value_type
  }
}

pub struct PluginMetadataBuilder {
  metadata: PluginMetadata;
}

impl PluginMetadataBuilder {
  pub fn set_name(&mut self, name: String) {
    &self.metadata.name = name;
  }

  pub fn set_usage(&mut self, usage: PluginUsage) {
    &self.metadata.usage = usage;
  }

  pub fn set_description(&mut self, description: Option<String>) {
    &self.metadata.description = description;
  }

  pub fn add_member(&mut self, name: String, description: Option<String>, member_type: MemberType, value_type: Type) {
    let member = Member {
      name: name,
      description: description,
      member_type: member_type,
      value_type: value_type
    };

    &self.metadata.members.set(member.get_name(), member);
  }

  pub fn add_config(&mut self, name: String, description: Option<String>, value_type: ConfigType) {
    let config_item = ConfigItem {
      name: name,
      description: description,
      value_type: value_type
    };

    &self.metadata.config.set(config_item.get_name(), config_item);
  }

  pub fn build(&mut self) -> PluginMetadata {
    &self.metadata
  }
}

