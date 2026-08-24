use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::register_ts;

// Adjust these paths to your actual module layout.
use crate::component_model::{ConfigItem, Member, MemberType, PluginUsage};
use crate::ui_model::{Action, ControlDisplay, ControlDisplayMapItem, DefaultWindow, Resource, Style};
use core_import_data::ObjectChange;
use core_validation::Item;

pub mod core_import_data;
pub mod core_validation;

// ===========================================================================
// Value-type string unions (standalone enums used as field types)
// ===========================================================================

/// add / update / delete
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    Add,
    Update,
    Delete,
}

register_ts!(ChangeType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "lowercase")]
pub enum ProjectType {
    Ui,
    Core,
}

register_ts!(ProjectType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "lowercase")]
pub enum CoreComponentDefinitionType {
    Plugin,
    Template,
}

register_ts!(CoreComponentDefinitionType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "lowercase")]
pub enum CoreToolboxDisplay {
    Show,
    Hide,
}

register_ts!(CoreToolboxDisplay);

// ---- core import value unions ----

// ---- core validation value unions ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager-core-validation.ts")]
#[serde(rename_all = "kebab-case")]
pub enum ItemType {
    PluginChanged,
    ExistingComponentId,
    BadExternalComponent,
    InvalidBindingApi,
    ComponentBadConfig,
    BindingMismatch,
}

register_ts!(ItemType);


/// null | 'standard' | 'external' on the wire; model the null via Option at the
/// use site, this enum is the non-null part.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "lowercase")]
pub enum ComponentsImportType {
    Standard,
    External,
}

register_ts!(ComponentsImportType);

// ===========================================================================
// Core project data model
// ===========================================================================

/// A component's configuration: opaque values keyed by config name.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[ts(type = "{ [name: string]: any; }")]
pub struct CoreComponentConfiguration(pub HashMap<String, serde_json::Value>);

register_ts!(CoreComponentConfiguration);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CorePosition {
    pub x: i32,
    pub y: i32,
}

register_ts!(CorePosition);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreComponentDefinition {
    pub r#type: CoreComponentDefinitionType,
    /// plugin points to plugin instanceName:module.name
    pub id: String,
}

register_ts!(CoreComponentDefinition);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreComponentData {
    pub id: String,
    pub definition: CoreComponentDefinition,
    pub position: CorePosition,
    pub config: CoreComponentConfiguration,
    pub external: bool,
}

register_ts!(CoreComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreBindingData {
    pub source_component: String,
    pub source_state: String,
    pub target_component: String,
    pub target_action: String,
}

register_ts!(CoreBindingData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CorePluginData {
    pub name: String,
    pub module: String,
    pub usage: PluginUsage,
    pub version: String,
    pub description: String,
    pub members: HashMap<String, Member>,
    pub config: HashMap<String, ConfigItem>,
    pub instance_name: String,
    pub toolbox_display: CoreToolboxDisplay,
}

register_ts!(CorePluginData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreTemplateConfigExport {
    pub component: String,
    pub config_name: String,
}

register_ts!(CoreTemplateConfigExport);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreTemplateMemberExport {
    pub component: String,
    pub member: String,
}

register_ts!(CoreTemplateMemberExport);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreTemplateExports {
    pub config: HashMap<String, CoreTemplateConfigExport>,
    pub members: HashMap<String, CoreTemplateMemberExport>,
}

register_ts!(CoreTemplateExports);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreTemplate {
    pub components: HashMap<String, CoreComponentData>,
    pub bindings: HashMap<String, CoreBindingData>,
    pub exports: CoreTemplateExports,
}

register_ts!(CoreTemplate);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreView {
    pub components: HashMap<String, CoreComponentData>,
    pub bindings: HashMap<String, CoreBindingData>,
}

register_ts!(CoreView);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreProject {
    pub components: HashMap<String, CoreComponentData>,
    pub bindings: HashMap<String, CoreBindingData>,
    pub plugins: HashMap<String, CorePluginData>,
    pub templates: HashMap<String, CoreTemplate>,
}

register_ts!(CoreProject);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CoreProjectInfo {
    pub instances_count: i32,
    pub plugins_count: i32,
    pub templates_count: i32,
    pub components_counts: HashMap<String, i32>,
    pub bindings_count: i32,
}

register_ts!(CoreProjectInfo);

// ===========================================================================
// UI project data model
// ===========================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiResourceData {
    pub mime: String,
    pub data: String,
}

register_ts!(UiResourceData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiStyleData {
    #[ts(type = "object")]
    pub properties: serde_json::Value,
}

register_ts!(UiStyleData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiComponentData {
    pub plugin: String,
}

register_ts!(UiComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiPluginData {
    pub name: String,
    pub module: String,
    pub version: String,
    pub description: String,
    pub members: HashMap<String, Member>,
    pub config: HashMap<String, ConfigItem>,
    pub instance_name: String,
}

register_ts!(UiPluginData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiTemplateInstanceBinding {
    pub component_id: String,
    pub member_name: String,
}

register_ts!(UiTemplateInstanceBinding);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiTemplateInstanceData {
    pub template_id: String,
    pub x: i32,
    pub y: i32,
    pub bindings: HashMap<String, UiTemplateInstanceBinding>,
}

register_ts!(UiTemplateInstanceData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiControlTextContextItemData {
    pub id: String,
    pub component_id: String,
    pub component_state: String,
    /// used only for designer render, not deployed
    #[ts(type = "any")]
    pub test_value: serde_json::Value,
}

register_ts!(UiControlTextContextItemData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiControlTextData {
    pub context: Vec<UiControlTextContextItemData>,
    pub format: String,
}

register_ts!(UiControlTextData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiControlData {
    pub id: String,
    pub style: Style,
    pub height: i32,
    pub width: i32,
    pub x: i32,
    pub y: i32,
    /// null when the control has no display
    pub display: Option<ControlDisplay>,
    pub text: UiControlTextData,
    /// null when there is no primary action
    pub primary_action: Option<Action>,
    /// null when there is no secondary action
    pub secondary_action: Option<Action>,
}

register_ts!(UiControlData);

/// Aliases of the shared ui/model types, re-exported under UI-project names.
pub type UiActionData = Action;
pub type UiControlDisplayData = ControlDisplay;
pub type UiControlDisplayMapItemData = ControlDisplayMapItem;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiViewData {
    pub height: i32,
    pub width: i32,
    pub controls: HashMap<String, UiControlData>,
    pub templates: HashMap<String, UiTemplateInstanceData>,
}

register_ts!(UiViewData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiWindowData {
    pub height: i32,
    pub width: i32,
    pub controls: HashMap<String, UiControlData>,
    pub templates: HashMap<String, UiTemplateInstanceData>,
    pub style: Style,
    pub background_resource: Resource,
}

register_ts!(UiWindowData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiTemplateExport {
    pub bulk_pattern: String,
    pub description: String,
    pub member_type: MemberType,
    pub value_type: String,
}

register_ts!(UiTemplateExport);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiTemplateData {
    pub height: i32,
    pub width: i32,
    pub controls: HashMap<String, UiControlData>,
    pub templates: HashMap<String, UiTemplateInstanceData>,
    pub exports: HashMap<String, UiTemplateExport>,
}

register_ts!(UiTemplateData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiProject {
    pub resources: HashMap<String, UiResourceData>,
    pub styles: HashMap<String, UiStyleData>,
    pub windows: HashMap<String, UiWindowData>,
    pub templates: HashMap<String, UiTemplateData>,
    pub default_window: DefaultWindow,
    pub components: HashMap<String, UiComponentData>,
    pub plugins: HashMap<String, UiPluginData>,
}

register_ts!(UiProject);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiProjectInfo {
    pub windows_count: i32,
    pub resources_count: i32,
    pub resources_size: i32,
    pub styles_count: i32,
    pub components_count: i32,
}

register_ts!(UiProjectInfo);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiElementPathNode {
    pub r#type: String,
    pub id: String,
}

register_ts!(UiElementPathNode);

pub type UiElementPath = Vec<UiElementPathNode>;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiValidationError {
    pub path: Vec<UiElementPathNode>,
    pub message: String,
}

register_ts!(UiValidationError);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UiBreakingOperation {
    pub operation: String,
    pub component_id: String,
    pub usage: Vec<Vec<UiElementPathNode>>,
}

register_ts!(UiBreakingOperation);

// ===========================================================================
// List notifications (tagged on `operation`: set / clear / rename)
// ===========================================================================

/// Notifications about the project list. Discriminated by `operation`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(tag = "operation", rename_all = "lowercase")]
pub enum UpdateListNotification {
    Set(SetListNotification),
    Clear(ClearListNotification),
    Rename(RenameListNotification),
}

register_ts!(UpdateListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetListNotification {
    pub r#type: ProjectType,
    pub name: String,
    #[ts(type = "ProjectInfo")]
    pub info: serde_json::Value,
}

register_ts!(SetListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearListNotification {
    pub r#type: ProjectType,
    pub name: String,
}

register_ts!(ClearListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameListNotification {
    pub r#type: ProjectType,
    pub name: String,
    pub new_name: String,
}

register_ts!(RenameListNotification);

// ===========================================================================
// Project update notifications (tagged on `operation`)
// ===========================================================================

/// All project-update notifications, discriminated by `operation`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub enum UpdateProjectNotification {
    SetName(SetNameProjectNotification),
    Reset(ResetProjectNotification),

    // UI
    SetUiDefaultWindow(SetUiDefaultWindowNotification),
    SetUiComponentData(SetUiComponentDataNotification),
    SetUiResource(SetUiResourceNotification),
    ClearUiResource(ClearUiResourceNotification),
    RenameUiResource(RenameUiResourceNotification),
    SetUiStyle(SetUiStyleNotification),
    ClearUiStyle(ClearUiStyleNotification),
    RenameUiStyle(RenameUiStyleNotification),
    SetUiWindow(SetUiWindowNotification),
    ClearUiWindow(ClearUiWindowNotification),
    RenameUiWindow(RenameUiWindowNotification),
    SetUiTemplate(SetUiTemplateNotification),
    ClearUiTemplate(ClearUiTemplateNotification),
    RenameUiTemplate(RenameUiTemplateNotification),

    // Core
    SetCorePlugins(SetCorePluginsNotification),
    SetCorePluginToolboxDisplay(SetCorePluginToolboxDisplayNotification),
    SetCorePlugin(SetCorePluginNotification),
    ClearCorePlugin(ClearCorePluginNotification),
    SetCoreComponent(SetCoreComponentNotification),
    ClearCoreComponent(ClearCoreComponentNotification),
    RenameCoreComponent(RenameCoreComponentNotification),
    SetCoreBinding(SetCoreBindingNotification),
    ClearCoreBinding(ClearCoreBindingNotification),
    SetCoreTemplate(SetCoreTemplateNotification),
    ClearCoreTemplate(ClearCoreTemplateNotification),
    RenameCoreTemplate(RenameCoreTemplateNotification),
}

register_ts!(UpdateProjectNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetNameProjectNotification {
    pub name: String,
}

register_ts!(SetNameProjectNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
pub struct ResetProjectNotification {}

register_ts!(ResetProjectNotification);

// ---- UI notifications ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetUiDefaultWindowNotification {
    pub default_window: DefaultWindow,
}

register_ts!(SetUiDefaultWindowNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetUiComponentDataNotification {
    pub components: HashMap<String, UiComponentData>,
    pub plugins: HashMap<String, UiPluginData>,
}

register_ts!(SetUiComponentDataNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetUiResourceNotification {
    pub id: String,
    pub resource: UiResourceData,
}

register_ts!(SetUiResourceNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearUiResourceNotification {
    pub id: String,
}

register_ts!(ClearUiResourceNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameUiResourceNotification {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameUiResourceNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetUiStyleNotification {
    pub id: String,
    pub style: UiStyleData,
}

register_ts!(SetUiStyleNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearUiStyleNotification {
    pub id: String,
}

register_ts!(ClearUiStyleNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameUiStyleNotification {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameUiStyleNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetUiWindowNotification {
    pub id: String,
    pub window: UiWindowData,
}

register_ts!(SetUiWindowNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearUiWindowNotification {
    pub id: String,
}

register_ts!(ClearUiWindowNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameUiWindowNotification {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameUiWindowNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetUiTemplateNotification {
    pub id: String,
    pub template: UiTemplateData,
}

register_ts!(SetUiTemplateNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearUiTemplateNotification {
    pub id: String,
}

register_ts!(ClearUiTemplateNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameUiTemplateNotification {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameUiTemplateNotification);

// ---- Core notifications ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetCorePluginsNotification {
    pub plugins: HashMap<String, CorePluginData>,
}

register_ts!(SetCorePluginsNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetCorePluginToolboxDisplayNotification {
    pub id: String,
    pub display: CoreToolboxDisplay,
}

register_ts!(SetCorePluginToolboxDisplayNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetCorePluginNotification {
    pub id: String,
    pub plugin: CorePluginData,
}

register_ts!(SetCorePluginNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearCorePluginNotification {
    pub id: String,
}

register_ts!(ClearCorePluginNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetCoreComponentNotification {
    /// null if no template
    pub template_id: Option<String>,
    pub id: String,
    pub component: CoreComponentData,
}

register_ts!(SetCoreComponentNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearCoreComponentNotification {
    /// null if no template
    pub template_id: Option<String>,
    pub id: String,
}

register_ts!(ClearCoreComponentNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameCoreComponentNotification {
    /// null if no template
    pub template_id: Option<String>,
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameCoreComponentNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetCoreBindingNotification {
    /// null if no template
    pub template_id: Option<String>,
    pub id: String,
    pub binding: CoreBindingData,
}

register_ts!(SetCoreBindingNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearCoreBindingNotification {
    /// null if no template
    pub template_id: Option<String>,
    pub id: String,
}

register_ts!(ClearCoreBindingNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetCoreTemplateNotification {
    pub id: String,
    pub exports: CoreTemplateExports,
}

register_ts!(SetCoreTemplateNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearCoreTemplateNotification {
    pub id: String,
}

register_ts!(ClearCoreTemplateNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameCoreTemplateNotification {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameCoreTemplateNotification);

// ===========================================================================
// UI project calls (tagged on `operation`)
// ===========================================================================

/// All UI project calls, discriminated by `operation`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub enum UiProjectCall {
    // validate / refresh / deploy handled by dedicated calls below where they carry data;
    // fieldless ones are unit variants.
    Validate,
    RefreshComponentsFromOnline,
    RefreshComponentsFromProject(RefreshComponentsFromProjectUiProjectCall),
    ApplyRefreshComponents(ApplyRefreshComponentsUiProjectCall),
    Deploy,
    SetDefaultWindow(SetDefaultWindowUiProjectCall),

    SetResource(SetResourceUiProjectCall),
    ClearResource(ClearResourceUiProjectCall),
    RenameResource(RenameResourceUiProjectCall),

    SetStyle(SetStyleUiProjectCall),
    ClearStyle(ClearStyleUiProjectCall),
    RenameStyle(RenameStyleUiProjectCall),

    NewWindow(NewWindowUiProjectCall),
    ClearWindow(ClearWindowUiProjectCall),
    RenameWindow(RenameWindowUiProjectCall),
    CloneWindow(CloneWindowUiProjectCall),
    SetWindowProperties(SetWindowPropertiesUiProjectCall),

    NewTemplate(NewTemplateUiProjectCall),
    ClearTemplate(ClearTemplateUiProjectCall),
    RenameTemplate(RenameTemplateUiProjectCall),
    CloneTemplate(CloneTemplateUiProjectCall),
    SetTemplateProperties(SetTemplatePropertiesUiProjectCall),
    SetTemplateExport(SetTemplateExportUiProjectCall),
    ClearTemplateExport(ClearTemplateExportUiProjectCall),
    SetTemplateBulkPatterns(SetTemplateBulkPatternsUiProjectCall),

    NewControl(NewControlUiProjectCall),
    ClearControl(ClearControlUiProjectCall),
    RenameControl(RenameControlUiProjectCall),
    CloneControl(CloneControlUiProjectCall),
    SetControlProperties(SetControlPropertiesUiProjectCall),

    NewTemplateInstance(NewTemplateInstanceUiProjectCall),
    ClearTemplateInstance(ClearTemplateInstanceUiProjectCall),
    RenameTemplateInstance(RenameTemplateInstanceUiProjectCall),
    CloneTemplateInstance(CloneTemplateInstanceUiProjectCall),
    MoveTemplateInstance(MoveTemplateInstanceUiProjectCall),
    SetTemplateInstanceTemplate(SetTemplateInstanceTemplateUiProjectCall),
    SetTemplateInstanceBindings(SetTemplateInstanceBindingsUiProjectCall),
}

register_ts!(UiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RefreshComponentsFromProjectUiProjectCall {
    pub project_id: String,
}

register_ts!(RefreshComponentsFromProjectUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ApplyRefreshComponentsUiProjectCall {
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(ApplyRefreshComponentsUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetDefaultWindowUiProjectCall {
    pub default_window: DefaultWindow,
}

register_ts!(SetDefaultWindowUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetResourceUiProjectCall {
    pub id: String,
    pub resource: UiResourceData,
}

register_ts!(SetResourceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearResourceUiProjectCall {
    pub id: String,
}

register_ts!(ClearResourceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameResourceUiProjectCall {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameResourceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetStyleUiProjectCall {
    pub id: String,
    pub style: UiStyleData,
}

register_ts!(SetStyleUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearStyleUiProjectCall {
    pub id: String,
}

register_ts!(ClearStyleUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameStyleUiProjectCall {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameStyleUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct NewWindowUiProjectCall {
    pub id: String,
}

register_ts!(NewWindowUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearWindowUiProjectCall {
    pub id: String,
}

register_ts!(ClearWindowUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameWindowUiProjectCall {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameWindowUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CloneWindowUiProjectCall {
    pub id: String,
    pub new_id: String,
}

register_ts!(CloneWindowUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetWindowPropertiesUiProjectCall {
    pub id: String,
    #[ts(type = "any")]
    pub properties: serde_json::Value,
}

register_ts!(SetWindowPropertiesUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct NewTemplateUiProjectCall {
    pub id: String,
}

register_ts!(NewTemplateUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearTemplateUiProjectCall {
    pub id: String,
}

register_ts!(ClearTemplateUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameTemplateUiProjectCall {
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameTemplateUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CloneTemplateUiProjectCall {
    pub id: String,
    pub new_id: String,
}

register_ts!(CloneTemplateUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplatePropertiesUiProjectCall {
    pub id: String,
    #[ts(type = "any")]
    pub properties: serde_json::Value,
}

register_ts!(SetTemplatePropertiesUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplateExportUiProjectCall {
    pub id: String,
    pub export_id: String,
    pub member_type: MemberType,
    pub value_type: String,
}

register_ts!(SetTemplateExportUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearTemplateExportUiProjectCall {
    pub id: String,
    pub export_id: String,
}

register_ts!(ClearTemplateExportUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplateBulkPatternsUiProjectCall {
    pub id: String,
    pub patterns: HashMap<String, String>,
}

register_ts!(SetTemplateBulkPatternsUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct NewControlUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub r#type: String,
}

register_ts!(NewControlUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearControlUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
}

register_ts!(ClearControlUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameControlUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameControlUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CloneControlUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub new_id: String,
    pub target_view_type: String,
    pub target_view_id: String,
}

register_ts!(CloneControlUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetControlPropertiesUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    #[ts(type = "any")]
    pub properties: serde_json::Value,
}

register_ts!(SetControlPropertiesUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct NewTemplateInstanceUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub template_id: String,
    pub x: i32,
    pub y: i32,
}

register_ts!(NewTemplateInstanceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearTemplateInstanceUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
}

register_ts!(ClearTemplateInstanceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameTemplateInstanceUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub new_id: String,
}

register_ts!(RenameTemplateInstanceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CloneTemplateInstanceUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub new_id: String,
}

register_ts!(CloneTemplateInstanceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct MoveTemplateInstanceUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub x: i32,
    pub y: i32,
}

register_ts!(MoveTemplateInstanceUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplateInstanceTemplateUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub template_id: String,
}

register_ts!(SetTemplateInstanceTemplateUiProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplateInstanceBindingsUiProjectCall {
    pub view_type: String,
    pub view_id: String,
    pub id: String,
    pub bindings: HashMap<String, UiTemplateInstanceBinding>,
}

register_ts!(SetTemplateInstanceBindingsUiProjectCall);

// ---- UI call results ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ValidateUiProjectCallResult {
    pub errors: Vec<UiValidationError>,
}

register_ts!(ValidateUiProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RefreshComponentsUiProjectCallResult {
    pub breaking_operations: Vec<UiBreakingOperation>,
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(RefreshComponentsUiProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct DeployUiProjectCallResult {
    /// null when there were no validation errors
    pub validation_errors: Option<Vec<UiValidationError>>,
    /// null when there was no deploy error
    pub deploy_error: Option<String>,
}

register_ts!(DeployUiProjectCallResult);

// ===========================================================================
// Core project calls (tagged on `operation`)
// ===========================================================================

/// All core project calls, discriminated by `operation`.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(tag = "operation", rename_all = "kebab-case")]
pub enum CoreProjectCall {
    UpdateToolbox(UpdateToolboxCoreProjectCall),
    SetTemplate(SetTemplateCoreProjectCall),
    ClearTemplate(ClearTemplateCoreProjectCall),
    RenameTemplate(RenameTemplateCoreProjectCall),
    SetTemplateExport(SetTemplateExportCoreProjectCall),
    ClearTemplateExport(ClearTemplateExportCoreProjectCall),
    SetComponent(SetComponentCoreProjectCall),
    MoveComponents(MoveComponentsCoreProjectCall),
    ConfigureComponent(ConfigureComponentCoreProjectCall),
    RenameComponent(RenameComponentCoreProjectCall),
    ClearComponents(ClearComponentsCoreProjectCall),
    CopyComponentsToTemplate(CopyComponentsToTemplateCoreProjectCall),
    SetBinding(SetBindingCoreProjectCall),
    ClearBinding(ClearBindingCoreProjectCall),
    PrepareImportFromOnline(PrepareImportFromOnlineCoreProjectCall),
    PrepareImportFromProject(PrepareImportFromProjectCoreProjectCall),
    ApplyBulkUpdates(ApplyBulkUpdatesCoreProject),
    Validate,
    PrepareDeployToFiles,
    ApplyDeployToFiles(ApplyDeployToFilesCoreProjectCall),
    PrepareDeployToOnline,
    ApplyDeployToOnline(ApplyDeployToOnlineCoreProjectCall),
}

register_ts!(CoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct UpdateToolboxCoreProjectCall {
    pub item_type: String,
    pub item_id: String,
    pub action: String,
}

register_ts!(UpdateToolboxCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplateCoreProjectCall {
    pub template_id: String,
}

register_ts!(SetTemplateCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearTemplateCoreProjectCall {
    pub template_id: String,
}

register_ts!(ClearTemplateCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameTemplateCoreProjectCall {
    pub template_id: String,
    pub new_id: String,
}

register_ts!(RenameTemplateCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetTemplateExportCoreProjectCall {
    pub template_id: String,
    pub export_type: String,
    pub export_id: String,
    pub component_id: String,
    /// config or member
    pub property_name: String,
}

register_ts!(SetTemplateExportCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearTemplateExportCoreProjectCall {
    pub template_id: String,
    pub export_type: String,
    pub export_id: String,
}

register_ts!(ClearTemplateExportCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetComponentCoreProjectCall {
    pub template_id: String,
    pub component_id: String,
    pub definition: CoreComponentDefinition,
    pub x: i32,
    pub y: i32,
}

register_ts!(SetComponentCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct MoveComponentsCoreProjectCall {
    pub template_id: String,
    pub components_ids: Vec<String>,
    pub delta: CorePosition,
}

register_ts!(MoveComponentsCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ConfigureComponentCoreProjectCall {
    pub template_id: String,
    pub component_id: String,
    pub config_id: String,
    #[ts(type = "any")]
    pub config_value: serde_json::Value,
}

register_ts!(ConfigureComponentCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct RenameComponentCoreProjectCall {
    pub template_id: String,
    pub component_id: String,
    pub new_id: String,
}

register_ts!(RenameComponentCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearComponentsCoreProjectCall {
    pub template_id: String,
    pub components_ids: Vec<String>,
}

register_ts!(ClearComponentsCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CopyComponentsToTemplateCoreProjectCall {
    pub template_id: String,
    pub components_ids: Vec<String>,
    pub target_template_id: String,
}

register_ts!(CopyComponentsToTemplateCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct SetBindingCoreProjectCall {
    pub template_id: String,
    pub binding: CoreBindingData,
}

register_ts!(SetBindingCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ClearBindingCoreProjectCall {
    pub template_id: String,
    pub binding_id: String,
}

register_ts!(ClearBindingCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct PrepareImportFromOnlineCoreProjectCall {
    pub config: ImportFromOnlineConfig,
}

register_ts!(PrepareImportFromOnlineCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct PrepareImportFromProjectCoreProjectCall {
    pub config: ImportFromProjectConfig,
}

register_ts!(PrepareImportFromProjectCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ApplyBulkUpdatesCoreProject {
    pub selection: Vec<String>,
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(ApplyBulkUpdatesCoreProject);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ApplyDeployToFilesCoreProjectCall {
    /// optional: omitted when the instance is left to be deduced
    pub bindings_instance_name: Option<String>,
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(ApplyDeployToFilesCoreProjectCall);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ApplyDeployToOnlineCoreProjectCall {
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(ApplyDeployToOnlineCoreProjectCall);

// ---- import configs ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ImportFromOnlineConfig {
    pub import_plugins: bool,
    /// always external, config is not published online
    pub import_components: bool,
}

register_ts!(ImportFromOnlineConfig);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ImportFromProjectConfig {
    pub import_plugins: bool,
    /// null | 'standard' | 'external'
    pub import_components: Option<ComponentsImportType>,
    pub project_id: String,
}

register_ts!(ImportFromProjectConfig);

// ---- core call results ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CopyComponentsStats {
    pub components: i32,
    pub bindings: i32,
}

register_ts!(CopyComponentsStats);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct CopyComponentsCoreProjectCallResult {
    pub stats: CopyComponentsStats,
}

register_ts!(CopyComponentsCoreProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct BulkUpdatesStats {
    pub plugins: i32,
    pub components: i32,
    pub templates: i32,
    pub bindings: i32,
}

register_ts!(BulkUpdatesStats);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ApplyBulkUpdatesCoreProjectCallResult {
    pub stats: BulkUpdatesStats,
}

register_ts!(ApplyBulkUpdatesCoreProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct PrepareBulkUpdatesCoreProjectCallResult {
    pub changes: Vec<ObjectChange>,
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(PrepareBulkUpdatesCoreProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ValidateCoreProjectCallResult {
    pub validation: Vec<Item>,
}

register_ts!(ValidateCoreProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct BindingsInstanceName {
    /// null if could not deduce
    pub actual: Option<String>,
    pub needed: bool,
}

register_ts!(BindingsInstanceName);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct PrepareDeployToFilesCoreProjectCallResult {
    pub validation: Vec<Item>,
    /// only adds
    pub changes: DeployChanges,
    pub files: Vec<String>,
    pub bindings_instance_name: BindingsInstanceName,
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(PrepareDeployToFilesCoreProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ApplyDeployToFilesCoreProjectCallResult {
    pub written_files_count: i32,
}

register_ts!(ApplyDeployToFilesCoreProjectCallResult);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct PrepareDeployToOnlineCoreProjectCallResult {
    pub validation: Vec<Item>,
    pub changes: DeployChanges,
    #[ts(type = "unknown")]
    pub server_data: serde_json::Value,
}

register_ts!(PrepareDeployToOnlineCoreProjectCallResult);

// ---- deploy changes ----

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct DeployChanges {
    pub components: Vec<ComponentDeployChange>,
    pub bindings: Vec<BindingDeployChange>,
}

register_ts!(DeployChanges);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct ComponentDeployChange {
    pub r#type: ChangeType,
    pub instance_name: String,
    pub component_id: String,
}

register_ts!(ComponentDeployChange);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "project-manager.ts")]
#[serde(rename_all = "camelCase")]
pub struct BindingDeployChange {
    /// no update
    pub r#type: ChangeType,
    /// may be null for files deploy if binding instance could not be deduced
    pub instance_name: Option<String>,
    pub binding_id: String,
}

register_ts!(BindingDeployChange);