use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

/// A message sent from the server. Discriminated by `type`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol.ts")]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    ServiceResponse(ServiceResponse),
    Notification(Notification),
}

register_ts!(ServerMessage);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol.ts")]
#[serde(rename_all = "camelCase")]
pub struct ServiceRequest {
    pub request_id: String,
    pub service: String,
    #[ts(type = "any")]
    pub payload: serde_json::Value,
}

register_ts!(ServiceRequest);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol.ts")]
#[serde(rename_all = "camelCase")]
pub struct ServiceResponse {
    pub request_id: String,
    /// present on success
    #[ts(type = "any")]
    pub result: Option<serde_json::Value>,
    /// present on failure
    pub error: Option<ServiceResponseError>,
}

register_ts!(ServiceResponse);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol.ts")]
#[serde(rename_all = "camelCase")]
pub struct ServiceResponseError {
    pub r#type: String,
    pub message: String,
    pub stack: String,
}

register_ts!(ServiceResponseError);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol.ts")]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub notifier_type: String,
    pub notifier_id: String,
    #[ts(type = "any")]
    pub data: serde_json::Value,
}

register_ts!(Notification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "protocol.ts")]
#[serde(rename_all = "camelCase")]
pub struct NotifierId {
    pub notifier_id: String,
}

register_ts!(NotifierId);
