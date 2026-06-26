use serde::{Deserialize, Serialize};

use super::Type;
use std::collections::HashMap;

/// PluginUsage represents the usage of a plugin, which can be Sensor, Actuator, Logic or Ui.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginUsage {
    /// Sensor represents a plugin that provides data, such as a temperature sensor or a motion sensor.
    Sensor,

    /// Actuator represents a plugin that can perform actions, such as a light switch or a thermostat.
    Actuator,

    /// Logic represents a plugin that can process data and make decisions, such as a rule engine or a scheduler.
    Logic,

    /// Ui represents a plugin that provides data and actions to the user interface.
    Ui,
}

/// PluginMetadata contains all the information about a plugin, including its members and config items.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "PluginMetadataShadow")]
pub struct PluginMetadata {
    #[serde(skip_serializing)]
    id: String,
    name: String,
    module: String,
    usage: PluginUsage,
    version: String,
    description: Option<String>,

    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl PluginMetadata {
    /// Creates a new PluginMetadata instance.
    pub fn new(
        name: String,
        module: String,
        usage: PluginUsage,
        version: String,
        description: Option<String>,
        members: HashMap<String, Member>,
        config: HashMap<String, ConfigItem>,
    ) -> PluginMetadata {
        PluginMetadata {
            id: format!("{}.{}", module, name),
            name,
            module,
            usage,
            version,
            description,
            members,
            config,
        }
    }

    /// Returns the unique identifier of the plugin, which is a combination of its module and name.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the name of the plugin.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the module of the plugin, which is used to group plugins together.
    pub fn module(&self) -> &str {
        &self.module
    }

    /// Returns the usage of the plugin, which indicates its purpose and how it should be used.
    pub fn usage(&self) -> PluginUsage {
        self.usage
    }

    /// Returns the version of the plugin, which can be used for compatibility checks and updates.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the description of the plugin, which provides additional information about its functionality and features.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the members of the plugin, which are the actions and states that the plugin provides.
    pub fn members(&self) -> &HashMap<String, Member> {
        &self.members
    }

    /// Returns the config items of the plugin, which are the configuration options that the plugin requires.
    pub fn config(&self) -> &HashMap<String, ConfigItem> {
        &self.config
    }
}

/// Mirror of PluginMetadata without the computed id, used only for deserialization.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginMetadataShadow {
    name: String,
    module: String,
    usage: PluginUsage,
    version: String,
    description: Option<String>,
    members: HashMap<String, Member>,
    config: HashMap<String, ConfigItem>,
}

impl From<PluginMetadataShadow> for PluginMetadata {
    fn from(s: PluginMetadataShadow) -> Self {
        PluginMetadata::new(
            s.name,
            s.module,
            s.usage,
            s.version,
            s.description,
            s.members,
            s.config,
        )
    }
}

/// MemberType represents the type of a plugin member, which can be an Action or a State.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MemberType {
    /// Action represents a plugin member that can be invoked to perform an action, such as turning on a light or setting a temperature.
    Action,

    /// State represents a plugin member that represents a state, such as the temperature of a room or the status of a device.
    State,
}

/// Member represents a member of a plugin, which can be an action or a state, and has a type and a value type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Member {
    description: Option<String>,
    member_type: MemberType,
    value_type: Type,
}

impl Member {
    /// Creates a new Member instance.
    pub fn new(description: Option<String>, member_type: MemberType, value_type: Type) -> Member {
        Member {
            description,
            member_type,
            value_type,
        }
    }

    /// Returns the description of the member, which provides additional information about its functionality and features.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the type of the member, which indicates whether it is an action or a state.
    pub fn member_type(&self) -> MemberType {
        self.member_type
    }

    /// Returns the value type of the member, which indicates the type of data that the member can accept or produce.
    pub fn value_type(&self) -> &Type {
        &self.value_type
    }
}

/// ConfigType represents the type of a configuration item, which can be String, Bool, Integer or Float.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigType {
    /// String represents a configuration item that accepts a string value, such as a device name or an API key.
    String,

    /// Bool represents a configuration item that accepts a boolean value, such as whether to enable a feature or not.
    Bool,

    /// Integer represents a configuration item that accepts an integer value, such as a polling interval or a threshold.
    Integer,

    /// Float represents a configuration item that accepts a floating-point value, such as a temperature setpoint or a brightness level.
    Float,
}

/// ConfigItem represents a configuration item of a plugin, which has a description and a value type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigItem {
    description: Option<String>,
    value_type: ConfigType,
}

impl ConfigItem {
    /// Creates a new ConfigItem instance.
    pub fn new(description: Option<String>, value_type: ConfigType) -> ConfigItem {
        ConfigItem {
            description,
            value_type,
        }
    }

    /// Returns the description of the configuration item, which provides additional information about its purpose and usage.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the value type of the configuration item, which indicates the type of data that the configuration item can accept.
    pub fn value_type(&self) -> ConfigType {
        self.value_type
    }
}
