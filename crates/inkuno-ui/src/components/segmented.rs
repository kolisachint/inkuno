//! Segmented controls. `.seg` in v2: a track of options, the chosen one lifted above it. The track
//! sits on the ground and the chosen option is the raised panel colour, lighter than the track in
//! both themes (a dark chip on a track reads as a hole).

use gpui::{Div, InteractiveElement, IntoElement, Stateful, div, prelude::*};

use crate::theme::Theme;

/// The track: a 2 px inset and a `large` radius, holding the options of `segment`.
pub fn segmented(t: Theme, options: Vec<Stateful<Div>>) -> Div {
    div()
        .flex_none()
        .flex()
        .gap(t.px(2.))
        .p(t.px(2.))
        .rounded(t.large())
        .bg(t.ground)
        .children(options)
}

/// One option: `name` is its id and debug selector; `on` lifts it. The click is the caller's.
pub fn segment(name: String, label: impl IntoElement, on: bool, t: Theme) -> Stateful<Div> {
    div()
        .id(gpui::SharedString::from(name.clone()))
        .debug_selector(move || name.clone())
        .h(t.px(26.))
        .px(t.px(10.))
        .flex()
        .items_center()
        .rounded(t.small())
        .text_size(t.px(12.))
        .cursor_pointer()
        .when(on, |d| {
            d.bg(t.popover).text_color(t.text).shadow(t.lifted())
        })
        .when(!on, |d| {
            d.text_color(t.soft).hover(|s| s.text_color(t.text))
        })
        .child(label)
}
