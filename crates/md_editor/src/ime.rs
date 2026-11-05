//! IME composition and marked text through `EntityInputHandler` (plan task 3.4). Owner: task 3.4.
//!
//! `Editor` implements `EntityInputHandler` in this file. The platform calls these methods only
//! after `register_input_handler` has been called from paint, which is a no-op until 3.4 fills
//! it in. So the `todo!` bodies below are not reached before 3.4 lands.

#![allow(dead_code, unused_variables)]

use std::ops::Range;

use gpui::{
    App, Bounds, Context, Entity, EntityInputHandler, FocusHandle, Pixels, TextRun, UTF16Selection,
    Window,
};

use crate::editor::Editor;

/// IME state held by `Editor` (field `ime`). Owned by 3.4.
#[derive(Debug, Clone, Default)]
pub struct ImeState {
    /// The marked (uncommitted) byte range in the buffer, if composition is in progress.
    pub marked: Option<Range<usize>>,
}

/// Convert a UTF-16 range, as the platform reports it, to a UTF-8 byte range in `text`.
///
/// Pure function, unit-tested in 3.4. Characters outside the BMP count as two UTF-16 units.
/// Out-of-range input is clamped to the text end.
pub fn utf16_to_utf8_range(text: &str, range: Range<usize>) -> Range<usize> {
    todo!("3.4")
}

/// Convert a UTF-8 byte range in `text` to a UTF-16 range. The inverse of
/// [`utf16_to_utf8_range`]. Pure function, unit-tested in 3.4.
pub fn utf8_to_utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    todo!("3.4")
}

/// Mark the part of a line's text runs that lies in the marked range (underline, as DESIGN
/// §4 and the platform expect).
///
/// Called by `TextElement::paint` for each line (3.1 makes the call). `line` is the byte range
/// of the line in the buffer. The skeleton returns `runs` unchanged.
pub fn decorate_runs(
    runs: Vec<TextRun>,
    line: Range<usize>,
    marked: Option<Range<usize>>,
) -> Vec<TextRun> {
    runs
}

/// Register `editor` as the window's text input handler for `focus`.
///
/// Called by `TextElement::paint` during paint (3.1 makes the call). The skeleton body is a
/// no-op, so the platform never calls the `EntityInputHandler` methods below yet.
pub fn register_input_handler(
    editor: &Entity<Editor>,
    focus: &FocusHandle,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &App,
) {
}

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        todo!("3.4")
    }

    fn selected_text_range(
        &mut self,
        ignore_disabled_input: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        todo!("3.4")
    }

    fn marked_text_range(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        todo!("3.4")
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        todo!("3.4")
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        todo!("3.4")
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        todo!("3.4")
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        element_bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        todo!("3.4")
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<usize> {
        todo!("3.4")
    }
}
