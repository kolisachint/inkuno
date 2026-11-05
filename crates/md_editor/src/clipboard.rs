//! Clipboard copy, cut and paste (plan task 3.3). Owner: task 3.3.
//!
//! Copy puts the Markdown source of the selection on the clipboard, unchanged. Paste inserts
//! plain text through `Editor::replace_selection` (3.2).

#![allow(dead_code, unused_variables)]

use std::ops::Range;

use gpui::{actions, Context};

use crate::editor::Editor;

actions!(md_editor_clipboard, [Copy, Cut, Paste]);

/// The clipboard text for `range` of `text`: the Markdown source bytes, unchanged.
/// Returns `None` for an empty range. Pure function, unit-tested in 3.3.
pub fn copy_text(text: &str, range: Range<usize>) -> Option<String> {
    todo!("3.3")
}

/// Normalise pasted text: convert `\r\n` and `\r` to `\n`. Pure function, unit-tested in 3.3.
pub fn normalize_paste(text: &str) -> String {
    todo!("3.3")
}

impl Editor {
    /// Copy the selected source text to the system clipboard. Does nothing for a caret.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        todo!("3.3")
    }

    /// Copy the selected source text, then delete it through `replace_range`.
    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        todo!("3.3")
    }

    /// Insert the clipboard's plain text at the selection through `replace_selection`.
    pub fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        todo!("3.3")
    }
}
