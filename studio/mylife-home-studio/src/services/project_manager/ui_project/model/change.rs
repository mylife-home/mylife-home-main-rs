use std::sync::Arc;

use super::Collection;
use super::ids::{ComponentId, PluginId, ResourceId, StyleId, TemplateId, WindowId};
use super::types::{Component, DefaultWindow, Plugin, Resource, StyleDefinition, Template, Window};

/// What an operation changed.
///
/// One per `UpdateProjectNotification`, in model terms. Each operation pushes
/// its own, explicitly, into a `&mut Vec` the caller discards if the operation
/// fails, so a change is emitted only for something that happened.
///
/// `wire::notifications` turns these into `UpdateProjectNotification` at the
/// actor boundary, which is what keeps the model from having to know the wire.
#[derive(Debug, Clone)]
pub enum Change {
    /// Everything said so far is void. Emitted when the project is replaced
    /// wholesale, ahead of the changes that describe the new one.
    Reset,

    SetDefaultWindow(Arc<DefaultWindow>),

    SetResource {
        id: ResourceId,
        resource: Arc<Resource>,
    },
    ClearResource {
        id: ResourceId,
    },
    RenameResource {
        id: ResourceId,
        new_id: ResourceId,
    },

    SetStyle {
        id: StyleId,
        style: Arc<StyleDefinition>,
    },
    ClearStyle {
        id: StyleId,
    },
    RenameStyle {
        id: StyleId,
        new_id: StyleId,
    },

    SetWindow {
        id: WindowId,
        window: Arc<Window>,
    },
    ClearWindow {
        id: WindowId,
    },
    RenameWindow {
        id: WindowId,
        new_id: WindowId,
    },

    SetTemplate {
        id: TemplateId,
        template: Arc<Template>,
    },
    ClearTemplate {
        id: TemplateId,
    },
    RenameTemplate {
        id: TemplateId,
        new_id: TemplateId,
    },

    SetComponentData {
        components: Collection<ComponentId, Component>,
        plugins: Collection<PluginId, Plugin>,
    },
}
