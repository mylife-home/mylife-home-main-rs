use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "protocol.ts")]
pub struct ServerMessage {
    pub r#type: String,
}

register_ts!(ServerMessage);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "protocol.ts")]
pub struct ServiceRequest {
    pub request_id: String,
    pub service: String,
    #[ts(type = "any")]
    pub payload: serde_json::Value,
}

register_ts!(ServiceRequest);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "protocol.ts")]
pub struct ServiceResponse {
    pub r#type: String,
    pub request_id: String,
    #[ts(type = "any")]
    pub result: Option<serde_json::Value>,
    pub error: Option<ServiceResponseError>,
}

register_ts!(ServiceResponse);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "protocol.ts")]
pub struct ServiceResponseError {
    pub r#type: String,
    pub message: String,
    pub stack: String,
}

register_ts!(ServiceResponseError);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "protocol.ts")]
pub struct Notification {
    pub r#type: String,
    pub notifier_type: String,
    pub notifier_id: String,
    #[ts(type = "any")]
    pub data: serde_json::Value,
}

register_ts!(Notification);
