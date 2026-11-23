//! Menu rows. `.mi` and `.msep` in v2: a row with its label and its shortcut at the right, and a hairline
//! between groups. The hover and the press are the caller's, as is the click.

use gpui::{Div, Hsla, InteractiveElement, Stateful, div, prelude::*, svg};

use super::kbd::keycaps;
use crate::theme::Theme;

/// A row of a menu: 32 px with a 5 px side margin, 8 px padding and a 6 px radius. The hover and
/// the selection are the caller's.
pub fn menu_row(t: Theme) -> Div {
    div()
        .h(t.px(32.))
        .mx(t.px(5.))
        .px(t.px(8.))
        .rounded(t.small())
        .flex_none()
}

/// A menu item: its icon (16 px, the slot is kept without one), its label, then its shortcut, dimmer,
/// pinned to the right; the label is ellipsized, the menu takes the width they need. `ink`: the colour
/// of the label when the item has one (a danger item). `id` is the label, so the item is found by its name.
pub fn menu_item(
    label: &'static str,
    icon: Option<&'static str>,
    keys: &str,
    ink: Option<Hsla>,
    on: bool,
    t: Theme,
) -> Stateful<Div> {
    menu_row(t)
        .id(label)
        .debug_selector(move || format!("menu-entry-{label}"))
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(|s| s.bg(t.hover))
        .when(on, |d| d.bg(t.hover))
        .when_some(ink, |d, color| d.text_color(color))
        .child(
            div()
                .size(t.px(16.))
                .mr_2()
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .children(icon.map(|icon| {
                    svg()
                        .path(icon)
                        .size(t.px(16.))
                        .flex_none()
                        .text_color(ink.unwrap_or(t.soft))
                })),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(label),
        )
        .child(
            keycaps(keys, t, on)
                .debug_selector(move || format!("menu-keys-{label}"))
                .flex_none()
                .ml_4(),
        )
}

/// The hairline between two groups of a menu.
pub fn menu_separator(t: Theme) -> Div {
    div().h(t.px(1.)).my(t.px(4.)).bg(t.border)
}
