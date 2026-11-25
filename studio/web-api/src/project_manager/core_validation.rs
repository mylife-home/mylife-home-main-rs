use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

// ===========================================================================
// coreValidation types
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Add,
    Update,
    Delete,
}

register_ts!(ChangeType);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

register_ts!(Severity);

/// One thing validation found.
///
/// Tagged by `type` on the wire, which is what the designer switches on to
/// pick how to display it. The variant is the tag, so a value cannot say it is
/// one thing and carry another.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Item {
    PluginChanged(PluginChanged),
    ExistingComponentId(ExistingComponentId),
    BadExternalComponent(BadExternalComponent),
    InvalidBindingApi(InvalidBindingApi),
    ComponentBadConfig(ComponentBadConfig),
    BindingMismatch(BindingMismatch),
}

register_ts!(Item);

impl Item {
    pub fn severity(&self) -> Severity {
        match self {
            Item::PluginChanged(item) => item.severity,
            Item::ExistingComponentId(item) => item.severity,
            Item::BadExternalComponent(item) => item.severity,
            Item::InvalidBindingApi(item) => item.severity,
            Item::ComponentBadConfig(item) => item.severity,
            Item::BindingMismatch(item) => item.severity,
        }
    }

    /// Whether anything found is bad enough to stop a deploy.
    pub fn has_error(items: &[Item]) -> bool {
        items
            .iter()
            .any(|item| matches!(item.severity(), Severity::Error))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct PluginChanged {
    pub severity: Severity,
    pub instance_name: String,
    pub module: String,
    pub name: String,
    /// update or delete only
    pub change_type: ChangeType,
    pub config: HashMap<String, ChangeType>,
    pub members: HashMap<String, ChangeType>,
    /// list of impacted components
    pub impacts: Vec<String>,
}

register_ts!(PluginChanged);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct PluginIdentity {
    pub instance_name: String,
    pub module: String,
    pub name: String,
}

register_ts!(PluginIdentity);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct VersionedPluginIdentity {
    pub instance_name: String,
    pub module: String,
    pub name: String,
    pub version: String,
}

register_ts!(VersionedPluginIdentity);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct ExistingComponentId {
    pub severity: Severity,
    pub component_id: String,
    pub project: PluginIdentity,
    pub existing: PluginIdentity,
}

register_ts!(ExistingComponentId);

/// may be only severity:info if plugin has same members;
/// if existing is empty then it's missing
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct BadExternalComponent {
    pub severity: Severity,
    pub component_id: String,
    pub project: VersionedPluginIdentity,
    /// `None` when nothing online carries that id at all.
    pub existing: Option<VersionedPluginIdentity>,
}

register_ts!(BadExternalComponent);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct InvalidBindingApi {
    pub severity: Severity,
    /// error if none or multiple
    pub instance_names: Vec<String>,
}

register_ts!(InvalidBindingApi);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentBadConfig {
    pub severity: Severity,
    pub component_id: String,
    pub instance_name: String,
    pub module: String,
    pub name: String,
    pub config: HashMap<String, String>,
}

register_ts!(ComponentBadConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "camelCase")]
pub struct BindingMismatch {
    pub severity: Severity,
    pub source_component: String,
    pub source_state: String,
    /// null = does not exist
    pub source_type: Option<String>,
    pub target_component: String,
    pub target_action: String,
    /// null = does not exist
    pub target_type: Option<String>,
}

register_ts!(BindingMismatch);
