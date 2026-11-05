//! Plain-text element for the editor (plan task 3.1). Owner: task 3.1.
//!
//! `TextElement` draws a byte range of the buffer as plain text, one shaped line per source
//! line. It is a custom GPUI `Element` (DESIGN §7). `Editor::render` creates it for the whole
//! buffer (HOOK 3.1); the block list (3.5) creates one per block.

#![allow(dead_code, unused_variables)]

use std::ops::Range;
use std::panic::Location;

use gpui::{
    App, Bounds, ElementId, Entity, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, ShapedLine, Window,
};

use crate::editor::Editor;

/// Byte range of each line in `text`, split on `\n`. Ranges exclude the `\n`. A trailing
/// newline gives a final empty range. Pure function, unit-tested in 3.1.
pub fn line_ranges(text: &str) -> Vec<Range<usize>> {
    todo!("3.1")
}

/// One laid-out source line.
///
/// Written during paint by `TextElement` and read by selection (3.2), hit testing (3.2) and IME
/// bounds (3.4).
pub struct LineLayout {
    /// Byte range of the line in the buffer, excluding its `\n`.
    pub source: Range<usize>,
    /// The shaped text of the line.
    pub shaped: ShapedLine,
    /// Top-left corner of the line in window coordinates.
    pub origin: Point<Pixels>,
}

/// Geometry of the last painted text, held by `Editor` (field `layout`).
#[derive(Default)]
pub struct LayoutSnapshot {
    pub lines: Vec<LineLayout>,
    /// Bounds of the element in window coordinates.
    pub bounds: Bounds<Pixels>,
    /// Height of one line.
    pub line_height: Pixels,
}

/// Paints `source` (a byte range of the buffer) as plain text.
///
/// Paint order, which 3.1 must follow because other tasks depend on it:
/// 1. `selection::paint_selection`
/// 2. the text lines, each with `ime::decorate_runs` applied to its runs
/// 3. `cursor::paint_caret`
/// 4. `cursor::register_mouse_handlers`
/// 5. `ime::register_input_handler`
///
/// After the text is painted, the element writes the new `LayoutSnapshot` into the editor.
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
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        todo!("3.1")
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        todo!("3.1")
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        todo!("3.1")
    }
}
