use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use web_api::model as api;

pub type DefaultWindow = HashMap<String, String>;
pub type Style = Vec<String>;
pub type Resource = String;

pub type ControlDisplay = api::ControlDisplay;
pub type ControlDisplayMapItem = api::ControlDisplayMapItem;
pub type ControlText = api::ControlText;
pub type ControlTextContextItem = api::ControlTextContextItem;
pub type Action = api::Action;
pub type ActionComponent = api::ActionComponent;
pub type ActionWindow = api::ActionWindow;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub resources: Vec<DefinitionResource>,
    pub styles: Vec<DefinitionStyle>,
    pub windows: Vec<Window>,
    pub default_window: DefaultWindow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub id: String,
    pub style: Style,
    pub height: i32,
    pub width: i32,
    pub background_resource: Option<Resource>,
    pub controls: Vec<Control>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionResource {
    pub id: String,
    pub mime: String,
    pub data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionStyle {
    pub id: String,
    pub properties: HashMap<String, Value>,
}
