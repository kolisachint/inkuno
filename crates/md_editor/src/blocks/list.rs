//! List blocks (plan task 4.2, DESIGN §4.2 and §4.3): bullet, ordered and task items, with
//! nesting.
//!
//! A list block is one top-level block with one line per source line. [`parse_list_lines`] is
//! pure and gives each line its nesting depth, its marker and the bytes that the marker takes.
//! `inline_render::block_hidden_ranges` hides those bytes on every line whose caret is not on
//! it, and `element.rs` draws the line at its depth. The marker is drawn in the gutter (a bullet,
//! a number or a checkbox) to the left of the text column.
//!
//! The nesting rule follows CommonMark: an item is a child of the nearest earlier item whose
//! text column it is indented to. So nesting works with 2 spaces, 4 spaces or a tab per level,
//! and with the parent's marker width (`1. ` is 3 columns).

use std::ops::Range;

use gpui::{
    fill, point, px, size, App, Bounds, Context, Font, Hsla, Pixels, Point, ShapedLine,
    SharedString, TextAlign, TextRun, Window,
};

use crate::editor::Editor;
use crate::selection::map_through_edit;

/// Left padding of the list gutter, in pixels.
pub const LIST_LEFT_PX: f32 = 16.0;
/// Width of one nesting level, in pixels. It is also the width of a gutter cell.
pub const LIST_STEP_PX: f32 = 24.0;
/// Side of a task checkbox, in pixels.
pub const CHECKBOX_PX: f32 = 12.0;
/// Font size of the tick in a checked box, and how far it rises above the box's top edge, in
/// pixels. The glyph box is taller than the tick, so it is lifted to centre the tick.
const TICK_PX: f32 = 10.0;
const TICK_LIFT_PX: f32 = 4.0;
/// Thickness of the outline of an unchecked checkbox, in pixels.
const BOX_LINE_PX: f32 = 1.5;
/// Gap between the end of a number and the text that follows it, in pixels.
const NUMBER_GAP_PX: f32 = 6.0;

/// Colours of the list gutter (DESIGN §9 will move these into the theme).
const BOX_BORDER: Hsla = Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.55,
    a: 1.0,
};
const BOX_CHECKED: Hsla = Hsla {
    h: 0.61,
    s: 0.75,
    l: 0.50,
    a: 1.0,
};

/// The marker of one list line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListMarker {
    /// `-`, `*` or `+`.
    Bullet,
    /// A number with its delimiter, as written: `3.` or `3)`.
    Ordered(String),
    /// A task item, `- [ ] ` or `- [x] `. `checked` is true for `x` and `X`.
    Task { checked: bool },
}

/// One line of a list block.
///
/// `marker_range` is what the line shows in place of its markers when the caret is not on it:
/// the indentation, and for an item also the marker, the space after it and the task box with
/// its space. A line without a marker (a continuation or a blank line) has only its indentation
/// in `marker_range`. `text_start` is the first byte of the item text, which is the end of
/// `marker_range` for an item. All offsets are absolute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListLine {
    pub depth: usize,
    pub marker: Option<ListMarker>,
    pub marker_range: Range<usize>,
    pub text_start: usize,
}

/// The gutter of one list line whose markers are hidden: the bullet, number or tick, and the
/// checkbox of a task item. Shaped and placed by [`gutter`], painted by [`ListGutter::paint`].
#[derive(Clone)]
pub struct ListGutter {
    /// The shaped glyph and its origin, in window coordinates.
    pub glyph: Option<(ShapedLine, Point<Pixels>)>,
    /// The checkbox of a task item, drawn as a square.
    pub checkbox: Option<CheckBox>,
}

/// A task checkbox, in window coordinates, and the bytes of its `[ ]` or `[x]` in the buffer.
#[derive(Clone)]
pub struct CheckBox {
    pub bounds: Bounds<Pixels>,
    pub checked: bool,
    pub source: Range<usize>,
}

impl ListGutter {
    /// Paint the checkbox square, then the glyph on top of it.
    pub fn paint(&self, line_height: Pixels, window: &mut Window, cx: &mut App) {
        if let Some(checkbox) = &self.checkbox {
            paint_checkbox(checkbox.bounds, checkbox.checked, window);
        }
        if let Some((glyph, origin)) = &self.glyph {
            if let Err(err) = glyph.paint(*origin, line_height, TextAlign::Left, None, window, cx) {
                eprintln!("md_editor: failed to paint list marker: {err}");
            }
        }
    }
}

/// Nesting depth of each line of `block_text` (a list block without its trailing newline), with
/// its marker. Returns one entry per line, in order, so the entries line up with the lines of
/// the block. `base` is the absolute offset of the block in the document.
///
/// Items nest by indentation: an item indented to at least the text column of an earlier open
/// item is its child. A line that is not an item belongs to the innermost open item, which
/// covers lazy continuations, indented continuations and blank lines.
pub fn parse_list_lines(block_text: &str, base: usize) -> Vec<ListLine> {
    let mut lines = Vec::new();
    // Text columns of the open items, outermost first.
    let mut open: Vec<usize> = Vec::new();
    let mut start = 0;
    for line in block_text.split('\n') {
        let abs = base + start;
        start += line.len() + 1;
        match parse_item(line) {
            Some(item) => {
                while open.last().is_some_and(|&column| item.indent_col < column) {
                    open.pop();
                }
                let depth = open.len();
                open.push(item.content_col);
                lines.push(ListLine {
                    depth,
                    marker: Some(item.marker),
                    marker_range: abs..abs + item.end,
                    text_start: abs + item.end,
                });
            }
            None => {
                let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
                lines.push(ListLine {
                    depth: open.len().saturating_sub(1),
                    marker: None,
                    marker_range: abs..abs + indent_len,
                    text_start: abs + indent_len,
                });
            }
        }
    }
    lines
}

/// The `[ ]` or `[x]` box of a task item line, relative to the start of `line`. `None` when the
/// line is not a task item.
pub fn task_box_range(line: &str) -> Option<Range<usize>> {
    let item = parse_item(line)?;
    let start = item.box_at?;
    Some(start..start + 3)
}

/// The text that replaces a task box when it is clicked: `[ ]` becomes `[x]`, and `[x]` or
/// `[X]` becomes `[ ]`. `None` for any other text.
pub fn toggled_box(text: &str) -> Option<&'static str> {
    match text {
        "[ ]" => Some("[x]"),
        "[x]" | "[X]" => Some("[ ]"),
        _ => None,
    }
}

/// Left edge of the text of a list line, in pixels from the left of the block. An item whose
/// markers show (`revealed`) starts its source at the depth's cell. Every other line starts its
/// text one cell further right, so that the marker can sit in the cell to the left. A line
/// without a marker is a continuation and aligns with the text of its item. Pure.
pub fn text_indent(item: &ListLine, revealed: bool) -> f32 {
    let column = LIST_LEFT_PX + item.depth as f32 * LIST_STEP_PX;
    if item.marker.is_some() && revealed {
        column
    } else {
        column + LIST_STEP_PX
    }
}

/// The glyph of a bullet at nesting `depth`: `•`, then `◦` and `▪` for deeper levels.
pub fn bullet_for_depth(depth: usize) -> &'static str {
    match depth {
        0 => "•",
        1 => "◦",
        _ => "▪",
    }
}

/// The gutter of a list line whose markers are hidden. `origin` is the line's text origin in
/// window coordinates, `line` is its text (starting at `source_start` in the buffer), and the
/// glyph is drawn in the cell to the left of `origin`.
pub(crate) fn gutter(
    item: &ListLine,
    line: &str,
    source_start: usize,
    origin: Point<Pixels>,
    line_height: Pixels,
    text: &MarkerText,
    window: &mut Window,
) -> ListGutter {
    let cell_center = origin.x - px(LIST_STEP_PX / 2.0);
    let shape = |window: &mut Window, value: &str, color: Hsla, size: Pixels| {
        let run = TextRun {
            len: value.len(),
            font: text.font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        window
            .text_system()
            .shape_line(SharedString::from(value.to_string()), size, &[run], None)
    };
    let mut glyph = None;
    let mut checkbox = None;
    match &item.marker {
        Some(ListMarker::Bullet) => {
            let shaped = shape(window, bullet_for_depth(item.depth), text.color, text.size);
            let x = cell_center - px(f32::from(shaped.width) / 2.0);
            glyph = Some((shaped, point(x, origin.y)));
        }
        Some(ListMarker::Ordered(number)) => {
            let shaped = shape(window, number, text.color, text.size);
            let x = origin.x - px(NUMBER_GAP_PX) - shaped.width;
            glyph = Some((shaped, point(x, origin.y)));
        }
        Some(ListMarker::Task { checked }) => {
            let box_top = origin.y + px((f32::from(line_height) - CHECKBOX_PX) / 2.0);
            let bounds = Bounds::new(
                point(cell_center - px(CHECKBOX_PX / 2.0), box_top),
                size(px(CHECKBOX_PX), px(CHECKBOX_PX)),
            );
            if *checked {
                let tick = shape(window, "✔", gpui::white(), px(TICK_PX));
                let x = bounds.left() + px((CHECKBOX_PX - f32::from(tick.width)) / 2.0);
                glyph = Some((tick, point(x, box_top - px(TICK_LIFT_PX))));
            }
            let source = task_box_range(line)
                .map(|range| (source_start + range.start)..(source_start + range.end));
            if let Some(source) = source {
                checkbox = Some(CheckBox {
                    bounds,
                    checked: *checked,
                    source,
                });
            }
        }
        None => {}
    }
    ListGutter { glyph, checkbox }
}

/// The font and colour of the markers of a list block.
pub(crate) struct MarkerText {
    pub font: Font,
    pub size: Pixels,
    pub color: Hsla,
}

/// Paint a checkbox: a square outline, or a filled square when checked.
fn paint_checkbox(bounds: Bounds<Pixels>, checked: bool, window: &mut Window) {
    if checked {
        window.paint_quad(fill(bounds, BOX_CHECKED));
        return;
    }
    let (left, top) = (bounds.left(), bounds.top());
    let (width, height) = (bounds.size.width, bounds.size.height);
    let line = px(BOX_LINE_PX);
    for edge in [
        Bounds::new(point(left, top), size(width, line)),
        Bounds::new(point(left, top + height - line), size(width, line)),
        Bounds::new(point(left, top), size(line, height)),
        Bounds::new(point(left + width - line, top), size(line, height)),
    ] {
        window.paint_quad(fill(edge, BOX_BORDER));
    }
}

/// The checkbox under window point `position` in the block layouts of the last frame, as the
/// buffer range of its `[ ]` or `[x]`. `None` when the point is not on a checkbox.
pub fn checkbox_at(editor: &Editor, position: Point<Pixels>) -> Option<Range<usize>> {
    let block = editor.layouts.block_for_y(position.y)?;
    block
        .snapshot
        .lines
        .iter()
        .filter_map(|line| line.gutter.as_ref()?.checkbox.as_ref())
        .find(|checkbox| checkbox.bounds.contains(&position))
        .map(|checkbox| checkbox.source.clone())
}

impl Editor {
    /// Toggle the task box at `range` (`[ ]` or `[x]`) as one edit. The caret and selection keep
    /// their places, so a click on a checkbox does not move the caret into the line.
    pub fn toggle_task(&mut self, range: Range<usize>, cx: &mut Context<Self>) {
        let Some(new) = toggled_box(&self.buffer.slice(range.clone())) else {
            return;
        };
        let before = self.selection.selection;
        self.replace_range(range.clone(), new, cx);
        self.selection.selection = map_through_edit(before, range, new.len());
        cx.notify();
    }
}

/// One list item line, before its nesting is known.
struct Item {
    marker: ListMarker,
    /// Columns of the indentation, tabs advancing to the next multiple of four.
    indent_col: usize,
    /// Columns up to the item text, which later items must be indented to in order to nest.
    content_col: usize,
    /// Bytes from the start of the line to the end of the marker range: the indentation, the
    /// marker, its space and, for a task, the box and its space.
    end: usize,
    /// Bytes from the start of the line to the task box, for a task item.
    box_at: Option<usize>,
}

/// Parse one line as a list item. Markers follow md_core (`-`, `*`, `+`, or digits and `.` or
/// `)`), and must be followed by a space, a tab or the end of the line.
fn parse_item(line: &str) -> Option<Item> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let rest = &line[indent_len..];
    let bytes = rest.as_bytes();
    let first = *bytes.first()?;
    let (mut marker, marker_len) = if matches!(first, b'-' | b'*' | b'+') {
        (ListMarker::Bullet, 1)
    } else {
        let digits = bytes
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if digits == 0 || digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
            return None;
        }
        (ListMarker::Ordered(rest[..=digits].to_string()), digits + 1)
    };
    let after = &rest[marker_len..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    let spaces = after.len() - after.trim_start_matches([' ', '\t']).len();
    // Five or more spaces after a marker make the text an indented code block, so only one
    // counts as the marker's space (CommonMark).
    let spaces = if spaces > 4 { 1 } else { spaces };
    let prefix_end = indent_len + marker_len + spaces;
    let content_col = columns(&line[..prefix_end]);
    let mut end = prefix_end;
    let mut box_at = None;
    if let Some(checked) = task_box(&after[spaces..]) {
        marker = ListMarker::Task { checked };
        box_at = Some(prefix_end);
        let box_rest = &line[prefix_end + 3..];
        let trailing = box_rest.len() - box_rest.trim_start_matches([' ', '\t']).len();
        end = prefix_end + 3 + trailing;
    }
    Some(Item {
        marker,
        indent_col: columns(&line[..indent_len]),
        content_col,
        end,
        box_at,
    })
}

/// Whether `text` starts with a task box followed by a space, a tab or the end of the line, and
/// if so whether the box is checked.
fn task_box(text: &str) -> Option<bool> {
    let bytes = text.as_bytes();
    if bytes.len() < 3 || bytes[0] != b'[' || bytes[2] != b']' {
        return None;
    }
    let checked = match bytes[1] {
        b' ' => false,
        b'x' | b'X' => true,
        _ => return None,
    };
    (bytes.len() == 3 || matches!(bytes[3], b' ' | b'\t')).then_some(checked)
}

/// Width in columns of `text`, where a tab advances to the next multiple of four.
fn columns(text: &str) -> usize {
    text.chars().fold(0, |column, ch| {
        if ch == '\t' {
            (column / 4 + 1) * 4
        } else {
            column + 1
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bullet(depth: usize, range: Range<usize>, text_start: usize) -> ListLine {
        ListLine {
            depth,
            marker: Some(ListMarker::Bullet),
            marker_range: range,
            text_start,
        }
    }

    #[test]
    fn nested_bullets_nest_one_level_per_indent() {
        let text = "- a\n  - b\n    - c";
        let lines = parse_list_lines(text, 0);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], bullet(0, 0..2, 2));
        assert_eq!(lines[1].depth, 1);
        assert_eq!(&text[lines[1].marker_range.clone()], "  - ");
        assert_eq!(lines[2].depth, 2);
        assert_eq!(&text[lines[2].marker_range.clone()], "    - ");
    }

    #[test]
    fn nesting_follows_the_parent_marker_width() {
        // Under "1. " (3 columns) a child needs 3 spaces, so 2 spaces make a sibling.
        let text = "1. a\n   - b\n  - c\n2. d";
        let depths: Vec<usize> = parse_list_lines(text, 0)
            .iter()
            .map(|line| line.depth)
            .collect();
        assert_eq!(depths, vec![0, 1, 0, 0]);
    }

    #[test]
    fn four_space_and_tab_indents_nest() {
        let depths = |text: &str| -> Vec<usize> {
            parse_list_lines(text, 0)
                .iter()
                .map(|line| line.depth)
                .collect()
        };
        assert_eq!(depths("- a\n    - b\n        - c"), vec![0, 1, 2]);
        assert_eq!(depths("- a\n\t- b\n\t\t- c"), vec![0, 1, 2]);
        // Going back out closes the deeper items.
        assert_eq!(depths("- a\n  - b\n    - c\n- d"), vec![0, 1, 2, 0]);
    }

    #[test]
    fn ordered_items_keep_their_numbers() {
        let text = "1. one\n2. two\n10) ten";
        let lines = parse_list_lines(text, 0);
        assert_eq!(lines[0].marker, Some(ListMarker::Ordered("1.".to_string())));
        assert_eq!(lines[1].marker, Some(ListMarker::Ordered("2.".to_string())));
        assert_eq!(
            lines[2].marker,
            Some(ListMarker::Ordered("10)".to_string()))
        );
        assert_eq!(&text[lines[2].marker_range.clone()], "10) ");
        assert_eq!(lines[2].text_start, text.find("ten").unwrap());
    }

    #[test]
    fn task_items_report_checked_and_unchecked() {
        let text = "- [ ] open\n- [x] done\n* [X] also done\n- [ ]";
        let lines = parse_list_lines(text, 0);
        assert_eq!(lines[0].marker, Some(ListMarker::Task { checked: false }));
        assert_eq!(lines[1].marker, Some(ListMarker::Task { checked: true }));
        assert_eq!(lines[2].marker, Some(ListMarker::Task { checked: true }));
        assert_eq!(lines[3].marker, Some(ListMarker::Task { checked: false }));
        // The marker range takes the box and the space after it.
        assert_eq!(&text[lines[0].marker_range.clone()], "- [ ] ");
        assert_eq!(lines[0].text_start, 6);
        assert_eq!(&text[lines[3].marker_range.clone()], "- [ ]");
    }

    #[test]
    fn task_box_is_not_a_task_without_a_separator() {
        let lines = parse_list_lines("- [ ]x", 0);
        assert_eq!(lines[0].marker, Some(ListMarker::Bullet));
        assert_eq!(&"- [ ]x"[lines[0].marker_range.clone()], "- ");
    }

    #[test]
    fn star_and_plus_markers_are_bullets() {
        let lines = parse_list_lines("* a\n+ b\n- c", 0);
        assert!(lines
            .iter()
            .all(|line| line.marker == Some(ListMarker::Bullet)));
        assert!(lines.iter().all(|line| line.depth == 0));
    }

    #[test]
    fn continuation_lines_belong_to_the_item() {
        let text = "- item\n  more of it\n\n  after a blank\n- next";
        let lines = parse_list_lines(text, 0);
        assert_eq!(lines[1].marker, None);
        assert_eq!(lines[1].depth, 0);
        assert_eq!(&text[lines[1].marker_range.clone()], "  ");
        assert_eq!(lines[1].text_start, lines[1].marker_range.end);
        assert_eq!(lines[2].marker, None, "a blank line");
        assert_eq!(lines[3].marker, None);
        assert_eq!(lines[3].depth, 0);
        assert_eq!(lines[4].marker, Some(ListMarker::Bullet));
    }

    #[test]
    fn lazy_continuation_under_a_nested_item_keeps_its_depth() {
        let lines = parse_list_lines("- a\n  - b\nlazy", 0);
        assert_eq!(lines[2].marker, None);
        assert_eq!(lines[2].depth, 1);
    }

    #[test]
    fn empty_item_has_a_marker_and_no_text() {
        let lines = parse_list_lines("-", 0);
        assert_eq!(lines[0], bullet(0, 0..1, 1));
    }

    #[test]
    fn offsets_are_absolute() {
        let lines = parse_list_lines("- a\n  - b", 100);
        assert_eq!(lines[1].marker_range, 104..108);
        assert_eq!(lines[1].text_start, 108);
    }

    #[test]
    fn one_entry_per_line_even_with_a_trailing_blank() {
        assert_eq!(parse_list_lines("- a\n", 0).len(), 2);
        assert_eq!(parse_list_lines("- a", 0).len(), 1);
    }

    #[test]
    fn task_box_range_is_relative_to_the_line() {
        assert_eq!(task_box_range("- [ ] x"), Some(2..5));
        assert_eq!(task_box_range("  1. [x]"), Some(5..8));
        assert_eq!(task_box_range("- plain"), None);
        assert_eq!(task_box_range("plain [ ]"), None);
    }

    #[test]
    fn toggled_box_flips_both_ways() {
        assert_eq!(toggled_box("[ ]"), Some("[x]"));
        assert_eq!(toggled_box("[x]"), Some("[ ]"));
        assert_eq!(toggled_box("[X]"), Some("[ ]"));
        assert_eq!(toggled_box("[a]"), None);
    }

    #[test]
    fn text_indent_steps_by_depth_and_reveal() {
        let item = |depth| bullet(depth, 0..0, 0);
        // A revealed item starts its source at its depth's cell.
        assert_eq!(text_indent(&item(0), true), LIST_LEFT_PX);
        assert_eq!(
            text_indent(&item(2), true),
            LIST_LEFT_PX + 2.0 * LIST_STEP_PX
        );
        // A hidden item and a continuation line start one cell right of it.
        assert_eq!(text_indent(&item(0), false), LIST_LEFT_PX + LIST_STEP_PX);
        let continuation = ListLine {
            depth: 1,
            marker: None,
            marker_range: 0..0,
            text_start: 0,
        };
        assert_eq!(
            text_indent(&continuation, false),
            LIST_LEFT_PX + 2.0 * LIST_STEP_PX
        );
        assert_eq!(
            text_indent(&continuation, true),
            LIST_LEFT_PX + 2.0 * LIST_STEP_PX,
            "a continuation line has no marker to reveal"
        );
    }

    #[test]
    fn bullet_glyphs_by_depth() {
        assert_eq!(bullet_for_depth(0), "•");
        assert_eq!(bullet_for_depth(1), "◦");
        assert_eq!(bullet_for_depth(2), "▪");
        assert_eq!(bullet_for_depth(7), "▪");
    }

    #[test]
    fn columns_count_tabs_to_the_next_stop() {
        assert_eq!(columns("  "), 2);
        assert_eq!(columns("\t"), 4);
        assert_eq!(columns(" \t"), 4);
        assert_eq!(columns("\t\t"), 8);
    }
}
