//! Block element rendering for live preview (plan tasks 4.1 onward).
//!
//! `basic` holds the pure style rules for headings, paragraphs, blockquotes and thematic
//! breaks. `element.rs` turns them into GPUI text runs and decorations. `table` and `image`
//! (task 4.3) draw read-only tables and images when the caret is outside their block.

use std::ops::Range;
use std::path::Path;

use gpui::{AnyElement, App, Entity, IntoElement};
use md_core::BlockKind;

use crate::editor::Editor;
use crate::element::TextElement;

pub mod basic;
pub mod code_fence;
pub mod image;
pub mod list;
pub mod table;

/// The element for one block item of the block list (task 3.5), covering `range` of the buffer.
///
/// A table or an image paragraph is drawn read-only when the caret is outside its block. Every
/// other block, and a table or image with the caret inside, is drawn as source text.
pub fn item_element(
    editor: &Entity<Editor>,
    range: Range<usize>,
    kind: BlockKind,
    cx: &App,
) -> AnyElement {
    let (head, base_dir, text, source_mode) = {
        let editor = editor.read(cx);
        (
            editor.selection.selection.head,
            editor.base_dir().map(Path::to_path_buf),
            editor.buffer().slice(range.clone()),
            editor.modes().source,
        )
    };
    // The caret is in the block when it lies within its range, end included.
    let caret_inside = range.start <= head && head <= range.end;
    // Source Mode (task 8.1) draws every block as source text, so no table or image is drawn.
    if !caret_inside && !source_mode {
        match kind {
            BlockKind::Table => {
                if let Some(parsed) = table::parse_table(&text) {
                    return table::TableElement::new(editor.clone(), range, parsed)
                        .into_any_element();
                }
            }
            BlockKind::Paragraph => {
                if let Some(parsed) = image::parse_image(&text) {
                    return image::image_element(
                        editor.clone(),
                        range,
                        parsed,
                        base_dir.as_deref(),
                    );
                }
            }
            _ => {}
        }
    }
    TextElement::new(editor.clone(), range, kind)
        .in_source_mode(source_mode)
        .into_any_element()
}
