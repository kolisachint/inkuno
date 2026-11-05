//! IME composition and marked text through `EntityInputHandler` (plan task 3.4). Owner: task 3.4.
//!
//! `Editor` implements `EntityInputHandler` in this file. `TextElement::paint` calls
//! `register_input_handler` on every paint, which hands the platform an `ElementInputHandler`
//! for the focused editor. Plain typed characters also arrive here, through
//! `replace_text_in_range`, so typing ordinary text inserts it.
//!
//! The platform speaks UTF-16 code units. The editor speaks UTF-8 byte offsets. Every method
//! converts at its boundary and does the edit through `Editor::replace_range` (3.2), the single
//! edit path. Marked text is tracked as a byte range in `ImeState::marked`.

use std::ops::Range;

use gpui::{
    point, px, App, Bounds, Context, ElementInputHandler, Entity, EntityInputHandler, FocusHandle,
    Pixels, TextRun, UTF16Selection, UnderlineStyle, Window,
};

use crate::editor::Editor;
use crate::selection::Selection;

/// IME state held by `Editor` (field `ime`). Owned by 3.4.
#[derive(Debug, Clone, Default)]
pub struct ImeState {
    /// The marked (uncommitted) byte range in the buffer, if composition is in progress.
    pub marked: Option<Range<usize>>,
}

/// Byte offset in `text` for a UTF-16 offset. An offset past the end gives `text.len()`. An
/// offset that falls inside a surrogate pair rounds up to the next character boundary.
fn utf16_offset_to_utf8(text: &str, target: usize) -> usize {
    let mut utf16 = 0;
    for (byte, ch) in text.char_indices() {
        if utf16 >= target {
            return byte;
        }
        utf16 += ch.len_utf16();
    }
    text.len()
}

/// UTF-16 offset in `text` for a byte offset. Offsets past the end are clamped to the text
/// length. An offset inside a character counts that whole character.
fn utf8_offset_to_utf16(text: &str, byte: usize) -> usize {
    text.char_indices()
        .take_while(|(index, _)| *index < byte)
        .map(|(_, ch)| ch.len_utf16())
        .sum()
}

/// Convert a UTF-16 range, as the platform reports it, to a UTF-8 byte range in `text`.
///
/// Pure function, unit-tested in 3.4. Characters outside the BMP count as two UTF-16 units.
/// Out-of-range input is clamped to the text end. A reversed range becomes empty.
pub fn utf16_to_utf8_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = utf16_offset_to_utf8(text, range.start);
    let end = utf16_offset_to_utf8(text, range.end).max(start);
    start..end
}

/// Convert a UTF-8 byte range in `text` to a UTF-16 range. The inverse of
/// [`utf16_to_utf8_range`]. Pure function, unit-tested in 3.4.
pub fn utf8_to_utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
    let start = utf8_offset_to_utf16(text, range.start);
    let end = utf8_offset_to_utf16(text, range.end).max(start);
    start..end
}

/// The marked range that a `replace_and_mark_text_in_range` of `new_text` at `start` leaves
/// behind. Empty text clears the marked range. Pure, unit-tested in 3.4.
fn marked_after_replace(start: usize, new_text: &str) -> Option<Range<usize>> {
    if new_text.is_empty() {
        None
    } else {
        Some(start..start + new_text.len())
    }
}

/// The selection after a marked replace. `selected_utf16` is the platform's selection inside
/// `new_text`, in UTF-16 units relative to the start of `new_text`. Without it the caret goes
/// to the end of the inserted text. Pure, unit-tested in 3.4.
fn selection_after_mark(
    start: usize,
    new_text: &str,
    selected_utf16: Option<Range<usize>>,
) -> Selection {
    match selected_utf16 {
        Some(selected) => {
            let inside = utf16_to_utf8_range(new_text, selected);
            Selection {
                anchor: start + inside.start,
                head: start + inside.end,
            }
        }
        None => Selection::caret(start + new_text.len()),
    }
}

/// Mark the part of a line's text runs that lies in the marked range (underline, as DESIGN
/// §4 and the platform expect).
///
/// Called by `TextElement::paint` for each line (3.1 makes the call). `line` is the byte range
/// of the line in the buffer. Each run is split at the edges of the marked range, and the part
/// inside it is underlined in the run's own colour. Runs outside the marked range are kept.
pub fn decorate_runs(
    runs: Vec<TextRun>,
    line: Range<usize>,
    marked: Option<Range<usize>>,
) -> Vec<TextRun> {
    let Some(marked) = marked else {
        return runs;
    };
    let start = marked.start.max(line.start);
    let end = marked.end.min(line.end);
    if start >= end {
        return runs;
    }

    let mut out = Vec::with_capacity(runs.len() + 2);
    let mut run_start = line.start;
    for run in runs {
        let run_end = run_start + run.len;
        // Up to three pieces per run: before the marked range, inside it, after it.
        let before = run_start..run_end.min(start);
        let inside = run_start.max(start)..run_end.min(end);
        let after = run_start.max(end)..run_end;
        for (piece, underlined) in [(before, false), (inside, true), (after, false)] {
            if piece.is_empty() {
                continue;
            }
            let mut piece_run = TextRun {
                len: piece.len(),
                ..run.clone()
            };
            if underlined {
                piece_run.underline = Some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(run.color),
                    wavy: false,
                });
            }
            out.push(piece_run);
        }
        run_start = run_end;
    }
    out
}

/// Register `editor` as the window's text input handler for `focus`.
///
/// Called by `TextElement::paint` during paint (3.1 makes the call). The platform then calls the
/// `EntityInputHandler` methods below for the focused editor.
pub fn register_input_handler(
    editor: &Entity<Editor>,
    focus: &FocusHandle,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &App,
) {
    window.handle_input(focus, ElementInputHandler::new(bounds, editor.clone()), cx);
}

impl Editor {
    /// The byte range that an input method call targets. `None` means the marked range when a
    /// composition is in progress, otherwise the selection.
    fn input_target(&self, range_utf16: Option<Range<usize>>) -> Range<usize> {
        match range_utf16 {
            Some(range) => utf16_to_utf8_range(&self.text(), range),
            None => self
                .ime
                .marked
                .clone()
                .unwrap_or_else(|| self.selection.selection.range()),
        }
    }

    /// The laid-out line that contains byte `offset`, if any.
    fn layout_line_for_offset(&self, offset: usize) -> Option<&crate::element::LineLayout> {
        self.layout
            .lines
            .iter()
            .find(|line| line.source.start <= offset && offset <= line.source.end)
    }
}

impl EntityInputHandler for Editor {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let text = self.text();
        let bytes = utf16_to_utf8_range(&text, range);
        *adjusted_range = Some(utf8_to_utf16_range(&text, bytes.clone()));
        Some(text[bytes].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        let selection = self.selection.selection;
        let text = self.text();
        Some(UTF16Selection {
            range: utf8_to_utf16_range(&text, selection.range()),
            reversed: selection.is_reversed(),
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let marked = self.ime.marked.clone()?;
        Some(utf8_to_utf16_range(&self.text(), marked))
    }

    fn unmark_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The marked text stays in the buffer as ordinary text.
        self.ime.marked = None;
        window.notify_text_input_changed(&self.focus_handle);
        cx.notify();
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // A commit (or a plain typed character) is one edit. The marked range ends here.
        let target = self.input_target(range);
        self.ime.marked = None;
        self.replace_range(target, text, cx);
        window.notify_text_input_changed(&self.focus_handle);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        new_text: &str,
        new_selected_range: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let target = self.input_target(range);
        let start = target.start;
        self.replace_range(target, new_text, cx);
        self.ime.marked = marked_after_replace(start, new_text);
        let selection = selection_after_mark(start, new_text, new_selected_range);
        self.set_selection(selection, cx);
        window.notify_text_input_changed(&self.focus_handle);
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        // Layout origins are already in window coordinates, so the element bounds are not needed.
        let range = utf16_to_utf8_range(&self.text(), range_utf16);
        let line = self.layout_line_for_offset(range.start)?;
        let start = range.start.clamp(line.source.start, line.source.end) - line.source.start;
        let end = range.end.clamp(line.source.start, line.source.end) - line.source.start;
        let top = line.origin.y;
        Some(Bounds::from_corners(
            point(line.origin.x + line.shaped.x_for_index(start), top),
            point(
                line.origin.x + line.shaped.x_for_index(end),
                top + self.layout.line_height,
            ),
        ))
    }

    fn character_index_for_point(
        &mut self,
        position: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let line_height = self.layout.line_height;
        let line =
            self.layout.lines.iter().find(|line| {
                position.y >= line.origin.y && position.y < line.origin.y + line_height
            })?;
        let column = line.shaped.closest_index_for_x(position.x - line.origin.x);
        let offset = line.source.start + column.min(line.source.len());
        Some(utf8_to_utf16_range(&self.text(), 0..offset).end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{font, rgba};
    use md_core::buffer::Buffer;

    use crate::selection::apply_replace;

    /// Replace the bytes at `range` with `text` in `buffer` and return the caret, the way
    /// `Editor::replace_range` does (it calls the same `apply_replace`).
    fn replace(buffer: &mut Buffer, range: Range<usize>, text: &str) -> Selection {
        apply_replace(buffer, range, text)
    }

    fn run(len: usize, color: u32) -> TextRun {
        TextRun {
            len,
            font: font("Test"),
            color: rgba(color).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    #[test]
    fn utf16_to_utf8_ascii() {
        let text = "hello";
        assert_eq!(utf16_to_utf8_range(text, 0..5), 0..5);
        assert_eq!(utf16_to_utf8_range(text, 1..3), 1..3);
        assert_eq!(utf16_to_utf8_range(text, 2..2), 2..2);
    }

    #[test]
    fn utf16_to_utf8_multi_byte() {
        // "é" is 1 UTF-16 unit and 2 bytes. "日" is 1 unit and 3 bytes.
        let text = "aé日b";
        assert_eq!(utf16_to_utf8_range(text, 1..2), 1..3); // é
        assert_eq!(utf16_to_utf8_range(text, 2..3), 3..6); // 日
        assert_eq!(utf16_to_utf8_range(text, 0..4), 0..7);
    }

    #[test]
    fn utf16_to_utf8_emoji_is_a_surrogate_pair() {
        // "🎉" is 4 bytes and 2 UTF-16 units.
        let text = "a🎉b";
        assert_eq!(utf16_to_utf8_range(text, 1..3), 1..5);
        assert_eq!(utf16_to_utf8_range(text, 3..4), 5..6);
        assert_eq!(utf16_to_utf8_range(text, 0..4), 0..6);
        // An offset in the middle of the pair rounds up to the next character boundary.
        assert_eq!(utf16_to_utf8_range(text, 2..3), 5..5);
    }

    #[test]
    fn utf16_to_utf8_cjk_and_mixed() {
        let text = "日本語 🎉 é";
        // "日本語 " is 4 units and 10 bytes.
        assert_eq!(utf16_to_utf8_range(text, 4..6), 10..14);
        assert_eq!(&text[utf16_to_utf8_range(text, 4..6)], "🎉");
        assert_eq!(&text[utf16_to_utf8_range(text, 7..8)], "é");
    }

    #[test]
    fn utf16_to_utf8_empty_text() {
        assert_eq!(utf16_to_utf8_range("", 0..0), 0..0);
        assert_eq!(utf16_to_utf8_range("", 0..3), 0..0);
    }

    #[test]
    fn utf16_to_utf8_clamps_out_of_range_input() {
        assert_eq!(utf16_to_utf8_range("abc", 1..99), 1..3);
        assert_eq!(utf16_to_utf8_range("abc", 10..20), 3..3);
        // A reversed range becomes empty at its start.
        let (start, end) = (2, 1);
        assert_eq!(utf16_to_utf8_range("abc", start..end), 2..2);
    }

    #[test]
    fn utf8_to_utf16_ascii_and_multi_byte() {
        assert_eq!(utf8_to_utf16_range("hello", 1..3), 1..3);
        assert_eq!(utf8_to_utf16_range("aé日b", 1..3), 1..2);
        assert_eq!(utf8_to_utf16_range("aé日b", 3..6), 2..3);
    }

    #[test]
    fn utf8_to_utf16_emoji_counts_two_units() {
        let text = "a🎉b";
        assert_eq!(utf8_to_utf16_range(text, 1..5), 1..3);
        assert_eq!(utf8_to_utf16_range(text, 0..6), 0..4);
    }

    #[test]
    fn utf8_to_utf16_empty_and_clamped() {
        assert_eq!(utf8_to_utf16_range("", 0..0), 0..0);
        assert_eq!(utf8_to_utf16_range("ab", 1..50), 1..2);
        assert_eq!(utf8_to_utf16_range("ab", 9..50), 2..2);
    }

    #[test]
    fn conversions_round_trip_on_char_boundaries() {
        let texts = ["", "abc", "é日🎉", "a\nb 🎉🎉 c", "日本語のテキスト"];
        for text in texts {
            let boundaries: Vec<usize> = (0..=text.len())
                .filter(|&i| text.is_char_boundary(i))
                .collect();
            for &start in &boundaries {
                for &end in boundaries.iter().filter(|&&end| end >= start) {
                    let utf16 = utf8_to_utf16_range(text, start..end);
                    assert_eq!(
                        utf16_to_utf8_range(text, utf16.clone()),
                        start..end,
                        "text {text:?}, range {start}..{end}, utf16 {utf16:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn utf16_matches_std_encoding() {
        let text = "x日🎉é";
        let full: Vec<u16> = text.encode_utf16().collect();
        assert_eq!(utf8_to_utf16_range(text, 0..text.len()), 0..full.len());
    }

    #[test]
    fn composition_sets_marked_and_second_update_replaces_it() {
        let mut buffer = Buffer::new("ab");
        // Start a composition at the caret (offset 1): "k" is marked.
        replace(&mut buffer, 1..1, "k");
        let marked = marked_after_replace(1, "k");
        assert_eq!(marked, Some(1..2));
        assert_eq!(buffer.text(), "akb");

        // A second update replaces the marked text, not the text around it.
        let target = marked.clone().unwrap();
        replace(&mut buffer, target.clone(), "ka");
        let marked = marked_after_replace(target.start, "ka");
        assert_eq!(marked, Some(1..3));
        assert_eq!(buffer.text(), "akab");
    }

    #[test]
    fn commit_replaces_marked_text_in_one_edit_and_clears_marked() {
        let mut buffer = Buffer::new("ab");
        // Compose "か" at offset 1, then "ka" over the marked range.
        replace(&mut buffer, 1..1, "か");
        let marked = marked_after_replace(1, "か").unwrap();
        replace(&mut buffer, marked.clone(), "ka");
        let marked = marked_after_replace(marked.start, "ka").unwrap();
        assert_eq!(buffer.text(), "akab");

        // Commit "漢字" over the marked range. This is one buffer edit (one version step and one
        // undo step). The caller clears `ImeState::marked`, which the commit path does before
        // the edit.
        let version_before = buffer.version();
        replace(&mut buffer, marked, "漢字");
        assert_eq!(buffer.version(), version_before + 1);
        assert_eq!(buffer.text(), "a漢字b");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "akab");
    }

    #[test]
    fn marked_cleared_when_text_is_empty() {
        assert_eq!(marked_after_replace(3, ""), None);
        assert_eq!(marked_after_replace(3, "é"), Some(3..5));
    }

    #[test]
    fn selection_after_mark_defaults_to_end_of_text() {
        assert_eq!(selection_after_mark(2, "日本", None), Selection::caret(8));
    }

    #[test]
    fn selection_after_mark_uses_platform_selection_relative_to_text() {
        // Select the second character ("本", UTF-16 1..2) inside "日本" inserted at byte 2.
        let selection = selection_after_mark(2, "日本", Some(1..2));
        assert_eq!(selection, Selection { anchor: 5, head: 8 });
        // A selection past the end is clamped to the inserted text.
        let selection = selection_after_mark(0, "ab", Some(1..9));
        assert_eq!(selection, Selection { anchor: 1, head: 2 });
    }

    #[test]
    fn decorate_runs_without_marked_text_is_identity() {
        let runs = vec![run(5, 0x000000ff)];
        let out = decorate_runs(runs.clone(), 0..5, None);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len, 5);
        assert!(out[0].underline.is_none());
    }

    #[test]
    fn decorate_runs_underlines_only_the_marked_part() {
        // Line 10..20, marked 12..15 inside one run.
        let out = decorate_runs(vec![run(10, 0x000000ff)], 10..20, Some(12..15));
        let lens: Vec<usize> = out.iter().map(|r| r.len).collect();
        assert_eq!(lens, vec![2, 3, 5]);
        assert!(out[0].underline.is_none());
        assert!(out[1].underline.is_some());
        assert!(out[2].underline.is_none());
        assert_eq!(lens.iter().sum::<usize>(), 10);
    }

    #[test]
    fn decorate_runs_splits_across_runs_and_clamps_to_the_line() {
        // Two runs (4 and 6 bytes) on a line 0..10; marked 2..8 crosses the boundary.
        let out = decorate_runs(vec![run(4, 1), run(6, 2)], 0..10, Some(2..8));
        let lens: Vec<usize> = out.iter().map(|r| r.len).collect();
        assert_eq!(lens, vec![2, 2, 4, 2]);
        assert_eq!(lens.iter().sum::<usize>(), 10);
        assert!(out[1].underline.is_some());
        assert!(out[2].underline.is_some());
        assert!(out[3].underline.is_none());

        // A marked range that starts before the line only underlines the line.
        let out = decorate_runs(vec![run(4, 1)], 5..9, Some(0..7));
        let lens: Vec<usize> = out.iter().map(|r| r.len).collect();
        assert_eq!(lens, vec![2, 2]);
        assert!(out[0].underline.is_some());
        assert!(out[1].underline.is_none());
    }

    #[test]
    fn decorate_runs_ignores_marked_range_outside_the_line() {
        let out = decorate_runs(vec![run(4, 1)], 0..4, Some(10..12));
        assert_eq!(out.len(), 1);
        assert!(out[0].underline.is_none());
    }
}
