//! Caret painting, click placement and drag selection (plan task 3.2). Owner: task 3.2.

#![allow(dead_code, unused_variables)]

use gpui::{Bounds, Entity, Pixels, Point, Window};
use md_core::display_map::DisplayMap;

use crate::editor::Editor;
use crate::element::LayoutSnapshot;

/// Source byte offset under `point` (window coordinates).
///
/// Uses `layout` for line geometry and `display` so that the caret lands on a caret stop
/// (DESIGN §3.2). In Phase 3 the display map has no hidden ranges, so the result is the
/// character boundary nearest the point, rounded to the left when the point is in the
/// right half of a character.
pub fn hit_test(layout: &LayoutSnapshot, display: &DisplayMap, point: Point<Pixels>) -> usize {
    todo!("3.2")
}

/// Paint the caret at the selection head when `focused` is true.
///
/// Called by `TextElement::paint` after the text (3.1 makes the call). The skeleton body is a
/// no-op.
pub fn paint_caret(editor: &Editor, layout: &LayoutSnapshot, focused: bool, window: &mut Window) {}

/// Register mouse handlers for the text element: mouse down places the caret, a drag extends
/// the selection, shift-click extends it once, and mouse up ends the drag.
///
/// Called by `TextElement::paint` during paint (3.1 makes the call). The skeleton body is a
/// no-op.
pub fn register_mouse_handlers(
    editor: &Entity<Editor>,
    layout: &LayoutSnapshot,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
}
