//! Core editing model for noteapp. No GPUI dependency.

pub mod anchor;
pub mod block;
pub mod buffer;
pub mod coords;
pub mod display_map;
pub mod inline;
pub mod input_rules;
pub mod parse;
pub mod types;

pub use types::*;
