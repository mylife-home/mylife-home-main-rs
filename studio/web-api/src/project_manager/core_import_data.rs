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

/// What accepting one change would drag along with it.
///
/// Tagged by `type` on the wire, which is what the designer switches on to
/// show it. Each one carries `templateId`, empty for the project view itself,
/// the same way a call spells it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Impact {
    ComponentConfig(ComponentConfigImpact),
    ComponentDelete(ComponentDeleteImpact),
    BindingDelete(BindingDeleteImpact),
    TemplateExport(TemplateExportImpact),
}

register_ts!(Impact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct BindingDeleteImpact {
    pub template_id: String,
    pub binding_id: String,
}

register_ts!(BindingDeleteImpact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentDeleteImpact {
    pub template_id: String,
    pub component_id: String,
}

register_ts!(ComponentDeleteImpact);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentConfigImpact {
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

/// One thing an import would change.
///
/// Tagged by `objectType` on the wire. Every member carries the same head:
/// `key` is what a selection names, `dependencies` are the other changes this
/// one needs, and `impacts` is what accepting it drags along.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(tag = "objectType", rename_all = "lowercase")]
pub enum ObjectChange {
    Plugin(PluginChange),
    Component(ComponentChange),
    Template(TemplateChange),
}

register_ts!(ObjectChange);

impl ObjectChange {
    pub fn key(&self) -> &str {
        match self {
            ObjectChange::Plugin(change) => &change.key,
            ObjectChange::Component(change) => &change.key,
            ObjectChange::Template(change) => &change.key,
        }
    }

    pub fn dependencies(&self) -> &[String] {
        match self {
            ObjectChange::Plugin(change) => &change.dependencies,
            ObjectChange::Component(change) => &change.dependencies,
            ObjectChange::Template(change) => &change.dependencies,
        }
    }

    pub fn impacts_mut(&mut self) -> &mut Vec<Impact> {
        match self {
            ObjectChange::Plugin(change) => &mut change.impacts,
            ObjectChange::Component(change) => &mut change.impacts,
            ObjectChange::Template(change) => &mut change.impacts,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-import-data.ts")]
#[serde(rename_all = "camelCase")]
pub struct PluginChange {
    pub key: String,
    pub id: String,
    pub change_type: ChangeType,
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
    pub dependencies: Vec<String>,
    pub impacts: Vec<Impact>,
    /// templates changes are only export deletion
    pub export_type: String,
    pub export_id: String,
}

register_ts!(TemplateChange);
