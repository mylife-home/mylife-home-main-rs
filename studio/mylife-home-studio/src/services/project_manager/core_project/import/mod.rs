//! Taking plugins and components in from somewhere else: what is running, or
//! another project.
//!
//! An import is three steps. The changes say what differs, and the designer
//! picks which ones it wants. The updates are what each accepted change
//! actually does, including everything it drags along. Applying them writes
//! the picked subset.
//!
//! The middle step is why this is not a diff: removing a plugin takes its
//! components, which takes their bindings and the template exports pointing at
//! them. That cascade is computed once, before the user is asked, so what they
//! accept is what happens.

mod apply;
mod diff;
mod load;
mod update;

pub use apply::apply_updates;
pub use diff::prepare_changes;
pub use load::{
    ImportData, OnlineComponent, OnlineSource, from_online, from_project, plugin_from_metadata,
};
pub use update::{Update, build_updates};
