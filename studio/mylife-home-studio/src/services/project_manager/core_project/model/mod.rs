//! Domain model for the studio core project.
//!
//! A core project is components, the bindings between them, the plugins they
//! instantiate and the templates that group them. It is built like the UI
//! project's model next door: owned types with inherent methods, self
//! validating ids, a typed error, and operations that return a new `Project`
//! while recording what moved. What the two share is `Collection`, the `Id`
//! and `IdError` traits it is written against, and the `define_id!` macro;
//! their ids, their kinds and their failures have almost nothing in common.
//!
//! Two things are different in kind from the UI side. A binding has no id of
//! its own: it *is* its four parts joined, so renaming a component re-keys
//! every binding that mentions it. And a component can instantiate a template,
//! which makes member and config lookups recurse through that template's
//! exports, and makes the ids a deploy would generate worth checking before an
//! operation is allowed (see `naming`).

/// `wire` keeps its module name at the call site, for the reason the UI
/// model's notes give: its contents are free functions whose module is the
/// whole of what they mean. Everything else reaches a caller as a type.
pub mod wire;

mod change;
mod error;
mod ids;
mod naming;
mod project;
mod types;
mod view;

/// The model's surface.
///
/// The actor does not name every type here: the config and export types, the
/// naming dry run and `View` are reached only by the tests, which is why this
/// needs the allow. A build without them would see them as dead.
#[allow(
    unused_imports,
    reason = "the tests reach parts of the model the actor never names"
)]
pub use self::{
    change::Change,
    error::CoreModelError,
    ids::{BindingId, ComponentId, ExportId, IdKind, InstanceId, PluginId, TemplateId},
    naming::NamingDryRun,
    project::Project,
    types::{
        Binding, Component, ComponentDefinition, Config, ConfigExport, Difference, ExportKind,
        MemberExport, Plugin, PluginDifferences, Position, Template, TemplateExports,
        ToolboxDisplay,
    },
    view::{View, ViewKey},
};

pub use crate::services::project_manager::model::{Collection, Id};
