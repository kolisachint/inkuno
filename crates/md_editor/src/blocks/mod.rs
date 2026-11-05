//! Block element rendering for live preview (plan tasks 4.1 onward).
//!
//! `basic` holds the pure style rules for headings, paragraphs, blockquotes and thematic
//! breaks. `element.rs` turns them into GPUI text runs and decorations. `table` and `image`
//! (task 4.3) draw read-only tables and images when the caret is outside their block.

use std::ops::Range;
use std::path::Path;

use gpui::{div, AnyElement, App, Entity, IntoElement, ParentElement, Styled};
use md_core::BlockKind;

use crate::editor::Editor;
use crate::element::TextElement;
use crate::modes::focus;

pub mod basic;
pub mod code_fence;
pub mod image;
pub mod list;
pub mod table;

/// The element for one block item of the block list (task 3.5), covering `range` of the buffer.
///
/// With Focus Mode on (task 8.3), a block without the caret is drawn at [`focus::DIM`] opacity.
/// The whole block is wrapped in a div with that opacity, so every paint call inside it (text,
/// gutters, decorations, backgrounds, tables, images) is dimmed alike.
pub fn item_element(
    editor: &Entity<Editor>,
    range: Range<usize>,
    kind: BlockKind,
    cx: &App,
) -> AnyElement {
    let (focus_on, head) = {
        let editor = editor.read(cx);
        (editor.modes().focus, editor.selection.selection.head)
    };
    let opacity = focus::block_opacity(focus_on, &range, head);
    let element = block_element(editor, range, kind, cx);
    if opacity >= 1.0 {
        return element;
    }
    div()
        .w_full()
        .opacity(opacity)
        .child(element)
        .into_any_element()
}

/// The element for one block, drawn as [`item_element`] describes without the focus dimming.
fn block_element(
    editor: &Entity<Editor>,
    range: Range<usize>,
    kind: BlockKind,
    cx: &App,
) -> AnyElement {
    let (head, base_dir, text) = {
        let editor = editor.read(cx);
        (
            editor.selection.selection.head,
            editor.base_dir().map(Path::to_path_buf),
            editor.buffer().slice(range.clone()),
        )
    };
    // The caret is in the block when it lies within its range, end included.
    let caret_inside = focus::caret_in_block(&range, head);
    if !caret_inside {
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
    TextElement::new(editor.clone(), range, kind).into_any_element()
}
