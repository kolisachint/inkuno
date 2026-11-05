//! Editing keys: Backspace, Delete, Enter, Tab, word delete, undo and redo, and select all
//! (plan task 3.7). Owner: task 3.7.
//!
//! Every text change goes through `Editor::replace_range` (3.2), except undo and redo, which
//! use the md_core `Buffer` history. The pure range logic lives in [`backward_delete_range`],
//! [`forward_delete_range`], [`word_delete_range`] and [`caret_after_history`] and has no GPUI
//! dependency.
//!
//! Bindings are platform-specific, as in `keys.rs`: macOS uses `cmd-` and `alt-`, Linux uses
//! `ctrl-`. Backspace, Delete, Enter and Tab are the same on both platforms.

use std::ops::Range;

use gpui::{actions, App, Context, Div, Entity, KeyBinding};

use crate::editor::Editor;
use crate::keys::{self, Motion};
use crate::selection::Selection;

actions!(
    md_editor_edit,
    [
        DeleteBackward,
        DeleteForward,
        DeleteWordBackward,
        InsertNewline,
        InsertTab,
        Undo,
        Redo,
        SelectAll,
    ]
);

/// Text inserted by Tab. The codebase has no indentation convention yet, so Tab inserts four
/// spaces (not a tab character).
pub const TAB_TEXT: &str = "    ";

/// Text inserted by Enter.
pub const NEWLINE_TEXT: &str = "\n";

/// The byte range that Backspace deletes: the selection when there is one, otherwise the one
/// character before the caret. Empty at the start of the text. Pure, unit-tested in 3.7.
pub fn backward_delete_range(text: &str, selection: Selection) -> Range<usize> {
    if !selection.is_empty() {
        return selection.range();
    }
    let caret = selection.head;
    keys::move_offset(text, caret, Motion::Left, None).offset..caret
}

/// The byte range that forward Delete removes: the selection when there is one, otherwise the
/// one character after the caret. Empty at the end of the text. Pure, unit-tested in 3.7.
pub fn forward_delete_range(text: &str, selection: Selection) -> Range<usize> {
    if !selection.is_empty() {
        return selection.range();
    }
    let caret = selection.head;
    caret..keys::move_offset(text, caret, Motion::Right, None).offset
}

/// The byte range that word delete removes: the selection when there is one, otherwise the
/// text back to the start of the previous word (the same stop as Option or Ctrl+Left).
/// Pure, unit-tested in 3.7.
pub fn word_delete_range(text: &str, selection: Selection) -> Range<usize> {
    if !selection.is_empty() {
        return selection.range();
    }
    let caret = selection.head;
    keys::move_offset(text, caret, Motion::WordLeft, None).offset..caret
}

/// The caret after an undo or redo, given the edits it applied (`(old_range, new_len)` as
/// `Buffer::edits_since` reports them). The caret goes to the end of the text the last edit
/// wrote, which is where the user is looking after the step. Pure, unit-tested in 3.7.
pub fn caret_after_history(changes: &[(Range<usize>, usize)]) -> usize {
    changes
        .last()
        .map_or(0, |(range, new_len)| range.start + new_len)
}

impl Editor {
    /// Delete `range` as one edit. An empty range is a no-op, so no empty undo step is made.
    fn delete_range(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        if !range.is_empty() {
            self.replace_range(range, "", cx);
        }
    }

    /// Backspace: delete the selection, or the character before the caret.
    pub fn delete_backward(&mut self, cx: &mut Context<Self>) {
        let range = backward_delete_range(&self.text(), self.selection.selection);
        self.delete_range(range, cx);
    }

    /// Delete: delete the selection, or the character after the caret.
    pub fn delete_forward(&mut self, cx: &mut Context<Self>) {
        let range = forward_delete_range(&self.text(), self.selection.selection);
        self.delete_range(range, cx);
    }

    /// Option+Backspace on macOS, Ctrl+Backspace on Linux: delete the selection, or the word
    /// before the caret.
    pub fn delete_word_backward(&mut self, cx: &mut Context<Self>) {
        let range = word_delete_range(&self.text(), self.selection.selection);
        self.delete_range(range, cx);
    }

    /// Enter: replace the selection with a newline.
    pub fn insert_newline(&mut self, cx: &mut Context<Self>) {
        self.replace_selection(NEWLINE_TEXT, cx);
    }

    /// Tab: replace the selection with [`TAB_TEXT`].
    pub fn insert_tab(&mut self, cx: &mut Context<Self>) {
        self.replace_selection(TAB_TEXT, cx);
    }

    /// Select the whole buffer. The head goes to the end, as after a Shift+End.
    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        let len = self.buffer.len();
        self.set_selection(
            Selection {
                anchor: 0,
                head: len,
            },
            cx,
        );
    }

    /// Undo the last step in the buffer history, then put a caret where the step was.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        self.step_history(false, cx);
    }

    /// Redo the last undone step, then put a caret where the step was.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        self.step_history(true, cx);
    }

    /// Shared body of undo and redo. The caret is found from the edits the step applied.
    fn step_history(&mut self, redo: bool, cx: &mut Context<Self>) {
        let version = self.buffer.version();
        let applied = if redo {
            self.buffer.redo()
        } else {
            self.buffer.undo()
        };
        if !applied {
            return;
        }
        let caret = caret_after_history(&self.buffer.edits_since(version));
        // set_selection clamps the offset and clears the goal column.
        self.set_selection(Selection::caret(caret), cx);
    }
}

/// Wire the editing actions to the editor's root element.
///
/// Called from `Editor::render` (HOOK 3.7). Each action calls one `Editor` method above.
pub fn attach(root: Div, editor: &Entity<Editor>) -> Div {
    let root = keys::on::<DeleteBackward>(root, editor, |e, cx| e.delete_backward(cx));
    let root = keys::on::<DeleteForward>(root, editor, |e, cx| e.delete_forward(cx));
    let root = keys::on::<DeleteWordBackward>(root, editor, |e, cx| e.delete_word_backward(cx));
    let root = keys::on::<InsertNewline>(root, editor, |e, cx| e.insert_newline(cx));
    let root = keys::on::<InsertTab>(root, editor, |e, cx| e.insert_tab(cx));
    let root = keys::on::<Undo>(root, editor, |e, cx| e.undo(cx));
    let root = keys::on::<Redo>(root, editor, |e, cx| e.redo(cx));
    keys::on::<SelectAll>(root, editor, |e, cx| e.select_all(cx))
}

/// Register the editing key bindings for the "MdEditor" key context.
///
/// Called from `editor::init` (HOOK 3.7). Linux uses Ctrl (and Ctrl+Y for redo); macOS uses
/// Cmd for undo, redo and select all, and Option for word delete.
pub fn bind_keys(cx: &mut App) {
    let ctx = Some("MdEditor");
    let mut bindings = vec![
        KeyBinding::new("backspace", DeleteBackward, ctx),
        KeyBinding::new("delete", DeleteForward, ctx),
        KeyBinding::new("enter", InsertNewline, ctx),
        KeyBinding::new("tab", InsertTab, ctx),
    ];
    #[cfg(target_os = "macos")]
    bindings.extend([
        KeyBinding::new("alt-backspace", DeleteWordBackward, ctx),
        KeyBinding::new("cmd-z", Undo, ctx),
        KeyBinding::new("cmd-shift-z", Redo, ctx),
        KeyBinding::new("cmd-a", SelectAll, ctx),
    ]);
    #[cfg(not(target_os = "macos"))]
    bindings.extend([
        KeyBinding::new("ctrl-backspace", DeleteWordBackward, ctx),
        KeyBinding::new("ctrl-z", Undo, ctx),
        KeyBinding::new("ctrl-shift-z", Redo, ctx),
        KeyBinding::new("ctrl-y", Redo, ctx),
        KeyBinding::new("ctrl-a", SelectAll, ctx),
    ]);
    cx.bind_keys(bindings);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::apply_replace;
    use md_core::buffer::Buffer;

    /// "a" 0, "é" 1..3, "😀" 3..7, "日" 7..10, "b" 10..11.
    const MIXED: &str = "aé😀日b";

    fn caret(offset: usize) -> Selection {
        Selection::caret(offset)
    }

    #[test]
    fn backward_delete_removes_one_character_across_scripts() {
        assert_eq!(backward_delete_range(MIXED, caret(11)), 10..11, "ASCII");
        assert_eq!(backward_delete_range(MIXED, caret(10)), 7..10, "CJK");
        assert_eq!(backward_delete_range(MIXED, caret(7)), 3..7, "emoji");
        assert_eq!(backward_delete_range(MIXED, caret(3)), 1..3, "accented");
        assert_eq!(backward_delete_range(MIXED, caret(1)), 0..1, "ASCII again");
        assert_eq!(
            backward_delete_range(MIXED, caret(0)),
            0..0,
            "start of text"
        );
    }

    #[test]
    fn forward_delete_removes_one_character_across_scripts() {
        assert_eq!(forward_delete_range(MIXED, caret(0)), 0..1, "ASCII");
        assert_eq!(forward_delete_range(MIXED, caret(1)), 1..3, "accented");
        assert_eq!(forward_delete_range(MIXED, caret(3)), 3..7, "emoji");
        assert_eq!(forward_delete_range(MIXED, caret(7)), 7..10, "CJK");
        assert_eq!(
            forward_delete_range(MIXED, caret(10)),
            10..11,
            "ASCII again"
        );
        assert_eq!(
            forward_delete_range(MIXED, caret(11)),
            11..11,
            "end of text"
        );
    }

    #[test]
    fn a_caret_is_never_split_inside_a_character() {
        // Every range starts and ends on a char boundary, so the buffer edit never panics.
        let mut buffer = Buffer::new(MIXED);
        for offset in [11, 10, 7, 3, 1, 0] {
            let range = backward_delete_range(MIXED, caret(offset));
            assert!(MIXED.is_char_boundary(range.start));
            assert!(MIXED.is_char_boundary(range.end));
        }
        // Delete the emoji with a real edit to make sure the range is usable.
        apply_replace(&mut buffer, 3..7, "");
        assert_eq!(buffer.text(), "aé日b");
    }

    #[test]
    fn selection_is_deleted_by_both_keys() {
        // A selection is removed whole, whichever key asks for it.
        let selection = Selection { anchor: 7, head: 3 };
        assert_eq!(backward_delete_range(MIXED, selection), 3..7);
        assert_eq!(forward_delete_range(MIXED, selection), 3..7);
        assert_eq!(word_delete_range(MIXED, selection), 3..7);
    }

    #[test]
    fn deleting_a_selection_in_the_buffer_leaves_the_caret_at_the_cut() {
        let mut buffer = Buffer::new("hello world");
        let selection = Selection {
            anchor: 5,
            head: 11,
        };
        let range = backward_delete_range("hello world", selection);
        let after = apply_replace(&mut buffer, range, "");
        assert_eq!(buffer.text(), "hello");
        assert_eq!(after, Selection::caret(5));
    }

    #[test]
    fn typing_over_a_selection_replaces_it_as_one_edit() {
        let mut buffer = Buffer::new("hello world");
        let range = Selection {
            anchor: 6,
            head: 11,
        }
        .range();
        let after = apply_replace(&mut buffer, range, "there");
        assert_eq!(buffer.text(), "hello there");
        assert_eq!(after, Selection::caret(11));
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "hello world");
    }

    #[test]
    fn word_delete_stops_at_word_starts() {
        let text = "hello, big world";
        assert_eq!(word_delete_range(text, caret(16)), 11..16, "world");
        assert_eq!(
            word_delete_range(text, caret(11)),
            7..11,
            "big and its space"
        );
        assert_eq!(word_delete_range(text, caret(0)), 0..0, "start of text");
    }

    #[test]
    fn word_delete_over_multibyte_words() {
        let text = "naïve 日本";
        // "日本" is one word (CJK letters are alphanumeric), so it goes as one run.
        assert_eq!(word_delete_range(text, caret(text.len())), 7..13);
        assert_eq!(word_delete_range(text, caret(6)), 0..6);
    }

    #[test]
    fn undo_puts_the_caret_where_the_deleted_text_came_back() {
        let mut buffer = Buffer::new("abc");
        apply_replace(&mut buffer, 1..2, "");
        let version = buffer.version();
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "abc");
        // The restored "b" sits at 1..2, so the caret goes to its end.
        assert_eq!(caret_after_history(&buffer.edits_since(version)), 2);
    }

    #[test]
    fn undo_of_an_insert_puts_the_caret_at_the_insert_point() {
        let mut buffer = Buffer::new("ab");
        apply_replace(&mut buffer, 1..1, "XYZ");
        let version = buffer.version();
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "ab");
        assert_eq!(caret_after_history(&buffer.edits_since(version)), 1);
    }

    #[test]
    fn redo_puts_the_caret_after_the_reapplied_text() {
        let mut buffer = Buffer::new("ab");
        apply_replace(&mut buffer, 1..1, "XYZ");
        assert!(buffer.undo());
        let version = buffer.version();
        assert!(buffer.redo());
        assert_eq!(buffer.text(), "aXYZb");
        assert_eq!(caret_after_history(&buffer.edits_since(version)), 4);
    }

    #[test]
    fn enter_and_tab_text() {
        assert_eq!(NEWLINE_TEXT, "\n");
        assert_eq!(TAB_TEXT, "    ");
    }
}
