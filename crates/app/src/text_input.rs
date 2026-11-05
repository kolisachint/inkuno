//! A one-line text field drawn with plain divs (tasks 9.3 and 9.4).
//!
//! The field holds no state. Its owner keeps the text and the keyboard focus and handles the
//! keys, so the same look serves the search query and the inline rename.

use gpui::{div, prelude::*, px, rgb, Div, Stateful};

const TEXT: u32 = 0x1f2937;
const MUTED_TEXT: u32 = 0x6b7280;

/// A bordered box `height` pixels tall. It shows `text` with a caret after it, or
/// `placeholder` in muted colour, with the caret, when `text` is empty.
pub fn field(id: &'static str, text: &str, placeholder: &str, height: f32) -> Stateful<Div> {
    let caret = div().w(px(1.)).h(px(14.)).bg(rgb(TEXT));
    let base = div()
        .id(id)
        .h(px(height))
        .px_2()
        .flex()
        .flex_row()
        .items_center()
        .gap_px()
        .overflow_hidden()
        .bg(gpui::white())
        .border_1()
        .border_color(rgb(0xd1d5db))
        .rounded_sm();
    if text.is_empty() {
        base.child(caret).child(
            div()
                .whitespace_nowrap()
                .text_color(rgb(MUTED_TEXT))
                .child(placeholder.to_owned()),
        )
    } else {
        base.child(
            div()
                .whitespace_nowrap()
                .text_color(rgb(TEXT))
                .child(text.to_owned()),
        )
        .child(caret)
    }
}
