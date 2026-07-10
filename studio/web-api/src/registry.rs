use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use ts_rs::TS;

use crate::register_ts;

// Note: needed for proper ts generation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "registry.ts")]
#[ts(type = "{ [key: string]: any }")]
pub struct ComponentStates(pub HashMap<String, Value>);

register_ts!(ComponentStates);

// Note: needed for proper ts generation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "registry.ts")]
pub struct Reset(pub HashMap<String, ComponentStates>);

register_ts!(Reset);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "registry.ts")]
pub struct ComponentAdd {
    pub id: String,
    pub attributes: ComponentStates,
}

register_ts!(ComponentAdd);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "registry.ts")]
pub struct ComponentRemove {
    pub id: String,
}

register_ts!(ComponentRemove);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "registry.ts")]
pub struct StateChange {
    pub id: String,
    pub name: String,
    #[ts(type = "any")]
    pub value: Value,
}

register_ts!(StateChange);
