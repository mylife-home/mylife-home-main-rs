//! Infrastructure shared by the core and UI project models.
//!
//! Each project owns its ids, its id kinds and its error type: the two domains
//! validate ids differently and share almost no failure. What they do share is
//! the shape of a project, which is a map of validated ids to shared values,
//! mutated by operations that return a new map instead of editing in place.
//!
//! That shape is what lives here. `Collection` is the map, and `Id` and
//! `IdError` are the two traits it is written against, so it can report
//! "window 'x' not found" in the right project's error type without knowing
//! what a window is.

mod collection;
mod ids;

pub use collection::Collection;
pub use ids::{Id, IdError};

/// A `macro_rules!` is crate-visible at most, so this re-export cannot be
/// `pub` the way the items above are.
pub(crate) use ids::define_id;
