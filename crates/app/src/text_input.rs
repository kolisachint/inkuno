//! A one-line text field drawn with plain divs (tasks 9.3 and 9.4).
//!
//! The field holds no state. Its owner keeps the text and the keyboard focus and handles the
//! keys, so the same look serves the search query and the inline rename.

use gpui::{div, prelude::*, px, Div, Stateful};
use md_editor::theme::Theme;

/// A bordered box `height` pixels tall. It shows `text` with a caret after it, or
/// `placeholder` in muted colour, with the caret, when `text` is empty. Colours come from `theme`.
pub fn field(
    theme: &Theme,
    id: &'static str,
    text: &str,
    placeholder: &str,
    height: f32,
) -> Stateful<Div> {
    let caret = div().w(px(1.)).h(px(14.)).bg(theme.ui_text);
    let base = div()
        .id(id)
        .h(px(height))
        .px_2()
        .flex()
        .flex_row()
        .items_center()
        .gap_px()
        .overflow_hidden()
        .bg(theme.ui_surface_raised)
        .border_1()
        .border_color(theme.ui_border)
        .rounded_sm();
    if text.is_empty() {
        base.child(caret).child(
            div()
                .whitespace_nowrap()
                .text_color(theme.ui_muted)
                .child(placeholder.to_owned()),
        )
    } else {
        base.child(
            div()
                .whitespace_nowrap()
                .text_color(theme.ui_text)
                .child(text.to_owned()),
        )
        .child(caret)
    }
}
