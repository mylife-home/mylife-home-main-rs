//! Conversions between the domain model and the existing API types.
//!
//! This module is the price of keeping the model and the wire format apart,
//! and it is the whole price: nothing else in the model mentions a wire type.
//! In exchange the wire format can change without dragging the model with it,
//! and an id can be a `WindowId` on one side and a `String` on the other.
//!
//! Everything that converts one value into one other value is a `From` or a
//! `TryFrom` impl. The orphan rule allows it even though the wire types are
//! foreign: `impl From<&LocalType> for ForeignType` puts a local type in the
//! first parameter of a foreign trait, which RFC 2451 permits. The few free
//! functions take more than one input (`member_ref_from_wire`, `view_key`) or
//! convert between two foreign types (`bindings`), so there is no impl to hang
//! them on.
//!
//! These impls live here rather than in `studio-web-api` because the model is
//! the half that knows the rules; moving them there would put the validation
//! in the crate that defines the unvalidated shape.

use std::collections::HashMap;
use std::sync::Arc;

use studio_web_api::{project_manager, utils::PartialField};

use super::change::Change;
use super::control::{
    Action, ActionComponent, ActionWindow, Control, ControlDisplay, ControlDisplayMapItem,
    ControlKind, ControlText, ControlTextContextItem,
};
use super::error::UiModelError;
use super::ids::{ComponentId, ExportId, PluginId, ResourceId, StyleId, TemplateId, WindowId};
use super::path::{ElementPath, PathNode};
use super::project::Project;
use super::properties::{ControlProperties, Patch, TemplateProperties, WindowProperties};
use super::types::{
    Binding, Component, DefaultWindow, MemberRef, Plugin, Resource, StyleDefinition, StyleList,
    Template, TemplateExport, TemplateInstance, Window,
};
use super::view::ViewKey;

// ---------------------------------------------------------------------------
// Generic map plumbing
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Leaf references
// ---------------------------------------------------------------------------

impl From<&ResourceId> for project_manager::Resource {
    fn from(value: &ResourceId) -> Self {
        project_manager::Resource(value.to_string())
    }
}

impl TryFrom<&project_manager::Resource> for ResourceId {
    type Error = UiModelError;

    fn try_from(value: &project_manager::Resource) -> Result<Self, UiModelError> {
        ResourceId::new(&value.0)
    }
}

impl From<&StyleList> for project_manager::Style {
    fn from(value: &StyleList) -> Self {
        project_manager::Style(value.as_slice().iter().map(ToString::to_string).collect())
    }
}

impl TryFrom<&project_manager::Style> for StyleList {
    type Error = UiModelError;

    fn try_from(value: &project_manager::Style) -> Result<Self, UiModelError> {
        Ok(StyleList::new(
            value
                .0
                .iter()
                .map(StyleId::new)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

/// The `(component, member)` pair, inbound.
///
/// Not a `TryFrom`: it takes the two halves separately, because the wire types
/// that hold them name their fields differently, plus a `context` that only
/// appears in the error. This is the one place the fourth combination of that
/// pair has to be dealt with instead of carried around; a component with no
/// member is rejected here, which is exactly the state no operation can
/// produce.
fn member_ref_from_wire(
    context: &'static str,
    component_id: &Option<String>,
    member: &Option<String>,
) -> Result<Option<MemberRef>, UiModelError> {
    match (component_id, member) {
        (Some(component_id), Some(member)) => Ok(Some(MemberRef::component(
            ComponentId::new(component_id)?,
            member.clone(),
        ))),
        (None, Some(member)) => Ok(Some(MemberRef::OwnerExport(ExportId::new(member)?))),
        (None, None) => Ok(None),
        (Some(component_id), None) => Err(UiModelError::ComponentWithoutMember {
            context,
            component: component_id.clone(),
        }),
    }
}

/// The same pair, outbound. Returns the two halves for the caller to put in
/// whatever the wire type calls them.
fn member_ref_to_wire(member: &Option<MemberRef>) -> (Option<String>, Option<String>) {
    match member {
        Some(MemberRef::Component { id, member }) => (Some(id.to_string()), Some(member.clone())),
        Some(MemberRef::OwnerExport(id)) => (None, Some(id.to_string())),
        None => (None, None),
    }
}

// ---------------------------------------------------------------------------
// Element paths
// ---------------------------------------------------------------------------

impl From<&PathNode> for project_manager::UiElementPathNode {
    fn from(value: &PathNode) -> Self {
        project_manager::UiElementPathNode {
            r#type: value.kind().to_owned(),
            id: value.id(),
        }
    }
}

impl TryFrom<&project_manager::UiElementPathNode> for PathNode {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiElementPathNode) -> Result<Self, UiModelError> {
        PathNode::parse(&value.r#type, &value.id)
    }
}

impl From<&ElementPath> for project_manager::UiElementPath {
    fn from(value: &ElementPath) -> Self {
        value
            .nodes()
            .iter()
            .map(|node| Arc::new(node.into()))
            .collect()
    }
}

impl TryFrom<&project_manager::UiElementPath> for ElementPath {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiElementPath) -> Result<Self, UiModelError> {
        value
            .iter()
            .map(|node| PathNode::try_from(node.as_ref()))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Project
// ---------------------------------------------------------------------------

impl TryFrom<&project_manager::UiProject> for Project {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiProject) -> Result<Self, UiModelError> {
        Ok(Project::from_parts(
            value.resources.as_ref().try_into()?,
            value.styles.as_ref().try_into()?,
            value.windows.as_ref().try_into()?,
            value.templates.as_ref().try_into()?,
            Arc::new(DefaultWindow::try_from(value.default_window.as_ref())?),
            value.components.as_ref().try_into()?,
            value.plugins.as_ref().try_into()?,
        ))
    }
}

impl From<&Project> for project_manager::UiProject {
    fn from(value: &Project) -> Self {
        project_manager::UiProject {
            resources: value.resources().to_wire(),
            styles: value.styles().to_wire(),
            windows: value.windows().to_wire(),
            templates: value.templates().to_wire(),
            default_window: Arc::new(value.default_window().as_ref().into()),
            components: value.components().to_wire(),
            plugins: value.plugins().to_wire(),
        }
    }
}

// ---------------------------------------------------------------------------
// Resources, styles, components, plugins
// ---------------------------------------------------------------------------

impl From<&Resource> for project_manager::UiResourceData {
    fn from(value: &Resource) -> Self {
        project_manager::UiResourceData {
            mime: value.mime.clone(),
            data: value.data.clone(),
        }
    }
}

impl TryFrom<&project_manager::UiResourceData> for Resource {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiResourceData) -> Result<Self, UiModelError> {
        Ok(Resource {
            mime: value.mime.clone(),
            data: value.data.clone(),
        })
    }
}

impl From<&StyleDefinition> for project_manager::UiStyleData {
    fn from(value: &StyleDefinition) -> Self {
        project_manager::UiStyleData {
            properties: value.properties.clone(),
        }
    }
}

impl TryFrom<&project_manager::UiStyleData> for StyleDefinition {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiStyleData) -> Result<Self, UiModelError> {
        Ok(StyleDefinition {
            properties: value.properties.clone(),
        })
    }
}

impl From<&Component> for project_manager::UiComponentData {
    fn from(value: &Component) -> Self {
        project_manager::UiComponentData {
            plugin: value.plugin.to_string(),
        }
    }
}

impl TryFrom<&project_manager::UiComponentData> for Component {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiComponentData) -> Result<Self, UiModelError> {
        Ok(Component {
            plugin: PluginId::new(&value.plugin)?,
        })
    }
}

impl From<&Plugin> for project_manager::UiPluginData {
    fn from(value: &Plugin) -> Self {
        project_manager::UiPluginData {
            name: value.name.clone(),
            module: value.module.clone(),
            version: value.version.clone(),
            description: value.description.clone(),
            members: value.members.clone(),
            instance_name: value.instance_name.clone(),
        }
    }
}

impl TryFrom<&project_manager::UiPluginData> for Plugin {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiPluginData) -> Result<Self, UiModelError> {
        Ok(Plugin {
            name: value.name.clone(),
            module: value.module.clone(),
            version: value.version.clone(),
            description: value.description.clone(),
            members: value.members.clone(),
            instance_name: value.instance_name.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// Default window
// ---------------------------------------------------------------------------

impl From<&DefaultWindow> for project_manager::UiDefaultWindowData {
    fn from(value: &DefaultWindow) -> Self {
        project_manager::UiDefaultWindowData(
            value
                .iter()
                .map(|(key, window)| (key.clone(), window.as_ref().map(ToString::to_string)))
                .collect(),
        )
    }
}

impl TryFrom<&project_manager::UiDefaultWindowData> for DefaultWindow {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiDefaultWindowData) -> Result<Self, UiModelError> {
        let entries = value
            .0
            .iter()
            .map(|(key, window)| {
                Ok((
                    key.clone(),
                    window.as_deref().map(WindowId::new).transpose()?,
                ))
            })
            .collect::<Result<HashMap<_, _>, UiModelError>>()?;

        Ok(DefaultWindow::new(entries))
    }
}

// ---------------------------------------------------------------------------
// Views
// ---------------------------------------------------------------------------

impl From<&Window> for project_manager::UiWindowData {
    fn from(value: &Window) -> Self {
        project_manager::UiWindowData {
            height: value.height,
            width: value.width,
            controls: value.controls.to_wire(),
            templates: value.templates.to_wire(),
            style: (&value.style).into(),
            background_resource: value.background_resource.as_ref().map(Into::into),
        }
    }
}

impl TryFrom<&project_manager::UiWindowData> for Window {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiWindowData) -> Result<Self, UiModelError> {
        Ok(Window {
            height: value.height,
            width: value.width,
            controls: value.controls.as_ref().try_into()?,
            templates: value.templates.as_ref().try_into()?,
            style: (&value.style).try_into()?,
            background_resource: value
                .background_resource
                .as_ref()
                .map(ResourceId::try_from)
                .transpose()?,
        })
    }
}

impl From<&Template> for project_manager::UiTemplateData {
    fn from(value: &Template) -> Self {
        project_manager::UiTemplateData {
            height: value.height,
            width: value.width,
            controls: value.controls.to_wire(),
            templates: value.templates.to_wire(),
            exports: value.exports.to_wire(),
        }
    }
}

impl TryFrom<&project_manager::UiTemplateData> for Template {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiTemplateData) -> Result<Self, UiModelError> {
        Ok(Template {
            height: value.height,
            width: value.width,
            controls: value.controls.as_ref().try_into()?,
            templates: value.templates.as_ref().try_into()?,
            exports: value.exports.as_ref().try_into()?,
        })
    }
}

impl From<&TemplateExport> for project_manager::UiTemplateExport {
    fn from(value: &TemplateExport) -> Self {
        project_manager::UiTemplateExport {
            bulk_pattern: value.bulk_pattern.clone(),
            description: value.description.clone(),
            member_type: value.member_type,
            value_type: value.value_type.clone(),
        }
    }
}

impl TryFrom<&project_manager::UiTemplateExport> for TemplateExport {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiTemplateExport) -> Result<Self, UiModelError> {
        Ok(TemplateExport {
            bulk_pattern: value.bulk_pattern.clone(),
            description: value.description.clone(),
            member_type: value.member_type,
            value_type: value.value_type.clone(),
        })
    }
}

impl From<&TemplateInstance> for project_manager::UiTemplateInstanceData {
    fn from(value: &TemplateInstance) -> Self {
        project_manager::UiTemplateInstanceData {
            template_id: value.template_id.to_string(),
            x: value.x,
            y: value.y,
            bindings: Arc::new(
                value
                    .bindings
                    .iter()
                    .map(|(id, binding)| {
                        let (component_id, member_name) = member_ref_to_wire(binding);

                        (
                            id.to_string(),
                            Arc::new(project_manager::UiTemplateInstanceBinding {
                                component_id,
                                member_name,
                            }),
                        )
                    })
                    .collect(),
            ),
        }
    }
}

impl TryFrom<&project_manager::UiTemplateInstanceData> for TemplateInstance {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiTemplateInstanceData) -> Result<Self, UiModelError> {
        let bindings = value
            .bindings
            .iter()
            .map(|(id, binding)| {
                Ok((
                    ExportId::new(id)?,
                    member_ref_from_wire(
                        "template instance binding",
                        &binding.component_id,
                        &binding.member_name,
                    )?,
                ))
            })
            .collect::<Result<HashMap<ExportId, Binding>, UiModelError>>()?;

        Ok(TemplateInstance {
            template_id: TemplateId::new(&value.template_id)?,
            x: value.x,
            y: value.y,
            bindings,
        })
    }
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

impl From<&Control> for project_manager::UiControlData {
    fn from(value: &Control) -> Self {
        project_manager::UiControlData {
            style: (&value.style).into(),
            height: value.height,
            width: value.width,
            x: value.x,
            y: value.y,
            display: value
                .display
                .as_ref()
                .map(|display| Arc::new(display.as_ref().into())),
            text: value
                .text
                .as_ref()
                .map(|text| Arc::new(text.as_ref().into())),
            primary_action: value
                .primary_action
                .as_ref()
                .map(|action| Arc::new(action.as_ref().into())),
            secondary_action: value
                .secondary_action
                .as_ref()
                .map(|action| Arc::new(action.as_ref().into())),
        }
    }
}

impl TryFrom<&project_manager::UiControlData> for Control {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiControlData) -> Result<Self, UiModelError> {
        Ok(Control {
            style: (&value.style).try_into()?,
            height: value.height,
            width: value.width,
            x: value.x,
            y: value.y,
            display: value
                .display
                .as_ref()
                .map(|display| ControlDisplay::try_from(display.as_ref()).map(Arc::new))
                .transpose()?,
            text: value
                .text
                .as_ref()
                .map(|text| ControlText::try_from(text.as_ref()).map(Arc::new))
                .transpose()?,
            primary_action: value
                .primary_action
                .as_ref()
                .map(|action| Action::try_from(action.as_ref()).map(Arc::new))
                .transpose()?,
            secondary_action: value
                .secondary_action
                .as_ref()
                .map(|action| Action::try_from(action.as_ref()).map(Arc::new))
                .transpose()?,
        })
    }
}

impl From<&ControlDisplay> for project_manager::ControlDisplay {
    fn from(value: &ControlDisplay) -> Self {
        let (component_id, component_state) = member_ref_to_wire(&value.member);

        project_manager::ControlDisplay {
            component_id,
            component_state,
            default_resource: value.default_resource.as_ref().map(Into::into),
            map: value.map.iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<&project_manager::ControlDisplay> for ControlDisplay {
    type Error = UiModelError;

    fn try_from(value: &project_manager::ControlDisplay) -> Result<Self, UiModelError> {
        Ok(ControlDisplay {
            member: member_ref_from_wire("display", &value.component_id, &value.component_state)?,
            default_resource: value
                .default_resource
                .as_ref()
                .map(ResourceId::try_from)
                .transpose()?,
            map: value
                .map
                .iter()
                .map(ControlDisplayMapItem::try_from)
                .collect::<Result<Vec<_>, UiModelError>>()?,
        })
    }
}

impl From<&ControlDisplayMapItem> for project_manager::UiControlDisplayMapItemData {
    fn from(value: &ControlDisplayMapItem) -> Self {
        project_manager::UiControlDisplayMapItemData {
            min: value.min,
            max: value.max,
            value: value.value.clone(),
            resource: value.resource.as_ref().map(Into::into),
        }
    }
}

impl TryFrom<&project_manager::UiControlDisplayMapItemData> for ControlDisplayMapItem {
    type Error = UiModelError;

    fn try_from(
        value: &project_manager::UiControlDisplayMapItemData,
    ) -> Result<Self, UiModelError> {
        Ok(ControlDisplayMapItem {
            min: value.min,
            max: value.max,
            value: value.value.clone(),
            resource: value
                .resource
                .as_ref()
                .map(ResourceId::try_from)
                .transpose()?,
        })
    }
}

impl From<&ControlText> for project_manager::UiControlTextData {
    fn from(value: &ControlText) -> Self {
        project_manager::UiControlTextData {
            format: value.format.clone(),
            context: value.context.iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<&project_manager::UiControlTextData> for ControlText {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiControlTextData) -> Result<Self, UiModelError> {
        Ok(ControlText {
            format: value.format.clone(),
            context: value
                .context
                .iter()
                .map(ControlTextContextItem::try_from)
                .collect::<Result<Vec<_>, UiModelError>>()?,
        })
    }
}

impl From<&ControlTextContextItem> for project_manager::UiControlTextContextItemData {
    fn from(value: &ControlTextContextItem) -> Self {
        let (component_id, component_state) = member_ref_to_wire(&value.member);

        project_manager::UiControlTextContextItemData {
            id: value.id.clone(),
            component_id,
            component_state,
            test_value: value.test_value.clone(),
        }
    }
}

impl TryFrom<&project_manager::UiControlTextContextItemData> for ControlTextContextItem {
    type Error = UiModelError;

    fn try_from(
        value: &project_manager::UiControlTextContextItemData,
    ) -> Result<Self, UiModelError> {
        Ok(ControlTextContextItem {
            id: value.id.clone(),
            member: member_ref_from_wire(
                "text context item",
                &value.component_id,
                &value.component_state,
            )?,
            test_value: value.test_value.clone(),
        })
    }
}

impl From<&Action> for project_manager::UiActionData {
    fn from(value: &Action) -> Self {
        project_manager::UiActionData {
            component: value.component.as_ref().map(Into::into),
            window: value.window.as_ref().map(Into::into),
        }
    }
}

impl TryFrom<&project_manager::UiActionData> for Action {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiActionData) -> Result<Self, UiModelError> {
        Ok(Action {
            component: value
                .component
                .as_ref()
                .map(ActionComponent::try_from)
                .transpose()?,
            window: value
                .window
                .as_ref()
                .map(ActionWindow::try_from)
                .transpose()?,
        })
    }
}

impl From<&ActionComponent> for project_manager::UiActionComponentData {
    fn from(value: &ActionComponent) -> Self {
        let (id, action) = member_ref_to_wire(&value.member);

        project_manager::UiActionComponentData { id, action }
    }
}

impl TryFrom<&project_manager::UiActionComponentData> for ActionComponent {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiActionComponentData) -> Result<Self, UiModelError> {
        Ok(ActionComponent {
            member: member_ref_from_wire("action", &value.id, &value.action)?,
        })
    }
}

impl From<&ActionWindow> for project_manager::UiActionWindow {
    fn from(value: &ActionWindow) -> Self {
        project_manager::UiActionWindow {
            id: value.id.as_ref().map(ToString::to_string),
            popup: value.popup,
        }
    }
}

impl TryFrom<&project_manager::UiActionWindow> for ActionWindow {
    type Error = UiModelError;

    fn try_from(value: &project_manager::UiActionWindow) -> Result<Self, UiModelError> {
        Ok(ActionWindow {
            id: value.id.as_deref().map(WindowId::new).transpose()?,
            popup: value.popup,
        })
    }
}

// ---------------------------------------------------------------------------
// Changes -> notifications
// ---------------------------------------------------------------------------

/// The one place model changes become wire notifications.
///
/// Every operation still pushes its own change explicitly; this only renames
/// them for the client.
impl From<&Change> for project_manager::UpdateProjectNotification {
    fn from(change: &Change) -> Self {
        use project_manager::UpdateProjectNotification as Notification;

        match change {
            Change::Reset => Notification::Reset(project_manager::ResetProjectNotification {}),

            Change::SetDefaultWindow(default_window) => {
                Notification::SetUiDefaultWindow(project_manager::SetUiDefaultWindowNotification {
                    default_window: Arc::new(default_window.as_ref().into()),
                })
            }

            Change::SetComponentData {
                components,
                plugins,
            } => {
                Notification::SetUiComponentData(project_manager::SetUiComponentDataNotification {
                    components: components.to_wire(),
                    plugins: plugins.to_wire(),
                })
            }

            Change::SetResource { id, resource } => {
                Notification::SetUiResource(project_manager::SetUiResourceNotification {
                    id: id.to_string(),
                    resource: Arc::new(resource.as_ref().into()),
                })
            }
            Change::ClearResource { id } => {
                Notification::ClearUiResource(project_manager::ClearUiResourceNotification {
                    id: id.to_string(),
                })
            }
            Change::RenameResource { id, new_id } => {
                Notification::RenameUiResource(project_manager::RenameUiResourceNotification {
                    id: id.to_string(),
                    new_id: new_id.to_string(),
                })
            }

            Change::SetStyle { id, style } => {
                Notification::SetUiStyle(project_manager::SetUiStyleNotification {
                    id: id.to_string(),
                    style: Arc::new(style.as_ref().into()),
                })
            }
            Change::ClearStyle { id } => {
                Notification::ClearUiStyle(project_manager::ClearUiStyleNotification {
                    id: id.to_string(),
                })
            }
            Change::RenameStyle { id, new_id } => {
                Notification::RenameUiStyle(project_manager::RenameUiStyleNotification {
                    id: id.to_string(),
                    new_id: new_id.to_string(),
                })
            }

            Change::SetWindow { id, window } => {
                Notification::SetUiWindow(project_manager::SetUiWindowNotification {
                    id: id.to_string(),
                    window: Arc::new(window.as_ref().into()),
                })
            }
            Change::ClearWindow { id } => {
                Notification::ClearUiWindow(project_manager::ClearUiWindowNotification {
                    id: id.to_string(),
                })
            }
            Change::RenameWindow { id, new_id } => {
                Notification::RenameUiWindow(project_manager::RenameUiWindowNotification {
                    id: id.to_string(),
                    new_id: new_id.to_string(),
                })
            }

            Change::SetTemplate { id, template } => {
                Notification::SetUiTemplate(project_manager::SetUiTemplateNotification {
                    id: id.to_string(),
                    template: Arc::new(template.as_ref().into()),
                })
            }
            Change::ClearTemplate { id } => {
                Notification::ClearUiTemplate(project_manager::ClearUiTemplateNotification {
                    id: id.to_string(),
                })
            }
            Change::RenameTemplate { id, new_id } => {
                Notification::RenameUiTemplate(project_manager::RenameUiTemplateNotification {
                    id: id.to_string(),
                    new_id: new_id.to_string(),
                })
            }
        }
    }
}

/// Convenience for the actor: a whole batch at once.
pub fn notifications(changes: &[Change]) -> Vec<project_manager::UpdateProjectNotification> {
    changes.iter().map(Into::into).collect()
}

// ---------------------------------------------------------------------------
// Call payloads
// ---------------------------------------------------------------------------

/// A call names its view with a `(ViewType, String)` pair, which every function
/// downstream then has to carry together. Resolving it once, here, is what lets
/// the model take a single `ViewKey`. Two inputs, so not a `TryFrom`.
pub fn view_key(view_type: project_manager::ViewType, id: &str) -> Result<ViewKey, UiModelError> {
    Ok(match view_type {
        project_manager::ViewType::Window => ViewKey::Window(WindowId::new(id)?),
        project_manager::ViewType::Template => ViewKey::Template(TemplateId::new(id)?),
    })
}

/// A `HashMap` to a `HashMap`: both foreign, so there is no impl to hang this
/// on either.
pub fn bindings(
    value: HashMap<String, project_manager::UiTemplateInstanceBinding>,
) -> Result<HashMap<ExportId, Binding>, UiModelError> {
    value
        .into_iter()
        .map(|(id, binding)| {
            Ok((
                ExportId::new(&id)?,
                member_ref_from_wire(
                    "template instance binding",
                    &binding.component_id,
                    &binding.member_name,
                )?,
            ))
        })
        .collect()
}

/// Same reason as `bindings`: a `HashMap` keyed by `String` becoming one keyed
/// by `ExportId`, with both types foreign.
pub fn bulk_patterns(
    value: HashMap<String, String>,
) -> Result<HashMap<ExportId, String>, UiModelError> {
    value
        .into_iter()
        .map(|(id, pattern)| Ok((ExportId::new(&id)?, pattern)))
        .collect()
}

impl From<project_manager::UiControlType> for ControlKind {
    fn from(value: project_manager::UiControlType) -> Self {
        match value {
            project_manager::UiControlType::Display => ControlKind::Display,
            project_manager::UiControlType::Text => ControlKind::Text,
        }
    }
}

fn patch<T, W>(
    field: PartialField<W>,
    convert: impl FnOnce(W) -> Result<T, UiModelError>,
) -> Result<Patch<T>, UiModelError> {
    Ok(match field {
        PartialField::Missing => Patch::Unchanged,
        PartialField::Value(value) => Patch::Set(convert(value)?),
    })
}

impl TryFrom<project_manager::UiWindowProperties> for WindowProperties {
    type Error = UiModelError;

    fn try_from(value: project_manager::UiWindowProperties) -> Result<Self, UiModelError> {
        Ok(WindowProperties {
            height: patch(value.height, Ok)?,
            width: patch(value.width, Ok)?,
            style: patch(value.style, |style| (&style).try_into())?,
            background_resource: patch(value.background_resource, |resource| {
                resource.as_ref().map(ResourceId::try_from).transpose()
            })?,
        })
    }
}

impl TryFrom<project_manager::UiTemplateProperties> for TemplateProperties {
    type Error = UiModelError;

    fn try_from(value: project_manager::UiTemplateProperties) -> Result<Self, UiModelError> {
        Ok(TemplateProperties {
            height: patch(value.height, Ok)?,
            width: patch(value.width, Ok)?,
        })
    }
}

impl TryFrom<project_manager::UiControlProperties> for ControlProperties {
    type Error = UiModelError;

    fn try_from(value: project_manager::UiControlProperties) -> Result<Self, UiModelError> {
        Ok(ControlProperties {
            style: patch(value.style, |style| (&style).try_into())?,
            height: patch(value.height, Ok)?,
            width: patch(value.width, Ok)?,
            x: patch(value.x, Ok)?,
            y: patch(value.y, Ok)?,
            display: patch(value.display, |display| {
                display
                    .as_ref()
                    .map(|display| ControlDisplay::try_from(display).map(Arc::new))
                    .transpose()
            })?,
            text: patch(value.text, |text| {
                text.as_ref()
                    .map(|text| ControlText::try_from(text).map(Arc::new))
                    .transpose()
            })?,
            primary_action: patch(value.primary_action, |action| {
                action
                    .as_ref()
                    .map(|action| Action::try_from(action).map(Arc::new))
                    .transpose()
            })?,
            secondary_action: patch(value.secondary_action, |action| {
                action
                    .as_ref()
                    .map(|action| Action::try_from(action).map(Arc::new))
                    .transpose()
            })?,
        })
    }
}
