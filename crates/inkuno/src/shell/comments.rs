//! The Comments view of the right panel: one card per comment of the note, with the passage it is on,
//! what is said and Resolve. A card leads to its comment in the note and flashes the passage; the
//! comment the caret is in is the card that is focused.

use std::{ops::Range, time::Duration};

use gpui::{Context, Div, Stateful, Window, div, prelude::*};

use super::*;
use crate::lang::tr;

/// How long a passage stays selected when its card is clicked.
const FLASH: Duration = Duration::from_millis(900);

/// One comment of the note, as the Comments view draws it (`markdown::all_comments`).
#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    /// The commented text; empty for a comment on its own.
    pub noted: String,
    /// What is said, without the name of the row or column it starts with.
    pub said: String,
    /// The byte where the whole starts, the one where the comment starts, and the one where it ends.
    pub at: usize,
    pub inside: usize,
    pub end: usize,
    /// What it is on, in a table (`Cell C2`); empty elsewhere.
    pub scope: String,
    /// The passage the card is about, in the note's bytes: the commented text (after its `{==` in the
    /// CriticMarkup form, from the start in the HTML form), or the whole comment when it has none.
    pub passage: Range<usize>,
}

impl Shell {
    /// The Comments view: the cards of the note, the focused one outlined.
    pub(super) fn render_comments(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        if self.comment_cards.is_empty() {
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_start()
                .justify_center()
                .p_3()
                .child(
                    div()
                        .debug_selector(|| "comments-empty".into())
                        .text_size(t.px(13.))
                        .text_color(t.soft)
                        .child(tr("This note has no comments", "Cette note n'a pas de commentaires")),
                )
                .into_any_element();
        }
        let focus = self.comment_focus;
        let cards = self.comment_cards.clone();
        div()
            .id("comments-view")
            .debug_selector(|| "comments-view".into())
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.comments_scroll)
            .px_2()
            .pb_3()
            .flex()
            .flex_col()
            .gap(t.px(6.))
            .children(
                cards
                    .iter()
                    .enumerate()
                    .map(|(i, card)| self.comment_card(i, card, focus == Some(i), cx)),
            )
            .into_any_element()
    }

    /// One card: the quoted passage (or the table's place), what is said, Resolve. A click on the card
    /// goes to the comment and flashes its passage.
    fn comment_card(
        &self,
        i: usize,
        card: &Card,
        focused: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let (at, inside, passage) = (card.at, card.inside, card.passage.clone());
        let group = format!("comment-{i}");
        // Where it is, in a table, and the passage it is on (or that place, for a comment on a row).
        // The passage as it reads: its block and inline markers go, as in the outline.
        let quote = match card.noted.is_empty() {
            true => card.scope.clone(),
            false => crate::logic::rank::plain_line(&card.noted),
        };
        let said = match card.said.is_empty() {
            true => tr("(nothing written yet)", "(rien d'écrit pour l'instant)").to_string(),
            false => card.said.clone(),
        };
        let said_soft = card.said.is_empty();
        div()
            .id(("comment", i))
            .debug_selector(move || format!("comment-{i}"))
            .group(group)
            .flex()
            .flex_col()
            .gap(t.px(6.))
            .px(t.px(12.))
            .py(t.px(10.))
            .rounded(t.large())
            .border_1()
            .border_color(if focused { t.accent } else { t.border })
            .when(focused, |d| d.border_2())
            .bg(t.bg)
            .cursor_pointer()
            .hover(|s| s.bg(t.hover))
            .when(!card.scope.is_empty() && !card.noted.is_empty(), |d| {
                d.child(
                    div()
                        .text_size(t.px(11.))
                        .text_color(t.dim)
                        .child(card.scope.clone()),
                )
            })
            .when(!quote.is_empty(), |d| {
                d.child(
                    div()
                        .min_w_0()
                        .pl(t.px(8.))
                        .border_l_2()
                        .border_color(t.accent)
                        .text_size(t.px(12.))
                        .text_color(t.soft)
                        .line_clamp(2)
                        .text_ellipsis()
                        .child(quote),
                )
            })
            .child(
                div()
                    .text_size(t.px(13.))
                    .text_color(if said_soft { t.soft } else { t.text })
                    .child(said),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id(("resolve", i))
                            .debug_selector(move || format!("resolve-{i}"))
                            .h(t.px(24.))
                            .px(t.px(9.))
                            .flex()
                            .items_center()
                            .rounded(t.small())
                            .border_1()
                            .border_color(t.border)
                            .text_size(t.px(12.))
                            .text_color(t.text)
                            .cursor_pointer()
                            .hover(|s| s.bg(t.hover))
                            .child(tr("Resolve", "Résoudre"))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.pane()
                                    .editor
                                    .update(cx, |e, cx| e.resolve_comment(at, cx));
                                window.focus(&this.pane().editor.focus_handle(cx));
                            })),
                    ),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.comment_go(i, inside, passage.clone(), window, cx)
            }))
    }

    /// A card was clicked: the caret goes to its comment, its passage is selected for `FLASH`, and
    /// the editor takes the keys back.
    pub(super) fn comment_go(
        &mut self,
        i: usize,
        inside: usize,
        passage: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.comment_focus = Some(i);
        let editor = self.pane().editor.clone();
        editor.update(cx, |e, cx| {
            e.jump(inside, false, cx);
            e.select_passage(passage.clone(), cx);
        });
        let back = inside;
        let flash = passage.clone();
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(FLASH).await;
            editor
                .update(cx, |e, cx| e.flash_end(flash, back, cx))
                .ok();
        })
        .detach();
        window.focus(&self.pane().editor.focus_handle(cx));
        cx.notify();
    }

    /// A click on a commented passage in the note (`EditorEvent::CommentAt`): the Comments view opens on its card.
    pub(super) fn comment_clicked(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.comment_focus = self.comment_cards.iter().position(|c| c.at == at);
        self.show_view(Mode::Comments, window, cx);
        if let Some(i) = self.comment_focus {
            self.comments_scroll.scroll_to_item(i);
        }
    }

    /// The card to focus when the Comments view opens: the one the caret is in, else the one clicked last,
    /// else the first.
    pub(super) fn focus_comment_card(&mut self) {
        let cards = self.comment_cards.len();
        self.comment_focus = self
            .comments_seen
            .or(self.comment_focus)
            .filter(|i| *i < cards)
            .or(Some(0));
    }

    /// Opens the right panel on the Comments view, the card of the caret focused.
    pub(super) fn show_comments(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.focus_comment_card();
        self.show_view(Mode::Comments, window, cx);
        if let Some(i) = self.comment_focus {
            self.comments_scroll.scroll_to_item(i);
        }
    }

    /// The note's corner badge: one quiet dot, on the note's foot on the right (none without comments).
    pub(super) fn render_comments_badge(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let t = self.theme;
        let count = self.comment_cards.len();
        (count > 0 && !self.focus_mode).then(|| {
            div()
                .id("comments-toggle")
                .debug_selector(|| "comments-toggle".into())
                .tooltip(ui::widgets::tip(
                    tr("Comments of the note", "Commentaires de la note"),
                    super::render::badge_label(count),
                    t,
                ))
                .absolute()
                .bottom_4()
                .right(t.px(12.))
                .size(t.px(30.))
                .rounded(t.large())
                .cursor_pointer()
                .occlude()
                .child(
                    div()
                        .debug_selector(|| "comments-badge".into())
                        .absolute()
                        .top(t.px(-3.))
                        .right(t.px(-3.))
                        .min_w(t.px(15.))
                        .h(t.px(15.))
                        .rounded_full()
                        .bg(t.accent),
                )
                .on_click(cx.listener(|this, _, window, cx| this.show_comments(window, cx)))
        })
    }
}
