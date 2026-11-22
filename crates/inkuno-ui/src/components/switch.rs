//! Switches. `.sw` in v2: 34 by 20, accent when on, a knob that slides under the pointer's click.

use gpui::{Div, InteractiveElement, Stateful, div, prelude::*};

use crate::theme::Theme;

/// A switch, 34 by 20: accent when on. `id` names it; the click is the caller's (`.on_click(...)`).
pub fn switch(id: &str, on: bool, t: Theme) -> Stateful<Div> {
    div()
        .id(gpui::SharedString::from(id.to_string()))
        .debug_selector(|| id.to_string())
        .relative()
        .flex_none()
        .w(t.px(34.))
        .h(t.px(20.))
        .rounded_full()
        .cursor_pointer()
        .bg(if on { t.accent } else { t.text.opacity(0.22) })
        .child(
            div()
                .absolute()
                .top(t.px(2.))
                .left(if on { t.px(16.) } else { t.px(2.) })
                .size(t.px(16.))
                .rounded_full()
                .bg(t.bg)
                .shadow(t.lifted()),
        )
}
