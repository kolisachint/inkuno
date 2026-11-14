//! Pure functions: they read and return values, never touch the disk, a window or gpui.
//! Everything here is unit-tested in its own module.
//!
//! The trim-survivor modules moved to the `inkuno-logic` crate and are re-exported here,
//! so every `crate::logic::…` path keeps working.

// Moved to `inkuno-logic` (English-only, no gpui): shims, do not add code here.
pub use inkuno_logic::block;
pub use inkuno_logic::commands;
pub use inkuno_logic::daily;
pub use inkuno_logic::layout;
pub use inkuno_logic::markdown;
pub use inkuno_logic::merge;
pub use inkuno_logic::note;
pub use inkuno_logic::prefs;
pub use inkuno_logic::rank;
pub use inkuno_logic::recall;
pub use inkuno_logic::revision;
pub use inkuno_logic::scratch;
pub use inkuno_logic::wiki;

pub mod mermaid;
