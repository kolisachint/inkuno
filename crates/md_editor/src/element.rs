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

use crate::cursor;
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

/// Geometry of the last painted text, held by `Editor` (field `layout`).
#[derive(Default, Clone)]
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
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let snapshot = std::mem::take(prepaint);
        let editor = self.editor.clone();
        let focus = editor.read(cx).focus_handle.clone();
        let focused = focus.is_focused(window);

        // 1. Selection behind the text.
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

        // 3 to 5. Caret, mouse handlers and the input handler.
        cursor::paint_caret(editor.read(cx), &snapshot, focused, window);
        cursor::register_mouse_handlers(&editor, &snapshot, bounds, window);
        ime::register_input_handler(&editor, &focus, bounds, window, cx);

        // Keep the geometry for hit testing and IME bounds.
        editor.update(cx, |editor, _cx| editor.layout = snapshot);
    }
}

#[cfg(test)]
mod tests {
    use super::line_ranges;

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
