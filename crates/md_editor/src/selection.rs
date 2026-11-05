//! Selection state and the edit primitive that keeps it valid (plan task 3.2). Owner: task 3.2.
//!
//! Offsets are byte offsets into the buffer text. `anchor` is the fixed end of the selection
//! and `head` is the moving end, which is also the caret. `anchor == head` means a plain caret.

use std::ops::Range;

use gpui::{point, size, Bounds, Context, Window};
use md_core::buffer::Buffer;
use md_core::Edit;

use crate::editor::Editor;
use crate::element::LayoutSnapshot;
use crate::theme::Theme;

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
        self.anchor.min(self.head)..self.anchor.max(self.head)
    }

    /// True when the head is before the anchor.
    pub fn is_reversed(&self) -> bool {
        self.head < self.anchor
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
    /// Horizontal position, in pixels from the row's text start, to keep across Up and Down
    /// moves between visual rows (task: soft wrap). Cleared like `goal_column`.
    pub goal_x: Option<f32>,
    /// True between mouse down and mouse up while a drag selection is in progress.
    pub dragging: bool,
}

/// Map a selection through one edit that replaced `old` with `new_len` bytes.
///
/// Pure function, unit-tested in 3.2. Offsets before `old.start` stay, offsets after `old.end`
/// shift by the size change, and offsets inside `old` snap to its end.
pub fn map_through_edit(selection: Selection, old: Range<usize>, new_len: usize) -> Selection {
    let map = |offset: usize| -> usize {
        if offset <= old.start {
            offset
        } else if offset < old.end {
            old.start + new_len
        } else {
            offset - old.len() + new_len
        }
    };
    Selection {
        anchor: map(selection.anchor),
        head: map(selection.head),
    }
}

/// Apply one replacement to `buffer` as a single edit and return the caret that follows it:
/// the end of the inserted text. Pure (no GPUI), so the edit path is unit-tested directly.
pub(crate) fn apply_replace(buffer: &mut Buffer, range: Range<usize>, text: &str) -> Selection {
    let inserted_end = range.start + text.len();
    buffer.edit(Edit {
        range,
        text: text.to_string(),
    });
    Selection::caret(inserted_end)
}

impl Editor {
    /// Replace the bytes in `range` with `text` as one buffer edit. The caret moves to the end
    /// of the inserted text. This is the only path that changes the buffer from input code
    /// (keys, clipboard and IME all call it).
    pub fn replace_range(&mut self, range: Range<usize>, text: &str, cx: &mut Context<Self>) {
        let before = self.buffer.version();
        self.selection.selection = apply_replace(&mut self.buffer, range.clone(), text);
        // The block list follows the edit by splitting only the blocks it touched (10.4).
        self.blocks.note_edit(before, &self.buffer, range, text);
        self.selection.goal_column = None;
        self.selection.goal_x = None;
        cx.notify();
    }

    /// Replace the current selection with `text`. Used for typing and paste.
    pub fn replace_selection(&mut self, text: &str, cx: &mut Context<Self>) {
        let range = self.selection.selection.range();
        self.replace_range(range, text, cx);
    }

    /// The current selection. The app reads the caret to keep it across a reload from disk (9.2).
    pub fn current_selection(&self) -> Selection {
        self.selection.selection
    }

    /// Set the selection. `anchor` stays where it is when `extend` is used from keys and
    /// shift-click; the caller passes the full new value. Offsets are clamped to the buffer.
    /// The goal column is cleared; a vertical move sets it again after this call.
    pub fn set_selection(&mut self, selection: Selection, cx: &mut Context<Self>) {
        let len = self.buffer.len();
        self.selection.selection = Selection {
            anchor: selection.anchor.min(len),
            head: selection.head.min(len),
        };
        self.selection.goal_column = None;
        self.selection.goal_x = None;
        cx.notify();
    }
}

/// Coverage of a read-only block (table, image) by the selection.
///
/// A caret alone never covers a block, and a selection that only touches the block's edge
/// does not cover it either. Any other overlap highlights the whole block: `Full` when the
/// selection covers the block's source range end to end, `Partial` otherwise. Pure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockCoverage {
    None,
    Partial,
    Full,
}

/// How much of the block at `block` the `selection` covers. Pure, unit-tested.
///
/// `selection` is the editor selection (caret-aware: `Selection::caret` gives `None`), and
/// `block` is the block's source byte range. Edge-touching does not count: a selection that
/// ends exactly at `block.start` or starts exactly at `block.end` is `None`.
pub fn block_coverage(selection: Selection, block: Range<usize>) -> BlockCoverage {
    if selection.is_empty() {
        return BlockCoverage::None;
    }
    let selected = selection.range();
    if selected.end <= block.start || selected.start >= block.end {
        return BlockCoverage::None;
    }
    if selected.start <= block.start && selected.end >= block.end {
        BlockCoverage::Full
    } else {
        BlockCoverage::Partial
    }
}

/// Width of the highlight drawn for a selected line break.
const NEWLINE_WIDTH: f32 = 0.4;

/// Paint selection rectangles behind the text.
///
/// Called by `TextElement::paint` before the text is painted (3.1 makes the call). Each line
/// that the selection touches gets one rectangle over the selected characters. A line whose
/// line break is selected also gets a short rectangle past its end.
pub fn paint_selection(
    editor: &Editor,
    layout: &LayoutSnapshot,
    theme: &Theme,
    window: &mut Window,
) {
    let selection = editor.selection.selection;
    if selection.is_empty() {
        return;
    }
    let range = selection.range();
    let color = theme.selection;
    let newline_width = layout.line_height * NEWLINE_WIDTH;

    for line in &layout.lines {
        let start = range.start.max(line.source.start);
        let end = range.end.min(line.source.end);
        // Only the last row of a source line has a line break after it. The end of a wrapped row
        // is not a break, so the selection continues on the next row.
        let includes_newline =
            line.last_row && range.start <= line.source.end && range.end > line.source.end;
        if start > end || (start == end && !includes_newline) {
            continue;
        }
        let x0 = line.x_for_source(start);
        let x1 = if includes_newline {
            line.shaped.width + newline_width
        } else {
            line.x_for_source(end)
        };
        let origin = point(line.origin.x + x0, line.origin.y);
        let bounds = Bounds::new(origin, size(x1 - x0, layout.line_height));
        window.paint_quad(gpui::fill(bounds, color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(anchor: usize, head: usize) -> Selection {
        Selection { anchor, head }
    }

    #[test]
    fn map_through_edit_before_the_edit_is_unchanged() {
        // Replace 5..8 with 2 bytes. Offsets up to and including 5 stay.
        assert_eq!(map_through_edit(sel(0, 3), 5..8, 2), sel(0, 3));
        assert_eq!(map_through_edit(sel(2, 5), 5..8, 2), sel(2, 5));
    }

    #[test]
    fn map_through_edit_after_the_edit_shifts_by_the_size_change() {
        // Replace 5..8 (3 bytes) with 2 bytes: shift by -1.
        assert_eq!(map_through_edit(sel(9, 12), 5..8, 2), sel(8, 11));
        // The end of the edit maps to the end of the inserted text.
        assert_eq!(map_through_edit(sel(8, 8), 5..8, 2), sel(7, 7));
        // Growing: replace 5..6 (1 byte) with 4 bytes: shift by +3.
        assert_eq!(map_through_edit(sel(6, 10), 5..6, 4), sel(9, 13));
    }

    #[test]
    fn map_through_edit_inside_the_edit_snaps_to_its_new_end() {
        // Replace 5..8 with 2 bytes. Offsets 6 and 7 are inside and snap to 5 + 2 = 7.
        assert_eq!(map_through_edit(sel(6, 7), 5..8, 2), sel(7, 7));
        // A selection that starts before and ends inside keeps its anchor.
        assert_eq!(map_through_edit(sel(3, 6), 5..8, 2), sel(3, 7));
    }

    #[test]
    fn map_through_edit_pure_insert_and_delete() {
        // Insert 3 bytes at 4. A caret at 4 stays before the text, one at 5 moves.
        assert_eq!(map_through_edit(sel(4, 4), 4..4, 3), sel(4, 4));
        assert_eq!(map_through_edit(sel(5, 5), 4..4, 3), sel(8, 8));
        // Delete 2..6. A caret at 4 snaps to 2, one at 7 moves to 3.
        assert_eq!(map_through_edit(sel(4, 4), 2..6, 0), sel(2, 2));
        assert_eq!(map_through_edit(sel(7, 7), 2..6, 0), sel(3, 3));
    }

    #[test]
    fn range_of_forward_and_reversed_selections() {
        let forward = sel(2, 7);
        assert_eq!(forward.range(), 2..7);
        assert!(!forward.is_reversed());

        let reversed = sel(7, 2);
        assert_eq!(reversed.range(), 2..7);
        assert!(reversed.is_reversed());

        assert_eq!(Selection::caret(4).range(), 4..4);
        assert!(Selection::caret(4).is_empty());
    }

    #[test]
    fn replace_keeps_the_caret_after_the_inserted_text() {
        let mut buffer = Buffer::new("hello world");
        // Replace "world" (6..11) with "there, friend".
        let caret = apply_replace(&mut buffer, 6..11, "there, friend");
        assert_eq!(buffer.text(), "hello there, friend");
        assert_eq!(caret, Selection::caret(19));
        assert_eq!(&buffer.text()[..caret.head], "hello there, friend");
    }

    #[test]
    fn replace_at_a_caret_inserts_and_moves_past_it() {
        let mut buffer = Buffer::new("ab");
        let caret = apply_replace(&mut buffer, 1..1, "XY");
        assert_eq!(buffer.text(), "aXYb");
        assert_eq!(caret, Selection::caret(3));
    }

    #[test]
    fn replace_with_empty_text_deletes_and_collapses() {
        let mut buffer = Buffer::new("abcdef");
        let caret = apply_replace(&mut buffer, 2..5, "");
        assert_eq!(buffer.text(), "abf");
        assert_eq!(caret, Selection::caret(2));
    }

    #[test]
    fn replace_is_one_undo_step() {
        let mut buffer = Buffer::new("abc");
        apply_replace(&mut buffer, 1..2, "XYZ");
        assert_eq!(buffer.text(), "aXYZc");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "abc");
    }

    #[test]
    fn block_coverage_caret_alone_is_none() {
        // A caret never highlights a block, even when it sits inside it.
        assert_eq!(
            block_coverage(Selection::caret(5), 0..10),
            BlockCoverage::None
        );
        assert_eq!(
            block_coverage(Selection::caret(0), 0..10),
            BlockCoverage::None
        );
    }

    #[test]
    fn block_coverage_edge_touching_is_none() {
        // A selection that ends where the block starts, or starts where it ends, misses it.
        assert_eq!(block_coverage(sel(0, 10), 10..20), BlockCoverage::None);
        assert_eq!(block_coverage(sel(20, 30), 10..20), BlockCoverage::None);
        assert_eq!(block_coverage(sel(10, 10), 10..20), BlockCoverage::None);
    }

    #[test]
    fn block_coverage_full_needs_the_whole_source_range() {
        assert_eq!(block_coverage(sel(10, 20), 10..20), BlockCoverage::Full);
        assert_eq!(
            block_coverage(sel(5, 25), 10..20),
            BlockCoverage::Full,
            "a wider selection still fully covers"
        );
        assert_eq!(
            block_coverage(sel(20, 10), 10..20),
            BlockCoverage::Full,
            "reversed selections normalize first"
        );
    }

    #[test]
    fn block_coverage_any_overlap_short_of_full_is_partial() {
        // The block still highlights as a whole; the variant only records the overlap.
        assert_eq!(block_coverage(sel(5, 15), 10..20), BlockCoverage::Partial);
        assert_eq!(block_coverage(sel(15, 25), 10..20), BlockCoverage::Partial);
        assert_eq!(
            block_coverage(sel(12, 18), 10..20),
            BlockCoverage::Partial,
            "a selection inside the block highlights it too"
        );
        assert_eq!(block_coverage(sel(18, 12), 10..20), BlockCoverage::Partial);
    }
}
