use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use crate::services::project_manager::model::{Id, IdError, define_id};

use super::error::UiModelError;

/// What an id names. Carried in errors so a message stays precise without each
/// call site formatting its own string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IdKind {
    Resource,
    Style,
    Window,
    Template,
    Control,
    TemplateInstance,
    Export,
    Component,
    Plugin,
}

impl IdKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            IdKind::Resource => "resource",
            IdKind::Style => "style",
            IdKind::Window => "window",
            IdKind::Template => "template",
            IdKind::Control => "control",
            IdKind::TemplateInstance => "template instance",
            IdKind::Export => "export",
            IdKind::Component => "component",
            IdKind::Plugin => "plugin",
        }
    }
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Ids the user types in the designer: non-empty, and ':' is reserved as the
/// deploy-time path separator.
fn validate_authored(kind: IdKind, value: &str) -> Result<(), UiModelError> {
    validate_not_empty(kind, value)?;

    if let Some(character) = value.chars().find(|character| *character == ':') {
        return Err(UiModelError::ForbiddenCharacter {
            kind,
            id: value.to_owned(),
            character,
        });
    }

    Ok(())
}

/// Ids that arrive from the core side. We do not get to pick their shape, so
/// the only thing we can assert is that they exist.
fn validate_not_empty(kind: IdKind, value: &str) -> Result<(), UiModelError> {
    if value.trim().is_empty() {
        return Err(UiModelError::EmptyId { kind });
    }

    Ok(())
}

define_id!(
    ResourceId,
    UiModelError,
    IdKind::Resource,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    StyleId,
    UiModelError,
    IdKind::Style,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    WindowId,
    UiModelError,
    IdKind::Window,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    TemplateId,
    UiModelError,
    IdKind::Template,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    ControlId,
    UiModelError,
    IdKind::Control,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    TemplateInstanceId,
    UiModelError,
    IdKind::TemplateInstance,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    ExportId,
    UiModelError,
    IdKind::Export,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    ComponentId,
    UiModelError,
    IdKind::Component,
    validate_not_empty,
    "Comes from the core side; we only assert it is non-empty."
);
define_id!(
    PluginId,
    UiModelError,
    IdKind::Plugin,
    validate_not_empty,
    "Structured as `instance:module.name`; see `PluginId::parts`."
);

impl PluginId {
    /// Plugin ids are built as `instance:module.name` (see
    /// `component::load_online_component_data`). They deliberately contain ':',
    /// which is why they are not validated as authored ids.
    ///
    /// Having their own type is what makes that readable: one kind of id plays
    /// by different rules, and the type says so.
    pub fn build(instance: &str, module: &str, name: &str) -> Result<Self, UiModelError> {
        Self::new(format!("{instance}:{module}.{name}"))
    }

    /// Splits back into `(instance, module, name)` when the id has the expected
    /// shape.
    pub fn parts(&self) -> Option<(&str, &str, &str)> {
        let (instance, rest) = self.as_str().split_once(':')?;
        let (module, name) = rest.rsplit_once('.')?;
        Some((instance, module, name))
    }
}
