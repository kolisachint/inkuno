//! Text buttons of the settings rows. `.btn` in v2: 28 px high, a hairline ring, a `large` radius and a hover tint.

use gpui::{Div, InteractiveElement, Stateful, div, prelude::*};

use crate::theme::Theme;

/// A text button: `id` names it (and its debug selector); the click is the caller's (`.on_click(...)`).
pub fn button(id: &str, label: impl IntoElement, t: Theme) -> Stateful<Div> {
    div()
        .id(gpui::SharedString::from(id.to_string()))
        .debug_selector(|| id.to_string())
        .flex_none()
        .h(t.px(28.))
        .px(t.px(12.))
        .flex()
        .items_center()
        .rounded(t.large())
        .border_1()
        .border_color(t.text.opacity(0.08))
        .text_size(t.px(13.))
        .text_color(t.text)
        .cursor_pointer()
        .hover(|s| s.bg(t.hover))
        .child(label)
}
