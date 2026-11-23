//! Floating surfaces. `.pop` in v2: what hovers over the page (the palette, the menus, the help, the toasts).

use gpui::{Div, div, prelude::*};

use crate::theme::Theme;

/// What floats over the page: the palette, the menus, the help, the toasts, a dragged tab. `bg`, the
/// sheet radius, a hairline ring of `border` and the `floating` shadow. The caller sets its size and place.
pub fn floating(t: Theme) -> Div {
    div()
        .bg(t.bg)
        .rounded(t.sheet())
        .border_1()
        .border_color(t.border)
        .shadow(t.floating())
}

/// A menu or the palette: the raised `popover` fill, a 1 px line of the text at 8 %, the sheet radius
/// and two shadows (0 1 3 at 5 %, 0 8 24 at 10 %).
pub fn popover(t: Theme) -> Div {
    div()
        .bg(t.popover)
        .rounded(t.sheet())
        .border_1()
        .border_color(t.text.opacity(0.08))
        .shadow(t.popover_shadow())
}
