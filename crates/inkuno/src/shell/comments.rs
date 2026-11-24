//! Comments written from the editor and the Review view: the draft of a comment being written (Enter
//! writes it, Escape takes it out), the caret that follows the draft's text, and the passage a card
//! flashes when it is picked. The feed and its cards are in `review.rs`.

use std::{ops::Range, time::Duration};

use gpui::{
    App, Context, Div, Entity, Stateful, Subscription, WeakEntity, Window, div, prelude::*,
};

use super::*;
use crate::lang::tr;
use crate::views::{editor::Editor, field::Field};

/// How long a passage stays selected when its card is clicked.
const FLASH: Duration = Duration::from_millis(900);

/// A comment being written: its draft card leads the Review feed. The passage it is on is quoted, what is
/// said goes in the field, Enter writes it into the note (`write_comment`) and Escape or Cancel takes its
/// markup out again (`resolve_comment`). The note holds the empty markup meanwhile.
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

impl Shell {
    /// A comment was just started in `editor` (`EditorEvent::CommentStarted`): the Review view opens with
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
        self.show_view(Mode::Review, window, cx);
        window.focus(&field.focus_handle(cx), cx);
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
            window.focus(&editor.focus_handle(cx), cx);
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
            window.focus(&editor.focus_handle(cx), cx);
        }
        cx.notify();
    }

    /// A card was picked: the caret goes to its comment, its passage is selected for `FLASH`, and
    /// the editor takes the keys back.
    pub(super) fn comment_go(
        &mut self,
        inside: usize,
        passage: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = self.pane().editor.clone();
        editor.update(cx, |e, cx| {
            e.jump(inside, false, cx);
            e.select_passage(passage.clone(), cx);
        });
        let back = inside;
        let flash = passage.clone();
        cx.spawn(async move |_, cx| {
            cx.background_executor().timer(FLASH).await;
            editor.update(cx, |e, cx| e.flash_end(flash, back, cx));
        })
        .detach();
        window.focus(&self.pane().editor.focus_handle(cx), cx);
        cx.notify();
    }

    /// The note's corner badge: one quiet dot, on the note's foot on the right, counting the open items of
    /// the Review feed (none without any).
    pub(super) fn render_comments_badge(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let t = self.theme;
        let count = self.review_total;
        (count > 0 && !self.focus_mode).then(|| {
            div()
                .id("comments-toggle")
                .debug_selector(|| "comments-toggle".into())
                .tooltip(ui::widgets::tip(
                    tr("Review of the note", "Relecture de la note"),
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
                .on_click(cx.listener(|this, _, window, cx| this.show_review(window, cx)))
        })
    }
}
