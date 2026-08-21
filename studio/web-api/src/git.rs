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

// FIXME
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "git.ts")]
#[ts(type = "diff.File & { feature: string; }")]
pub struct GitDiffFile(pub GitDiffFileLayout);

register_ts!(GitDiffFile);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffFileLayout {
    pub feature: String,
}
