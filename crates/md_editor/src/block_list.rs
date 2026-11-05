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
    list, px, relative, AnyElement, App, Bounds, Context, ElementId, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, ListAlignment, ListOffset, ListState, Pixels, Style,
    Styled, Window,
};
use md_core::Block;

use crate::cursor;
use crate::editor::Editor;
use crate::element::{index_containing, TextElement};
use crate::ime;

/// Block list state held by `Editor` (field `blocks`). Owned by 3.5.
pub struct BlockListState {
    /// The GPUI list state. Its item count must match the number of blocks.
    pub list: ListState,
    /// Number of block items laid out in the most recent frame (reset at the start of each
    /// frame). The acceptance counter for 3.5: it stays near the number of visible items, shared
    /// with the list's render closure.
    pub layouts: Rc<Cell<usize>>,
    /// Buffer version the items were built from. `None` before the first sync.
    version: Option<u64>,
    /// Drawn byte range of each list item. One item per top-level block, its trailing newline
    /// excluded. A final empty item is added when the text ends with a newline, to match
    /// `line_ranges`.
    items: Vec<Range<usize>>,
    /// Hash of each item's text, so a re-sync only replaces the items that changed.
    hashes: Vec<u64>,
    /// Buffer version and caret offset at the last caret reveal. The list scrolls to the caret
    /// again only when one of them changes, so wheel scrolling away from the caret is kept.
    revealed: Option<(u64, usize)>,
}

impl Default for BlockListState {
    fn default() -> Self {
        Self {
            list: ListState::new(0, ListAlignment::Top, px(200.)),
            layouts: Rc::new(Cell::new(0)),
            version: None,
            items: Vec::new(),
            hashes: Vec::new(),
            revealed: None,
        }
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
    if let Some(index) = item_index_for_offset(&editor.read(cx).blocks.items, head) {
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

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Bring the list items up to date with the buffer. Does nothing when the buffer version is
/// unchanged, so a re-render without edits does not re-split the document.
fn sync(editor: &Entity<Editor>, cx: &mut App) {
    let version = editor.read(cx).buffer().version();
    if editor.read(cx).blocks.version == Some(version) {
        return;
    }
    let text = editor.read(cx).text();
    let items = item_ranges(&text, &blocks_for(&text));
    let hashes: Vec<u64> = items.iter().map(|r| hash_text(&text[r.clone()])).collect();
    editor.update(cx, |editor, _cx| {
        let state = &mut editor.blocks;
        let (old, count) = changed_span(&state.hashes, &hashes);
        state.list.splice(old, count);
        state.items = items;
        state.hashes = hashes;
        state.version = Some(version);
    });
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

        let editor = self.editor.clone();
        let counter = layouts.clone();
        // The item closure cannot borrow the editor, so it holds its own clone of the entity.
        let mut child = list(state, move |ix, _window, cx| {
            counter.set(counter.get() + 1);
            let range = editor
                .read(cx)
                .blocks
                .items
                .get(ix)
                .cloned()
                .unwrap_or(0..0);
            TextElement::new(editor.clone(), range).into_any_element()
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
        // After the visible items are laid out, so the caret block's height is known.
        reveal_caret(&self.editor, window, cx);
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
        // Mouse and input handlers are registered once for the whole list, not per block, so
        // each mouse event and each IME call is handled once.
        cursor::register_mouse_handlers(&self.editor, bounds, window);
        let focus = self.editor.read(cx).focus_handle.clone();
        ime::register_input_handler(&self.editor, &focus, bounds, window, cx);
        // Debug aid for the 3.5 acceptance counter: `MD_EDITOR_LAYOUT_LOG=1` prints, for each
        // frame, the number of items laid out and the total item count.
        if std::env::var_os("MD_EDITOR_LAYOUT_LOG").is_some() {
            let laid_out = request_layout.1.get();
            let items = self.editor.read(cx).blocks.items.len();
            eprintln!("md_editor: block layouts this frame={laid_out} items={items}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::line_ranges;
    use md_core::block::split_blocks;
    use md_core::block::BlockMap;

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
    fn changed_span_replaces_only_the_middle() {
        assert_eq!(changed_span(&[1, 2, 3, 4], &[1, 9, 3, 4]), (1..2, 1));
        assert_eq!(changed_span(&[1, 2], &[1, 5, 2]), (1..1, 1));
        assert_eq!(changed_span(&[], &[1, 2]), (0..0, 2));
        assert_eq!(changed_span(&[1, 2], &[]), (0..2, 0));
        assert_eq!(changed_span(&[1, 2], &[1, 2]), (2..2, 0));
    }
}
