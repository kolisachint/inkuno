//! Scroll areas for the popup lists (the palette, the completion, the menus, the language picker). A list taller
//! than `max_h` scrolls with the wheel and the keys; while it overflows, a thin thumb on its right shows where the
//! view is. The thumb can be dragged, and a press on the lane outside it pages the list toward the press. Nothing
//! is drawn when the list fits.

use gpui::{
    Bounds, DispatchPhase, Div, ElementId, Entity, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, Pixels, ScrollHandle, Stateful, canvas, div, fill, point, prelude::*, px, size,
};

use crate::theme::Theme;

/// The thumb's width and its inset from the right edge of the viewport.
const THUMB_W: f32 = 6.;
const THUMB_INSET: f32 = 2.;
/// The shortest thumb, so a very long list still shows a grip to aim at.
const THUMB_MIN: f32 = 24.;
/// The lane at the right edge that takes presses: the thumb's grip is its width, and a press beside it pages.
const LANE_W: f32 = 14.;

/// The scroll distance (from the top) of a list whose thumb is dragged: the thumb's top follows the pointer,
/// kept `grab` below where the press took it. `lane_top` is the top of the track the thumb moves in, `track` its
/// height, `thumb` the thumb's length and `max` the overflow. Clamped to the track.
pub fn drag_distance(
    pointer: f32,
    grab: f32,
    lane_top: f32,
    track: f32,
    thumb: f32,
    max: f32,
) -> f32 {
    let room = track - thumb;
    if room <= 0. || max <= 0. {
        return 0.;
    }
    (pointer - grab - lane_top).clamp(0., room) / room * max
}

/// The scroll distance after a press on the track outside the thumb: one page (`track`) toward the press, from
/// `current`, clamped to `0..=max`. A press above the thumb pages up, one below it pages down.
pub fn page_distance(press: f32, thumb_top: f32, current: f32, track: f32, max: f32) -> f32 {
    let next = if press < thumb_top {
        current - track
    } else {
        current + track
    };
    next.clamp(0., max.max(0.))
}

/// Where a thumb being dragged was grabbed: how far below its top the press took it. It lives as long as the
/// list does, across frames, in the keyed state of its frame.
#[derive(Default)]
struct Drag {
    grab: Option<Pixels>,
}

/// A list of `rows` in a viewport of at most `max_h`, scrolled through `handle`. The caller keeps the handle
/// across frames, and scrolls it to a row with `ScrollHandle::scroll_to_item` (the index counts the rows given
/// here, in order). The rows keep their own size and look: the area only clips and scrolls them.
pub fn scroll_area<E: IntoElement>(
    id: impl Into<ElementId>,
    handle: &ScrollHandle,
    t: Theme,
    max_h: Pixels,
    rows: impl IntoIterator<Item = E>,
) -> Stateful<Div> {
    let id: ElementId = id.into();
    let list = div()
        .id((id.clone(), "list"))
        .max_h(max_h)
        .overflow_y_scroll()
        .track_scroll(handle)
        .children(rows);
    div()
        .id(id)
        .relative()
        .w_full()
        // A wheel over the list refreshes the window, and the thumb follows the hover.
        .on_mouse_move(|_, window, _| window.refresh())
        .child(list)
        .child(thumb(handle.clone(), t))
}

/// The thumb: painted once the list is laid out, so it reads this frame's viewport and offset. Its presses and
/// drags are listeners of this frame, registered with the paint.
fn thumb(handle: ScrollHandle, t: Theme) -> impl IntoElement {
    canvas(
        |_, window, cx| window.use_keyed_state("thumb-drag", cx, |_, _| Drag::default()),
        move |_, drag: Entity<Drag>, window, cx| {
            let view = handle.bounds();
            let overflow = handle.max_offset().y;
            if overflow <= px(0.) || view.size.height <= px(0.) {
                drag.update(cx, |d, _| d.grab = None);
                return;
            }
            let track = view.size.height;
            let content = track + overflow;
            let len = (track * (track / content)).max(px(THUMB_MIN)).min(track);
            // The offset is zero at the top and minus `overflow` at the bottom.
            let along = (-handle.offset().y / overflow).clamp(0., 1.);
            let top = view.top() + (track - len) * along;
            let left = view.right() - px(THUMB_W + THUMB_INSET);
            let bar = Bounds::new(point(left, top), size(px(THUMB_W), len));
            let lane = Bounds::new(
                point(view.right() - px(LANE_W), view.top()),
                size(px(LANE_W), track),
            );
            let grip = Bounds::new(point(view.right() - px(LANE_W), top), size(px(LANE_W), len));
            let grab = drag.read(cx).grab;
            let hot = view.contains(&window.mouse_position());
            let alpha = if grab.is_some() {
                0.9
            } else if hot {
                0.7
            } else {
                0.4
            };
            window.paint_quad(fill(bar, t.dim.opacity(alpha)).corner_radii(px(THUMB_W / 2.)));

            // A press on the thumb grabs it; a press elsewhere in the lane pages toward it.
            let (press_handle, press_drag) = (handle.clone(), drag.clone());
            window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
                if phase != DispatchPhase::Bubble
                    || e.button != MouseButton::Left
                    || !lane.contains(&e.position)
                {
                    return;
                }
                if grip.contains(&e.position) {
                    let grab = e.position.y - bar.top();
                    press_drag.update(cx, |d, _| d.grab = Some(grab));
                } else {
                    let next = page_distance(
                        f32::from(e.position.y),
                        f32::from(bar.top()),
                        f32::from(-press_handle.offset().y),
                        f32::from(track),
                        f32::from(overflow),
                    );
                    press_handle.set_offset(point(press_handle.offset().x, px(-next)));
                }
                cx.stop_propagation();
                window.refresh();
            });

            // A drag moves the list with the pointer, wherever it goes in the window; a release, or a
            // button that is no longer down, ends it.
            let (move_handle, move_drag) = (handle.clone(), drag.clone());
            window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                if phase != DispatchPhase::Capture {
                    return;
                }
                let Some(grab) = move_drag.read(cx).grab else {
                    return;
                };
                if e.pressed_button != Some(MouseButton::Left) {
                    move_drag.update(cx, |d, _| d.grab = None);
                    window.refresh();
                    return;
                }
                let next = drag_distance(
                    f32::from(e.position.y),
                    f32::from(grab),
                    f32::from(view.top()),
                    f32::from(track),
                    f32::from(len),
                    f32::from(overflow),
                );
                move_handle.set_offset(point(move_handle.offset().x, px(-next)));
                window.refresh();
            });

            let release = drag.clone();
            window.on_mouse_event(move |e: &MouseUpEvent, phase, window, cx| {
                if phase == DispatchPhase::Bubble
                    && e.button == MouseButton::Left
                    && release.read(cx).grab.is_some()
                {
                    release.update(cx, |d, _| d.grab = None);
                    window.refresh();
                }
            });
        },
    )
    .absolute()
    .size_full()
}

#[cfg(test)]
mod tests {
    use super::{drag_distance, page_distance};

    #[test]
    fn a_dragged_thumb_maps_the_pointer_to_the_offset() {
        // A 100 px lane under a 40 px thumb, over 300 px of overflow: the thumb's top runs 60 px.
        let at = |pointer: f32, grab: f32| drag_distance(pointer, grab, 0., 100., 40., 300.);
        // Grabbed at the top of the thumb: the top at the pointer.
        assert_eq!(at(0., 0.), 0.);
        assert_eq!(at(30., 0.), 150.);
        assert_eq!(at(60., 0.), 300.);
        // Grabbed 10 px down the thumb: the pointer moves the thumb's top by the same amount.
        assert_eq!(at(10., 10.), 0.);
        assert_eq!(at(40., 10.), 150.);
        assert_eq!(at(70., 10.), 300.);
        // Clamped to the track: above it, 0; below it, the overflow.
        assert_eq!(at(-50., 0.), 0.);
        assert_eq!(at(500., 0.), 300.);
        // The lane's top is subtracted: a pointer 20 px into the lane is 20 px of its 60 px room.
        assert_eq!(drag_distance(120., 0., 100., 100., 40., 300.), 100.);
    }

    #[test]
    fn a_list_that_does_not_overflow_does_not_drag() {
        assert_eq!(drag_distance(50., 0., 0., 100., 40., 0.), 0.);
        assert_eq!(drag_distance(50., 0., 0., 40., 40., 300.), 0.);
    }

    #[test]
    fn a_press_on_the_track_pages_toward_it() {
        // Lane of 100 px, thumb at 10..50, a list at 120 px of 300 overflow.
        assert_eq!(page_distance(5., 10., 120., 100., 300.), 20.);
        assert_eq!(page_distance(80., 10., 120., 100., 300.), 220.);
        // Clamped at both ends.
        assert_eq!(page_distance(5., 10., 30., 100., 300.), 0.);
        assert_eq!(page_distance(80., 10., 250., 100., 300.), 300.);
    }
}
