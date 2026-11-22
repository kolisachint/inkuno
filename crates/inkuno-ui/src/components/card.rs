//! Settings cards. `.card` and `.sr` in v2: a rounded group of rows with a hairline between them; each row has
//! its label and a soft note under it, and the control at the right.

use gpui::{Div, FontWeight, IntoElement, div, prelude::*};

use crate::theme::Theme;

/// One row of a card: its label, a soft note under it, the control at the right.
pub fn card_row(t: Theme, label: &str, note: &str, control: impl IntoElement) -> Div {
    div()
        .min_h(t.px(52.))
        .px(t.px(14.))
        .py(t.px(12.))
        .flex()
        .items_center()
        .gap(t.px(16.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_size(t.px(13.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(t.text)
                        .child(label.to_string()),
                )
                .when(!note.is_empty(), |d| {
                    d.child(
                        div()
                            .text_size(t.px(12.))
                            .text_color(t.soft)
                            .child(note.to_string()),
                    )
                }),
        )
        .child(control)
}

/// A card of rows, a hairline between them.
pub fn card(t: Theme, rows: Vec<Div>) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(t.card())
        .border_1()
        .border_color(t.text.opacity(0.08))
        .overflow_hidden()
        .children(rows.into_iter().enumerate().map(|(i, row)| {
            if i == 0 {
                row
            } else {
                row.border_t_1().border_color(t.text.opacity(0.08))
            }
        }))
}
