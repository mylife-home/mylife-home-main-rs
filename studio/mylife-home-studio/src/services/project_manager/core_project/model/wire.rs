//! Conversions between the core domain model and the existing API types.
//!
//! Same approach as the UI project's: anything that is one value becoming one
//! other value is a `From` or `TryFrom` impl. The exception here is the maps.
//! The core wire types store `HashMap<String, T>` directly rather than
//! `Arc<HashMap<String, Arc<T>>>`, and a generic
//! `TryFrom<&HashMap<String, W>> for Collection` would overlap the UI one on
//! `W = Arc<X>`, so these two stay free functions.

use std::collections::HashMap;
use std::sync::Arc;

use studio_web_api::project_manager;

use super::Collection;
use super::Id;
use super::change::Change;
use super::ids::{BindingId, ComponentId, ExportId, InstanceId, PluginId, TemplateId};
use super::project::Project;
use super::types::{
    Binding, Component, ComponentDefinition, Config, ConfigExport, MemberExport, Plugin, Position,
    Template, TemplateExports, ToolboxDisplay,
};
use super::view::ViewKey;

use super::error::CoreModelError;

// ---------------------------------------------------------------------------
// Map plumbing
// ---------------------------------------------------------------------------

fn to_wire_map<K, V, W>(collection: &Collection<K, V>) -> HashMap<String, W>
where
    K: Id,
    W: for<'a> From<&'a V>,
{
    collection
        .iter()
        .map(|(id, value)| (id.to_string(), W::from(value.as_ref())))
        .collect()
}

fn from_wire_map<K, V, W>(map: &HashMap<String, W>) -> Result<Collection<K, V>, CoreModelError>
where
    K: Id + TryFrom<String, Error = CoreModelError>,
    V: for<'a> TryFrom<&'a W, Error = CoreModelError>,
{
    map.iter()
        .map(|(id, value)| Ok((K::try_from(id.clone())?, Arc::new(V::try_from(value)?))))
        .collect()
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

impl TryFrom<&project_manager::CoreProject> for Project {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreProject) -> Result<Self, CoreModelError> {
        Ok(Project::from_parts(
            from_wire_map(&value.components)?,
            from_wire_map(&value.bindings)?,
            from_wire_map(&value.plugins)?,
            from_wire_map(&value.templates)?,
        ))
    }
}

impl From<&Project> for project_manager::CoreProject {
    fn from(value: &Project) -> Self {
        project_manager::CoreProject {
            components: to_wire_map(value.components()),
            bindings: to_wire_map(value.bindings()),
            plugins: to_wire_map(value.plugins()),
            templates: to_wire_map(value.templates()),
        }
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

impl From<&Position> for project_manager::CorePosition {
    fn from(value: &Position) -> Self {
        project_manager::CorePosition {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<&project_manager::CorePosition> for Position {
    fn from(value: &project_manager::CorePosition) -> Self {
        Position {
            x: value.x,
            y: value.y,
        }
    }
}

impl From<&Config> for project_manager::CoreComponentConfiguration {
    fn from(value: &Config) -> Self {
        project_manager::CoreComponentConfiguration(
            value
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect(),
        )
    }
}

impl From<&project_manager::CoreComponentConfiguration> for Config {
    fn from(value: &project_manager::CoreComponentConfiguration) -> Self {
        Config::new(value.0.clone())
    }
}

impl From<&ComponentDefinition> for project_manager::CoreComponentDefinition {
    fn from(value: &ComponentDefinition) -> Self {
        match value {
            ComponentDefinition::Plugin(id) => project_manager::CoreComponentDefinition {
                r#type: project_manager::CoreComponentDefinitionType::Plugin,
                id: id.to_string(),
            },
            ComponentDefinition::Template(id) => project_manager::CoreComponentDefinition {
                r#type: project_manager::CoreComponentDefinitionType::Template,
                id: id.to_string(),
            },
        }
    }
}

impl TryFrom<&project_manager::CoreComponentDefinition> for ComponentDefinition {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreComponentDefinition) -> Result<Self, CoreModelError> {
        Ok(match value.r#type {
            project_manager::CoreComponentDefinitionType::Plugin => {
                ComponentDefinition::Plugin(PluginId::new(&value.id)?)
            }
            project_manager::CoreComponentDefinitionType::Template => {
                ComponentDefinition::Template(TemplateId::new(&value.id)?)
            }
        })
    }
}

impl From<&Component> for project_manager::CoreComponentData {
    fn from(value: &Component) -> Self {
        project_manager::CoreComponentData {
            definition: (&value.definition).into(),
            position: (&value.position).into(),
            config: value.config.as_ref().map(Into::into),
            external: value.external,
        }
    }
}

impl TryFrom<&project_manager::CoreComponentData> for Component {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreComponentData) -> Result<Self, CoreModelError> {
        Ok(Component {
            definition: (&value.definition).try_into()?,
            position: (&value.position).into(),
            config: value.config.as_ref().map(Into::into),
            external: value.external,
        })
    }
}

// ---------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------

impl From<&Binding> for project_manager::CoreBindingData {
    fn from(value: &Binding) -> Self {
        project_manager::CoreBindingData {
            source_component: value.source_component.to_string(),
            source_state: value.source_state.clone(),
            target_component: value.target_component.to_string(),
            target_action: value.target_action.clone(),
        }
    }
}

impl TryFrom<&project_manager::CoreBindingData> for Binding {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreBindingData) -> Result<Self, CoreModelError> {
        Ok(Binding {
            source_component: ComponentId::new(&value.source_component)?,
            source_state: value.source_state.clone(),
            target_component: ComponentId::new(&value.target_component)?,
            target_action: value.target_action.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Plugins
// ---------------------------------------------------------------------------

impl From<ToolboxDisplay> for project_manager::CoreToolboxDisplay {
    fn from(value: ToolboxDisplay) -> Self {
        match value {
            ToolboxDisplay::Show => project_manager::CoreToolboxDisplay::Show,
            ToolboxDisplay::Hide => project_manager::CoreToolboxDisplay::Hide,
        }
    }
}

impl From<project_manager::CoreToolboxDisplay> for ToolboxDisplay {
    fn from(value: project_manager::CoreToolboxDisplay) -> Self {
        match value {
            project_manager::CoreToolboxDisplay::Show => ToolboxDisplay::Show,
            project_manager::CoreToolboxDisplay::Hide => ToolboxDisplay::Hide,
        }
    }
}

impl From<&Plugin> for project_manager::CorePluginData {
    fn from(value: &Plugin) -> Self {
        project_manager::CorePluginData {
            name: value.name.clone(),
            module: value.module.clone(),
            usage: value.usage,
            version: value.version.clone(),
            description: value.description.clone(),
            members: value.members.clone(),
            config: value.config.clone(),
            instance_name: value.instance_name.to_string(),
            toolbox_display: value.toolbox_display.into(),
        }
    }
}

impl TryFrom<&project_manager::CorePluginData> for Plugin {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CorePluginData) -> Result<Self, CoreModelError> {
        Ok(Plugin {
            name: value.name.clone(),
            module: value.module.clone(),
            usage: value.usage,
            version: value.version.clone(),
            description: value.description.clone(),
            members: value.members.clone(),
            config: value.config.clone(),
            instance_name: InstanceId::new(&value.instance_name)?,
            toolbox_display: value.toolbox_display.into(),
        })
    }
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

impl From<&ConfigExport> for project_manager::CoreTemplateConfigExport {
    fn from(value: &ConfigExport) -> Self {
        project_manager::CoreTemplateConfigExport {
            component: value.component.to_string(),
            config_name: value.config_name.clone(),
        }
    }
}

impl TryFrom<&project_manager::CoreTemplateConfigExport> for ConfigExport {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreTemplateConfigExport) -> Result<Self, CoreModelError> {
        Ok(ConfigExport {
            component: ComponentId::new(&value.component)?,
            config_name: value.config_name.clone(),
        })
    }
}

impl From<&MemberExport> for project_manager::CoreTemplateMemberExport {
    fn from(value: &MemberExport) -> Self {
        project_manager::CoreTemplateMemberExport {
            component: value.component.to_string(),
            member: value.member.clone(),
        }
    }
}

impl TryFrom<&project_manager::CoreTemplateMemberExport> for MemberExport {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreTemplateMemberExport) -> Result<Self, CoreModelError> {
        Ok(MemberExport {
            component: ComponentId::new(&value.component)?,
            member: value.member.clone(),
        })
    }
}

impl From<&TemplateExports> for project_manager::CoreTemplateExports {
    fn from(value: &TemplateExports) -> Self {
        project_manager::CoreTemplateExports {
            config: value
                .config
                .iter()
                .map(|(id, export)| (id.to_string(), export.into()))
                .collect(),
            members: value
                .members
                .iter()
                .map(|(id, export)| (id.to_string(), export.into()))
                .collect(),
        }
    }
}

impl TryFrom<&project_manager::CoreTemplateExports> for TemplateExports {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreTemplateExports) -> Result<Self, CoreModelError> {
        Ok(TemplateExports {
            config: value
                .config
                .iter()
                .map(|(id, export)| Ok((ExportId::new(id)?, export.try_into()?)))
                .collect::<Result<HashMap<_, _>, CoreModelError>>()?,
            members: value
                .members
                .iter()
                .map(|(id, export)| Ok((ExportId::new(id)?, export.try_into()?)))
                .collect::<Result<HashMap<_, _>, CoreModelError>>()?,
        })
    }
}

impl From<&Template> for project_manager::CoreTemplate {
    fn from(value: &Template) -> Self {
        project_manager::CoreTemplate {
            components: to_wire_map(&value.components),
            bindings: to_wire_map(&value.bindings),
            exports: (&value.exports).into(),
        }
    }
}

impl TryFrom<&project_manager::CoreTemplate> for Template {
    type Error = CoreModelError;

    fn try_from(value: &project_manager::CoreTemplate) -> Result<Self, CoreModelError> {
        Ok(Template {
            components: from_wire_map(&value.components)?,
            bindings: from_wire_map(&value.bindings)?,
            exports: (&value.exports).try_into()?,
        })
    }
}

// ---------------------------------------------------------------------------
// Changes -> notifications
// ---------------------------------------------------------------------------

impl From<&Change> for project_manager::UpdateProjectNotification {
    fn from(change: &Change) -> Self {
        use project_manager::UpdateProjectNotification as Notification;

        match change {
            Change::Reset => Notification::Reset(project_manager::ResetProjectNotification {}),

            Change::SetPlugins(plugins) => {
                Notification::SetCorePlugins(project_manager::SetCorePluginsNotification {
                    plugins: to_wire_map(plugins),
                })
            }
            Change::SetPluginToolboxDisplay { id, display } => {
                Notification::SetCorePluginToolboxDisplay(
                    project_manager::SetCorePluginToolboxDisplayNotification {
                        id: id.to_string(),
                        display: (*display).into(),
                    },
                )
            }
            Change::SetPlugin { id, plugin } => {
                Notification::SetCorePlugin(project_manager::SetCorePluginNotification {
                    id: id.to_string(),
                    plugin: plugin.as_ref().into(),
                })
            }
            Change::ClearPlugin { id } => {
                Notification::ClearCorePlugin(project_manager::ClearCorePluginNotification {
                    id: id.to_string(),
                })
            }

            Change::SetTemplate { id, exports } => {
                Notification::SetCoreTemplate(project_manager::SetCoreTemplateNotification {
                    id: id.to_string(),
                    exports: exports.into(),
                })
            }
            Change::ClearTemplate { id } => {
                Notification::ClearCoreTemplate(project_manager::ClearCoreTemplateNotification {
                    id: id.to_string(),
                })
            }
            Change::RenameTemplate { id, new_id } => {
                Notification::RenameCoreTemplate(project_manager::RenameCoreTemplateNotification {
                    id: id.to_string(),
                    new_id: new_id.to_string(),
                })
            }

            Change::SetComponent {
                view,
                id,
                component,
            } => Notification::SetCoreComponent(project_manager::SetCoreComponentNotification {
                template_id: template_id(view),
                id: id.to_string(),
                component: component.as_ref().into(),
            }),
            Change::ClearComponent { view, id } => {
                Notification::ClearCoreComponent(project_manager::ClearCoreComponentNotification {
                    template_id: template_id(view),
                    id: id.to_string(),
                })
            }
            Change::RenameComponent { view, id, new_id } => Notification::RenameCoreComponent(
                project_manager::RenameCoreComponentNotification {
                    template_id: template_id(view),
                    id: id.to_string(),
                    new_id: new_id.to_string(),
                },
            ),

            Change::SetBinding { view, id, binding } => {
                Notification::SetCoreBinding(project_manager::SetCoreBindingNotification {
                    template_id: template_id(view),
                    id: id.to_string(),
                    binding: binding.as_ref().into(),
                })
            }
            Change::ClearBinding { view, id } => {
                Notification::ClearCoreBinding(project_manager::ClearCoreBindingNotification {
                    template_id: template_id(view),
                    id: id.to_string(),
                })
            }
        }
    }
}

fn template_id(view: &ViewKey) -> Option<String> {
    view.template().map(ToString::to_string)
}

/// Convenience for the actor: a whole batch at once.
pub fn notifications(changes: &[Change]) -> Vec<project_manager::UpdateProjectNotification> {
    changes.iter().map(Into::into).collect()
}

// ---------------------------------------------------------------------------
// Call payloads
// ---------------------------------------------------------------------------

/// A call names its view with a nullable `templateId`. Resolving it once, here,
/// is what lets the model take a `ViewKey` and never test for null again.
pub fn view_key(template_id: &str) -> Result<ViewKey, CoreModelError> {
    if template_id.is_empty() {
        return Ok(ViewKey::Project);
    }

    Ok(ViewKey::Template(TemplateId::new(template_id)?))
}

pub fn component_ids(ids: &[String]) -> Result<Vec<ComponentId>, CoreModelError> {
    ids.iter().map(ComponentId::new).collect()
}

pub fn binding_id(id: &str) -> Result<BindingId, CoreModelError> {
    BindingId::new(id)
}
