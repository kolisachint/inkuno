//! Virtualized block list (plan task 3.5). Owner: task 3.5.
//!
//! The list is a GPUI `ListState` (`list()` element). Each item draws one top-level md_core
//! block with `TextElement`, so only visible blocks are laid out (DESIGN §3.1).
//!
//! The list count must follow the buffer, but `Editor::render` cannot read the editor (it is
//! leased while it renders). So `render_blocks` returns a small element, [`BlockHost`], that
//! syncs the list in its layout pass, where the editor can be read, and then lays out the list.

use std::cell::Cell;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::panic::Location;
use std::rc::Rc;

use gpui::{
    div, list, px, relative, AnyElement, App, Bounds, Context, ElementId, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ListAlignment, ListOffset, ListState, Pixels, Style,
    Styled, Window,
};
use md_core::block::BlockMap;
use md_core::buffer::Buffer;
use md_core::{Block, BlockKind, Edit};

use crate::cursor;
use crate::editor::Editor;
use crate::element::index_containing;
use crate::ime;
use crate::inline_render::{block_hidden_ranges, document_hidden_ranges, merge_ranges};
use crate::modes::{anchor_for, offset_for, ScrollAnchor};

/// Block list state held by `Editor` (field `blocks`). Owned by 3.5.
pub struct BlockListState {
    /// The GPUI list state. Its item count must match the number of blocks.
    pub list: ListState,
    /// Number of block items laid out in the most recent frame (reset at the start of each
    /// frame). The acceptance counter for 3.5: it stays near the number of visible items, shared
    /// with the list's render closure.
    pub layouts: Rc<Cell<usize>>,
    /// The items the list shows, kept in step with the buffer (10.4).
    pub(crate) index: ItemIndex,
    /// Buffer version and caret offset at the last caret reveal. The list scrolls to the caret
    /// again only when one of them changes, so wheel scrolling away from the caret is kept.
    revealed: Option<(u64, usize)>,
    /// Whether Typewriter Mode's spacer items are in the list: one blank item before the first
    /// block and one after the last (task 8.4). When they are, the list holds two more items
    /// than `items`.
    spacers: bool,
    /// Height of each spacer, in pixels. Zero while `spacers` is off.
    spacer_px: f32,
}

impl BlockListState {
    /// Number of spacer items before the first block: 1 while the spacers are in the list.
    fn lead(&self) -> usize {
        usize::from(self.spacers)
    }

    /// The list item that holds byte `offset` of the buffer, counting the leading spacer, if any
    /// (task 8.4).
    pub(crate) fn item_index_at(&self, offset: usize) -> Option<usize> {
        item_index_for_offset(&self.index.items, offset).map(|ix| ix + self.lead())
    }

    /// The block at the top of the list and its scroll past that top (task 8.2).
    pub(crate) fn top_anchor(&self) -> ScrollAnchor {
        anchor_for(
            self.list.logical_scroll_top(),
            self.lead(),
            self.index.len(),
        )
    }

    /// Bring the list up to date after one buffer edit (10.4). The index splits again only the
    /// blocks the edit touched, and the list gets a splice for those items alone. When the index
    /// cannot take the edit, it is left stale and the next sync rebuilds it.
    ///
    /// `before` is the buffer version before the edit, and `inserted` is the text that replaced
    /// `range`.
    pub(crate) fn note_edit(
        &mut self,
        before: u64,
        buffer: &Buffer,
        range: Range<usize>,
        inserted: &str,
    ) {
        if let Some((old, count)) = self.index.apply_edit(before, buffer, range, inserted) {
            let lead = self.lead();
            self.list.splice(old.start + lead..old.end + lead, count);
        }
    }

    /// Scroll the list so that `anchor` is at the top (task 8.2).
    pub(crate) fn scroll_to_anchor(&self, anchor: ScrollAnchor) {
        self.list.scroll_to(offset_for(anchor, self.lead()));
    }
}

impl Default for BlockListState {
    fn default() -> Self {
        Self {
            list: ListState::new(0, ListAlignment::Top, px(200.)),
            layouts: Rc::new(Cell::new(0)),
            index: ItemIndex::default(),
            revealed: None,
            spacers: false,
            spacer_px: 0.,
        }
    }
}

/// The list items for the buffer text, kept in step with it across edits (10.4). Pure, with no
/// GPUI types, so the incremental path is unit-tested against a full rebuild.
///
/// The items are one per top-level block (see [`item_ranges`]). `map` holds the blocks, and
/// `hashes` and `hidden` run alongside the items:
/// - `hashes` lets a rebuild replace only the items whose kind or text changed;
/// - `hidden` caches each item's hidden ranges for caret movement, relative to the item start.
///   It is filled on demand, and it is dropped for the items an edit changed.
#[doc(hidden)]
#[derive(Default)]
pub struct ItemIndex {
    /// Buffer version the index describes. `None` before the first build.
    version: Option<u64>,
    /// Block map of the text at `version`. `None` before the first build.
    map: Option<BlockMap>,
    /// Whether the text ends with a newline, which adds a final empty item.
    trailing: bool,
    /// Drawn byte range of each list item, its trailing newline excluded.
    items: Vec<Range<usize>>,
    /// Block kind of each item, in the same order as `items` (task 4.1 draws items by kind).
    kinds: Vec<BlockKind>,
    /// Hash of each item's kind and text.
    hashes: Vec<u64>,
    /// Unrevealed hidden ranges of each item, relative to its start. `None` until computed.
    hidden: Vec<Option<Vec<Range<usize>>>>,
}

impl ItemIndex {
    /// Buffer version the index describes, `None` before the first build.
    pub fn version(&self) -> Option<u64> {
        self.version
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True when there are no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Rebuild the index for `text` at `version`, splitting every block. Returns the old items
    /// that differ from the new ones and how many new items replace them, for the list splice.
    pub fn build(&mut self, text: &str, version: u64) -> (Range<usize>, usize) {
        let map = BlockMap::new(text);
        let (items, kinds, hashes) = items_for(text, map.blocks());
        let (old, count) = changed_span(&self.hashes, &hashes);
        self.hidden
            .splice(old.clone(), std::iter::repeat_with(|| None).take(count));
        self.items = items;
        self.kinds = kinds;
        self.hashes = hashes;
        self.trailing = text.ends_with('\n');
        self.map = Some(map);
        self.version = Some(version);
        (old, count)
    }

    /// Update the index for one edit that replaced `range` with `inserted`. The index must be at
    /// `before`, the buffer version before the edit. Only the blocks the edit touched are split
    /// and hashed again.
    ///
    /// Returns the item span that was replaced and how many items replace it, the same shape as
    /// [`ItemIndex::build`]. Returns `None`, and changes nothing, when the index cannot take the
    /// edit: it is stale, the edit replaced the whole text, the text is or becomes empty, or the
    /// trailing newline changed. The caller then rebuilds with `build`.
    pub fn apply_edit(
        &mut self,
        before: u64,
        buffer: &Buffer,
        range: Range<usize>,
        inserted: &str,
    ) -> Option<(Range<usize>, usize)> {
        if self.version != Some(before) {
            return None;
        }
        let map = self.map.as_mut()?;
        let old_len = map.blocks().last()?.range.end;
        if range == (0..old_len) {
            return None;
        }
        let text = buffer.text();
        if text.is_empty() || text.ends_with('\n') != self.trailing {
            return None;
        }
        let edit = Edit {
            range,
            text: inserted.to_string(),
        };
        let (old, count) = map.apply_edit(&edit, &text);
        let items = item_ranges(&text, map.blocks());
        let kinds = item_kinds(&text, map.blocks());
        // Items outside `old` keep their kind and text, so their hashes and hidden ranges stay.
        // The trailing empty item, if any, is after them and keeps its hash too.
        let fresh: Vec<u64> = (old.start..old.start + count)
            .map(|ix| hash_item(&kinds[ix], &text[items[ix].clone()]))
            .collect();
        self.hashes.splice(old.clone(), fresh);
        self.hidden
            .splice(old.clone(), std::iter::repeat_with(|| None).take(count));
        self.items = items;
        self.kinds = kinds;
        self.version = Some(buffer.version());
        Some((old, count))
    }

    /// Hidden byte ranges of the whole document for `caret`, sorted and merged. It equals
    /// [`document_hidden_ranges`] on `text`, which must be the text this index describes. Blocks
    /// the caret is not in reuse their cached ranges, so a caret move costs integer work per item
    /// plus the blocks that changed since the last move.
    pub fn hidden_ranges(&mut self, text: &str, caret: Option<usize>) -> Vec<Range<usize>> {
        let mut hidden = Vec::new();
        for (ix, item) in self.items.iter().enumerate() {
            let kind = &self.kinds[ix];
            if caret.is_some_and(|caret| (item.start..=item.end).contains(&caret)) {
                hidden.extend(block_hidden_ranges(
                    &text[item.clone()],
                    item.start,
                    kind,
                    caret,
                ));
            } else {
                // Without the caret, the ranges do not depend on it, and they move with the block,
                // so they are cached relative to the block start.
                let cached = self.hidden[ix]
                    .get_or_insert_with(|| block_hidden_ranges(&text[item.clone()], 0, kind, None));
                hidden.extend(
                    cached
                        .iter()
                        .map(|r| r.start + item.start..r.end + item.start),
                );
            }
        }
        merge_ranges(hidden)
    }
}

/// Index of the item that holds byte `offset`, for items drawn in buffer order. Pure,
/// unit-tested.
pub(crate) fn item_index_for_offset(items: &[Range<usize>], offset: usize) -> Option<usize> {
    index_containing(items, |item| item.clone(), offset)
}

/// Scroll the list so the block holding the caret is in view, when the caret or the text has
/// changed since the last reveal.
///
/// Called from `BlockHost::prepaint` after the list has laid out its visible items. A block that
/// was laid out in this frame is revealed with `scroll_to_reveal_item`, which needs measured
/// heights. A block beyond the laid-out range (typically pasted text below the screen) has no
/// measured height yet, so it is put at the top of the list instead. The list was already
/// painted this frame, so the editor is notified for the next frame to show the new scroll.
fn reveal_caret(editor: &Entity<Editor>, window: &mut Window, cx: &mut App) {
    let (version, head, revealed) = {
        let editor = editor.read(cx);
        (
            editor.buffer().version(),
            editor.selection.selection.head,
            editor.blocks.revealed,
        )
    };
    if revealed == Some((version, head)) {
        return;
    }
    if let Some(index) = editor.read(cx).blocks.item_index_at(head) {
        let list = editor.read(cx).blocks.list.clone();
        let before = list.logical_scroll_top();
        let laid_out = list.bounds_for_item(index).is_some();
        if laid_out || index < before.item_ix {
            list.scroll_to_reveal_item(index);
        } else {
            list.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: px(0.),
            });
        }
        if list.logical_scroll_top() != before {
            // The list has already been painted this frame, so paint again to show the scroll.
            let entity = editor.entity_id();
            window.on_next_frame(move |_window, cx| cx.notify(entity));
        }
    }
    editor.update(cx, |editor, _cx| {
        editor.blocks.revealed = Some((version, head));
    });
}

/// Top-level md_core blocks for `text`. Pure function, unit-tested in 3.5. 3.5 may cache the
/// result, for example with `md_core::block::BlockMap`.
pub fn blocks_for(text: &str) -> Vec<Block> {
    md_core::block::split_blocks(text)
}

/// Drawn byte range of each list item for `blocks`, which must tile `text`.
///
/// A block's trailing newline is left out, so each item's line count is the number of lines in
/// the block. Together the items give the same lines as `line_ranges(text)`.
fn item_ranges(text: &str, blocks: &[Block]) -> Vec<Range<usize>> {
    if blocks.is_empty() {
        return std::iter::once(0..0).collect();
    }
    let mut items: Vec<Range<usize>> = blocks
        .iter()
        .map(|block| {
            let mut end = block.range.end;
            if end > block.range.start && text.as_bytes()[end - 1] == b'\n' {
                end -= 1;
            }
            block.range.start..end
        })
        .collect();
    if text.ends_with('\n') {
        items.push(text.len()..text.len());
    }
    items
}

/// Block kind of each item, in the same order as [`item_ranges`]. The extra item for a trailing
/// newline is `Blank`. Pure, unit-tested.
fn item_kinds(text: &str, blocks: &[Block]) -> Vec<BlockKind> {
    if blocks.is_empty() {
        return vec![BlockKind::Blank];
    }
    let mut kinds: Vec<BlockKind> = blocks.iter().map(|block| block.kind).collect();
    if text.ends_with('\n') {
        kinds.push(BlockKind::Blank);
    }
    kinds
}

/// The old items to replace and how many new items to put in their place, given the item
/// hashes before and after an edit. Items that match at the start and at the end are kept.
fn changed_span(old: &[u64], new: &[u64]) -> (Range<usize>, usize) {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let max_suffix = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    (prefix..old.len() - suffix, new.len() - suffix - prefix)
}

/// Hash of one item. The kind is part of it, because a change of neighbouring text can change
/// the kind of a block whose own text is the same.
fn hash_item(kind: &BlockKind, text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}

/// Bring the list items up to date with the buffer. Does nothing when the buffer version is
/// unchanged, so a re-render without edits does not re-split the document.
fn sync(editor: &Entity<Editor>, cx: &mut App) {
    let version = editor.read(cx).buffer().version();
    if editor.read(cx).blocks.index.version() != Some(version) {
        sync_items(editor, version, cx);
    }
    let want = editor.read(cx).modes().typewriter;
    if editor.read(cx).blocks.spacers != want {
        set_spacers(editor, want, cx);
    }
}

/// Add or remove the Typewriter Mode spacer items at both ends of the list (task 8.4). Called
/// after the block items are up to date, so `items` gives the count to splice around.
fn set_spacers(editor: &Entity<Editor>, on: bool, cx: &mut App) {
    editor.update(cx, |editor, _cx| {
        let state = &mut editor.blocks;
        let n = state.index.len();
        if on {
            state.list.splice(0..0, 1);
            state.list.splice(n + 1..n + 1, 1);
        } else {
            state.list.splice(n + 1..n + 2, 0);
            state.list.splice(0..1, 0);
        }
        state.spacers = on;
    });
}

/// Items, item kinds and item hashes for `text` split into `blocks`, as the list stores them.
/// Pure.
fn items_for(text: &str, blocks: &[Block]) -> (Vec<Range<usize>>, Vec<BlockKind>, Vec<u64>) {
    let items = item_ranges(text, blocks);
    let kinds = item_kinds(text, blocks);
    let hashes: Vec<u64> = items
        .iter()
        .zip(&kinds)
        .map(|(range, kind)| hash_item(kind, &text[range.clone()]))
        .collect();
    (items, kinds, hashes)
}

/// The item hashes of `text` for a full rebuild: re-splits and re-hashes the whole text. Public
/// only so `benches/pure.rs` can time that work; the editor does not call it.
#[doc(hidden)]
pub fn sync_hashes(text: &str) -> Vec<u64> {
    items_for(text, &blocks_for(text)).2
}

/// Bring the block items up to date with the buffer at `version`, by a full rebuild. Used when
/// the incremental path cannot take the edits (undo, redo, a wholesale replacement, or the
/// first sync).
fn sync_items(editor: &Entity<Editor>, version: u64, cx: &mut App) {
    let text = editor.read(cx).text();
    editor.update(cx, |editor, _cx| {
        let state = &mut editor.blocks;
        let (old, count) = state.index.build(&text, version);
        let lead = state.lead();
        state.list.splice(old.start + lead..old.end + lead, count);
    });
}

impl Editor {
    /// Hidden byte ranges of the document for caret movement (`keys.rs`). Empty in Source Mode.
    /// It equals [`document_hidden_ranges`], but it reuses the list's cached per-block ranges
    /// when the list is up to date with the buffer (10.4), and it computes the full value
    /// otherwise.
    pub(crate) fn hidden_ranges_for_caret(
        &mut self,
        text: &str,
        caret: Option<usize>,
    ) -> Vec<Range<usize>> {
        if self.modes().source {
            return Vec::new();
        }
        if self.blocks.index.version() == Some(self.buffer.version()) {
            self.blocks.index.hidden_ranges(text, caret)
        } else {
            document_hidden_ranges(text, caret)
        }
    }
}

/// Build the list element for the editor content.
///
/// Called from `Editor::render` (HOOK 3.5). It replaces the HOOK 3.1 line, which draws the
/// whole buffer as one `TextElement`.
pub fn render_blocks(editor: &Entity<Editor>, _cx: &mut Context<Editor>) -> AnyElement {
    BlockHost {
        editor: editor.clone(),
    }
    .into_any_element()
}

/// Element that syncs the list with the buffer and then lays the list out.
struct BlockHost {
    editor: Entity<Editor>,
}

impl IntoElement for BlockHost {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for BlockHost {
    /// The list element and the item counter.
    type RequestLayoutState = (AnyElement, Rc<Cell<usize>>);
    type PrepaintState = ();

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
        sync(&self.editor, cx);
        // Each block's TextElement writes its layout during paint, so this frame's map starts empty.
        self.editor.update(cx, |editor, _cx| editor.layouts.clear());
        let state = self.editor.read(cx).blocks.list.clone();
        let layouts = self.editor.read(cx).blocks.layouts.clone();
        // The items are laid out after this point in the same frame, so the counter then holds
        // this frame's count.
        layouts.set(0);
        // Typewriter Mode (8.4) spacers are half the viewport tall, so the first and last blocks
        // can be centred. The viewport height is the last frame's. A changed height is measured
        // again, since the spacers are list items.
        let spacer_px = if self.editor.read(cx).modes().typewriter {
            crate::modes::typewriter::spacer_height(f32::from(state.viewport_bounds().size.height))
        } else {
            0.
        };
        if self.editor.read(cx).blocks.spacer_px != spacer_px {
            self.editor.update(cx, |editor, _cx| {
                let state = &mut editor.blocks;
                state.spacer_px = spacer_px;
                if state.spacers {
                    let n = state.index.len();
                    state.list.remeasure_items(0..1);
                    state.list.remeasure_items(n + 1..n + 2);
                }
            });
        }

        let editor = self.editor.clone();
        let counter = layouts.clone();
        // The item closure cannot borrow the editor, so it holds its own clone of the entity.
        let mut child = list(state, move |ix, _window, cx| {
            counter.set(counter.get() + 1);
            let (lead, count) = {
                let state = &editor.read(cx).blocks;
                (state.lead(), state.index.len())
            };
            if lead == 1 && (ix == 0 || ix == count + 1) {
                return div().w_full().h(px(spacer_px)).into_any_element();
            }
            let ix = ix - lead;
            let (range, kind) = {
                let state = &editor.read(cx).blocks;
                (
                    state.index.items.get(ix).cloned().unwrap_or(0..0),
                    state
                        .index
                        .kinds
                        .get(ix)
                        .copied()
                        .unwrap_or(BlockKind::Blank),
                )
            };
            crate::blocks::item_element(&editor, range, kind, cx)
        })
        .size_full()
        .into_any_element();
        let child_id = child.request_layout(window, cx);

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (
            window.request_layout(style, [child_id], cx),
            (child, layouts),
        )
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.0.prepaint(window, cx);
        // After the visible items are laid out, so the caret block's height is known. In
        // Typewriter Mode the caret is centred after paint instead (`paint` below).
        if !self.editor.read(cx).modes().typewriter {
            reveal_caret(&self.editor, window, cx);
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.0.paint(window, cx);
        // Typewriter Mode (8.4): the caret's block is painted now, so its line position is known.
        crate::modes::typewriter::step(&self.editor, window, cx);
        // Mouse and input handlers are registered once for the whole list, not per block, so
        // each mouse event and each IME call is handled once.
        cursor::register_mouse_handlers(&self.editor, bounds, window);
        let focus = self.editor.read(cx).focus_handle.clone();
        ime::register_input_handler(&self.editor, &focus, bounds, window, cx);
        // Debug aid for the 3.5 acceptance counter: `MD_EDITOR_LAYOUT_LOG=1` prints, for each
        // frame, the number of items laid out and the total item count.
        if std::env::var_os("MD_EDITOR_LAYOUT_LOG").is_some() {
            let laid_out = request_layout.1.get();
            let items = self.editor.read(cx).blocks.index.len();
            eprintln!("md_editor: block layouts this frame={laid_out} items={items}");
        }
        crate::perf::painted();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::line_ranges;
    use md_core::block::split_blocks;
    use md_core::block::BlockMap;
    use md_core::display_map::{Direction, DisplayMap};

    const SAMPLE: &str =
        "# Title\n\nA paragraph that\ncontinues here.\n\n- item one\n- item two\n\n\
                          ```rust\nlet x = 1;\n```\n\n---\n";

    /// The lines of every item, in order, as text. Must equal the lines of the whole text.
    fn item_lines(text: &str) -> Vec<String> {
        let mut lines = Vec::new();
        for item in item_ranges(text, &blocks_for(text)) {
            for line in line_ranges(&text[item.clone()]) {
                lines.push(text[item.start + line.start..item.start + line.end].to_string());
            }
        }
        lines
    }

    fn whole_text_lines(text: &str) -> Vec<String> {
        line_ranges(text)
            .into_iter()
            .map(|line| text[line].to_string())
            .collect()
    }

    #[test]
    fn blocks_for_matches_split_blocks() {
        assert_eq!(blocks_for(SAMPLE), split_blocks(SAMPLE));
        assert!(
            blocks_for(SAMPLE).len() > 5,
            "the sample has several block kinds"
        );
    }

    #[test]
    fn blocks_for_is_the_same_after_a_resplit() {
        let first = blocks_for(SAMPLE);
        assert_eq!(first, blocks_for(SAMPLE));
        assert_eq!(first, BlockMap::new(SAMPLE).blocks());
    }

    #[test]
    fn items_give_every_line_in_order() {
        let texts = [
            SAMPLE,
            "",
            "a",
            "a\n",
            "a\n\nb\n",
            "\n\n\n",
            "x\n\n\ny",
            "日本\n\n🎉\n",
        ];
        for text in texts {
            assert_eq!(item_lines(text), whole_text_lines(text), "text {text:?}");
        }
    }

    #[test]
    fn ten_thousand_line_document_has_one_item_per_block() {
        // Paragraphs separated by blank lines: each paragraph line and each blank line is its
        // own block, so the list has one item per line.
        let text: String = (0..5000).map(|i| format!("Paragraph {i}\n\n")).collect();
        assert_eq!(text.lines().count(), 10_000);
        let items = item_ranges(&text, &blocks_for(&text));
        assert_eq!(items.len(), 10_000 + 1, "plus the final empty line");
        assert_eq!(item_lines(&text), whole_text_lines(&text));
    }

    #[test]
    fn item_index_for_offset_finds_the_item_holding_a_caret() {
        // "# T" 0..3, a blank line at 4, "para" 5..9, text ends with a newline at 9, so a final
        // empty item sits at 10.
        let items = item_ranges("# T\n\npara\n", &blocks_for("# T\n\npara\n"));
        assert_eq!(item_index_for_offset(&items, 0), Some(0));
        assert_eq!(
            item_index_for_offset(&items, 3),
            Some(0),
            "the line break of the heading"
        );
        assert_eq!(item_index_for_offset(&items, 5), Some(2));
        assert_eq!(
            item_index_for_offset(&items, 9),
            Some(2),
            "end of the paragraph"
        );
        assert_eq!(
            item_index_for_offset(&items, 10),
            Some(3),
            "the empty last line"
        );
        assert_eq!(item_index_for_offset(&items, 11), None);
    }

    #[test]
    fn item_kinds_match_item_ranges_and_kinds() {
        for text in [SAMPLE, "", "a", "a\n", "# H\n"] {
            let blocks = blocks_for(text);
            assert_eq!(
                item_kinds(text, &blocks).len(),
                item_ranges(text, &blocks).len(),
                "text {text:?}"
            );
        }
        let blocks = blocks_for("# Title\n\n---\n");
        assert_eq!(
            item_kinds("# Title\n\n---\n", &blocks),
            vec![
                BlockKind::Heading(1),
                BlockKind::Blank,
                BlockKind::ThematicBreak,
                BlockKind::Blank,
            ]
        );
    }

    #[test]
    fn changed_span_replaces_only_the_middle() {
        assert_eq!(changed_span(&[1, 2, 3, 4], &[1, 9, 3, 4]), (1..2, 1));
        assert_eq!(changed_span(&[1, 2], &[1, 5, 2]), (1..1, 1));
        assert_eq!(changed_span(&[], &[1, 2]), (0..0, 2));
        assert_eq!(changed_span(&[1, 2], &[]), (0..2, 0));
        assert_eq!(changed_span(&[1, 2], &[1, 2]), (2..2, 0));
    }

    /// Mixed document with every block kind the list and caret movement treat differently.
    const MIXED: &str = "# Title\n\nPara with **bold** and `code` and [link](http://x).\n\n\
                         - item *one*\n  - nested\n\n1. first\n\n> quote **q**\n> more\n\n\
                         | a | b |\n|---|---|\n| 1 | 2 |\n\n```rust\nfn x() {}\n```\n\n---\n\n\
                         last ~~gone~~ line\n";

    /// Deterministic xorshift, so the edit sequences repeat.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// Insertions that make and break block boundaries and markers. All ASCII, so random byte
    /// offsets stay on char boundaries.
    const PIECES: &[&str] = &[
        "a", " ", "\n", "\n", "\n\n", "#", "# ", "- ", "> ", "```\n", "1. ", "|", "*", "---\n", "`",
    ];

    /// The index holds exactly what a full rebuild of `text` gives.
    fn assert_matches_full(index: &ItemIndex, text: &str) {
        let (items, kinds, hashes) = items_for(text, &blocks_for(text));
        assert_eq!(index.items, items, "items for {text:?}");
        assert_eq!(index.kinds, kinds, "kinds for {text:?}");
        assert_eq!(index.hashes, hashes, "hashes for {text:?}");
        assert_eq!(index.hidden.len(), items.len());
        let map = index.map.as_ref().expect("built");
        assert_eq!(
            map.blocks(),
            split_blocks(text).as_slice(),
            "blocks for {text:?}"
        );
    }

    #[test]
    fn incremental_edits_match_a_full_rebuild() {
        let mut buffer = Buffer::new(MIXED);
        let mut index = ItemIndex::default();
        index.build(MIXED, buffer.version());
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        let mut incremental = 0;
        for _ in 0..200 {
            let len = buffer.len();
            let start = rng.below(len + 1);
            let (range, inserted) = if start < len && rng.below(3) == 0 {
                (start..(start + 1 + rng.below(4)).min(len), "")
            } else {
                (start..start, PIECES[rng.below(PIECES.len())])
            };
            let items_before = index.len();
            let before = buffer.version();
            buffer.edit(Edit {
                range: range.clone(),
                text: inserted.to_string(),
            });
            match index.apply_edit(before, &buffer, range, inserted) {
                Some((old, count)) => {
                    incremental += 1;
                    assert_eq!(index.len(), items_before - old.len() + count, "splice size");
                }
                None => {
                    index.build(&buffer.text(), buffer.version());
                }
            }
            let text = buffer.text();
            assert_matches_full(&index, &text);

            // Caret movement sees the same hidden ranges as a full computation.
            let caret = rng.below(text.len() + 1);
            assert_eq!(
                index.hidden_ranges(&text, Some(caret)),
                document_hidden_ranges(&text, Some(caret)),
                "caret {caret} in {text:?}"
            );
            assert_eq!(
                index.hidden_ranges(&text, None),
                document_hidden_ranges(&text, None)
            );
        }
        assert!(
            incremental > 150,
            "the incremental path took {incremental} of 200 edits"
        );
    }

    #[test]
    fn whole_text_replacement_and_empty_text_fall_back_to_a_rebuild() {
        let mut buffer = Buffer::new(MIXED);
        let mut index = ItemIndex::default();
        index.build(MIXED, buffer.version());
        let before = buffer.version();
        let len = buffer.len();
        buffer.edit(Edit::replace(0..len, "new text\n"));
        assert_eq!(
            index.apply_edit(before, &buffer, 0..len, "new text\n"),
            None,
            "a replaced whole text is rebuilt"
        );
        index.build(&buffer.text(), buffer.version());
        let before = buffer.version();
        let len = buffer.len();
        buffer.edit(Edit::delete(0..len));
        assert_eq!(
            index.apply_edit(before, &buffer, 0..len, ""),
            None,
            "an emptied text is rebuilt"
        );
        index.build("", buffer.version());
        assert_matches_full(&index, "");
    }

    #[test]
    fn cached_hidden_ranges_give_the_same_caret_moves() {
        let text = format!("{MIXED}\n{MIXED}");
        let mut index = ItemIndex::default();
        index.build(&text, 0);
        let len = text.len();
        for caret in (0..=len).filter(|&c| text.is_char_boundary(c)) {
            let cached = index.hidden_ranges(&text, Some(caret));
            let full = document_hidden_ranges(&text, Some(caret));
            assert_eq!(cached, full, "caret {caret}");
            let (a, b) = (DisplayMap::new(len, cached), DisplayMap::new(len, full));
            for dir in [Direction::Left, Direction::Right] {
                assert_eq!(
                    a.move_caret(&text, caret, dir),
                    b.move_caret(&text, caret, dir),
                    "caret {caret} {dir:?}"
                );
            }
        }
    }
}
