//! List rows. `.row` in v2: the full-width row of the palette and the lists, with the selection pill or the
//! hover under it. The height, the content and the click are the caller's.

use gpui::{Div, div, prelude::*};

use crate::theme::Theme;

/// A row of a list: a 6 px side margin, 12 px padding and a `large` radius; selected, the selection tint,
/// otherwise the hover tint.
pub fn list_row(on: bool, t: Theme) -> Div {
    div()
        .mx(t.px(6.))
        .px_3()
        .rounded(t.large())
        .cursor_pointer()
        .when(on, |d| d.bg(t.hover))
        .when(!on, |d| d.hover(|s| s.bg(t.hover)))
}
