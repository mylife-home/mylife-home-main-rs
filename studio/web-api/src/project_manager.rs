use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

use crate::{
    component_model::{Member, MemberType, PluginUsage},
    register_ts,
    ui_model::{Action, ControlDisplay, DefaultWindow, Resource, Style},
};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
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
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiResourceData {
    pub id: String,
    pub mime: String,
    pub data: String,
}

register_ts!(UiResourceData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiStyleData {
    pub id: String,
    #[ts(type = "object")]
    pub properties: serde_json::Value,
}

register_ts!(UiStyleData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiViewData {
    pub height: i32,
    pub width: i32,
    pub controls: HashMap<String, UiControlData>,
    pub templates: HashMap<String, UiTemplateInstanceData>,
}

register_ts!(UiViewData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
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
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiTemplateExport {
    pub bulk_pattern: String,
    pub description: String,
    pub member_type: MemberType,
    pub value_type: String,
}

register_ts!(UiTemplateExport);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiTemplateData {
    pub height: i32,
    pub width: i32,
    pub controls: HashMap<String, UiControlData>,
    pub templates: HashMap<String, UiTemplateInstanceData>,
    pub exports: HashMap<String, UiTemplateExport>,
}

register_ts!(UiTemplateData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiTemplateInstanceData {
    pub template_id: String,
    pub x: i32,
    pub y: i32,
    pub bindings: HashMap<String, UiTemplateInstanceBinding>,
}

register_ts!(UiTemplateInstanceData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiTemplateInstanceBinding {
    pub component_id: String,
    pub member_name: String,
}

register_ts!(UiTemplateInstanceBinding);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiControlData {
    pub id: String,
    pub style: Style,
    pub height: i32,
    pub width: i32,
    pub x: i32,
    pub y: i32,
    pub display: Option<ControlDisplay>,
    pub text: UiControlTextData,
    pub primary_action: Option<Action>,
    pub secondary_action: Option<Action>,
}

register_ts!(UiControlData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiControlTextData {
    pub context: Vec<UiControlTextContextItemData>,
    pub format: String,
}

register_ts!(UiControlTextData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiControlTextContextItemData {
    pub id: String,
    pub component_id: String,
    pub component_state: String,
    #[ts(type = "any")]
    pub test_value: serde_json::Value,
}

register_ts!(UiControlTextContextItemData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiComponentData {
    pub plugin: String,
}

register_ts!(UiComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiPluginData {
    pub name: String,
    pub module: String,
    pub version: String,
    pub description: String,
    pub members: HashMap<String, Member>,
    pub config: HashMap<String, crate::component_model::ConfigItem>,
    pub instance_name: String,
}

register_ts!(UiPluginData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreProject {
    pub components: HashMap<String, CoreComponentData>,
    pub bindings: HashMap<String, CoreBindingData>,
    pub plugins: HashMap<String, CorePluginData>,
    pub templates: HashMap<String, CoreTemplate>,
}

register_ts!(CoreProject);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreView {
    pub components: HashMap<String, CoreComponentData>,
    pub bindings: HashMap<String, CoreBindingData>,
}

register_ts!(CoreView);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreTemplate {
    pub components: HashMap<String, CoreComponentData>,
    pub bindings: HashMap<String, CoreBindingData>,
    pub exports: CoreTemplateExports,
}

register_ts!(CoreTemplate);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreTemplateExports {
    pub config: HashMap<String, CoreTemplateConfigExport>,
    pub members: HashMap<String, CoreTemplateMemberExport>,
}

register_ts!(CoreTemplateExports);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreTemplateConfigExport {
    pub component: String,
    pub config_name: String,
}

register_ts!(CoreTemplateConfigExport);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreTemplateMemberExport {
    pub component: String,
    pub member: String,
}

register_ts!(CoreTemplateMemberExport);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreBindingData {
    pub source_component: String,
    pub source_state: String,
    pub target_component: String,
    pub target_action: String,
}

register_ts!(CoreBindingData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub enum CoreComponentDefinitionType {
    Plugin,
    Template,
}

register_ts!(CoreComponentDefinitionType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreComponentDefinition {
    pub r#type: CoreComponentDefinitionType,
    pub id: String,
}

register_ts!(CoreComponentDefinition);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreComponentData {
    pub id: String,
    pub definition: CoreComponentDefinition,
    pub position: CorePosition,
    #[ts(type = "{ [key: string]: any }")]
    pub config: HashMap<String, serde_json::Value>,
    pub external: bool,
}

register_ts!(CoreComponentData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CorePosition {
    pub x: i32,
    pub y: i32,
}

register_ts!(CorePosition);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub enum CoreToolboxDisplay {
    Show,
    Hide,
}

register_ts!(CoreToolboxDisplay);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CorePluginData {
    pub name: String,
    pub module: String,
    pub usage: PluginUsage,
    pub version: String,
    pub description: String,
    pub members: HashMap<String, Member>,
    pub config: HashMap<String, crate::component_model::ConfigItem>,
    pub instance_name: String,
    pub toolbox_display: CoreToolboxDisplay,
}

register_ts!(CorePluginData);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub enum ProjectType {
    Ui,
    Core,
}

register_ts!(ProjectType);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UpdateListNotification {
    pub operation: String,
    pub r#type: ProjectType,
    pub name: String,
}

register_ts!(UpdateListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct SetListNotification {
    pub operation: String,
    pub r#type: ProjectType,
    pub name: String,
    #[ts(type = "any")]
    pub info: serde_json::Value,
}

register_ts!(SetListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct ClearListNotification {
    pub operation: String,
    pub r#type: ProjectType,
    pub name: String,
}

register_ts!(ClearListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct RenameListNotification {
    pub operation: String,
    pub r#type: ProjectType,
    pub name: String,
    pub new_name: String,
}

register_ts!(RenameListNotification);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct UiProjectInfo {
    pub windows_count: i32,
    pub resources_count: i32,
    pub resources_size: i32,
    pub styles_count: i32,
    pub components_count: i32,
}

register_ts!(UiProjectInfo);

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "project-manager.ts")]
pub struct CoreProjectInfo {
    pub instances_count: i32,
    pub plugins_count: i32,
    pub templates_count: i32,
    pub components_counts: HashMap<String, i32>,
    pub bindings_count: i32,
}

register_ts!(CoreProjectInfo);
