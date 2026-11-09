//! Canvas stub: the diagram editor was removed (owner-approved cut).
//! This module stays only because `views::viewer` and `shell::keymap`
//! (owned by other crews) still refer to these actions. The integrator
//! removes this file together with those references.
//!
//! REPORT for integrator: delete this file and remove
//! - `src/views/viewer.rs:25` (`use crate::views::canvas::{...}`), the `ToCanvas`
//!   event (`ViewerEvent::ToCanvas`, `edit_on_canvas`), its `.on_action` handlers
//!   for ZoomIn/ZoomOut/ZoomFit/Left/Right/Up/Down/Cancel, the `key_context("Canvas")`,
//!   and the "Edit on the canvas" button,
//! - `src/shell/keymap.rs` canvas rows and bindings (see trim-canvas report),
//! - `src/views/mod.rs` (`pub mod canvas`),
//! - `src/shell/mod.rs` (`canvas` in the views import + the `#[allow]` note),
//! - `src/shell/render.rs` (`canvas::Cancel` Escape handler).

use gpui::actions;

actions!(
    canvas,
    [
        ZoomIn, ZoomOut, ZoomFit, Erase, EraseNext, Duplicate, SelectAll, Undo, Redo, Confirm,
        Cancel, Left, Right, Up, Down, SelectUp, SelectDown, Paste, NextText
    ]
);
