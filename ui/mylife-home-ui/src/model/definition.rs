use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use web_api::model as api;

pub type Window = api::Window;
pub type DefaultWindow = api::DefaultWindow;
pub type Control = api::Control;
pub type ControlDisplay = api::ControlDisplay;
pub type ControlText = api::ControlText;
pub type ControlTextContextItem = api::ControlTextContextItem;
pub type Action = api::Action;
pub type Style = api::Style;

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
