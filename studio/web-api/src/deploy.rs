use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct Run {
    pub id: String,
    pub recipe: String,
    pub logs: Vec<RunLog>,
    pub status: RunStatus,
    #[ts(type = "number")]
    pub creation: i64,
    #[ts(type = "number")]
    pub end: i64,
    pub err: RunError,
}

register_ts!(Run);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub enum RunStatus {
    Created,
    Running,
    Ended,
}

register_ts!(RunStatus);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct RunError {
    pub message: String,
    pub name: String,
    pub stack: String,
}

register_ts!(RunError);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct RunLog {
    #[ts(type = "number")]
    pub date: i64,
    pub category: String,
    pub severity: RunLogSeverity,
    pub message: String,
}

register_ts!(RunLog);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub enum RunLogSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

register_ts!(RunLogSeverity);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct RecipeConfig {
    pub description: String,
    pub steps: Vec<StepConfig>,
}

register_ts!(RecipeConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub enum StepType {
    Task,
    Recipe,
}

register_ts!(StepType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct StepConfig {
    pub r#type: StepType,
    pub enabled: bool,
    pub note: String,
}

register_ts!(StepConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct TaskStepConfig {
    pub r#type: StepType,
    pub enabled: bool,
    pub note: String,
    pub task: String,
    pub parameters: std::collections::HashMap<String, String>,
}

register_ts!(TaskStepConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct RecipeStepConfig {
    pub r#type: StepType,
    pub enabled: bool,
    pub note: String,
    pub recipe: String,
}

register_ts!(RecipeStepConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct FileInfo {
    pub id: String,
    #[ts(type = "number")]
    pub size: usize,
    #[ts(type = "number")]
    pub modified_date: i64,
}

register_ts!(FileInfo);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct TaskMetadata {
    pub description: String,
    pub parameters: Vec<TaskParameterMetadata>,
}

register_ts!(TaskMetadata);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct TaskParameterMetadata {
    pub name: String,
    pub description: String,
    pub r#type: String,
    #[ts(type = "any")]
    pub default: Option<serde_json::Value>,
}

register_ts!(TaskParameterMetadata);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct TaskParameters(pub std::collections::HashMap<String, String>);

register_ts!(TaskParameters);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct UpdateDataNotification {
    pub operation: String,
}

register_ts!(UpdateDataNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct SetTaskNotification {
    pub operation: String,
    pub id: String,
    pub metadata: TaskMetadata,
}

register_ts!(SetTaskNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct SetRecipeNotification {
    pub operation: String,
    pub id: String,
    pub config: RecipeConfig,
}

register_ts!(SetRecipeNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct ClearRecipeNotification {
    pub operation: String,
    pub id: String,
}

register_ts!(ClearRecipeNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct PinRecipeNotification {
    pub operation: String,
    pub id: String,
    pub value: bool,
}

register_ts!(PinRecipeNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct SetRunNotification {
    pub operation: String,
    pub run: Run,
}

register_ts!(SetRunNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct ClearRunNotification {
    pub operation: String,
    pub id: String,
}

register_ts!(ClearRunNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct AddRunLogNotification {
    pub operation: String,
    pub id: String,
    pub log: RunLog,
}

register_ts!(AddRunLogNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct SetFileNotification {
    pub operation: String,
    pub file: FileInfo,
}

register_ts!(SetFileNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "deploy.ts")]
pub struct ClearFileNotification {
    pub operation: String,
    pub id: String,
}

register_ts!(ClearFileNotification);
