//! Icon buttons. `.ib` in v2: a round 24 px target for panel headers and note copy (`icon_button`), and the
//! 30 px square, radius 8, of the sidebar bands and the tab strip (`band_button`), which has a ring when it has the keys.

use gpui::{Div, InteractiveElement, MouseButton, Stateful, div, prelude::*, svg};

use crate::theme::Theme;
use crate::widgets::Press;

/// Round icon button: window controls, note copy; 24 px, the smallest target a finger or a shaky hand hits.
pub fn icon_button(id: &'static str, icon: &'static str, t: Theme) -> Stateful<Div> {
    icon_button_over(id, icon, t.hover, t)
}

/// The same, with its own colour under the pointer (the close button's red).
pub fn icon_button_over(
    id: &'static str,
    icon: &'static str,
    over: gpui::Hsla,
    t: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .size(t.px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .hover(|s| s.bg(over))
        .active(|s| s.bg(t.pressed))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            svg()
                .path(icon)
                .size(t.px(16.))
                .flex_none()
                .text_color(t.text),
        )
}

/// Icon button of the sidebar bands and of panel headers. Tab reaches it, a ring shows it has the keys, Enter
/// or Space presses it; a click does not take the focus from the note.
pub fn band_button(
    id: impl Into<gpui::ElementId>,
    icon: &'static str,
    active: bool,
    t: Theme,
) -> Stateful<Div> {
    band_button_turned(id, icon, active, false, t)
}

/// The same, its icon turned half a way round: one drawing for the two ends of an arrow.
pub fn band_button_turned(
    id: impl Into<gpui::ElementId>,
    icon: &'static str,
    active: bool,
    turned: bool,
    t: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .size(t.px(30.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(t.px(8.))
        .cursor_pointer()
        .tab_index(0)
        .focus(|s| s.border_2().border_color(t.accent))
        // Enter belongs to the button that has the keys, not to the panel around it: gpui presses
        // it when the key comes up.
        .key_context("Button")
        .on_action(|_: &Press, _, _| {})
        .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
        .hover(|s| s.bg(t.hover))
        .active(|s| s.bg(t.pressed))
        .when(active, |d| d.bg(t.selection))
        .child(
            svg()
                .path(icon)
                .size(t.px(16.))
                .flex_none()
                .when(turned, |s| {
                    s.with_transformation(gpui::Transformation::rotate(gpui::radians(
                        std::f32::consts::PI,
                    )))
                })
                .text_color(if active { t.accent } else { t.soft }),
        )
}
