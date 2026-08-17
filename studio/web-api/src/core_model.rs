use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core-model.ts")]
pub enum StoreItemType {
    Component,
    Binding,
}

register_ts!(StoreItemType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core-model.ts")]
pub struct StoreItem {
    pub r#type: StoreItemType,
    #[ts(type = "any")]
    pub config: serde_json::Value,
}

register_ts!(StoreItem);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core-model.ts")]
pub struct ComponentConfig {
    pub id: String,
    pub plugin: String,
    #[ts(type = "{ [key: string]: any }")]
    pub config: std::collections::HashMap<String, serde_json::Value>,
}

register_ts!(ComponentConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "core-model.ts")]
pub struct BindingConfig {
    pub source_component: String,
    pub source_state: String,
    pub target_component: String,
    pub target_action: String,
}

register_ts!(BindingConfig);
