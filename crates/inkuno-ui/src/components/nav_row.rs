//! Navigation rows of the sidebar. `.row` in the Quiet sidebar: 30 px, an 18 px icon, the label, an optional key
//! hint at the right. Selected, a neutral pill and the full text; hovered, the hover token.

use gpui::{Div, InteractiveElement, Stateful, div, prelude::*, svg};

use crate::theme::Theme;

/// A row of the Quiet sidebar: 30 px, an 18 px icon 9 px before the 13 px label (text at 85 %), an optional
/// key hint at the right. Selected, a neutral pill and the full text; hovered, the hover token.
pub fn nav_row(
    id: &'static str,
    icon: &'static str,
    label: String,
    key: String,
    on: bool,
    t: Theme,
) -> Stateful<Div> {
    let ink = if on { t.text } else { t.text.opacity(0.85) };
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .h(t.px(30.))
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .gap(t.px(9.))
        .px(t.px(8.))
        .rounded(t.large())
        .text_size(t.px(13.))
        .text_color(ink)
        .cursor_pointer()
        .when(on, |d| d.bg(t.pill))
        .when(!on, |d| d.hover(|s| s.bg(t.hover)))
        .child(svg().path(icon).size(t.px(18.)).flex_none().text_color(ink))
        .child(div().flex_1().min_w_0().truncate().child(label))
        .when(!key.is_empty(), |d| {
            d.child(
                div()
                    .flex_none()
                    .text_size(t.px(11.))
                    .text_color(t.soft)
                    .child(key),
            )
        })
}
