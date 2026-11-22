//! Key hints. `.kchip` in v2: one badge per key of a shortcut, in a row.

use gpui::{Div, div, prelude::*};

use crate::theme::Theme;

/// A shortcut ("Ctrl+Maj+C") as one badge per key, in a row; nothing when it is empty.
/// `on`: the row behind is selected, so the badges are lighter than it.
pub fn keycaps(keys: &str, t: Theme, on: bool) -> Div {
    let caps = inkuno_logic::commands::keycaps(keys)
        .into_iter()
        .map(|key| {
            div()
                .flex_none()
                .h(t.px(18.))
                .min_w(t.px(20.))
                .px(t.px(6.))
                .rounded(t.small())
                .flex()
                .items_center()
                .justify_center()
                .whitespace_nowrap()
                .bg(if on { t.panel.opacity(0.7) } else { t.code_bg })
                .text_size(t.px(11.5))
                .text_color(if on { t.text } else { t.soft })
                .child(div().flex_none().whitespace_nowrap().child(key))
        });
    div().flex().items_center().gap(t.px(3.)).children(caps)
}
