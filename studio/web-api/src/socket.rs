use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "socket.ts")]
pub enum MessageType {
    // server to client
    State,
    Add,
    Remove,
    Change,
    ModelHash,
    Pong,

    // client to server
    Ping,
    Action,
}

register_ts!(MessageType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "socket.ts")]
pub struct SocketMessage {
    pub r#type: MessageType,

    #[ts(type = "any")]
    pub data: Value,
}

register_ts!(SocketMessage);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "socket.ts")]
pub struct ActionMessage {
    pub id: String,
    pub action: String,
}

register_ts!(ActionMessage);
