use std::sync::Arc;

use super::control::{Action, ControlDisplay, ControlText};
use super::ids::ResourceId;
use super::types::StyleList;

/// A field a patch either leaves alone or sets: the model side of
/// `studio_web_api::utils::PartialField`.
///
/// Kept as its own type rather than `Option<T>` because several of the fields
/// below are `Patch<Option<T>>`, where "not mentioned" and "explicitly cleared"
/// are different requests.
#[derive(Debug, Clone, Default)]
pub enum Patch<T> {
    #[default]
    Unchanged,
    Set(T),
}

impl<T> Patch<T> {
    pub fn is_unchanged(&self) -> bool {
        matches!(self, Patch::Unchanged)
    }

    /// Writes into `target` and reports whether it did. Non-short-circuiting
    /// `|=` over a series of these is how the `set_properties` operations
    /// decide whether anything moved.
    pub fn apply_to(self, target: &mut T) -> bool {
        match self {
            Patch::Unchanged => false,
            Patch::Set(value) => {
                *target = value;
                true
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct WindowProperties {
    pub height: Patch<i32>,
    pub width: Patch<i32>,
    /// A `StyleList` is sorted by its constructor, so there is no normalize
    /// step for a caller to forget.
    pub style: Patch<StyleList>,
    pub background_resource: Patch<Option<ResourceId>>,
}

#[derive(Debug, Clone, Default)]
pub struct TemplateProperties {
    pub height: Patch<i32>,
    pub width: Patch<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct ControlProperties {
    pub style: Patch<StyleList>,
    pub height: Patch<i32>,
    pub width: Patch<i32>,
    pub x: Patch<i32>,
    pub y: Patch<i32>,
    pub display: Patch<Option<Arc<ControlDisplay>>>,
    pub text: Patch<Option<Arc<ControlText>>>,
    pub primary_action: Patch<Option<Arc<Action>>>,
    pub secondary_action: Patch<Option<Arc<Action>>>,
}
