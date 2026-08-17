use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "logging.ts")]
pub struct LogRecord {
    pub name: String,
    pub instance_name: String,
    pub hostname: String,
    pub pid: i32,
    pub level: i32,
    pub msg: String,
    pub time: String,
    pub v: i32,
    pub err: Option<LogRecordError>,
}

register_ts!(LogRecord);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "logging.ts")]
pub struct LogRecordError {
    pub message: String,
    pub name: String,
    pub stack: String,
}

register_ts!(LogRecordError);
