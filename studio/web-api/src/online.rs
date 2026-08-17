use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct InstanceInfo {
    pub r#type: String,
    pub hardware: std::collections::HashMap<String, String>,
    pub versions: std::collections::HashMap<String, String>,
    pub system_uptime: i64,
    pub instance_uptime: i64,
    pub hostname: String,
    pub capabilities: Vec<String>,
    pub wifi: Option<WifiInfo>,
}

register_ts!(InstanceInfo);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct WifiInfo {
    pub rssi: i32,
}

register_ts!(WifiInfo);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct UpdateInstanceInfoData {
    pub operation: String,
    pub instance_name: String,
    pub data: Option<InstanceInfo>,
}

register_ts!(UpdateInstanceInfoData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct State {
    pub component: String,
    pub name: String,
    #[ts(type = "any")]
    pub value: serde_json::Value,
}

register_ts!(State);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct UpdateComponentData {
    pub operation: String,
    pub instance_name: String,
    pub r#type: String,
}

register_ts!(UpdateComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct ClearData {
    pub operation: String,
    pub instance_name: String,
    pub r#type: String,
    pub id: String,
}

register_ts!(ClearData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct SetComponentData {
    pub operation: String,
    pub instance_name: String,
    pub r#type: String,
    pub data: crate::component_model::Component,
}

register_ts!(SetComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct SetPluginData {
    pub operation: String,
    pub instance_name: String,
    pub r#type: String,
    pub data: crate::component_model::Plugin,
}

register_ts!(SetPluginData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct SetStateData {
    pub operation: String,
    pub instance_name: String,
    pub r#type: String,
    pub data: State,
}

register_ts!(SetStateData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct HistoryRecord {
    pub timestamp: i64,
    pub r#type: String,
}

register_ts!(HistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct InstanceHistoryRecord {
    pub timestamp: i64,
    pub r#type: String,
    pub instance_name: String,
}

register_ts!(InstanceHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct ComponentSetHistoryRecord {
    pub timestamp: i64,
    pub r#type: String,
    pub instance_name: String,
    pub component_id: String,
    #[ts(type = "{ [key: string]: any } | undefined")]
    pub states: Option<std::collections::HashMap<String, serde_json::Value>>,
}

register_ts!(ComponentSetHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct ComponentClearHistoryRecord {
    pub timestamp: i64,
    pub r#type: String,
    pub instance_name: String,
    pub component_id: String,
}

register_ts!(ComponentClearHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct StateHistoryRecord {
    pub timestamp: i64,
    pub r#type: String,
    pub instance_name: String,
    pub component_id: String,
    pub state_name: String,
    #[ts(type = "any")]
    pub state_value: serde_json::Value,
}

register_ts!(StateHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "online.ts")]
pub struct Status {
    pub transport_connected: bool,
}

register_ts!(Status);
