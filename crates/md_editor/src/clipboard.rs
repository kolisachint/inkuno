//! Clipboard copy, cut and paste (plan task 3.3). Owner: task 3.3.
//!
//! Copy puts the Markdown source of the selection on the clipboard, unchanged. Paste inserts
//! plain text through `Editor::replace_selection` (3.2).
//!
//! Key bindings for these actions are in `keys.rs` (`bind_keys`); `keys::attach` dispatches
//! them to the methods below.

use std::ops::Range;

use gpui::{actions, ClipboardItem, Context};

use crate::editor::Editor;

actions!(md_editor_clipboard, [Copy, Cut, Paste]);

/// The clipboard text for `range` of `text`: the Markdown source bytes, unchanged.
/// Returns `None` for an empty range. Pure function, unit-tested in 3.3.
pub fn copy_text(text: &str, range: Range<usize>) -> Option<String> {
    if range.is_empty() {
        return None;
    }
    text.get(range).map(str::to_string)
}

/// Normalise pasted text: convert `\r\n` and `\r` to `\n`. Pure function, unit-tested in 3.3.
pub fn normalize_paste(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

impl Editor {
    /// Copy the selected source text to the system clipboard. Does nothing for a caret.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let range = self.selection.selection.range();
        let text = self.text();
        if let Some(copied) = copy_text(&text, range) {
            cx.write_to_clipboard(ClipboardItem::new_string(copied));
        }
    }

    /// Copy the selected source text, then delete it through `replace_range`.
    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        let range = self.selection.selection.range();
        if range.is_empty() {
            return;
        }
        self.copy_selection(cx);
        self.replace_range(range, "", cx);
    }

    /// Insert the clipboard's plain text at the selection through `replace_selection`.
    /// Does nothing when the clipboard has no text.
    pub fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(clipboard) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let text = normalize_paste(&clipboard);
        if text.is_empty() {
            return;
        }
        self.replace_selection(&text, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_text_returns_the_source_bytes() {
        let text = "Some **bold** text";
        assert_eq!(copy_text(text, 5..13).as_deref(), Some("**bold**"));
        assert_eq!(copy_text(text, 0..text.len()).as_deref(), Some(text));
    }

    #[test]
    fn copy_text_keeps_multibyte_text_and_rejects_an_empty_range() {
        let text = "日本語 and emoji 😀";
        assert_eq!(copy_text(text, 0..9).as_deref(), Some("日本語"));
        assert_eq!(copy_text(text, 3..3), None);
        assert_eq!(copy_text("", 0..0), None);
        // A range that splits a character is not valid text, so nothing is copied.
        assert_eq!(copy_text(text, 1..4), None);
    }

    #[test]
    fn normalize_paste_converts_crlf_and_cr_to_lf() {
        assert_eq!(normalize_paste("a\r\nb\rc\nd"), "a\nb\nc\nd");
        assert_eq!(normalize_paste("\r\n\r\n"), "\n\n");
        assert_eq!(normalize_paste("plain"), "plain");
    }

    #[test]
    fn copy_then_paste_round_trips_plain_text() {
        // Copy the selection, then normalize it as paste does. Plain text comes back unchanged.
        let text = "one\r\ntwo words";
        let copied = copy_text(text, 0..text.len()).unwrap();
        assert_eq!(normalize_paste(&copied), "one\ntwo words");
        let fragment = copy_text("abc def", 4..7).unwrap();
        assert_eq!(normalize_paste(&fragment), "def");
    }
}
