//! Tests for the UI project.
//!
//! They run against the internal API: the model operations, the wire
//! conversions and the deploy pass, called directly. Nothing here spawns an
//! actor or touches the filesystem, so a test is a function call and an
//! assertion on what came back.
//!
//! Three things are asserted about almost every operation, because each of the
//! three has its own way of going wrong:
//!
//! * what it returns, including `None` for "nothing changed" and the right
//!   error for a refusal;
//! * what it emitted, since a change that is not emitted leaves the designer
//!   looking at a project that no longer exists;
//! * what the resulting project holds, including the parts the operation was
//!   not aimed at.

mod components;
mod controls;
mod deploy;
mod fixture;
mod ids;
mod instances;
mod paths;
mod resources;
mod styles;
mod templates;
mod windows;
mod wire;
