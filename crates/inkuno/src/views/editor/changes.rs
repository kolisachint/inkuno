//! Pending changes in the editor (`logic::pending`): accept or reject one change or all of them,
//! and comment on one. Each is one undo step. The text changes through `splice_now` (the route of
//! an outside change), so the guard against typing in a pending change does not stop them.

use super::*;
use crate::logic::{assistant, markdown, pending};

impl Editor {
    /// The change the caret is in; else the first one after the caret; else the first of the note.
    fn change_index(&self) -> Option<usize> {
        let at = self.cursor();
        self.pending
            .iter()
            .position(|area| area.range.start <= at && at < area.range.end)
            .or_else(|| self.pending.iter().position(|area| area.range.start >= at))
            .or(if self.pending.is_empty() {
                None
            } else {
                Some(0)
            })
    }

    /// Says that there is no change to act on: the one message of the actions that find none.
    fn no_change(&mut self, cx: &mut Context<Self>) {
        cx.emit(EditorEvent::Notice(
            tr("No pending change.", "Aucun changement en attente.").into(),
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
        let Some(index) = self.change_index() else {
            return self.no_change(cx);
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

    /// Asks the shell for the line of a comment on the change under the caret.
    pub(super) fn ask_change_comment(&mut self, cx: &mut Context<Self>) {
        match self.change_index() {
            Some(index) => cx.emit(EditorEvent::AskChangeComment(index)),
            None => self.no_change(cx),
        }
    }

    /// Writes `memo` as a comment on the first added line of change `index` (its first removed line
    /// when nothing is added). The change stays pending. Refused, with the reason, when the line
    /// cannot carry a comment: empty, or holding comment marks already.
    pub fn comment_on_change(&mut self, index: usize, memo: &str, cx: &mut Context<Self>) {
        let Some(area) = self.pending.get(index).cloned() else {
            return self.no_change(cx);
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
