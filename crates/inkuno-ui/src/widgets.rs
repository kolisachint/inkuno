//! Small widgets shared by the views: the tooltips, the window-drag handle, the search fields. The pieces that
//! recur (icon buttons, key hints, popovers, menu rows) live in `components`; they are re-exported here so
//! the views keep one path.

use gpui::{
    AnyView, App, Context, Div, InteractiveElement, MouseButton, SharedString, Stateful, Window,
    div, prelude::*,
};

use crate::theme::Theme;

pub use crate::components::icon_button::{
    band_button as button, band_button_turned as button_turned, icon_button, icon_button_over,
};
pub use crate::components::kbd::keycaps;
pub use crate::components::menu::menu_row;
pub use crate::components::popover::{floating, popover};

/// Tooltip of an icon button: what it does and, if it has one, its shortcut. Without it,
/// a bare icon forces you to know the documentation to know where to click.
pub struct Tip {
    label: SharedString,
    keys: String,
    theme: Theme,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        div()
            .px_2()
            .py(t.px(3.))
            .rounded(t.small())
            .bg(t.panel)
            .border_1()
            .border_color(t.border)
            .shadow(t.lifted())
            .text_size(t.px(12.))
            .text_color(t.text)
            .flex()
            .items_center()
            .gap_2()
            .child(self.label.clone())
            .child(keycaps(&self.keys, t, false))
    }
}

/// To put on a button: `.tooltip(tip(label, keys, theme))`.
pub fn tip(
    label: impl Into<SharedString>,
    keys: String,
    theme: Theme,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let label = label.into();
    move |_, cx| {
        cx.new(|_| Tip {
            label: label.clone(),
            keys: keys.clone(),
            theme,
        })
        .into()
    }
}
/// The tint of what is open (a pill, the open row of the panel): the accent at 10 %, quieter than the selection.
pub fn pill_tint(t: Theme) -> gpui::Hsla {
    t.accent.opacity(0.1)
}

/// The wash of a commented passage: v2's yellow `--cmw`, told from the selection and read under the text.
pub fn passage_tint(t: Theme) -> gpui::Hsla {
    t.comment_wash
}

/// Turns an element into a window handle, where the app draws its title bar
/// (Linux): dragging moves it, double-click maximizes it, right-click opens the system menu.
pub fn drag_window<E: InteractiveElement>(element: E) -> E {
    element
        .on_mouse_down(MouseButton::Left, |e, window, _| {
            if e.click_count == 2 {
                window.zoom_window()
            } else {
                window.start_window_move()
            }
        })
        .on_mouse_down(MouseButton::Right, |e, window, _| {
            window.show_window_menu(e.position)
        })
}

/// The frame of a one-line field (the find bar, the vault search): `accent` border when it has the keys, the
/// hint in `soft` while it is empty.
pub fn field_frame(
    id: &'static str,
    field: impl IntoElement,
    on: bool,
    empty: bool,
    hint: &'static str,
    t: Theme,
) -> Div {
    div()
        .debug_selector(move || id.to_string())
        .flex_1()
        .min_w(t.px(80.))
        .h(t.px(26.))
        .px_2()
        .rounded(t.small())
        .border_1()
        .bg(t.bg)
        .border_color(if on { t.accent } else { t.border })
        .relative()
        .flex()
        .items_center()
        .overflow_hidden()
        .child(field)
        .when(empty, |d| {
            d.child(div().absolute().left_2().text_color(t.soft).child(hint))
        })
}

/// A search setting shown as its sign (`Aa`, `.*`): lit when on, its name and keys in the tooltip.
pub fn toggle(
    id: &'static str,
    label: &'static str,
    on: bool,
    name: &'static str,
    keys: String,
    t: Theme,
) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .flex_none()
        .h(t.px(26.))
        .min_w(t.px(26.))
        .px_1p5()
        .rounded(t.small())
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(if on { t.accent } else { t.soft })
        .when(on, |d| d.bg(t.selection))
        .hover(|s| s.bg(t.hover))
        .tooltip(tip(name, keys, t))
        .child(label)
}

/// How long a fade lasts: under a key repeat, so a new gesture never waits for the last one.
pub const FADE: std::time::Duration = std::time::Duration::from_millis(130);

/// `element` fades in when it first appears under `id` (the state lives as long as it is drawn);
/// with `motion=off` it is drawn as is. Opacity only, so the layout never moves.
pub fn fade_in(
    element: impl IntoElement + Styled + 'static,
    id: impl Into<gpui::ElementId>,
    t: &Theme,
) -> gpui::AnyElement {
    use gpui::{Animation, AnimationExt, ease_out_quint};
    match t.motion {
        true => element
            .with_animation(
                id,
                Animation::new(FADE).with_easing(ease_out_quint()),
                |e, delta| e.opacity(delta),
            )
            .into_any_element(),
        false => element.into_any_element(),
    }
}
