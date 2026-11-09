//! Pure functions: they read and return values, never touch the disk, a window or gpui.
//! Everything here is unit-tested in its own module.

pub mod block;
pub mod commands;
pub mod daily;
pub mod diagram;
pub mod formula;
pub mod history;
pub mod layout;
pub mod markdown;
pub mod merge;
pub mod mermaid;
pub mod note;
pub mod prefs;
pub mod rank;
pub mod recall;
pub mod sharing;
pub mod table;
pub mod wiki;
