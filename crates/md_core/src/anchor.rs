//! Anchors that track positions through edits (task 2.2).

use crate::buffer::{Buffer, Change};

/// Which side an anchor sticks to when text is inserted exactly at its offset.
/// `Left` stays before the inserted text; `Right` moves after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    Left,
    Right,
}

/// A position that follows its text through later edits.
///
/// `offset` is the position when the anchor was created, at buffer `version`. Resolve it with
/// [`Buffer::resolve`] to get its current position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    pub offset: usize,
    pub bias: Bias,
    pub version: u64,
}

impl Buffer {
    /// Create an anchor at `offset`. Panics if `offset` is out of bounds or not on a char
    /// boundary.
    pub fn anchor(&self, offset: usize, bias: Bias) -> Anchor {
        assert!(
            self.is_char_boundary(offset),
            "anchor offset {offset} is out of bounds or not on a char boundary"
        );
        Anchor {
            offset,
            bias,
            version: self.version(),
        }
    }

    /// Current offset of `anchor`, found by replaying every edit made since it was created.
    ///
    /// For each edit that replaces `start..end` with `n` bytes:
    /// - an offset before `start` is unchanged;
    /// - an offset after `end` shifts by `n - (end - start)`;
    /// - an offset strictly inside `start..end` moves to `start`;
    /// - an offset equal to `start` or `end` follows the bias, as if the range had been deleted
    ///   and the new text inserted at `start`: `Left` stays at `start`, `Right` moves to
    ///   `start + n`.
    ///
    /// Panics if the anchor's version is newer than this buffer's.
    pub fn resolve(&self, anchor: &Anchor) -> usize {
        self.changes_since(anchor.version)
            .iter()
            .fold(anchor.offset, |offset, change| {
                transform(offset, anchor.bias, change)
            })
    }
}

fn transform(offset: usize, bias: Bias, change: &Change) -> usize {
    let start = change.range.start;
    let end = change.range.end;
    if offset < start {
        offset
    } else if offset > end {
        offset - (end - start) + change.new_len
    } else if offset > start && offset < end {
        start
    } else {
        match bias {
            Bias::Left => start,
            Bias::Right => start + change.new_len,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Edit;

    #[test]
    fn no_edits_resolves_to_original_offset() {
        let buffer = Buffer::new("hello");
        let anchor = buffer.anchor(3, Bias::Left);
        assert_eq!(buffer.resolve(&anchor), 3);
        assert_eq!(anchor.version, 0);
    }

    #[test]
    fn anchor_survives_edit_before_it() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(6, Bias::Left);
        buffer.edit(Edit::insert(0, "XX"));
        let offset = buffer.resolve(&anchor);
        assert_eq!(offset, 8);
        assert_eq!(buffer.slice(offset..buffer.len()), "world");
    }

    #[test]
    fn anchor_survives_deletion_before_it() {
        let mut buffer = Buffer::new("one two three");
        let anchor = buffer.anchor(8, Bias::Right);
        buffer.edit(Edit::delete(0..4));
        let offset = buffer.resolve(&anchor);
        assert_eq!(buffer.slice(offset..offset + 5), "three");
    }

    #[test]
    fn anchor_unaffected_by_edit_after_it() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(6, Bias::Right);
        buffer.edit(Edit::insert(11, "!!"));
        buffer.edit(Edit::delete(7..9));
        assert_eq!(buffer.resolve(&anchor), 6);
        assert_eq!(buffer.text(), "hello wld!!");
    }

    #[test]
    fn insertion_at_anchor_respects_left_bias() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(6, Bias::Left);
        buffer.edit(Edit::insert(6, "_"));
        assert_eq!(buffer.resolve(&anchor), 6);
    }

    #[test]
    fn insertion_at_anchor_respects_right_bias() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(6, Bias::Right);
        buffer.edit(Edit::insert(6, "_"));
        let offset = buffer.resolve(&anchor);
        assert_eq!(offset, 7);
        assert_eq!(buffer.slice(offset..buffer.len()), "world");
    }

    #[test]
    fn insertion_at_end_of_text_respects_bias() {
        let mut buffer = Buffer::new("abc");
        let left = buffer.anchor(3, Bias::Left);
        let right = buffer.anchor(3, Bias::Right);
        buffer.edit(Edit::insert(3, "de"));
        assert_eq!(buffer.resolve(&left), 3);
        assert_eq!(buffer.resolve(&right), 5);
    }

    #[test]
    fn anchor_inside_deleted_range_moves_to_deletion_start() {
        let mut buffer = Buffer::new("hello world");
        let left = buffer.anchor(6, Bias::Left);
        let right = buffer.anchor(6, Bias::Right);
        buffer.edit(Edit::delete(4..9));
        assert_eq!(buffer.resolve(&left), 4);
        assert_eq!(buffer.resolve(&right), 4);
        assert_eq!(buffer.text(), "hellld");
    }

    #[test]
    fn anchor_at_end_of_deleted_range_moves_to_deletion_start() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(9, Bias::Left);
        buffer.edit(Edit::delete(4..9));
        assert_eq!(buffer.resolve(&anchor), 4);
    }

    #[test]
    fn anchor_inside_replaced_range_moves_to_range_start() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(7, Bias::Right);
        buffer.edit(Edit::replace(4..9, "XYZ"));
        assert_eq!(buffer.resolve(&anchor), 4);
    }

    #[test]
    fn anchor_at_start_of_replaced_range_follows_bias() {
        let mut buffer = Buffer::new("hello world");
        let left = buffer.anchor(6, Bias::Left);
        let right = buffer.anchor(6, Bias::Right);
        buffer.edit(Edit::replace(6..11, "there!"));
        assert_eq!(buffer.resolve(&left), 6);
        assert_eq!(buffer.resolve(&right), 12);
    }

    #[test]
    fn anchor_after_replaced_range_shifts_by_length_difference() {
        let mut buffer = Buffer::new("abcdef");
        let anchor = buffer.anchor(5, Bias::Left);
        buffer.edit(Edit::replace(1..3, "XXXXX")); // +3 bytes
        let offset = buffer.resolve(&anchor);
        assert_eq!(offset, 8);
        assert_eq!(buffer.slice(offset..offset + 1), "f");
    }

    #[test]
    fn anchor_follows_text_across_many_edits() {
        let mut buffer = Buffer::new("0123456789AB");
        let anchor = buffer.anchor(10, Bias::Left); // at "A"
        buffer.edit(Edit::insert(0, "xx"));
        buffer.edit(Edit::delete(2..5));
        buffer.transact(|b| {
            b.edit(Edit::replace(0..1, "😀"));
            b.edit(Edit::insert(b.len(), "tail"));
        });
        let offset = buffer.resolve(&anchor);
        assert_eq!(buffer.slice(offset..offset + 1), "A");
    }

    #[test]
    fn anchor_created_after_edits_uses_current_version() {
        let mut buffer = Buffer::new("abc");
        buffer.edit(Edit::insert(0, "zz"));
        let anchor = buffer.anchor(4, Bias::Left);
        assert_eq!(anchor.version, buffer.version());
        buffer.edit(Edit::insert(0, "q"));
        assert_eq!(buffer.resolve(&anchor), 5);
    }

    #[test]
    fn anchor_is_restored_by_undo() {
        let mut buffer = Buffer::new("hello world");
        let anchor = buffer.anchor(8, Bias::Left); // at 'r'
        buffer.edit(Edit::delete(0..6));
        assert_eq!(buffer.resolve(&anchor), 2);
        assert!(buffer.undo());
        assert_eq!(buffer.resolve(&anchor), 8);
        assert!(buffer.redo());
        assert_eq!(buffer.resolve(&anchor), 2);
    }

    #[test]
    fn anchors_track_multibyte_text() {
        let mut buffer = Buffer::new("héllo 日本語 🎉");
        let anchor = buffer.anchor(7, Bias::Left); // at 日
        buffer.edit(Edit::insert(0, "🎉🎉"));
        let offset = buffer.resolve(&anchor);
        assert_eq!(offset, 15);
        assert_eq!(buffer.slice(offset..offset + 3), "日");
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn anchor_past_end_panics() {
        Buffer::new("abc").anchor(4, Bias::Left);
    }

    #[test]
    #[should_panic(expected = "char boundary")]
    fn anchor_inside_char_panics() {
        Buffer::new("é").anchor(1, Bias::Right);
    }

    #[test]
    #[should_panic(expected = "newer than")]
    fn resolving_anchor_from_newer_version_panics() {
        let mut other = Buffer::new("abc");
        other.edit(Edit::insert(0, "x"));
        let anchor = other.anchor(0, Bias::Left);
        Buffer::new("abc").resolve(&anchor);
    }
}
