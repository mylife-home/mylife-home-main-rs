use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitStatus {
    pub app_url: Option<String>,
    pub branch: String,
    pub changed_features: Vec<String>,
    pub ahead: Option<i32>,
    pub behind: Option<i32>,
}

register_ts!(GitStatus);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitStatusNotification {
    pub status: GitStatus,
}

register_ts!(GitStatusNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitCommit {
    pub message: String,
    pub files: Vec<String>,
}

register_ts!(GitCommit);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitRestore {
    pub files: Vec<String>,
}

register_ts!(GitRestore);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitDiff {
    pub files: Vec<GitDiffFile>,
}

register_ts!(GitDiff);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitDiffFile {
    pub feature: String,
    pub path: String,
    #[ts(type = "any[]")]
    pub chunks: Vec<serde_json::Value>,
}

register_ts!(GitDiffFile);
