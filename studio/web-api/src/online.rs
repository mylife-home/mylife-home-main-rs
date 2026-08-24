use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;
use crate::component_model::{Component, Plugin};

// ===========================================================================
// Instance info
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfoWifi {
    pub rssi: i32,
}

register_ts!(InstanceInfoWifi);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    /// 'ui' | 'studio' | 'core' | 'driver? (for arduino/esp/...)'
    pub r#type: String,
    /// main: Raspberry ... | nodemcu | x64; others are details like ram, cpu, ...
    pub hardware: HashMap<String, String>,
    /// per-component versions (os, node, mylife-home-core, ...)
    pub versions: HashMap<String, String>,
    #[ts(type = "number")]
    pub system_uptime: i64,
    #[ts(type = "number")]
    pub instance_uptime: i64,
    pub hostname: String,
    pub capabilities: Vec<String>,
    /// present only when the instance has wifi
    pub wifi: Option<InstanceInfoWifi>,
}

register_ts!(InstanceInfo);

/// Update to an instance's info. Discriminated by `operation`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(tag = "operation", rename_all = "lowercase")]
pub enum UpdateInstanceInfoData {
    Set(SetInstanceInfoData),
    Clear(ClearInstanceInfoData),
}

register_ts!(UpdateInstanceInfoData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetInstanceInfoData {
    pub instance_name: String,
    pub data: InstanceInfo,
}

register_ts!(SetInstanceInfoData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearInstanceInfoData {
    pub instance_name: String,
}

register_ts!(ClearInstanceInfoData);

// ===========================================================================
// Component / plugin / state updates (tagged on `operation` then `type`)
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub component: String,
    pub name: String,
    #[ts(type = "any")]
    pub value: serde_json::Value,
}

register_ts!(State);

/// Update to the component registry. Discriminated by `operation`, then by
/// `type` within the `set` case.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(tag = "operation", rename_all = "lowercase")]
pub enum UpdateComponentData {
    Clear(ClearData),
    Set(SetData),
}

register_ts!(UpdateComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearData {
    pub instance_name: String,
    pub r#type: ComponentDataType,
    pub id: String,
}

register_ts!(ClearData);

/// The `set` payload, discriminated by `type` into component / plugin / state.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SetData {
    Component(SetComponentData),
    Plugin(SetPluginData),
    State(SetStateData),
}

register_ts!(SetData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetComponentData {
    pub instance_name: String,
    pub data: Component,
}

register_ts!(SetComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetPluginData {
    pub instance_name: String,
    pub data: Plugin,
}

register_ts!(SetPluginData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetStateData {
    pub instance_name: String,
    pub data: State,
}

register_ts!(SetStateData);

/// Component update target kind, used by ClearData.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "lowercase")]
pub enum ComponentDataType {
    Plugin,
    Component,
    State,
}

register_ts!(ComponentDataType);

// ===========================================================================
// History records (tagged on `type`)
// ===========================================================================

/// A history record. Discriminated by `type`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum HistoryRecord {
    InstanceSet(InstanceHistoryRecord),
    InstanceClear(InstanceHistoryRecord),
    ComponentSet(ComponentSetHistoryRecord),
    ComponentClear(ComponentClearHistoryRecord),
    StateSet(StateHistoryRecord),
}

register_ts!(HistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct InstanceHistoryRecord {
    #[ts(type = "number")]
    pub timestamp: i64,
    pub instance_name: String,
}

register_ts!(InstanceHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentSetHistoryRecord {
    #[ts(type = "number")]
    pub timestamp: i64,
    pub instance_name: String,
    pub component_id: String,
    /// present only for component-set records that carry state
    #[ts(type = "{ [name: string]: any } | null")]
    pub states: Option<HashMap<String, serde_json::Value>>,
}

register_ts!(ComponentSetHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentClearHistoryRecord {
    #[ts(type = "number")]
    pub timestamp: i64,
    pub instance_name: String,
    pub component_id: String,
}

register_ts!(ComponentClearHistoryRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct StateHistoryRecord {
    #[ts(type = "number")]
    pub timestamp: i64,
    pub instance_name: String,
    pub component_id: String,
    pub state_name: String,
    #[ts(type = "any")]
    pub state_value: serde_json::Value,
}

register_ts!(StateHistoryRecord);

// ===========================================================================
// Status
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "online.ts")]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub transport_connected: bool,
}

register_ts!(Status);