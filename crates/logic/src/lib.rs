//! The pure logic of Inkuno: it reads and returns values, never touches the disk, a
//! window or gpui. Everything here is unit-tested in its own module.
//!
//! Moved verbatim out of the app's `logic/` layer (trim-survivor files only); the app keeps
//! shim modules that re-export these, so no call site changed.

pub mod block;
pub mod commands;
pub mod daily;
pub mod history;
pub mod lang;
pub mod layout;
pub mod markdown;
pub mod merge;
pub mod note;
pub mod prefs;
pub mod rank;
pub mod recall;
pub mod wiki;
