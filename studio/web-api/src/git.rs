use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
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

/// One file of the working tree diff.
///
/// `from` and `to` are the paths on each side, so a rename carries both and a
/// delete carries no `to`. `feature` is the registered path feature the file
/// falls under, and is null for a file outside all of them.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitDiffFile {
    pub from: Option<String>,
    pub to: Option<String>,
    pub feature: Option<String>,
    pub chunks: Vec<GitDiffChunk>,
}

register_ts!(GitDiffFile);

/// A hunk: its header line, the line ranges it covers on each side, and the
/// lines themselves.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "git.ts")]
pub struct GitDiffChunk {
    pub content: String,
    pub old_start: i32,
    pub old_lines: i32,
    pub new_start: i32,
    pub new_lines: i32,
    pub changes: Vec<GitDiffChange>,
}

register_ts!(GitDiffChunk);

/// One line of a hunk. A context line has a number on both sides, an added or
/// deleted line only on its own.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "lowercase")]
#[ts(export_to = "git.ts")]
pub enum GitDiffChange {
    #[serde(rename_all = "camelCase")]
    Normal { ln1: i32, ln2: i32, content: String },
    #[serde(rename_all = "camelCase")]
    Add { ln: i32, content: String },
    #[serde(rename_all = "camelCase")]
    Del { ln: i32, content: String },
}

register_ts!(GitDiffChange);
