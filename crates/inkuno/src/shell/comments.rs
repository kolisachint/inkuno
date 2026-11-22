//! The Comments view of the right panel: one card per comment of the note, with the passage it is on,
//! what is said and Resolve. A card leads to its comment in the note and flashes the passage; the
//! comment the caret is in is the card that is focused.

use std::{ops::Range, time::Duration};

use gpui::{
    App, Context, Div, Entity, Stateful, Subscription, WeakEntity, Window, div, prelude::*,
};

use super::*;
use crate::lang::tr;
use crate::ui::components::button::button;
use crate::views::{editor::Editor, field::Field};

/// How long a passage stays selected when its card is clicked.
const FLASH: Duration = Duration::from_millis(900);

/// A comment being written: its draft card leads the Comments view. The passage it is on is quoted, what is
/// said goes in the field, Enter writes it into the note (`write_comment`) and Escape or Cancel takes its
/// markup out again (`resolve_comment`). The note holds the empty markup meanwhile, as it always did.
pub struct Draft {
    /// Where the caret is in the comment (a card's `inside`), kept to the note's text as it changes, and
    /// the passage, cell or row it is on.
    pub inside: usize,
    pub noted: String,
    /// The note the comment is in. Weak: a draft never keeps a closed tab's note alive, and it ends with
    /// the tab (`end_closed_draft`).
    pub editor: WeakEntity<Editor>,
    /// The note's text as the draft last saw it: what a change is measured against.
    text: String,
    pub field: Entity<Field>,
    _subscription: Subscription,
    _edits: Subscription,
}

/// Where byte `at` of `old` is in `new`, once the one edit that turns `old` into `new` is taken out (what they
/// share at the start and at the end). None when `at` is inside that edit: it has no place then.
fn shifted(old: &str, new: &str, at: usize) -> Option<usize> {
    let mut head = old
        .bytes()
        .zip(new.bytes())
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(head) || !new.is_char_boundary(head) {
        head -= 1;
    }
    let room = old.len().min(new.len()) - head;
    let mut tail = old
        .bytes()
        .rev()
        .zip(new.bytes().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    while !old.is_char_boundary(old.len() - tail) || !new.is_char_boundary(new.len() - tail) {
        tail -= 1;
    }
    let (old_end, new_end) = (old.len() - tail, new.len() - tail);
    if at <= head {
        Some(at)
    } else if at >= old_end {
        Some(new_end + (at - old_end))
    } else {
        None
    }
}

/// Whether the comment at byte `at` of `text` is an empty one, the marker a draft writes: the only comment
/// a draft writes into or takes out.
fn own_marker(text: &str, at: usize) -> bool {
    let Some(before) = text.get(..at) else {
        return false;
    };
    let start = before.rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let line = &text[start..end];
    let col = at - start;
    markdown::comments(line).iter().any(|(whole, _, said)| {
        // A comment on a table, a row or a column starts with its scope's name (`Column "Âge": `), and
        // its empty text follows that name: the name ends in a colon, and the text is at or after it.
        let written = line[said.clone()].trim_end();
        (whole.start..=whole.end).contains(&col)
            && (written.is_empty() || (written.ends_with(':') && col >= said.end))
    })
}

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

/// The byline a comment's text may open with, `Name, HH:MM: ` (the author and the time it was written):
/// the name, the time and what is said after them. None when the text does not open with one.
pub fn split_byline(said: &str) -> Option<(&str, &str, &str)> {
    let head_end = said.find(": ")?;
    let (name, time) = said[..head_end].rsplit_once(", ")?;
    let (name, time) = (name.trim(), time.trim());
    let clock = time.len() == 5
        && time.is_char_boundary(2)
        && time.as_bytes()[2] == b':'
        && time
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 2 || b.is_ascii_digit());
    (clock && !name.is_empty() && name.chars().count() <= 40 && !name.contains('\n'))
        .then(|| (name, time, &said[head_end + 2..]))
}

impl Shell {
    /// The Comments view: the cards of the note, the focused one outlined.
    pub(super) fn render_comments(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let draft = self.comment_draft.as_ref().map(|d| {
            let shown_here = d.editor.entity_id() == self.pane().editor.entity_id();
            (
                self.comment_draft_card(d, cx),
                shown_here.then_some(d.inside),
            )
        });
        if self.comment_cards.is_empty() && draft.is_none() {
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
                        .child(tr(
                            "This note has no comments",
                            "Cette note n'a pas de commentaires",
                        )),
                )
                .into_any_element();
        }
        let focus = self.comment_focus;
        let cards = self.comment_cards.clone();
        // The comment being written is listed by its draft card alone, not also by its empty card.
        let draft_inside = draft.as_ref().and_then(|(_, inside)| *inside);
        // v2 `.rbody`: 12 px at the sides, the cards 8 px apart.
        div()
            .id("comments-view")
            .debug_selector(|| "comments-view".into())
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.comments_scroll)
            .px(t.px(12.))
            .pt(t.px(6.))
            .pb(t.px(16.))
            .flex()
            .flex_col()
            .gap(t.px(8.))
            .children(draft.map(|(card, _)| card))
            .children(
                cards
                    .iter()
                    .enumerate()
                    .filter(|(_, card)| Some(card.inside) != draft_inside)
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
        // A text that opens with `Name, HH:MM: ` shows the name and the time on the card's first line.
        let (byline, body) = match split_byline(&card.said) {
            Some((name, time, rest)) => (Some((name.to_string(), time.to_string())), rest),
            None => (None, card.said.as_str()),
        };
        let said_soft = body.trim().is_empty();
        let said = match said_soft {
            true => tr("(nothing written yet)", "(rien d'écrit pour l'instant)").to_string(),
            false => body.to_string(),
        };
        div()
            .id(("comment", i))
            .debug_selector(move || format!("comment-{i}"))
            .group(group)
            .flex()
            .flex_col()
            .gap(t.px(6.))
            .px(t.px(12.))
            .py(t.px(10.))
            .rounded(t.card())
            .border_1()
            .border_color(if focused { t.accent } else { t.border })
            .when(focused, |d| d.border_2())
            .bg(t.bg)
            .cursor_pointer()
            .hover(|s| s.bg(t.hover))
            .when_some(byline, |d, (name, time)| {
                d.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(t.px(6.))
                        .text_size(t.px(12.))
                        .text_color(t.soft)
                        .child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(t.text)
                                .child(name),
                        )
                        .child(time),
                )
            })
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
                        .border_l_3()
                        .border_color(t.comment_gold)
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
                    .line_height(t.px(19.))
                    .text_color(if said_soft { t.soft } else { t.text })
                    .child(said),
            )
            .child(
                div().flex().items_center().child(
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

    /// The draft card of a comment being written: the passage in a gold bar, the field (Enter writes it),
    /// Comment (primary) and Cancel.
    fn comment_draft_card(&self, draft: &Draft, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let field = draft.field.clone();
        let blank = field.read(cx).line.text.is_empty();
        let quote = crate::logic::rank::plain_line(&draft.noted);
        div()
            .id("comment-draft")
            .debug_selector(|| "comment-draft".into())
            .flex()
            .flex_col()
            .gap(t.px(6.))
            .px(t.px(12.))
            .py(t.px(10.))
            .rounded(t.card())
            .border_2()
            .border_color(t.accent)
            .bg(t.bg)
            .when(!quote.is_empty(), |d| {
                d.child(
                    div()
                        .min_w_0()
                        .pl(t.px(8.))
                        .border_l_3()
                        .border_color(t.comment_gold)
                        .text_size(t.px(12.))
                        .text_color(t.soft)
                        .line_clamp(2)
                        .text_ellipsis()
                        .child(quote),
                )
            })
            .child(
                div()
                    .debug_selector(|| "comment-draft-field".into())
                    .relative()
                    .min_h(t.px(26.))
                    .px(t.px(8.))
                    .py(t.px(4.))
                    .rounded(t.small())
                    .border_1()
                    .border_color(t.border)
                    .child(field)
                    .when(blank, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(t.px(8.))
                                .top(t.px(4.))
                                .text_size(t.px(13.))
                                .text_color(t.soft)
                                .child(tr(
                                    "Add a comment, then press Enter",
                                    "Ajoutez un commentaire, puis Entrée",
                                )),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(t.px(6.))
                    .child(
                        button("comment-draft-save", tr("Comment", "Commenter"), t)
                            .h(t.px(24.))
                            .px(t.px(9.))
                            .text_size(t.px(12.))
                            .bg(t.accent)
                            .text_color(t.bg)
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.comment_write(window, cx)
                            })),
                    )
                    .child(
                        button("comment-draft-cancel", tr("Cancel", "Annuler"), t)
                            .h(t.px(24.))
                            .px(t.px(9.))
                            .text_size(t.px(12.))
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.comment_cancel(window, cx)
                            })),
                    ),
            )
    }

    /// A comment was just started in `editor` (`EditorEvent::CommentStarted`): the Comments view opens with
    /// its draft card, the field focused. A draft still open keeps its empty markup, as any unfinished one.
    pub(super) fn comment_started(
        &mut self,
        editor: &Entity<Editor>,
        inside: usize,
        noted: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let field = cx.new(|cx| Field::new("", self.theme, cx));
        let subscription = cx.subscribe_in(
            &field,
            window,
            |this, _, event: &crate::views::field::FieldEvent, window, cx| match event {
                crate::views::field::FieldEvent::Done => this.comment_write(window, cx),
                crate::views::field::FieldEvent::Cancel => this.comment_cancel(window, cx),
                crate::views::field::FieldEvent::Changed => cx.notify(),
            },
        );
        let edits = cx.subscribe_in(
            editor,
            window,
            |this, editor, event: &EditorEvent, _, cx| {
                if matches!(event, EditorEvent::Changed) {
                    this.comment_text_changed(editor, cx);
                }
            },
        );
        self.comment_draft = Some(Draft {
            inside,
            noted,
            editor: editor.downgrade(),
            text: editor.read(cx).text().to_string(),
            field: field.clone(),
            _subscription: subscription,
            _edits: edits,
        });
        self.comment_focus = None;
        self.show_view(Mode::Comments, window, cx);
        window.focus(&field.focus_handle(cx));
        cx.notify();
    }

    /// The note of `draft` while it is open in a pane and its own marker is still where the draft says;
    /// None otherwise, so that nothing is written.
    fn draft_note(&self, draft: &Draft, cx: &App) -> Option<Entity<Editor>> {
        let id = draft.editor.entity_id();
        let pane = self
            .panes
            .iter()
            .find(|pane| pane.editor.entity_id() == id)?;
        own_marker(pane.editor.read(cx).text(), draft.inside).then(|| pane.editor.clone())
    }

    /// A draft ends with its tab: when its note is no longer in any pane, the draft goes, its card with it.
    /// Nothing is written.
    pub(super) fn end_closed_draft(&mut self) {
        let open = self.comment_draft.as_ref().is_none_or(|draft| {
            let id = draft.editor.entity_id();
            self.panes.iter().any(|pane| pane.editor.entity_id() == id)
        });
        if !open {
            self.comment_draft = None;
        }
    }

    /// The note's text changed while the draft is open: the draft's comment moves with the text. When the
    /// change cannot be followed, or the marker is no longer an empty comment, the draft ends and nothing is
    /// written: a write or a take-out only ever goes to the draft's own marker, found where it is now.
    fn comment_text_changed(&mut self, editor: &Entity<Editor>, cx: &mut Context<Self>) {
        let Some(draft) = self.comment_draft.as_mut() else {
            return;
        };
        if draft.editor.entity_id() != editor.entity_id() {
            return;
        }
        let now = editor.read(cx).text().to_string();
        match shifted(&draft.text, &now, draft.inside).filter(|at| own_marker(&now, *at)) {
            Some(at) => {
                draft.inside = at;
                draft.text = now;
            }
            None => self.comment_draft = None,
        }
        cx.notify();
    }

    /// Enter in the draft: what is written is written into its comment. An empty text keeps the draft open.
    pub(super) fn comment_write(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.comment_draft.as_ref() else {
            return;
        };
        let text = draft.field.read(cx).line.text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let Some(draft) = self.comment_draft.take() else {
            return;
        };
        if let Some(editor) = self.draft_note(&draft, cx) {
            editor.update(cx, |e, cx| e.write_comment(draft.inside, &text, cx));
            window.focus(&editor.focus_handle(cx));
        }
        cx.notify();
    }

    /// Escape or Cancel in the draft: its markup goes, the passage stays as it was.
    pub(super) fn comment_cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = self.comment_draft.take() else {
            return;
        };
        if let Some(editor) = self.draft_note(&draft, cx) {
            editor.update(cx, |e, cx| e.resolve_comment(draft.inside, cx));
            window.focus(&editor.focus_handle(cx));
        }
        cx.notify();
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
            editor.update(cx, |e, cx| e.flash_end(flash, back, cx)).ok();
        })
        .detach();
        window.focus(&self.pane().editor.focus_handle(cx));
        cx.notify();
    }

    /// A click on a commented passage in the note (`EditorEvent::CommentAt`): the Comments view opens on its card.
    pub(super) fn comment_clicked(
        &mut self,
        at: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

#[cfg(test)]
mod tests {
    use super::split_byline;

    #[test]
    fn a_byline_is_a_name_and_a_clock_time_before_the_colon() {
        assert_eq!(
            split_byline("Ana, 10:02: à relire"),
            Some(("Ana", "10:02", "à relire"))
        );
        // The name may hold commas; the last comma before the time separates it.
        assert_eq!(
            split_byline("Doe, Ana, 09:45: ok"),
            Some(("Doe, Ana", "09:45", "ok"))
        );
        // An empty body is still a byline.
        assert_eq!(split_byline("Ana, 10:02: "), Some(("Ana", "10:02", "")));
    }

    #[test]
    fn text_without_a_byline_is_left_alone() {
        for said in [
            "",
            "à relire",
            "Note: le titre",
            "Ana, 10h02: x",
            "Ana, 10:02 x",
            ", 10:02: x",
            "Ana, 1:02: x",
            "Ana, 10:02:30: x",
            "Ana: 10:02: x",
            "Ana, 10:0a: x",
        ] {
            assert_eq!(split_byline(said), None, "{said:?}");
        }
    }
}
