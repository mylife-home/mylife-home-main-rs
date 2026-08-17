use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    model::{Action, ActionComponent, ActionWindow, Control, ControlDisplay, ControlDisplayMapItem, ControlText, ControlTextContextItem, DefaultWindow, Resource, Style, Window},
    register_ts,
};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "ui-model.ts")]
pub struct Definition {
    pub resources: Vec<DefinitionResource>,
    pub styles: Vec<DefinitionStyle>,
    pub windows: Vec<Window>,
    pub default_window: DefaultWindow,
}

register_ts!(Definition);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "ui-model.ts")]
pub struct DefinitionResource {
    pub id: String,
    pub mime: String,
    pub data: String,
}

register_ts!(DefinitionResource);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "ui-model.ts")]
pub struct DefinitionStyle {
    pub id: String,
    #[ts(type = "object")]
    pub properties: serde_json::Value,
}

register_ts!(DefinitionStyle);

// Re-export the same model names most of the UI app relies on.
register_ts!(Resource);
register_ts!(Style);
register_ts!(DefaultWindow);
register_ts!(Window);
register_ts!(Control);
register_ts!(ControlDisplay);
register_ts!(ControlDisplayMapItem);
register_ts!(ControlText);
register_ts!(ControlTextContextItem);
register_ts!(Action);
register_ts!(ActionComponent);
register_ts!(ActionWindow);
