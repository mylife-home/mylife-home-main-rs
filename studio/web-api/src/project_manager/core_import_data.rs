use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::component_model::PluginUsage;
use crate::register_ts;

// ===========================================================================
// coreImportData types
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Add,
    Update,
    Delete,
}

register_ts!(ChangeType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "lowercase")]
pub enum ObjectType {
    Component,
    Plugin,
    Template,
}

register_ts!(ObjectType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct Impact {
    pub r#type: String,
    pub template_id: String,
}

register_ts!(Impact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct BindingDeleteImpact {
    pub r#type: String,
    pub template_id: String,
    pub binding_id: String,
}

register_ts!(BindingDeleteImpact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentDeleteImpact {
    pub r#type: String,
    pub template_id: String,
    pub component_id: String,
}

register_ts!(ComponentDeleteImpact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentConfigImpact {
    pub r#type: String,
    pub template_id: String,
    pub component_id: String,
    /// update = reset
    pub config: HashMap<String, ChangeType>,
}

register_ts!(ComponentConfigImpact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct TemplateExportImpact {
    pub r#type: String,
    pub template_id: String,
    pub config_export_deletes: Vec<String>,
    pub member_export_deletes: Vec<String>,
}

register_ts!(TemplateExportImpact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct VersionChange {
    pub before: String,
    pub after: String,
}

register_ts!(VersionChange);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ObjectChange {
    /// for selection
    pub key: String,
    /// component/plugin/template id
    pub id: String,
    pub change_type: ChangeType,
    pub object_type: ObjectType,
    /// components changes may depend on plugins changes
    pub dependencies: Vec<String>,
    pub impacts: Vec<Impact>,
}

register_ts!(ObjectChange);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct PluginChange {
    pub key: String,
    pub id: String,
    pub change_type: ChangeType,
    pub object_type: ObjectType,
    pub dependencies: Vec<String>,
    pub impacts: Vec<Impact>,
    pub instance_name: String,
    pub version: VersionChange,
    /// or null if no change
    pub usage: Option<PluginUsage>,
    pub config: HashMap<String, ChangeType>,
    pub members: HashMap<String, ChangeType>,
}

register_ts!(PluginChange);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentChangeConfigEntry {
    pub r#type: ChangeType,
    #[ts(type = "any")]
    pub value: serde_json::Value,
}

register_ts!(ComponentChangeConfigEntry);

/// component changes are always on project directly, not inside templates
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentChange {
    pub key: String,
    pub id: String,
    pub change_type: ChangeType,
    pub object_type: ObjectType,
    pub dependencies: Vec<String>,
    pub impacts: Vec<Impact>,
    pub instance_name: String,
    pub config: HashMap<String, ComponentChangeConfigEntry>,
    /// or null if no change
    pub external: Option<bool>,
    /// or null if no change
    pub plugin_id: Option<String>,
}

register_ts!(ComponentChange);

/// used on exports removes, to apply the impact analysis on the changes
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct TemplateChange {
    pub key: String,
    pub id: String,
    pub change_type: ChangeType,
    pub object_type: ObjectType,
    pub dependencies: Vec<String>,
    pub impacts: Vec<Impact>,
    /// templates changes are only export deletion
    pub export_type: String,
    pub export_id: String,
}

register_ts!(TemplateChange);
