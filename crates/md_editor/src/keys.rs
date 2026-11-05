//! Keyboard navigation (plan task 3.3). Owner: task 3.3.
//!
//! Key bindings use the "MdEditor" key context that `Editor::render` sets. The pure motion
//! logic lives in [`move_offset`] and has no GPUI dependency.

#![allow(dead_code, unused_variables)]

use gpui::{actions, App, Context, Div, Entity};

use crate::editor::Editor;

actions!(
    md_editor_keys,
    [
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        MoveWordLeft,
        MoveWordRight,
        MoveLineStart,
        MoveLineEnd,
        MoveDocStart,
        MoveDocEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectWordLeft,
        SelectWordRight,
        SelectLineStart,
        SelectLineEnd,
        SelectDocStart,
        SelectDocEnd,
    ]
);

/// A caret motion. Each action above maps to one of these plus an `extend` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
}

/// Result of a motion: the new offset, and the goal column to keep for the next vertical move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    pub offset: usize,
    pub goal_column: Option<usize>,
}

/// Where `motion` takes the caret from `offset` in `text`.
///
/// Pure function, unit-tested in 3.3. Left and Right move by one character (not one byte).
/// Up and Down keep `goal_column` (bytes from the line start) when it is given, and set it from
/// the current column otherwise. Word motion uses Unicode word boundaries. Line and document
/// motion stop at `\n` boundaries and the text ends.
pub fn move_offset(text: &str, offset: usize, motion: Motion, goal_column: Option<usize>) -> Moved {
    todo!("3.3")
}

impl Editor {
    /// Move the caret by `motion`, or move the selection head when `extend` is true.
    /// Called by the action handlers that [`attach`] wires (3.3).
    pub fn move_caret(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        todo!("3.3")
    }
}

/// Attach the movement, copy, cut and paste action handlers to the editor's root element.
///
/// Called from `Editor::render` (HOOK 3.3). The skeleton returns `root` unchanged. 3.3 adds
/// one `.on_action(...)` per action and dispatches to `Editor::move_caret` and the clipboard
/// methods.
pub fn attach(root: Div, editor: &Entity<Editor>) -> Div {
    root
}

/// Register the default key bindings for the "MdEditor" key context.
///
/// Called from `editor::init` (HOOK 3.3). The skeleton binds nothing.
pub fn bind_keys(cx: &mut App) {}
