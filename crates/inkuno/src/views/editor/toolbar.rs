//! The selection toolbar: a popover 8 px above a non-empty selection, with Text ▾ (the block kinds),
//! Bold, Italic, Strikethrough, Code, Link and Comment. It runs the editor's own commands, and hides
//! while typing, on scroll and when the selection collapses.

use super::*;
use crate::logic::block::Turn;
use crate::ui::widgets;
use gpui::{AnyElement, Div, FontWeight, MouseDownEvent};

/// Height of the toolbar's row and the gap between it and the selection.
const HEIGHT: f32 = 36.;
const GAP: f32 = 8.;
/// Width of the toolbar: Text ▾, six buttons and two dividers.
const WIDTH: f32 = 262.;

/// What a button of the toolbar does.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tool {
    Bold,
    Italic,
    Strike,
    Code,
    Link,
    Comment,
    /// A block kind chosen in Text ▾.
    Turn(Turn),
}

/// The Text ▾ choices, in their order: the block kinds the toolbar offers.
const TURNS: [(&str, &str, Turn); 8] = [
    ("Paragraph", "Paragraphe", Turn::Text),
    ("Heading 1", "Titre 1", Turn::Heading(1)),
    ("Heading 2", "Titre 2", Turn::Heading(2)),
    ("Heading 3", "Titre 3", Turn::Heading(3)),
    ("Bullet list", "Liste à puces", Turn::Bullet),
    ("Numbered list", "Liste numérotée", Turn::Ordered),
    ("Task", "Tâche à cocher", Turn::Task),
    ("Quote", "Citation", Turn::Quote),
];

impl Editor {
    /// Runs a toolbar button on the selection. The change it makes does not hide the toolbar: only
    /// typing and scrolling do.
    pub(super) fn run_tool(&mut self, tool: Tool, cx: &mut Context<Self>) {
        self.toolbar_text = false;
        match tool {
            Tool::Bold => self.each(cx, |this, cx| this.mark(md::BOLD, cx)),
            Tool::Italic => self.each(cx, |this, cx| this.mark(md::ITALIC, cx)),
            Tool::Strike => self.each(cx, |this, cx| this.mark(md::STRIKE, cx)),
            Tool::Code => self.each(cx, |this, cx| this.mark(md::CODE, cx)),
            Tool::Link => self.link(cx),
            Tool::Comment => self.comment(cx),
            Tool::Turn(turn) => self.block_op(BlockOp::Turn(turn), cx),
        }
        self.toolbar_off = false;
        cx.notify();
    }

    /// Window point where byte `i` is, in the middle of its line (`point_of`, in every build).
    fn point_at(&self, i: usize) -> Option<Point<Pixels>> {
        let row = &self.rows[self.row_at(i)?];
        Some(self.origin + point(px(0.), row.y + row.pad + row.lh / 2.) + row.pos(i - row.start))
    }

    /// Where the toolbar goes, in the window's own coordinates: centred over the selection's start, above
    /// its line, or below when there is no room above. `None` while the selection is off screen or
    /// the toolbar is hidden.
    fn toolbar_at(&self) -> Option<Point<Pixels>> {
        let t = self.theme;
        // A table's handles are grabbed or picked: they keep the space above the table.
        let handles = self.pick.is_some() || self.moving.is_some();
        if self.sel.is_empty()
            || self.toolbar_off
            || !self.more.is_empty()
            || self.readonly
            || handles
        {
            return None;
        }
        let start = self.row_at(self.sel.start)?;
        let row = &self.rows[start];
        let from = self.point_at(self.sel.start)?;
        let to = self.point_at(self.sel.end).unwrap_or(from);
        let same = self.row_at(self.sel.end) == Some(start);
        // Centred on the selection when it is on one line, otherwise over where it starts.
        let mid = if same { (from.x + to.x) / 2. } else { from.x };
        let width = t.px(WIDTH);
        let viewport = self.viewport;
        let left = (mid - width / 2.)
            .max(viewport.left() + t.px(8.))
            .min(viewport.right() - width - t.px(8.));
        let line_top = from.y - row.lh / 2.;
        let line_bottom = from.y + row.lh / 2.;
        let above = line_top - t.px(GAP) - t.px(HEIGHT);
        let y = if above >= viewport.top() {
            above
        } else {
            line_bottom + t.px(GAP)
        };
        if line_top < viewport.top() || line_top > viewport.bottom() {
            return None;
        }
        Some(point(left, y))
    }

    /// The toolbar, when a selection calls for it: a row of buttons, and Text ▾'s list under it while open.
    pub(super) fn selection_toolbar(&self, cx: &mut Context<Self>) -> Option<Div> {
        let t = self.theme;
        let at = self.toolbar_at()? - self.viewport.origin;
        let button = |id: &'static str, content: AnyElement, tool: Tool, cx: &mut Context<Self>| {
            div()
                .id(id)
                .debug_selector(move || format!("toolbar-{id}"))
                .h(t.px(28.))
                .min_w(t.px(28.))
                .px(t.px(6.))
                .rounded(t.small())
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_color(t.text)
                .hover(|s| s.bg(t.hover))
                .child(content)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.run_tool(tool, cx)
                    }),
                )
        };
        let letter = |text: &'static str| div().text_size(t.px(14.)).child(text).into_any_element();
        let icon = |path: &'static str| {
            gpui::svg()
                .path(path)
                .size(t.px(16.))
                .text_color(t.text)
                .into_any_element()
        };
        let divider = || div().w(t.px(1.)).h(t.px(16.)).mx(t.px(2.)).bg(t.border);
        let open = self.toolbar_text;
        let text_btn = div()
            .id("toolbar-text")
            .debug_selector(|| "toolbar-text".into())
            .h(t.px(28.))
            .px(t.px(8.))
            .rounded(t.small())
            .flex()
            .items_center()
            .gap(t.px(4.))
            .cursor_pointer()
            .text_size(t.px(13.))
            .text_color(t.text)
            .when(open, |d| d.bg(t.selection))
            .hover(|s| s.bg(t.hover))
            .child(tr("Text", "Texte"))
            .child(
                gpui::svg()
                    .path("chevron-down.svg")
                    .size(t.px(14.))
                    .text_color(t.soft),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.toolbar_text = !this.toolbar_text;
                    cx.notify();
                }),
            );
        let bar = widgets::popover(t)
            .debug_selector(|| "selection-toolbar".into())
            .absolute()
            .left(at.x)
            .top(at.y)
            .w(t.px(WIDTH))
            .h(t.px(HEIGHT))
            .p(t.px(4.))
            .flex()
            .items_center()
            .gap(t.px(2.))
            .text_size(t.px(13.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(text_btn)
            .child(divider())
            .child(button("tb-bold", letter("B"), Tool::Bold, cx).font_weight(FontWeight::BOLD))
            .child(button("tb-italic", letter("I"), Tool::Italic, cx).italic())
            .child(button("tb-strike", letter("S"), Tool::Strike, cx).line_through())
            .child(button("tb-code", icon("code.svg"), Tool::Code, cx))
            .child(divider())
            .child(button("tb-link", icon("link.svg"), Tool::Link, cx))
            .child(button("tb-comment", icon("comment.svg"), Tool::Comment, cx));
        let list = open.then(|| {
            let rows = TURNS.iter().map(|&(en, fr, turn)| {
                let id = en;
                widgets::menu_row(t)
                    .id(id)
                    .debug_selector(move || format!("turn-{id}"))
                    .flex()
                    .items_center()
                    .text_size(t.px(13.))
                    .text_color(t.text)
                    .hover(|s| s.bg(t.hover))
                    .child(tr(en, fr))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                            cx.stop_propagation();
                            this.run_tool(Tool::Turn(turn), cx)
                        }),
                    )
            });
            widgets::popover(t)
                .debug_selector(|| "turn-menu".into())
                .absolute()
                .left(at.x + t.px(4.))
                .top(at.y + t.px(HEIGHT + 4.))
                .w(t.px(190.))
                .py(t.px(5.))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .children(rows)
        });
        Some(div().absolute().inset_0().child(bar).children(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_offers_the_block_kinds_in_order() {
        let names: Vec<&str> = TURNS.iter().map(|t| t.0).collect();
        assert_eq!(names.first(), Some(&"Paragraph"));
        assert_eq!(names.last(), Some(&"Quote"));
        assert_eq!(names.len(), 8);
    }
}
