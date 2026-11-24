//! Tests for the core project.
//!
//! They run against the internal API: the model operations and the wire
//! conversions, called directly. Nothing here spawns an actor or touches the
//! filesystem.
//!
//! Three things are asserted about almost every operation, because each has
//! its own way of going wrong: what it returns, including `None` for "nothing
//! changed" and the right error for a refusal; what it emitted, since the core
//! designer is told about individual components and bindings and a missing
//! change leaves it showing a project that no longer exists; and what the
//! resulting project holds, including the parts the operation was not aimed
//! at.

mod bindings;
mod components;
mod copy;
mod fixture;
mod ids;
mod import_ops;
mod naming;
mod plugins;
mod templates;
mod wire;
