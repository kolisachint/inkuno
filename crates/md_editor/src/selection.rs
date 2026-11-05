//! Selection state and the edit primitive that keeps it valid (plan task 3.2). Owner: task 3.2.
//!
//! Offsets are byte offsets into the buffer text. `anchor` is the fixed end of the selection
//! and `head` is the moving end, which is also the caret. `anchor == head` means a plain caret.

#![allow(dead_code, unused_variables)]

use std::ops::Range;

use gpui::{Context, Window};

use crate::editor::Editor;
use crate::element::LayoutSnapshot;

/// A selection in source byte offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    /// A collapsed selection (a caret) at `offset`.
    pub fn caret(offset: usize) -> Self {
        Self {
            anchor: offset,
            head: offset,
        }
    }

    /// True when the selection is a caret.
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// The selected byte range, with start <= end.
    pub fn range(&self) -> Range<usize> {
        todo!("3.2")
    }

    /// True when the head is before the anchor.
    pub fn is_reversed(&self) -> bool {
        todo!("3.2")
    }
}

/// Selection state held by `Editor` (field `selection`).
///
/// 3.2 owns this type. `goal_column` is part of the interface because 3.3 reads and writes it
/// for Up and Down motion. 3.2 must keep the field and its meaning: the column to return to
/// on a vertical move, cleared by any horizontal move or edit.
#[derive(Debug, Clone, Default)]
pub struct SelectionState {
    pub selection: Selection,
    /// Column to keep across vertical moves, in bytes from the start of the line.
    pub goal_column: Option<usize>,
    /// True between mouse down and mouse up while a drag selection is in progress.
    pub dragging: bool,
}

/// Map a selection through one edit that replaced `old` with `new_len` bytes.
///
/// Pure function, unit-tested in 3.2. Offsets before `old.start` stay, offsets after `old.end`
/// shift by the size change, and offsets inside `old` snap to its end.
pub fn map_through_edit(selection: Selection, old: Range<usize>, new_len: usize) -> Selection {
    todo!("3.2")
}

impl Editor {
    /// Replace the bytes in `range` with `text` as one buffer edit. The caret moves to the end
    /// of the inserted text. This is the only path that changes the buffer from input code
    /// (keys, clipboard and IME all call it).
    pub fn replace_range(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        todo!("3.2")
    }

    /// Replace the current selection with `text`. Used for typing and paste.
    pub fn replace_selection(&mut self, text: &str, cx: &mut Context<Self>) {
        todo!("3.2")
    }

    /// Set the selection. `anchor` stays where it is when `extend` is used from keys and
    /// shift-click; the caller passes the full new value.
    pub fn set_selection(&mut self, selection: Selection, cx: &mut Context<Self>) {
        todo!("3.2")
    }
}

/// Paint selection rectangles behind the text.
///
/// Called by `TextElement::paint` before the text is painted (3.1 makes the call). The skeleton
/// body is a no-op so that 3.1 can call it before 3.2 exists.
pub fn paint_selection(editor: &Editor, layout: &LayoutSnapshot, window: &mut Window) {}
