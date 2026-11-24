//! Domain model for the studio UI project.
//!
//! A UI project is windows, templates, resources, styles, and the component
//! data the designer binds against. This module owns that as plain Rust: owned
//! types with inherent methods, ids that validate themselves on construction,
//! and one typed error per way an operation can fail.
//!
//! Every operation takes `&self` and returns a new `Project`, recording what
//! moved into a `&mut Vec<Change>`. A failed operation leaves nothing behind,
//! because the caller still holds the project it started from and simply drops
//! the half-built one.
//!
//! `studio_web_api::project_manager::UiProject` is the other half of the pair:
//! the serde shape on disk and the ts-rs shape on the wire. `wire` is the only
//! thing that converts between the two, which is what lets an id be a
//! `WindowId` here and a `String` there.

/// `wire` keeps its module name at the call site: it holds free functions
/// whose module is the whole of what they mean, and `model::wire::view_key`
/// says something that `model::view_key` would not. Everything else in the
/// model reaches a caller as a type, so the submodules below are an
/// arrangement of this file's contents and nothing has to know them.
pub mod wire;

mod change;
mod control;
mod error;
mod ids;
mod path;
mod project;
mod properties;
mod types;
mod usage;
mod view;

pub use crate::services::project_manager::model::{Collection, Id};

/// The model's surface.
///
/// The actor does not name every type here: `Patch`, `ViewKey`, `IdKind`,
/// `ElementPath` and the action parts are reached only by the tests, which is
/// why this needs the allow. A build without them would see them as dead.
#[allow(
    unused_imports,
    reason = "the tests reach parts of the model the actor never names"
)]
pub use self::{
    change::Change,
    control::{
        Action, ActionComponent, ActionWindow, Control, ControlDisplay, ControlDisplayMapItem,
        ControlKind, ControlText, ControlTextContextItem,
    },
    error::UiModelError,
    ids::{
        ComponentId, ControlId, ExportId, IdKind, PluginId, ResourceId, StyleId, TemplateId,
        TemplateInstanceId, WindowId,
    },
    path::{ElementPath, PathNode},
    project::Project,
    properties::{ControlProperties, Patch, TemplateProperties, WindowProperties},
    types::{
        Component, DefaultWindow, MemberRef, Plugin, Resource, StyleDefinition, StyleList,
        Template, TemplateInstance, Window,
    },
    usage::ComponentUsage,
    view::ViewKey,
};
