use thiserror::Error;

use crate::services::project_manager::model::IdError;

use super::ids::IdKind;

/// Every way a core project operation can fail.
///
/// The variants carry their parts rather than a formatted string, so a caller
/// can match on the failure and the actor layer decides what the wire protocol
/// should say about it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CoreModelError {
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

    #[error("{kind} '{id}' is in use")]
    InUse { kind: IdKind, id: String },

    #[error("component '{component}' has no {member_type} member '{member}'")]
    MemberNotFound {
        component: String,
        member: String,
        member_type: &'static str,
    },

    #[error("plugin id '{id}' is not 'instance:module.name'")]
    MalformedPluginId { id: String },

    #[error("an external component carries no configuration")]
    ComponentHasNoConfig,

    #[error("config '{name}' is supplied by a template export and cannot be set here")]
    ConfigNotSettable { name: String },

    #[error("config '{name}' does not exist on '{owner}'")]
    ConfigNotFound { owner: String, name: String },

    #[error("config '{name}' on '{owner}' expects a {expected}")]
    ConfigValueMismatch {
        owner: String,
        name: String,
        expected: String,
    },

    #[error("'{value}' is not an export kind")]
    UnknownExportKind { value: String },

    #[error("'{value}' is not a {kind}")]
    UnknownValue { kind: &'static str, value: String },

    #[error("a binding cannot have the same component on both ends")]
    BindingOnSelf,

    #[error("cannot bind a '{source_type}' state to a '{target_type}' action")]
    BindingMemberMismatch {
        source_type: String,
        target_type: String,
    },

    #[error("component '{component}' is used by template export '{export}'")]
    ComponentExported { component: String, export: String },

    #[error("config '{config_name}' of component '{component}' is already exported as '{export}'")]
    ConfigAlreadyExported {
        component: String,
        config_name: String,
        export: String,
    },

    #[error("a component using template '{id}' here would make the template contain itself")]
    CircularTemplate { id: String },

    #[error("cannot copy a view onto itself")]
    CopyToSelf,

    #[error("cannot import a project into itself")]
    ImportFromSelf,

    #[error("cannot copy component '{id}': {reason}")]
    NotCopyable { id: String, reason: &'static str },

    #[error(
        "the operation would generate duplicate component ids:
{report}"
    )]
    DuplicateGeneratedIds { report: String },

    #[error("the bindings need an instance to run on, and there is more than one to choose from")]
    BindingsInstanceNeeded,

    #[error("could not build a store item: {0}")]
    StoreItem(String),
}

impl CoreModelError {
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

    pub fn in_use(kind: IdKind, id: impl ToString) -> Self {
        Self::InUse {
            kind,
            id: id.to_string(),
        }
    }
}

impl IdError for CoreModelError {
    type Kind = IdKind;

    fn not_found(kind: IdKind, id: &str) -> Self {
        Self::not_found(kind, id)
    }

    fn already_exists(kind: IdKind, id: &str) -> Self {
        Self::already_exists(kind, id)
    }
}
