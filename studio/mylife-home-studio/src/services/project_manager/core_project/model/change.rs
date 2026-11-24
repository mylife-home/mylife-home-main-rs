use std::sync::Arc;

use super::Collection;
use super::ids::{BindingId, ComponentId, PluginId, TemplateId};
use super::types::{Binding, Component, Plugin, TemplateExports, ToolboxDisplay};
use super::view::ViewKey;

/// What an operation changed.
///
/// The core designer is told about individual components and bindings rather
/// than whole views, so these are finer-grained than the UI project's. Emission
/// is the same contract: each operation pushes its own, explicitly, into a
/// `&mut Vec` the caller discards if the operation fails.
///
/// Every component and binding change carries the view it happened in, as a
/// `ViewKey` rather than the wire's nullable `templateId`.
#[derive(Debug, Clone)]
pub enum Change {
    /// Everything said so far is void. Emitted when the project is replaced
    /// wholesale, ahead of the changes that describe the new one.
    Reset,

    SetPlugins(Collection<PluginId, Plugin>),
    SetPluginToolboxDisplay {
        id: PluginId,
        display: ToolboxDisplay,
    },
    SetPlugin {
        id: PluginId,
        plugin: Arc<Plugin>,
    },
    ClearPlugin {
        id: PluginId,
    },

    SetTemplate {
        id: TemplateId,
        exports: TemplateExports,
    },
    ClearTemplate {
        id: TemplateId,
    },
    RenameTemplate {
        id: TemplateId,
        new_id: TemplateId,
    },

    SetComponent {
        view: ViewKey,
        id: ComponentId,
        component: Arc<Component>,
    },
    ClearComponent {
        view: ViewKey,
        id: ComponentId,
    },
    RenameComponent {
        view: ViewKey,
        id: ComponentId,
        new_id: ComponentId,
    },

    SetBinding {
        view: ViewKey,
        id: BindingId,
        binding: Arc<Binding>,
    },
    ClearBinding {
        view: ViewKey,
        id: BindingId,
    },
}
