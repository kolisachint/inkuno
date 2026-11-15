//! Pending changes in the editor (`logic::pending`): accept or reject one change or all of them,
//! and comment on one. Each is one undo step. The text changes through `splice_now` (the route of
//! an outside change), so the guard against typing in a pending change does not stop them.

use super::*;
use crate::logic::{assistant, markdown, pending};

impl Editor {
    /// The change the caret is in, or the selection touches. `None` when neither is in a change.
    /// The key context follows it: `Editor Change` while it is `Some` (`render.rs`).
    pub(super) fn change_at_caret(&self) -> Option<usize> {
        let (start, end) = (self.sel.start, self.sel.end);
        self.pending.iter().position(|area| {
            if start == end {
                area.range.start <= start && start < area.range.end
            } else {
                area.range.start < end && start < area.range.end
            }
        })
    }

    /// Says that there is no change to act on: the one message of the actions that find none.
    fn no_change(&mut self, cx: &mut Context<Self>) {
        cx.emit(EditorEvent::Notice(
            tr("No pending change.", "Aucun changement en attente.").into(),
        ));
    }

    /// The first press of accept, reject or comment with the caret outside every change: the caret
    /// goes to the next change (the first one, past the last), where it shows, and nothing is acted on.
    fn go_to_next_change(&mut self, cx: &mut Context<Self>) {
        let at = self.sel.end;
        let next = self
            .pending
            .iter()
            .map(|area| area.range.start)
            .find(|&start| start > at)
            .or_else(|| self.pending.first().map(|area| area.range.start));
        if let Some(start) = next {
            self.move_to(start, cx);
        }
        cx.emit(EditorEvent::Notice(
            tr(
                "Caret moved to the next change. Repeat to accept or reject it.",
                "Le curseur va au changement suivant. Recommencez pour l'accepter ou le refuser.",
            )
            .into(),
        ));
    }

    /// Puts `new` in place of the whole text as one undo step, the change of the note's text
    /// going through the outside-change route, then puts the caret at `at`.
    fn settle(&mut self, new: String, at: usize, cx: &mut Context<Self>) {
        if let Some(edit) = assistant::edit_between(&self.content, &new) {
            self.absorb(&[edit], cx);
        }
        self.more.clear();
        self.sel = at..at;
        self.reversed = false;
        cx.notify();
    }

    /// Accepts the change under the caret: its removed lines go, its added lines stay.
    pub(super) fn accept_change(&mut self, cx: &mut Context<Self>) {
        self.settle_one(cx, pending::accept)
    }

    /// Rejects the change under the caret: its added lines go, its removed lines stay.
    pub(super) fn reject_change(&mut self, cx: &mut Context<Self>) {
        self.settle_one(cx, pending::reject)
    }

    fn settle_one(&mut self, cx: &mut Context<Self>, how: fn(&str, usize) -> String) {
        if self.pending.is_empty() {
            return self.no_change(cx);
        }
        let Some(index) = self.change_at_caret() else {
            return self.go_to_next_change(cx);
        };
        let at = self.pending[index].range.start;
        let new = how(&self.content, index);
        self.settle(new, at, cx);
    }

    /// Accepts every change of the note.
    pub(super) fn accept_all_changes(&mut self, cx: &mut Context<Self>) {
        self.settle_all(cx, pending::accept_all)
    }

    /// Rejects every change of the note.
    pub(super) fn reject_all_changes(&mut self, cx: &mut Context<Self>) {
        self.settle_all(cx, pending::reject_all)
    }

    fn settle_all(&mut self, cx: &mut Context<Self>, how: fn(&str) -> String) {
        let Some(first) = self.pending.first().map(|area| area.range.start) else {
            return self.no_change(cx);
        };
        let new = how(&self.content);
        self.settle(new, first, cx);
    }

    /// How many pending changes the note has.
    pub fn pending_changes(&self) -> usize {
        self.pending.len()
    }

    /// Moves the caret to the first pending change (the note's hint).
    pub fn go_to_first_change(&mut self, cx: &mut Context<Self>) {
        if let Some(start) = self.pending.first().map(|area| area.range.start) {
            self.move_to(start, cx);
        }
    }

    /// Asks the shell for the line of a comment on the change under the caret. The change is named
    /// by its text, which the prompt carries back (`comment_on_change`). With the caret outside
    /// every change, the first press only moves the caret to the next one.
    pub(super) fn ask_change_comment(&mut self, cx: &mut Context<Self>) {
        if self.pending.is_empty() {
            return self.no_change(cx);
        }
        match self.change_at_caret() {
            Some(index) => {
                let text = self.content[self.pending[index].range.clone()].to_string();
                cx.emit(EditorEvent::AskChangeComment(text));
            }
            None => self.go_to_next_change(cx),
        }
    }

    /// Writes `memo` as a comment on the first added line of the change whose text is `change` (its
    /// first removed line when nothing is added). The change stays pending. When no change has that
    /// text any more (an outside edit moved or changed it), nothing is written and the person is told.
    /// Refused, with the reason, when the line cannot carry a comment: empty, or holding comment marks.
    pub fn comment_on_change(&mut self, change: &str, memo: &str, cx: &mut Context<Self>) {
        let Some(area) = self
            .pending
            .iter()
            .find(|area| self.content[area.range.clone()] == *change)
            .cloned()
        else {
            cx.emit(EditorEvent::Notice(
                tr(
                    "That change is gone or changed: nothing was commented.",
                    "Ce changement a disparu ou a changé : rien n'a été commenté.",
                )
                .into(),
            ));
            return;
        };
        let block = if area.added.is_empty() {
            area.removed.clone()
        } else {
            area.added.clone()
        };
        let passage = self.content[block].split('\n').next().unwrap_or("");
        let passage = passage.trim_end_matches('\r').to_string();
        match markdown::comment_on(&self.content, &passage, memo) {
            Ok(new) => {
                let at = self.sel.start;
                self.settle(new, at, cx);
            }
            Err(reason) => cx.emit(EditorEvent::Notice(reason)),
        }
    }
}
