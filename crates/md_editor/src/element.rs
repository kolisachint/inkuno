//! Plain-text element for the editor (plan task 3.1). Owner: task 3.1.
//!
//! `TextElement` draws a byte range of the buffer as plain text, one shaped line per source
//! line. It is a custom GPUI `Element` (DESIGN §7). `Editor::render` creates it for the whole
//! buffer (HOOK 3.1); the block list (3.5) creates one per block.

use std::ops::Range;
use std::panic::Location;

use gpui::{
    point, relative, App, Bounds, ElementId, Entity, GlobalElementId, InspectorElementId,
    IntoElement, LayoutId, Pixels, Point, ShapedLine, SharedString, Style, TextAlign, TextRun,
    Window,
};

use md_core::display_map::DisplayMap;

use crate::cursor::{self, hit_test};
use crate::editor::Editor;
use crate::ime;
use crate::selection;

/// Byte range of each line in `text`, split on `\n`. Ranges exclude the `\n`. A trailing
/// newline gives a final empty range. Pure function, unit-tested in 3.1.
pub fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            ranges.push(start..index);
            start = index + 1;
        }
    }
    ranges.push(start..text.len());
    ranges
}

/// One laid-out source line.
///
/// Written during paint by `TextElement` and read by selection (3.2), hit testing (3.2) and IME
/// bounds (3.4).
#[derive(Clone)]
pub struct LineLayout {
    /// Byte range of the line in the buffer, excluding its `\n`.
    pub source: Range<usize>,
    /// The shaped text of the line.
    pub shaped: ShapedLine,
    /// Top-left corner of the line in window coordinates.
    pub origin: Point<Pixels>,
}

/// Geometry of one painted block: the lines of one `TextElement`.
#[derive(Default, Clone)]
pub struct LayoutSnapshot {
    pub lines: Vec<LineLayout>,
    /// Bounds of the element in window coordinates.
    pub bounds: Bounds<Pixels>,
    /// Height of one line.
    pub line_height: Pixels,
}

/// One painted block. `source` is the block's byte range in the buffer (its trailing newline
/// excluded), the same range its `TextElement` draws.
#[derive(Clone)]
pub struct BlockLayout {
    pub source: Range<usize>,
    pub snapshot: LayoutSnapshot,
}

impl BlockLayout {
    /// The line that contains byte `offset`. A line break belongs to the line before it, so the
    /// offset at the end of a line is found on that line.
    pub fn line_for_offset(&self, offset: usize) -> Option<&LineLayout> {
        self.snapshot
            .lines
            .iter()
            .find(|line| line.source.start <= offset && offset <= line.source.end)
    }
}

/// Layouts of the visible blocks from the most recent frame, held by `Editor` (field
/// `layouts`), in buffer order.
///
/// Every block is painted by its own `TextElement`. `TextElement::paint` inserts its block
/// here, and `BlockHost` clears the map at the start of each frame (3.5). A block scrolled out
/// of view has no entry, so offsets there have no position until they are scrolled back in.
/// Points and offsets are mapped through this map, never through one block's layout.
#[derive(Default, Clone)]
pub struct Layouts {
    blocks: Vec<BlockLayout>,
}

impl Layouts {
    /// Forget every block. Called at the start of each frame.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }

    /// Record a painted block. Blocks are kept in buffer order whatever the paint order is. A
    /// block painted twice in one frame replaces its earlier entry.
    pub fn insert(&mut self, source: Range<usize>, snapshot: LayoutSnapshot) {
        let index = self
            .blocks
            .partition_point(|block| block.source.start < source.start);
        let block = BlockLayout { source, snapshot };
        match self.blocks.get(index) {
            Some(existing) if existing.source.start == block.source.start => {
                self.blocks[index] = block;
            }
            _ => self.blocks.insert(index, block),
        }
    }

    /// The visible block whose source contains byte `offset`, if the block is on screen.
    pub fn block_for_offset(&self, offset: usize) -> Option<&BlockLayout> {
        index_containing(&self.blocks, |block| block.source.clone(), offset)
            .map(|index| &self.blocks[index])
    }

    /// The visible block under window-space height `y`. Above the first block or below the last
    /// one this is that end block, and between two blocks it is the nearer one.
    pub fn block_for_y(&self, y: Pixels) -> Option<&BlockLayout> {
        index_for_y(&self.blocks, f32::from(y)).map(|index| &self.blocks[index])
    }

    /// The source offset under `point` (window coordinates), found in the block under the
    /// point, or in the nearest visible block when the point is between blocks or past them.
    /// Returns 0 when no block is visible.
    pub fn offset_at(&self, display: &DisplayMap, point: Point<Pixels>) -> usize {
        match self.block_for_y(point.y) {
            Some(block) => hit_test(&block.snapshot, display, point),
            None => 0,
        }
    }
}

/// Index of the item whose range contains `offset`. `items` must be in buffer order, with
/// ranges that do not overlap. An offset at the end of an item's range belongs to that item,
/// since it is the position of the line break after it. Pure, unit-tested.
pub(crate) fn index_containing<T>(
    items: &[T],
    range_of: impl Fn(&T) -> Range<usize>,
    offset: usize,
) -> Option<usize> {
    let count = items.partition_point(|item| range_of(item).start <= offset);
    let index = count.checked_sub(1)?;
    (offset <= range_of(&items[index]).end).then_some(index)
}

/// Index of the block at height `y`, for blocks stacked in buffer order. Inside a block this is
/// that block. Above the first block it is 0, below the last it is the last index, and between
/// two blocks it is the nearer one. Pure, unit-tested.
pub(crate) fn index_for_y(blocks: &[BlockLayout], y: f32) -> Option<usize> {
    let top = |block: &BlockLayout| f32::from(block.snapshot.bounds.top());
    let bottom = |block: &BlockLayout| f32::from(block.snapshot.bounds.bottom());
    if blocks.is_empty() {
        return None;
    }
    // The first block whose bottom is below `y`.
    let after = blocks.partition_point(|block| bottom(block) <= y);
    if after == blocks.len() {
        return Some(blocks.len() - 1);
    }
    if top(&blocks[after]) <= y {
        return Some(after);
    }
    if after == 0 {
        return Some(0);
    }
    let previous = after - 1;
    if y - bottom(&blocks[previous]) <= top(&blocks[after]) - y {
        Some(previous)
    } else {
        Some(after)
    }
}

/// Paints `source` (a byte range of the buffer) as plain text.
///
/// Each block (one list item, see `block_list.rs`) gets its own `TextElement`. Paint order
/// within one block:
/// 1. `selection::paint_selection`, for the part of the selection in this block
/// 2. the text lines, each with `ime::decorate_runs` applied to its runs
/// 3. `cursor::paint_caret`, which draws only when the caret is in this block
///
/// Mouse handlers and the input handler are not registered here. `BlockHost` registers them
/// once for the whole list. After the text is painted, the element inserts this block's
/// `LayoutSnapshot` into `Editor::layouts`.
pub struct TextElement {
    editor: Entity<Editor>,
    /// Byte range of the buffer to draw.
    source: Range<usize>,
}

impl TextElement {
    /// An element that draws `source` of the editor's buffer.
    pub fn new(editor: Entity<Editor>, source: Range<usize>) -> Self {
        Self { editor, source }
    }
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = LayoutSnapshot;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let text = self.editor.read(cx).buffer().slice(self.source.clone());
        let line_count = line_ranges(&text).len();
        let line_height = window.line_height();

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (line_height * line_count as f32).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let text = editor.buffer().slice(self.source.clone());
        let marked = editor.ime.marked.clone();

        let style = window.text_style();
        let font = style.font();
        let font_size = style.font_size.to_pixels(window.rem_size());
        let color = style.color;
        let line_height = window.line_height();

        let mut lines = Vec::new();
        let mut top = bounds.top();
        for range in line_ranges(&text) {
            // Line ranges are relative to `text`, which starts at `source.start` in the buffer.
            let source = (self.source.start + range.start)..(self.source.start + range.end);
            let line_text: SharedString = text[range].to_string().into();
            let run = TextRun {
                len: line_text.len(),
                font: font.clone(),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            // Zero-length runs are dropped, so an empty line is shaped with no runs.
            let runs: Vec<TextRun> = ime::decorate_runs(vec![run], source.clone(), marked.clone())
                .into_iter()
                .filter(|run| run.len > 0)
                .collect();
            let shaped = window
                .text_system()
                .shape_line(line_text, font_size, &runs, None);
            lines.push(LineLayout {
                source,
                shaped,
                origin: point(bounds.left(), top),
            });
            top += line_height;
        }

        LayoutSnapshot {
            lines,
            bounds,
            line_height,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let snapshot = std::mem::take(prepaint);
        let editor = self.editor.clone();
        let focus = editor.read(cx).focus_handle.clone();
        let focused = focus.is_focused(window);

        // 1. Selection behind the text, for the part of it that lies in this block.
        selection::paint_selection(editor.read(cx), &snapshot, window);

        // 2. The text, one line at a time.
        for line in &snapshot.lines {
            if let Err(err) = line.shaped.paint(
                line.origin,
                snapshot.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            ) {
                eprintln!("md_editor: failed to paint line {:?}: {err}", line.source);
            }
        }

        // 3. The caret, only when it is in this block.
        cursor::paint_caret(editor.read(cx), &snapshot, focused, window);

        // Keep this block's geometry for hit testing, caret reveal and IME bounds.
        let source = self.source.clone();
        editor.update(cx, |editor, _cx| editor.layouts.insert(source, snapshot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size, Bounds};

    /// A block with no shaped lines whose box is `top..top + height` in window coordinates.
    /// The mapping helpers only read `source` and `bounds`.
    fn block(source: Range<usize>, top: f32, height: f32) -> BlockLayout {
        BlockLayout {
            source,
            snapshot: LayoutSnapshot {
                bounds: Bounds::new(point(px(0.), px(top)), size(px(100.), px(height))),
                ..Default::default()
            },
        }
    }

    #[test]
    fn index_containing_finds_the_item_for_each_offset() {
        // "ab" 0..2, a blank line 3..3, "cd" 4..6, with the line breaks at 2, 3 and 6.
        let items = [0..2, 3..3, 4..6];
        assert_eq!(index_containing(&items, |r| r.clone(), 0), Some(0));
        assert_eq!(
            index_containing(&items, |r| r.clone(), 2),
            Some(0),
            "its line break"
        );
        assert_eq!(
            index_containing(&items, |r| r.clone(), 3),
            Some(1),
            "blank line"
        );
        assert_eq!(index_containing(&items, |r| r.clone(), 4), Some(2));
        assert_eq!(
            index_containing(&items, |r| r.clone(), 6),
            Some(2),
            "end of text"
        );
        assert_eq!(
            index_containing(&items, |r| r.clone(), 7),
            None,
            "past the end"
        );
    }

    #[test]
    fn index_containing_skips_offscreen_items() {
        // Only the items 0..2 and 6..8 are visible; offsets in the hidden 3..5 have no item.
        let visible = [(0..2, 0), (6..8, 3)];
        let range_of = |item: &(Range<usize>, usize)| item.0.clone();
        assert_eq!(index_containing(&visible, range_of, 1), Some(0));
        assert_eq!(index_containing(&visible, range_of, 4), None);
        assert_eq!(index_containing(&visible, range_of, 7), Some(1));
        assert_eq!(
            index_containing::<(Range<usize>, usize)>(&[], range_of, 0),
            None
        );
    }

    #[test]
    fn index_for_y_inside_above_below_and_between_blocks() {
        // Blocks at 0..20, 20..60 and 60..80 (stacked, as the list lays them out).
        let blocks = [
            block(0..2, 0., 20.),
            block(3..5, 20., 40.),
            block(6..9, 60., 20.),
        ];
        assert_eq!(index_for_y(&blocks, 0.), Some(0));
        assert_eq!(index_for_y(&blocks, 19.9), Some(0));
        assert_eq!(index_for_y(&blocks, 20.), Some(1));
        assert_eq!(index_for_y(&blocks, 79.), Some(2));
        // Above the first block and below the last clamp to the end blocks.
        assert_eq!(index_for_y(&blocks, -50.), Some(0));
        assert_eq!(index_for_y(&blocks, 500.), Some(2));
        assert_eq!(index_for_y(&[], 10.), None);
    }

    #[test]
    fn index_for_y_between_blocks_takes_the_nearer_one() {
        // A 10 px gap between 0..20 and 30..50.
        let blocks = [block(0..2, 0., 20.), block(3..5, 30., 20.)];
        assert_eq!(index_for_y(&blocks, 22.), Some(0));
        assert_eq!(
            index_for_y(&blocks, 27.),
            Some(1),
            "closer to the second block"
        );
        assert_eq!(
            index_for_y(&blocks, 25.),
            Some(0),
            "exactly halfway goes up"
        );
    }

    #[test]
    fn layouts_insert_keeps_buffer_order_and_replaces_duplicates() {
        let mut layouts = Layouts::default();
        layouts.insert(4..6, LayoutSnapshot::default());
        layouts.insert(0..2, LayoutSnapshot::default());
        layouts.insert(2..4, LayoutSnapshot::default());
        let starts: Vec<usize> = layouts.blocks.iter().map(|b| b.source.start).collect();
        assert_eq!(starts, vec![0, 2, 4]);
        layouts.insert(2..9, LayoutSnapshot::default());
        assert_eq!(layouts.blocks.len(), 3, "same start replaces the entry");
        assert_eq!(layouts.blocks[1].source, 2..9);
        layouts.clear();
        assert!(layouts.block_for_offset(0).is_none());
    }

    #[test]
    fn line_ranges_of_small_inputs() {
        assert_eq!(line_ranges(""), vec![0..0]);
        assert_eq!(line_ranges("a"), vec![0..1]);
        assert_eq!(line_ranges("a\n"), vec![0..1, 2..2]);
        assert_eq!(line_ranges("a\n\nb"), vec![0..1, 2..2, 3..4]);
    }

    #[test]
    fn line_ranges_use_byte_offsets() {
        // "é" is 2 bytes, "日" is 3 bytes, "🎉" is 4 bytes.
        let text = "é\n日本\n🎉";
        let ranges = line_ranges(text);
        assert_eq!(ranges, vec![0..2, 3..9, 10..14]);
        let lines: Vec<&str> = ranges.iter().map(|r| &text[r.clone()]).collect();
        assert_eq!(lines, vec!["é", "日本", "🎉"]);
    }

    #[test]
    fn line_ranges_of_100_lines() {
        let text: String = (1..=100).map(|i| format!("line {i}\n")).collect();
        let ranges = line_ranges(&text);
        assert_eq!(
            ranges.len(),
            101,
            "a trailing newline gives a final empty line"
        );
        assert_eq!(&text[ranges[0].clone()], "line 1");
        assert_eq!(&text[ranges[99].clone()], "line 100");
        assert_eq!(&text[ranges[100].clone()], "");

        let no_trailing: String = (1..=100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ranges = line_ranges(&no_trailing);
        assert_eq!(ranges.len(), 100);
        for (i, range) in ranges.iter().enumerate() {
            assert_eq!(&no_trailing[range.clone()], format!("line {}", i + 1));
        }
    }
}
