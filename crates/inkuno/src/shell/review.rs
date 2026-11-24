//! The Review view of the right panel (app-ui.md §23): one feed of the note's comments and pending
//! changes, oldest first. One card is expanded at a time: a comment's shows its text, its replies, a
//! reply field and Resolve; a change's shows its old and new text and Accept or Reject. Picking a card
//! selects its passage in the note. The feed is built only while the view is shown, and kept while the note
//! and its saved changes stay the same (`refresh_review`); the count the switcher and the note's corner show is kept
//! the same way, for the view hidden too.

use std::ops::Range;

use gpui::{
    App, Context, Div, Entity, EntityId, Hsla, Stateful, Subscription, Window, div, prelude::*, svg,
};

use super::*;
use crate::lang::tr;
use crate::logic::{markdown, rank, review as feed};
use crate::ui::components::button::button;
use crate::views::field::{Field, FieldEvent};

/// What the feed of the active note is read from: the editor (its identity and its text's version) and the
/// changes saved beside the note (`Pane::saved_version`). Equal sources give an equal feed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Source {
    editor: EntityId,
    text: u64,
    saved: u64,
}

/// The identity of a feed item: a comment by its start in the note, a pending change by its first byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Comment(usize),
    Change(usize),
}

/// What an item is about, beside what every item has.
#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Comment {
        /// The commented text; empty for a comment on its own.
        noted: String,
        /// The table cell, row or column it is on; empty elsewhere.
        scope: String,
        replies: Vec<markdown::Reply>,
        /// Where the caret goes in its text.
        inside: usize,
        /// The passage a pick selects.
        passage: Range<usize>,
    },
    Change {
        /// Its place among the note's pending changes (`Editor::pending_areas`).
        index: usize,
        old: String,
        new: String,
        /// The passage a pick selects: its added lines, or its removed ones when nothing is added.
        passage: Range<usize>,
    },
}

/// One entry of the feed.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub key: Key,
    /// Where it starts in the note, and where it ends.
    pub at: usize,
    pub end: usize,
    /// The UTC minute it orders by: a comment's time, or a change's `created`. None when it has none.
    pub time: Option<String>,
    /// Who it is by; None is shown as "Unknown".
    pub author: Option<String>,
    pub kind: feed::Kind,
    /// The heading it is under, as plain text.
    pub section: Option<String>,
    /// The line of its anchor in the file.
    pub line: usize,
    /// What the collapsed card shows: a comment's text, or a change's new text (its old one when it
    /// removes).
    pub said: String,
    pub body: Body,
}

impl Item {
    /// Whether this is the comment whose text the caret is in at `inside`.
    fn is_inside(&self, inside: usize) -> bool {
        matches!(self.body, Body::Comment { inside: at, .. } if at == inside)
    }
}

/// Where an item is, as its card says it: the heading it is under (cut short when long), and the line,
/// which always shows.
fn location(item: &Item) -> (Option<String>, String) {
    (
        item.section.clone(),
        format!("{} {}", tr("Line", "Ligne"), item.line),
    )
}

/// How long ago a UTC minute was, as the card says it: `now`, `12 min`, `3 h`, `2 j`, then the date.
fn ago(time: &str) -> String {
    let Some(secs) = feed::seconds_since(crate::platform::revisions::now(), time) else {
        return String::new();
    };
    match secs {
        s if s < 60 => tr("now", "maintenant").to_string(),
        s if s < 3_600 => format!("{} min", s / 60),
        s if s < 86_400 => format!("{} h", s / 3_600),
        s if s < 7 * 86_400 => format!("{} {}", s / 86_400, tr("d", "j")),
        _ => time[..10].to_string(),
    }
}

/// The author a card shows: the name, or Unknown when there is none. None, an empty name and `unknown` in any
/// case are all Unknown (the comments of older files, and changes saved without an author).
pub(super) fn shown_author(author: Option<&str>) -> String {
    let author = author.map(str::to_string);
    markdown::shown_author(&author).map_or_else(
        || tr("Unknown", "Inconnu").to_string(),
        |name| name.trim().to_string(),
    )
}

/// Comment text as a card shows it: an escaped `\↳` (a literal arrow in the text) reads as the arrow. Display only.
fn shown_text(text: &str) -> String {
    text.replace("\\↳", "↳")
}

/// The name a card shows, and the letter of its avatar.
fn name_of(author: &Option<String>) -> (String, String) {
    let name = shown_author(author.as_deref());
    let initial = name
        .chars()
        .next()
        .map_or_else(|| "?".to_string(), |c| c.to_uppercase().to_string());
    (name, initial)
}

/// A block of the expanded change: its label, and its text one line at a time, bar on the left.
fn side(t: crate::ui::theme::Theme, label: &str, text: &str, bar: Hsla) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(t.px(2.))
        .child(
            div()
                .text_size(t.px(11.))
                .text_color(t.dim)
                .child(label.to_string()),
        )
        .child(
            div()
                .pl(t.px(8.))
                .border_l_3()
                .border_color(crate::ui::theme::legible(bar, t.bg))
                .flex()
                .flex_col()
                .children(text.lines().map(|line| {
                    div()
                        .min_h(t.px(19.))
                        .text_size(t.px(13.))
                        .line_height(t.px(19.))
                        .text_color(t.text)
                        .child(line.to_string())
                })),
        )
}

impl Shell {
    /// Whether the Review view is drawn: its tool is in an open dock.
    fn review_shown(&self) -> bool {
        self.docked(Mode::Review)
    }

    /// The source the feed of the active pane is read from; None for a file or a list page, which have none.
    fn review_source(&self, cx: &App) -> Option<Source> {
        let pane = self.pane();
        (pane.file.is_none() && pane.listing.is_none()).then(|| Source {
            editor: pane.editor.entity_id(),
            text: pane.editor.read(cx).version(),
            saved: pane.saved_version,
        })
    }

    /// Brings the feed and its count up to date with the note, once per frame: the feed is built only while
    /// the Review view is shown, and only when its source moved; the count follows the source for both.
    pub(super) fn refresh_review(&mut self, cx: &App) {
        let source = self.review_source(cx);
        if self.review_shown() {
            if self.review_items_source != source {
                self.review_items = match source {
                    Some(_) => self.review_feed(cx),
                    None => Vec::new(),
                };
                self.review_items_source = source;
                self.review_total = self.review_items.len();
                self.review_total_source = source;
            }
        } else if self.review_total_source != source {
            self.review_total = self.review_count(cx);
            self.review_total_source = source;
        }
    }

    /// How many items the feed of the active pane has, without building it: the comments and the pending changes.
    fn review_count(&self, cx: &App) -> usize {
        if self.review_source(cx).is_none() {
            return 0;
        }
        let editor = self.pane().editor.read(cx);
        markdown::all_comments(editor.text()).len() + editor.pending_areas().len()
    }

    /// The feed of the note in the active pane: its comments and its pending changes, in the order the view
    /// lists them. Empty for a file or a list page, which have none.
    pub(super) fn review_feed(&self, cx: &App) -> Vec<Item> {
        let pane = self.pane();
        if pane.file.is_some() || pane.listing.is_some() {
            return Vec::new();
        }
        let editor = pane.editor.read(cx);
        let text = editor.text();
        let areas = editor.pending_areas();
        let hidden = feed::hidden(&areas);
        // One numbering for the whole feed: the file's lines, the marks and added lines left out.
        let lines = crate::logic::lines::FileLines::new(text, &hidden);
        let heads = feed::headings(text);
        let section = |at: usize| feed::section(&heads, at).map(rank::plain_line);
        let mut items = Vec::new();
        for note in markdown::all_comments(text) {
            // The CriticMarkup form starts its passage after `{==`; the HTML form starts it at the whole.
            let from = match text[note.at..].starts_with("{==") {
                true => note.at + 3,
                false => note.at,
            };
            let passage = match note.noted.is_empty() {
                true => note.at..note.end,
                false => from..from + note.noted.len(),
            };
            items.push(Item {
                key: Key::Comment(note.at),
                at: note.at,
                end: note.end,
                time: note.time.clone(),
                author: note.author.clone(),
                kind: feed::Kind::Comment,
                section: section(note.at),
                line: lines.line_of(note.at),
                said: note.said.to_string(),
                body: Body::Comment {
                    noted: note.noted.to_string(),
                    scope: note.scope.clone(),
                    replies: note.replies.clone(),
                    inside: note.inside,
                    passage,
                },
            });
        }
        for (index, area) in areas.iter().enumerate() {
            let old = text[area.removed.clone()].to_string();
            let new = text[area.added.clone()].to_string();
            let saved = feed::entry_of(&pane.pending_saved, &old, &new);
            let passage = match area.added.is_empty() {
                true => area.removed.clone(),
                false => area.added.clone(),
            };
            items.push(Item {
                key: Key::Change(area.range.start),
                at: area.range.start,
                end: area.range.end,
                time: saved.and_then(|entry| entry.created.clone()),
                author: saved
                    .map(|entry| entry.author.clone())
                    .filter(|author| !author.trim().is_empty()),
                kind: feed::change_kind(&old, &new),
                section: section(area.range.start),
                line: lines.line_of(area.range.start),
                // The line breaks at the end of a side are not lines of the card.
                said: match new.is_empty() {
                    true => old.trim_end().to_string(),
                    false => new.trim_end().to_string(),
                },
                body: Body::Change {
                    index,
                    old,
                    new,
                    passage,
                },
            });
        }
        feed::sort(&mut items, |item| item.time.as_deref(), |item| item.at);
        items
    }

    /// The feed as the view lists it: without the comment a draft stands for (its draft card leads).
    pub(super) fn review_visible(&self) -> Vec<&Item> {
        let drafted = self
            .comment_draft
            .as_ref()
            .filter(|draft| draft.editor.entity_id() == self.pane().editor.entity_id())
            .map(|draft| draft.inside);
        self.review_items
            .iter()
            .filter(|item| !drafted.is_some_and(|inside| item.is_inside(inside)))
            .collect()
    }

    /// The position of `key` among the children of the view's list, the draft card first.
    fn review_child(&self, key: Key) -> Option<usize> {
        let draft = self.comment_draft.is_some() as usize;
        self.review_visible()
            .iter()
            .position(|item| item.key == key)
            .map(|at| at + draft)
    }

    /// The view's head: `Review · N open`, N the items of the note's feed.
    pub(super) fn review_title(&self) -> String {
        format!(
            "{} · {} {}",
            self.tool_title(Mode::Review),
            self.review_total,
            tr("open", "ouverts")
        )
    }

    /// The editors write their comments by the revisions' author: it is pushed to each of them when the
    /// setting changed (the setting is read, not kept, so the editors are told once).
    pub(super) fn sync_comment_author(&mut self, cx: &mut Context<Self>) {
        let author = self.revision_author();
        if author == self.comment_author {
            return;
        }
        for pane in &self.panes {
            let author = author.clone();
            pane.editor.update(cx, |e, _| e.set_author(author));
        }
        self.comment_author = author;
    }

    /// The Review view's body: the draft of a comment being written, then the feed, one card each.
    pub(super) fn render_review(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let items = self.review_visible();
        let draft = self
            .comment_draft
            .as_ref()
            .map(|d| self.comment_draft_card(d, cx));
        if items.is_empty() && draft.is_none() {
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_start()
                .justify_center()
                .p_3()
                .child(
                    div()
                        .debug_selector(|| "review-empty".into())
                        .text_size(t.px(13.))
                        .text_color(t.soft)
                        .child(tr(
                            "Nothing to review in this note",
                            "Rien à relire dans cette note",
                        )),
                )
                .into_any_element();
        }
        div()
            .id("review-view")
            .debug_selector(|| "review-view".into())
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.review_scroll)
            .px(t.px(12.))
            .pt(t.px(6.))
            .pb(t.px(16.))
            .flex()
            .flex_col()
            .gap(t.px(8.))
            .children(draft)
            .children(
                items
                    .into_iter()
                    .enumerate()
                    .map(|(i, item)| self.review_card(i, item, cx)),
            )
            .into_any_element()
    }

    /// One card of the feed. Collapsed: the author, the kind, the time, the first lines of what it says and
    /// where it is. Expanded: all of it. A click on a collapsed card opens it (`review_pick`).
    fn review_card(&self, i: usize, item: &Item, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let key = item.key;
        let open = self.review_open == Some(key);
        let id = match key {
            Key::Comment(at) => ("review-comment", at),
            Key::Change(at) => ("review-change", at),
        };
        let card = div()
            .id(id)
            .debug_selector(move || format!("review-card-{i}"))
            .flex()
            .flex_col()
            .gap(t.px(6.))
            .px(t.px(10.))
            .py(t.px(8.))
            .rounded(t.card())
            // The open card has a hairline; a closed one is bare, its hairline the colour of the panel.
            .border_1()
            .border_color(if open { t.border } else { t.bg })
            .when(open, |d| d.bg(t.panel))
            .when(!open, |d| d.cursor_pointer().hover(|s| s.bg(t.hover)))
            .child(self.review_head(item));
        match open {
            true => card.child(self.review_detail(item, cx)),
            false => card.child(self.review_summary(item)).on_click(
                cx.listener(move |this, _, window, cx| this.review_pick(key, window, cx)),
            ),
        }
    }

    /// The author, the kind and the time of a card.
    fn review_head(&self, item: &Item) -> Div {
        let t = self.theme;
        let (name, initial) = name_of(&item.author);
        let icon = match item.kind {
            feed::Kind::Comment => "comment.svg",
            feed::Kind::Replace => "replace-one.svg",
            feed::Kind::Insert => "plus.svg",
            feed::Kind::Delete => "trash.svg",
        };
        div()
            .flex()
            .items_center()
            .gap(t.px(6.))
            .min_w_0()
            .child(
                div()
                    .size(t.px(20.))
                    .flex_none()
                    .rounded_full()
                    .bg(t.pill)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(t.px(11.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.soft)
                    .child(initial),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(t.px(12.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(t.text)
                    .child(name),
            )
            .child(
                svg()
                    .path(icon)
                    .size(t.px(12.))
                    .flex_none()
                    .text_color(t.soft),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex_none()
                    .text_size(t.px(11.))
                    .text_color(t.dim)
                    .child(item.time.as_deref().map(ago).unwrap_or_default()),
            )
    }

    /// The collapsed body: at most three lines of what it says, then where it is.
    fn review_summary(&self, item: &Item) -> Div {
        let t = self.theme;
        let said = match (&item.body, item.said.trim().is_empty()) {
            (Body::Comment { .. }, true) => {
                tr("(nothing written yet)", "(rien d'écrit pour l'instant)").to_string()
            }
            _ => shown_text(&item.said),
        };
        div()
            .flex()
            .flex_col()
            .gap(t.px(2.))
            .min_w_0()
            .child(
                div()
                    .min_w_0()
                    .text_size(t.px(13.))
                    .line_height(t.px(19.))
                    .text_color(t.text)
                    .line_clamp(3)
                    .text_ellipsis()
                    .child(said),
            )
            .child({
                let (section, line) = location(item);
                div()
                    .flex()
                    .min_w_0()
                    .text_size(t.px(11.))
                    .text_color(t.dim)
                    .children(
                        section.map(|section| {
                            div().min_w_0().truncate().child(format!("{section} · "))
                        }),
                    )
                    .child(div().flex_none().child(line))
            })
    }

    /// The expanded body: a comment's text, replies, the reply field and Resolve; a change's two sides and
    /// Accept or Reject.
    fn review_detail(&self, item: &Item, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        match &item.body {
            Body::Comment {
                noted,
                scope,
                replies,
                ..
            } => {
                let at = item.at;
                let said = match item.said.trim().is_empty() {
                    true => {
                        tr("(nothing written yet)", "(rien d'écrit pour l'instant)").to_string()
                    }
                    false => shown_text(&item.said),
                };
                let quote = rank::plain_line(noted);
                let mut body = div().flex().flex_col().gap(t.px(8.));
                if !scope.is_empty() {
                    body = body.child(
                        div()
                            .text_size(t.px(11.))
                            .text_color(t.dim)
                            .child(scope.clone()),
                    );
                }
                if !quote.is_empty() {
                    body = body.child(
                        div()
                            .pl(t.px(8.))
                            .border_l_3()
                            .border_color(t.comment_gold)
                            .text_size(t.px(12.))
                            .text_color(t.soft)
                            .line_clamp(2)
                            .text_ellipsis()
                            .child(quote),
                    );
                }
                body = body.child(
                    div()
                        .text_size(t.px(13.))
                        .line_height(t.px(19.))
                        .text_color(t.text)
                        .child(said),
                );
                for reply in replies {
                    let (name, _) = name_of(&reply.author);
                    let when = reply.time.as_deref().map(ago).unwrap_or_default();
                    body = body.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(t.px(2.))
                            .pl(t.px(8.))
                            .border_l_2()
                            .border_color(t.border)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(t.px(6.))
                                    .text_size(t.px(12.))
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .text_color(t.text)
                                            .child(name),
                                    )
                                    .child(
                                        div().text_size(t.px(11.)).text_color(t.dim).child(when),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(t.px(13.))
                                    .line_height(t.px(19.))
                                    .text_color(t.text)
                                    .child(shown_text(&reply.text)),
                            ),
                    );
                }
                if let Some((_, field, _)) = self
                    .review_reply
                    .as_ref()
                    .filter(|(key, ..)| *key == Key::Comment(at))
                {
                    body = body.child(self.reply_row(field, cx));
                }
                body.child(
                    div().flex().items_center().child(
                        button("review-resolve", tr("Resolve", "Résoudre"), t)
                            .h(t.px(24.))
                            .px(t.px(9.))
                            .text_size(t.px(12.))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.review_resolve(at, window, cx)
                            })),
                    ),
                )
            }
            Body::Change {
                index, old, new, ..
            } => {
                let index = *index;
                let mut body = div().flex().flex_col().gap(t.px(8.));
                if !old.is_empty() {
                    body = body.child(side(t, tr("Before", "Avant"), old, t.removed));
                }
                if !new.is_empty() {
                    body = body.child(side(t, tr("After", "Après"), new, t.added));
                }
                body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(t.px(6.))
                        .child(
                            button("review-accept", tr("Accept", "Accepter"), t)
                                .h(t.px(24.))
                                .px(t.px(9.))
                                .text_size(t.px(12.))
                                .bg(t.accent)
                                .text_color(t.bg)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.review_decide(index, true, window, cx)
                                })),
                        )
                        .child(
                            button("review-reject", tr("Reject", "Refuser"), t)
                                .h(t.px(24.))
                                .px(t.px(9.))
                                .text_size(t.px(12.))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.review_decide(index, false, window, cx)
                                })),
                        ),
                )
            }
        }
    }

    /// The reply field of the expanded comment, its hint while blank, and Reply (Enter sends too).
    fn reply_row(&self, field: &Entity<Field>, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let blank = field.read(cx).line.text.is_empty();
        div()
            .flex()
            .items_center()
            .gap(t.px(6.))
            .child(
                div()
                    .debug_selector(|| "review-reply-field".into())
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .min_h(t.px(26.))
                    .px(t.px(8.))
                    .py(t.px(4.))
                    .rounded(t.small())
                    .border_1()
                    .border_color(t.border)
                    .child(field.clone())
                    .when(blank, |d| {
                        d.child(
                            div()
                                .absolute()
                                .left(t.px(8.))
                                .top(t.px(4.))
                                .text_size(t.px(13.))
                                .text_color(t.soft)
                                .child(tr("Reply, then Enter", "Répondre, puis Entrée")),
                        )
                    }),
            )
            .child(
                button("review-reply-send", tr("Reply", "Répondre"), t)
                    .h(t.px(24.))
                    .px(t.px(9.))
                    .text_size(t.px(12.))
                    .on_click(cx.listener(|this, _, window, cx| {
                        cx.stop_propagation();
                        this.review_reply_send(window, cx)
                    })),
            )
    }

    /// The draft card of a comment being written: the passage in a gold bar, the field (Enter writes it),
    /// Comment (primary) and Cancel.
    fn comment_draft_card(&self, draft: &comments::Draft, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let field = draft.field.clone();
        let blank = field.read(cx).line.text.is_empty();
        let quote = rank::plain_line(&draft.noted);
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

    /// Opens the card of `key` (the one open closes). A comment gets its reply field; nothing is selected.
    fn review_expand(&mut self, key: Key, window: &mut Window, cx: &mut Context<Self>) {
        self.review_open = Some(key);
        self.review_reply = match key {
            Key::Comment(_) => Some(self.reply_field(key, window, cx)),
            Key::Change(_) => None,
        };
        if let Some(i) = self.review_child(key) {
            self.review_scroll.scroll_to_item(i);
        }
        cx.notify();
    }

    /// A collapsed card was clicked: it opens, and its passage is selected in the note (a comment's
    /// flashes). The keys go back to the note.
    pub(super) fn review_pick(&mut self, key: Key, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self
            .review_items
            .iter()
            .find(|item| item.key == key)
            .cloned()
        else {
            return;
        };
        self.review_expand(key, window, cx);
        match &item.body {
            Body::Comment {
                inside, passage, ..
            } => self.comment_go(*inside, passage.clone(), window, cx),
            Body::Change { index, .. } => {
                let editor = self.pane().editor.clone();
                editor.update(cx, |e, cx| e.select_change(*index, cx));
                window.focus(&editor.focus_handle(cx), cx);
            }
        }
    }

    /// The hint's click on the pending changes: the Review view opens on the first change, expanded, its
    /// passage selected.
    pub(super) fn review_first_change(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_view(Mode::Review, window, cx);
        let first = self
            .review_items
            .iter()
            .find(|item| matches!(item.key, Key::Change(_)))
            .map(|item| item.key);
        if let Some(key) = first {
            self.review_pick(key, window, cx);
        }
    }

    /// A click on a commented passage in the note (`EditorEvent::CommentAt`): the Review view opens on
    /// its card, expanded.
    pub(super) fn comment_clicked(
        &mut self,
        at: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_view(Mode::Review, window, cx);
        self.review_expand(Key::Comment(at), window, cx);
    }

    /// Opens the right panel on the Review view. No card opens by itself: a card opens on a pick, or on a
    /// click on its commented passage.
    pub(super) fn show_review(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.show_view(Mode::Review, window, cx);
    }

    /// The reply field of the expanded comment `key`: Enter sends the reply, Escape gives the keys back to
    /// the note.
    fn reply_field(
        &self,
        key: Key,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Key, Entity<Field>, Subscription) {
        let theme = self.theme;
        let field = cx.new(|cx| Field::new("", theme, cx));
        let subscription = cx.subscribe_in(
            &field,
            window,
            |this, _, event: &FieldEvent, window, cx| match event {
                FieldEvent::Done => this.review_reply_send(window, cx),
                FieldEvent::Cancel => this.focus_note(window, cx),
                FieldEvent::Changed => cx.notify(),
            },
        );
        (key, field, subscription)
    }

    /// Reply: the text of the reply field is written after the replies of the expanded comment, by the
    /// author now. A blank reply writes nothing.
    fn review_reply_send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some((Key::Comment(at), field)) = self
            .review_reply
            .as_ref()
            .map(|(key, field, _)| (*key, field.clone()))
        else {
            return;
        };
        let text = field.read(cx).line.text.trim().to_string();
        if text.is_empty() {
            return;
        }
        let editor = self.pane().editor.clone();
        editor.update(cx, |e, cx| e.reply_comment(at, &text, cx));
        field.update(cx, |f, cx| f.set("", cx));
        window.focus(&field.focus_handle(cx), cx);
        cx.notify();
    }

    /// Resolve: the comment's markup goes, its passage stays, as one undo step.
    fn review_resolve(&mut self, at: usize, window: &mut Window, cx: &mut Context<Self>) {
        let editor = self.pane().editor.clone();
        editor.update(cx, |e, cx| e.resolve_comment(at, cx));
        self.review_open = None;
        self.review_reply = None;
        window.focus(&editor.focus_handle(cx), cx);
        cx.notify();
    }

    /// Accept or Reject of the pending change `index`, as one undo step.
    fn review_decide(
        &mut self,
        index: usize,
        accept: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = self.pane().editor.clone();
        editor.update(cx, |e, cx| e.decide_change(index, accept, cx));
        self.review_open = None;
        self.review_reply = None;
        window.focus(&editor.focus_handle(cx), cx);
        cx.notify();
    }
}
