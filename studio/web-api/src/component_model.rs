use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub struct Component {
    pub id: String,
    pub plugin: String,
}

register_ts!(Component);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub struct Member {
    pub description: String,
    pub member_type: MemberType,
    pub value_type: String,
}

register_ts!(Member);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub struct Plugin {
    pub name: String,
    pub module: String,
    pub usage: PluginUsage,
    pub version: String,
    pub description: String,
    pub members: std::collections::HashMap<String, Member>,
    pub config: std::collections::HashMap<String, ConfigItem>,
}

register_ts!(Plugin);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub enum MemberType {
    Action,
    State,
}

register_ts!(MemberType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub enum PluginUsage {
    Sensor,
    Actuator,
    Logic,
    Ui,
}

register_ts!(PluginUsage);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub struct ConfigItem {
    pub description: String,
    pub value_type: ConfigType,
}

register_ts!(ConfigItem);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "component-model.ts")]
pub enum ConfigType {
    String,
    Bool,
    Integer,
    Float,
}

register_ts!(ConfigType);
