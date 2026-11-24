use std::borrow::Borrow;
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;

use crate::services::project_manager::model::{Id, IdError, define_id};

use super::error::CoreModelError;

/// What an id names. Carried in errors so a message stays precise without each
/// call site formatting its own string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IdKind {
    Component,
    Template,
    Export,
    Instance,
    Plugin,
    Binding,
}

impl IdKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            IdKind::Component => "component",
            IdKind::Template => "template",
            IdKind::Export => "export",
            IdKind::Instance => "instance",
            IdKind::Plugin => "plugin",
            IdKind::Binding => "binding",
        }
    }
}

impl fmt::Display for IdKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Ids the user types in the designer: non-empty, and ':' is reserved, both as
/// the separator inside a plugin or binding id and as the path separator a
/// deploy generates.
fn validate_authored(kind: IdKind, value: &str) -> Result<(), CoreModelError> {
    validate_not_empty(kind, value)?;

    if let Some(character) = value.chars().find(|character| *character == ':') {
        return Err(CoreModelError::ForbiddenCharacter {
            kind,
            id: value.to_owned(),
            character,
        });
    }

    Ok(())
}

/// Ids this project does not author: they come from the network, or are built
/// by joining parts that are themselves checked. The only thing left to assert
/// is that they exist.
fn validate_not_empty(kind: IdKind, value: &str) -> Result<(), CoreModelError> {
    if value.trim().is_empty() {
        return Err(CoreModelError::EmptyId { kind });
    }

    Ok(())
}

define_id!(
    ComponentId,
    CoreModelError,
    IdKind::Component,
    validate_authored,
    "Authored id: non-empty, no ':'. Copies into a template carry a `{{id}}` \
     placeholder, which is substituted when the project is flattened."
);
define_id!(
    TemplateId,
    CoreModelError,
    IdKind::Template,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    ExportId,
    CoreModelError,
    IdKind::Export,
    validate_authored,
    "Authored id: non-empty, no ':'."
);
define_id!(
    InstanceId,
    CoreModelError,
    IdKind::Instance,
    validate_not_empty,
    "The name of a core instance. Comes from the network, so only non-empty."
);
define_id!(
    PluginId,
    CoreModelError,
    IdKind::Plugin,
    validate_not_empty,
    "Structured as `instance:module.name`; see `PluginId::parts`."
);
define_id!(
    BindingId,
    CoreModelError,
    IdKind::Binding,
    validate_not_empty,
    "Derived, not authored: `source:state:target:action`. See `Binding::id`."
);

impl ComponentId {
    /// What a component id inside a template carries where the id of whatever
    /// instantiates the template will go.
    ///
    /// Copying components into a template names them `{{id}}-<source id>` for
    /// this reason: without it, two uses of the template would generate the
    /// same ids.
    pub const PLACEHOLDER: &'static str = "{{id}}";

    /// This id with its placeholder filled in. Ids without one come back
    /// unchanged, which is what makes a fixed id usable exactly once.
    pub fn substituted(&self, id: &str) -> Result<Self, CoreModelError> {
        Self::new(self.as_str().replace(Self::PLACEHOLDER, id))
    }
}

impl PluginId {
    /// Plugin ids are built as `instance:module.name`, so they deliberately
    /// contain ':' and are not validated as authored ids.
    pub fn build(instance: &InstanceId, module: &str, name: &str) -> Result<Self, CoreModelError> {
        Self::new(format!("{instance}:{module}.{name}"))
    }

    pub fn parts(&self) -> Option<(&str, &str, &str)> {
        let (instance, rest) = self.as_str().split_once(':')?;
        let (module, name) = rest.rsplit_once('.')?;
        Some((instance, module, name))
    }

    pub fn instance(&self) -> Result<InstanceId, CoreModelError> {
        let (instance, _, _) = self
            .parts()
            .ok_or_else(|| CoreModelError::MalformedPluginId {
                id: self.to_string(),
            })?;

        InstanceId::new(instance)
    }
}

impl BindingId {
    /// A binding has no id of its own: it *is* its four parts, joined. Building
    /// it here rather than at each call site is what keeps the id and the data
    /// from drifting apart when a component is renamed.
    pub fn build(
        source_component: &ComponentId,
        source_state: &str,
        target_component: &ComponentId,
        target_action: &str,
    ) -> Result<Self, CoreModelError> {
        Self::new(format!(
            "{source_component}:{source_state}:{target_component}:{target_action}"
        ))
    }
}
