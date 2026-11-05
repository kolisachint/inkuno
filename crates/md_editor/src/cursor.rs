//! Caret painting, click placement and drag selection (plan task 3.2). Owner: task 3.2.

use gpui::{
    fill, point, px, size, Bounds, Entity, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, Point, Window,
};
use md_core::display_map::{Bias, DisplayMap};

use crate::blocks::list;
use crate::editor::Editor;
use crate::element::{LayoutSnapshot, LineLayout};
use crate::selection::Selection;
use crate::theme::Theme;

/// Width of the caret in pixels.
const CARET_WIDTH: f32 = 2.0;

/// Source byte offset under `point` (window coordinates).
///
/// Uses `layout` for line geometry and `display` so that the caret lands on a caret stop
/// (DESIGN §3.2). The line is the one under the point's y (clamped to the first or last
/// line). Within it, the result is the character boundary nearest the point's x: a point in
/// the left half of a character gives the offset before it, a point in the right half gives
/// the offset after it. The boundaries are visible offsets, so they go back to source offsets
/// through the line's hidden ranges (task 4.4). `clip` then snaps the result to a caret stop.
pub fn hit_test(layout: &LayoutSnapshot, display: &DisplayMap, point: Point<Pixels>) -> usize {
    let Some(first) = layout.lines.first() else {
        return 0;
    };
    let count = layout.lines.len();
    let index = line_index_for_y(
        count,
        f32::from(first.origin.y),
        f32::from(layout.line_height),
        f32::from(point.y),
    );
    let line = &layout.lines[index];
    let x = f32::from(point.x) - f32::from(line.origin.x);
    // The boundaries are indices into the visible text, so they are converted to source offsets.
    let local = nearest_boundary(&boundary_positions(line), x);
    display.clip(line.shaped_to_source(local, Bias::Left))
}

/// Index of the line under `y`, for lines of `line_height` starting at `top`, clamped to
/// `0..count`. Pure, unit-tested.
pub(crate) fn line_index_for_y(count: usize, top: f32, line_height: f32, y: f32) -> usize {
    if count == 0 || line_height <= 0.0 {
        return 0;
    }
    let row = ((y - top) / line_height).floor();
    if row <= 0.0 {
        0
    } else {
        (row as usize).min(count - 1)
    }
}

/// Character boundaries of a shaped line as `(local byte offset, x in pixels)`, sorted by offset.
/// Each glyph contributes its start, and the end of the line contributes its width.
fn boundary_positions(line: &LineLayout) -> Vec<(usize, f32)> {
    let mut positions: Vec<(usize, f32)> = line
        .shaped
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter())
        .map(|glyph| (glyph.index, f32::from(glyph.position.x)))
        .collect();
    positions.push((line.shaped.len, f32::from(line.shaped.width)));
    positions.sort_by_key(|(index, _)| *index);
    positions
}

/// The boundary whose x is nearest to `x`. `boundaries` is `(offset, x)` pairs. Ties go to the
/// first one. Pure, unit-tested.
pub(crate) fn nearest_boundary(boundaries: &[(usize, f32)], x: f32) -> usize {
    boundaries
        .iter()
        .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
        .map_or(0, |(offset, _)| *offset)
}

/// The line whose source range contains `offset`. A line break belongs to the line before it,
/// so the head at the end of a line is found on that line.
fn line_containing(layout: &LayoutSnapshot, offset: usize) -> Option<&LineLayout> {
    layout
        .lines
        .iter()
        .find(|line| line.source.start <= offset && offset <= line.source.end)
}

/// Paint the caret at the selection head when `focused` is true.
///
/// Called by `TextElement::paint` after the text (3.1 makes the call). The caret is a thin
/// bar, as tall as one line, at the x position of the head.
pub fn paint_caret(
    editor: &Editor,
    layout: &LayoutSnapshot,
    focused: bool,
    theme: &Theme,
    window: &mut Window,
) {
    if !focused {
        return;
    }
    let head = editor.selection.selection.head;
    let Some(line) = line_containing(layout, head) else {
        return;
    };
    let x = line.x_for_source(head);
    let origin = point(line.origin.x + x, line.origin.y);
    let bounds = Bounds::new(origin, size(px(CARET_WIDTH), layout.line_height));
    window.paint_quad(fill(bounds, theme.caret));
}

/// Source offset of `position` (window coordinates), from the blocks painted in the last frame.
///
/// The block under the point is used, or the nearest visible block when the point is between
/// blocks or past the first or last one (a drag may leave the list). Pure geometry, so it reads
/// the editor's `layouts` only.
pub(crate) fn offset_at(editor: &Editor, position: Point<Pixels>) -> usize {
    // A rendered block with no text lines (a table or an image, task 4.3) puts the caret at its
    // start, so that the next frame shows its source.
    if let Some(block) = editor.layouts.block_for_y(position.y) {
        if block.snapshot.lines.is_empty() {
            return block.source.start;
        }
    }
    let display = DisplayMap::new(editor.buffer().len(), vec![]);
    editor.layouts.offset_at(&display, position)
}

/// Register the mouse handlers for the whole block list: mouse down places the caret, shift
/// click extends the selection, a drag extends it across blocks and mouse up ends the drag.
///
/// Called once per frame by `BlockHost::paint` (3.5), not once per block, so each mouse event is
/// handled once. The handlers read the block layouts at event time, so they see the frame that
/// was last painted. A mouse down outside `bounds` (the list) is ignored. A drag that leaves the
/// list above or below scrolls it by the distance outside.
pub fn register_mouse_handlers(
    editor: &Entity<Editor>,
    bounds: Bounds<Pixels>,
    window: &mut Window,
) {
    // Mouse down inside the list: focus, place the caret, or extend with shift.
    let down_editor = editor.clone();
    window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
        if !phase.bubble() || event.button != MouseButton::Left {
            return;
        }
        if !bounds.contains(&event.position) {
            return;
        }
        let focus = down_editor.read(cx).focus_handle.clone();
        window.focus(&focus, cx);
        // A click on a task checkbox toggles it and leaves the caret where it is (task 4.2).
        if let Some(range) = list::checkbox_at(down_editor.read(cx), event.position) {
            down_editor.update(cx, |editor, cx| editor.toggle_task(range, cx));
            return;
        }
        let head = offset_at(down_editor.read(cx), event.position);
        down_editor.update(cx, |editor, cx| {
            let anchor = if event.modifiers.shift {
                editor.selection.selection.anchor
            } else {
                head
            };
            editor.selection.dragging = true;
            editor.set_selection(Selection { anchor, head }, cx);
        });
    });

    // Drag: move the head while the left button is held after a mouse down in the list.
    let move_editor = editor.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
        if !phase.bubble() || !move_editor.read(cx).selection.dragging {
            return;
        }
        if event.pressed_button != Some(MouseButton::Left) {
            // The button was released outside the window, so no mouse up arrives.
            move_editor.update(cx, |editor, _cx| editor.selection.dragging = false);
            return;
        }
        let head = offset_at(move_editor.read(cx), event.position);
        let list = move_editor.read(cx).blocks.list.clone();
        // Dragging above or below the list scrolls it, so the head can reach further blocks.
        let y = event.position.y;
        if y < bounds.top() {
            list.scroll_by(y - bounds.top());
        } else if y > bounds.bottom() {
            list.scroll_by(y - bounds.bottom());
        }
        move_editor.update(cx, |editor, cx| {
            let anchor = editor.selection.selection.anchor;
            editor.set_selection(Selection { anchor, head }, cx);
        });
    });

    // Mouse up ends the drag.
    let up_editor = editor.clone();
    window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, cx| {
        if !phase.bubble() || event.button != MouseButton::Left {
            return;
        }
        up_editor.update(cx, |editor, cx| {
            if editor.selection.dragging {
                editor.selection.dragging = false;
                cx.notify();
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::{line_index_for_y, nearest_boundary};

    #[test]
    fn nearest_boundary_picks_the_closer_character_edge() {
        // Three characters of width 10 at x = 0, 10, 20; the line ends at 30.
        let boundaries = [(0, 0.0), (1, 10.0), (2, 20.0), (3, 30.0)];
        // Left half of character 0: the caret goes before it.
        assert_eq!(nearest_boundary(&boundaries, 2.0), 0);
        // Right half of character 0: the caret goes after it, before character 1.
        assert_eq!(nearest_boundary(&boundaries, 7.0), 1);
        // Left half of character 2 gives its start, not the end of the line.
        assert_eq!(nearest_boundary(&boundaries, 21.0), 2);
        // Right of the last boundary gives the end of the line.
        assert_eq!(nearest_boundary(&boundaries, 99.0), 3);
        // Left of the first boundary gives the start of the line.
        assert_eq!(nearest_boundary(&boundaries, -5.0), 0);
    }

    #[test]
    fn nearest_boundary_with_multibyte_offsets() {
        // "é" is 2 bytes wide 8 px, "x" is 1 byte wide 6 px.
        let boundaries = [(0, 0.0), (2, 8.0), (3, 14.0)];
        assert_eq!(nearest_boundary(&boundaries, 9.0), 2);
        assert_eq!(nearest_boundary(&boundaries, 3.0), 0);
    }

    #[test]
    fn line_index_for_y_maps_rows_and_clamps() {
        // Three lines, 20 px tall, starting at y = 100.
        assert_eq!(line_index_for_y(3, 100.0, 20.0, 100.0), 0);
        assert_eq!(line_index_for_y(3, 100.0, 20.0, 119.9), 0);
        assert_eq!(line_index_for_y(3, 100.0, 20.0, 120.0), 1);
        assert_eq!(line_index_for_y(3, 100.0, 20.0, 159.0), 2);
        // Above the first line and below the last line clamp.
        assert_eq!(line_index_for_y(3, 100.0, 20.0, 10.0), 0);
        assert_eq!(line_index_for_y(3, 100.0, 20.0, 900.0), 2);
        // No lines or no height gives line 0.
        assert_eq!(line_index_for_y(0, 100.0, 20.0, 110.0), 0);
        assert_eq!(line_index_for_y(3, 100.0, 0.0, 110.0), 0);
    }
}
