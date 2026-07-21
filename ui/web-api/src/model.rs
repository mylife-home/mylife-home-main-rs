use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[ts(type = "string")]
pub struct Resource(pub String);

register_ts!(Resource);

/// Style is a list of CSS class names (static for now).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[ts(type = "string[]")]
pub struct Style(pub Vec<String>);

register_ts!(Style);

/// DefaultWindow maps a context key to a window id.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[ts(type = "{ [key: string]: string }")]
pub struct DefaultWindow(pub HashMap<String, String>);

register_ts!(DefaultWindow);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub windows: Vec<Window>,
    pub default_window: DefaultWindow,
    /// css file to fetch
    pub style_hash: String,
}

register_ts!(Model);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub id: String,
    pub style: Style,
    pub height: i32,
    pub width: i32,
    pub background_resource: Option<Resource>,
    pub controls: Vec<Control>,
}

register_ts!(Window);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct Control {
    pub id: String,
    pub style: Style,
    pub height: i32,
    pub width: i32,
    pub x: i32,
    pub y: i32,
    pub display: Option<ControlDisplay>,
    pub text: Option<ControlText>,
    pub primary_action: Option<Action>,
    pub secondary_action: Option<Action>,
}

register_ts!(Control);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct ControlDisplay {
    pub component_id: Option<String>,
    pub component_state: Option<String>,
    pub default_resource: Option<Resource>,
    pub map: Vec<ControlDisplayMapItem>,
}

register_ts!(ControlDisplay);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct ControlDisplayMapItem {
    #[ts(type = "number | null")]
    pub min: Option<i64>,
    #[ts(type = "number | null")]
    pub max: Option<i64>,
    #[ts(type = "number | null")]
    pub value: serde_json::Value,
    pub resource: Option<Resource>,
}

register_ts!(ControlDisplayMapItem);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct ControlText {
    pub context: Vec<ControlTextContextItem>,
    pub format: String,
}

register_ts!(ControlText);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct ControlTextContextItem {
    pub id: String,
    pub component_id: String,
    pub component_state: String,
}

register_ts!(ControlTextContextItem);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub component: Option<ActionComponent>,
    pub window: Option<ActionWindow>,
}

register_ts!(Action);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct ActionComponent {
    pub id: String,
    pub action: String,
}

register_ts!(ActionComponent);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "model.ts")]
#[serde(rename_all = "camelCase")]
pub struct ActionWindow {
    pub id: String,
    pub popup: bool,
}

register_ts!(ActionWindow);
