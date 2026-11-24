use thiserror::Error;

use crate::services::project_manager::model::IdError;

use super::ids::IdKind;

/// Every way a UI project operation can fail.
///
/// The variants carry their parts rather than a formatted string, so a caller
/// can match on the failure and the actor layer decides what the wire protocol
/// should say about it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum UiModelError {
    #[error("{kind} id is empty")]
    EmptyId { kind: IdKind },

    #[error("{kind} id '{id}' contains forbidden character '{character}'")]
    ForbiddenCharacter {
        kind: IdKind,
        id: String,
        character: char,
    },

    #[error("{kind} '{id}' not found")]
    NotFound { kind: IdKind, id: String },

    #[error("{kind} '{id}' already exists")]
    AlreadyExists { kind: IdKind, id: String },

    #[error("element path is empty")]
    EmptyPath,

    #[error("element path starts at '{node}', which is not a view")]
    PathRootIsNotAView { node: &'static str },

    #[error("element path node '{kind}' / '{id}' is not one this model knows")]
    UnknownPathNode { kind: String, id: String },

    #[error("element path has '{node}' where a control or template instance was expected")]
    UnexpectedPathElement { node: &'static str },

    #[error("component '{component}' has no {member_type} member '{member}'")]
    MemberNotFound {
        component: String,
        member: String,
        member_type: &'static str,
    },

    #[error("export '{id}' is a {expected}, used here as a {actual}")]
    ExportMemberTypeMismatch {
        id: String,
        expected: &'static str,
        actual: &'static str,
    },

    #[error("export '{export}' has type '{expected}', bound to '{actual}'")]
    BindingTypeMismatch {
        export: String,
        expected: String,
        actual: String,
    },

    #[error("binding on export '{export}' has no target")]
    BindingWithoutTarget { export: String },

    /// Binding to an export of the enclosing template only means something
    /// inside a template. In a window there is no enclosing template to export
    /// anything.
    #[error("export '{export}' can only be bound to an enclosing template from inside a template")]
    OwnerExportOutsideTemplate { export: String },

    /// The `(Some(component), None)` pair the wire format allows and the model
    /// does not. Unreachable through the operations; a hand-edited project file
    /// is the only way in.
    #[error("{context} names component '{component}' but no member")]
    ComponentWithoutMember {
        context: &'static str,
        component: String,
    },
}

impl UiModelError {
    pub fn not_found(kind: IdKind, id: impl ToString) -> Self {
        Self::NotFound {
            kind,
            id: id.to_string(),
        }
    }

    pub fn already_exists(kind: IdKind, id: impl ToString) -> Self {
        Self::AlreadyExists {
            kind,
            id: id.to_string(),
        }
    }
}

impl IdError for UiModelError {
    type Kind = IdKind;

    fn not_found(kind: IdKind, id: &str) -> Self {
        Self::not_found(kind, id)
    }

    fn already_exists(kind: IdKind, id: &str) -> Self {
        Self::already_exists(kind, id)
    }
}
