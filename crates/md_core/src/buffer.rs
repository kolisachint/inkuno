//! Backend: ropey (Zed text crate deferred; see DESIGN §16).
//! Buffer, transactions, undo and redo (task 2.1).
//! Anchors that track positions through edits (task 2.2).
//! Coordinate conversion between offsets and points (task 2.2).

use crate::Edit;
use ropey::Rope;
use std::ops::Range;

/// One applied primitive edit, as seen by anchors: the replaced range in the text as it was
/// just before the edit, and the length of the text that replaced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Change {
    pub(crate) range: Range<usize>,
    pub(crate) new_len: usize,
}

/// Undo record for one primitive edit: at `start`, `old` was replaced by `new`.
#[derive(Debug, Clone)]
struct Record {
    start: usize,
    old: String,
    new: String,
}

/// A rope-backed text buffer with grouped transactions, undo/redo and an edit history that
/// anchors replay. All offsets are byte offsets and must fall on char boundaries.
///
/// `version` increases by one for every primitive edit, including the edits that undo and
/// redo apply.
#[derive(Debug, Clone)]
pub struct Buffer {
    rope: Rope,
    version: u64,
    /// `history[i]` is the edit that produced version `i + 1`.
    history: Vec<Change>,
    undo_stack: Vec<Vec<Record>>,
    redo_stack: Vec<Vec<Record>>,
    /// Edits recorded since the outermost open transaction.
    open: Vec<Record>,
    transaction_depth: usize,
}

impl Buffer {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            version: 0,
            history: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            open: Vec::new(),
            transaction_depth: 0,
        }
    }

    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.rope.len_bytes()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    /// Text in the byte range. Panics like [`Buffer::edit`] on a bad range.
    pub fn slice(&self, range: Range<usize>) -> String {
        self.check_range(&range);
        self.rope.byte_slice(range).to_string()
    }

    /// Apply one edit. Panics if the range is out of bounds or not on char boundaries.
    /// If no transaction is open, the edit is its own undo step. Clears the redo stack.
    pub fn edit(&mut self, edit: Edit) {
        let start = edit.range.start;
        let old = self.replace_raw(edit.range, &edit.text);
        let record = Record {
            start,
            old,
            new: edit.text,
        };
        self.redo_stack.clear();
        if self.transaction_depth == 0 {
            self.undo_stack.push(vec![record]);
        } else {
            self.open.push(record);
        }
    }

    /// Open a transaction. Transactions nest; only the outermost `end_transaction` closes the
    /// undo step.
    pub fn start_transaction(&mut self) {
        self.transaction_depth += 1;
    }

    /// Close the innermost transaction. When the outermost one closes, all edits made since
    /// then become one undo step (unless there were none). Panics without a matching start.
    pub fn end_transaction(&mut self) {
        assert!(
            self.transaction_depth > 0,
            "end_transaction called without a matching start_transaction"
        );
        self.transaction_depth -= 1;
        if self.transaction_depth == 0 {
            let group = std::mem::take(&mut self.open);
            if !group.is_empty() {
                self.undo_stack.push(group);
            }
        }
    }

    /// Run `f` inside a transaction.
    pub fn transact(&mut self, f: impl FnOnce(&mut Self)) {
        self.start_transaction();
        f(self);
        self.end_transaction();
    }

    /// Undo the most recent step. Returns false if there is nothing to undo.
    /// Panics if a transaction is open.
    pub fn undo(&mut self) -> bool {
        self.assert_no_open_transaction("undo");
        let Some(group) = self.undo_stack.pop() else {
            return false;
        };
        for record in group.iter().rev() {
            let end = record.start + record.new.len();
            self.replace_raw(record.start..end, &record.old);
        }
        self.redo_stack.push(group);
        true
    }

    /// Redo the most recently undone step. Returns false if there is nothing to redo.
    /// Panics if a transaction is open.
    pub fn redo(&mut self) -> bool {
        self.assert_no_open_transaction("redo");
        let Some(group) = self.redo_stack.pop() else {
            return false;
        };
        for record in &group {
            let end = record.start + record.old.len();
            self.replace_raw(record.start..end, &record.new);
        }
        self.undo_stack.push(group);
        true
    }

    /// All edits applied since `version`, in order, as `(old_range, new_len)`. `old_range` is
    /// in the text as it was just before that edit. Panics if `version` is in the future.
    pub fn edits_since(&self, version: u64) -> Vec<(Range<usize>, usize)> {
        self.changes_since(version)
            .iter()
            .map(|c| (c.range.clone(), c.new_len))
            .collect()
    }

    pub(crate) fn changes_since(&self, version: u64) -> &[Change] {
        assert!(
            version <= self.version,
            "version {version} is newer than the buffer's version {}",
            self.version
        );
        &self.history[version as usize..]
    }

    /// Replace `range` with `text` in the rope and record it in the history. Does not touch
    /// the undo stacks. Returns the replaced text.
    fn replace_raw(&mut self, range: Range<usize>, text: &str) -> String {
        self.check_range(&range);
        let old = self.rope.byte_slice(range.clone()).to_string();
        let start_char = self.rope.byte_to_char(range.start);
        let end_char = self.rope.byte_to_char(range.end);
        self.rope.remove(start_char..end_char);
        self.rope.insert(start_char, text);
        self.version += 1;
        self.history.push(Change {
            range,
            new_len: text.len(),
        });
        old
    }

    fn check_range(&self, range: &Range<usize>) {
        assert!(
            range.start <= range.end && range.end <= self.len(),
            "range {range:?} is out of bounds for a buffer of length {}",
            self.len()
        );
        assert!(
            self.is_char_boundary(range.start) && self.is_char_boundary(range.end),
            "range {range:?} is not on char boundaries"
        );
    }

    /// True if `offset` is at most `len()` and not inside a multi-byte character.
    pub(crate) fn is_char_boundary(&self, offset: usize) -> bool {
        if offset > self.len() {
            return false;
        }
        offset == self.len() || (self.rope.byte(offset) & 0xC0) != 0x80
    }

    pub(crate) fn rope_bytes(&self) -> impl Iterator<Item = u8> + '_ {
        self.rope.bytes()
    }

    fn assert_no_open_transaction(&self, operation: &str) {
        assert_eq!(
            self.transaction_depth, 0,
            "{operation} called while a transaction is open"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTI: &str = "héllo 日本語 🎉";

    #[test]
    fn new_and_text_round_trip() {
        let buffer = Buffer::new("abc\ndef");
        assert_eq!(buffer.text(), "abc\ndef");
        assert_eq!(buffer.len(), 7);
        assert!(!buffer.is_empty());
        assert_eq!(buffer.version(), 0);
    }

    #[test]
    fn empty_buffer() {
        let mut buffer = Buffer::new("");
        assert_eq!(buffer.text(), "");
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.slice(0..0), "");
        assert!(!buffer.undo());
        assert!(!buffer.redo());
        assert_eq!(buffer.version(), 0);

        buffer.edit(Edit::insert(0, "x"));
        assert_eq!(buffer.text(), "x");
        assert!(!buffer.is_empty());
        assert!(buffer.undo());
        assert!(buffer.is_empty());
    }

    #[test]
    fn insert_delete_and_replace() {
        let mut buffer = Buffer::new("hello world");

        buffer.edit(Edit::insert(5, ","));
        assert_eq!(buffer.text(), "hello, world");

        buffer.edit(Edit::delete(5..6));
        assert_eq!(buffer.text(), "hello world");

        buffer.edit(Edit::replace(6..11, "there"));
        assert_eq!(buffer.text(), "hello there");

        buffer.edit(Edit::insert(0, "> "));
        buffer.edit(Edit::insert(buffer.len(), "!"));
        assert_eq!(buffer.text(), "> hello there!");
        assert_eq!(buffer.version(), 5);
    }

    #[test]
    fn slice_returns_text_in_range() {
        let buffer = Buffer::new("hello world");
        assert_eq!(buffer.slice(0..5), "hello");
        assert_eq!(buffer.slice(6..11), "world");
        assert_eq!(buffer.slice(3..3), "");
    }

    #[test]
    fn multibyte_edits() {
        let mut buffer = Buffer::new(MULTI);
        // "héllo" (6) + space (1) + "日本語" (9) + space (1) + "🎉" (4)
        assert_eq!(buffer.len(), 21);

        // Delete 日 (bytes 7..10).
        assert_eq!(buffer.slice(7..10), "日");
        buffer.edit(Edit::delete(7..10));
        assert_eq!(buffer.text(), "héllo 本語 🎉");

        // Replace 🎉 with a short ASCII word.
        let len = buffer.len();
        buffer.edit(Edit::replace(len - 4..len, "ok"));
        assert_eq!(buffer.text(), "héllo 本語 ok");

        // Insert between two-byte and three-byte characters.
        buffer.edit(Edit::insert(7, "·"));
        assert_eq!(buffer.text(), "héllo ·本語 ok");
        assert!(buffer.undo());
        assert!(buffer.undo());
        assert!(buffer.undo());
        assert_eq!(buffer.text(), MULTI);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn edit_past_end_panics() {
        let mut buffer = Buffer::new("abc");
        buffer.edit(Edit::delete(2..4));
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn reversed_range_panics() {
        let buffer = Buffer::new("abc");
        let reversed = Range { start: 2, end: 1 };
        buffer.slice(reversed);
    }

    #[test]
    #[should_panic(expected = "char boundaries")]
    fn edit_inside_char_panics() {
        let mut buffer = Buffer::new("é");
        buffer.edit(Edit::insert(1, "x"));
    }

    #[test]
    #[should_panic(expected = "char boundaries")]
    fn slice_inside_char_panics() {
        let buffer = Buffer::new("日本");
        buffer.slice(0..1);
    }

    #[test]
    fn each_edit_is_its_own_undo_step_without_transaction() {
        let mut buffer = Buffer::new("a");
        buffer.edit(Edit::insert(1, "b"));
        buffer.edit(Edit::insert(2, "c"));
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "ab");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "a");
        assert!(!buffer.undo());
    }

    #[test]
    fn transaction_groups_edits_into_one_undo_step() {
        let mut buffer = Buffer::new("hello");
        buffer.start_transaction();
        buffer.edit(Edit::insert(5, " world"));
        buffer.edit(Edit::replace(0..1, "J"));
        buffer.edit(Edit::insert(0, ">"));
        buffer.end_transaction();
        assert_eq!(buffer.text(), ">Jello world");

        assert!(buffer.undo());
        assert_eq!(buffer.text(), "hello");
        assert!(!buffer.undo());

        assert!(buffer.redo());
        assert_eq!(buffer.text(), ">Jello world");
        assert!(!buffer.redo());
    }

    #[test]
    fn nested_transactions_form_one_step() {
        let mut buffer = Buffer::new("x");
        buffer.start_transaction();
        buffer.edit(Edit::insert(1, "1"));
        buffer.start_transaction();
        buffer.edit(Edit::insert(2, "2"));
        buffer.end_transaction();
        // Inner end does not close the outer transaction.
        buffer.edit(Edit::insert(3, "3"));
        buffer.end_transaction();
        assert_eq!(buffer.text(), "x123");

        buffer.edit(Edit::insert(4, "!"));
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "x123");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "x");
        assert!(!buffer.undo());
    }

    #[test]
    fn transact_runs_closure_as_one_step() {
        let mut buffer = Buffer::new("");
        buffer.transact(|b| {
            b.edit(Edit::insert(0, "a"));
            b.edit(Edit::insert(1, "b"));
        });
        assert_eq!(buffer.text(), "ab");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "");
        assert!(!buffer.undo());
    }

    #[test]
    fn empty_transaction_is_not_an_undo_step() {
        let mut buffer = Buffer::new("a");
        buffer.transact(|_| {});
        assert!(!buffer.undo());
    }

    #[test]
    #[should_panic(expected = "without a matching")]
    fn end_without_start_panics() {
        Buffer::new("").end_transaction();
    }

    #[test]
    #[should_panic(expected = "transaction is open")]
    fn undo_inside_transaction_panics() {
        let mut buffer = Buffer::new("a");
        buffer.edit(Edit::insert(1, "b"));
        buffer.start_transaction();
        buffer.undo();
    }

    #[test]
    fn undo_then_redo_restores_exact_text_at_every_step() {
        let mut buffer = Buffer::new(MULTI);
        let mut states = vec![buffer.text()];

        buffer.edit(Edit::insert(0, "¡"));
        states.push(buffer.text());
        buffer.transact(|b| {
            b.edit(Edit::delete(3..6));
            b.edit(Edit::replace(0..2, "🙂🙂"));
        });
        states.push(buffer.text());
        buffer.edit(Edit::insert(buffer.len(), "\n\n"));
        states.push(buffer.text());

        let final_text = buffer.text();
        for expected in states.iter().rev().skip(1) {
            assert!(buffer.undo());
            assert_eq!(&buffer.text(), expected);
        }
        assert!(!buffer.undo());
        assert_eq!(buffer.text(), MULTI);

        for expected in states.iter().skip(1) {
            assert!(buffer.redo());
            assert_eq!(&buffer.text(), expected);
        }
        assert!(!buffer.redo());
        assert_eq!(buffer.text(), final_text);
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut buffer = Buffer::new("a");
        buffer.edit(Edit::insert(1, "b"));
        assert!(buffer.undo());
        buffer.edit(Edit::insert(1, "c"));
        assert!(!buffer.redo());
        assert_eq!(buffer.text(), "ac");
    }

    #[test]
    fn undo_and_redo_increase_version() {
        let mut buffer = Buffer::new("a");
        buffer.edit(Edit::insert(1, "b"));
        assert_eq!(buffer.version(), 1);
        assert!(buffer.undo());
        assert_eq!(buffer.version(), 2);
        assert!(buffer.redo());
        assert_eq!(buffer.version(), 3);
        assert!(!buffer.redo());
        assert_eq!(buffer.version(), 3);
    }

    #[test]
    fn edits_since_lists_edits_in_order_including_undo_and_redo() {
        let mut buffer = Buffer::new("abc");
        buffer.edit(Edit::insert(1, "XY")); // "aXYbc", version 1
        buffer.edit(Edit::delete(0..1)); // "XYbc", version 2
        assert_eq!(buffer.edits_since(0), vec![(1..1, 2), (0..1, 0)]);
        assert_eq!(buffer.edits_since(1), vec![(0..1, 0)]);
        assert!(buffer.edits_since(2).is_empty());

        assert!(buffer.undo()); // reinserts "a" at 0: range 0..0, new_len 1
        assert_eq!(buffer.edits_since(2), vec![(0..0, 1)]);
    }

    #[test]
    #[should_panic(expected = "newer than")]
    fn edits_since_future_version_panics() {
        Buffer::new("").edits_since(1);
    }
}
