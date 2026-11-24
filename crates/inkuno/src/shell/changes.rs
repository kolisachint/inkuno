//! Comments on a pending change (`Comment on the change`): asks for the comment's line in a prompt,
//! then writes it into the note with the change still pending (`Editor::comment_on_change`).

use super::*;

impl Shell {
    /// The changes of the note in the pane that its text does not place (orphans): their count, as
    /// last counted (`count_unplaced_changes`). They are shown nowhere, so the pill is the only place
    /// that says so.
    pub(super) fn unplaced_changes(&self) -> usize {
        self.pane().orphans
    }

    /// Counts the orphans of the note in the pane from its list and its file, when it is loaded, saved
    /// or discarded, and when the file may have changed: the pill then reads the count, not the disk.
    /// The list itself is kept beside it, for the Review feed (`pending_saved`).
    pub(super) fn count_unplaced_changes(&mut self) {
        let (saved, count) = self
            .pane()
            .path
            .as_deref()
            .and_then(|path| {
                let home = platform::revisions::home_of(path, self.vault.as_deref())?;
                let saved = platform::sidecar::read(&home, path);
                if saved.is_empty() {
                    return None;
                }
                let disk = std::fs::read_to_string(path).unwrap_or_default();
                let count = platform::sidecar::unplaced(&home, path, &disk).len();
                Some((saved, count))
            })
            .unwrap_or_default();
        self.pane_mut().orphans = count;
        self.pane_mut().pending_saved = saved;
    }

    /// Discard: the orphans of the note in the pane leave its list beside it. The text is not touched,
    /// and this is not an undo step: an orphan is in no text, so there is no edit to put back.
    pub(super) fn discard_unplaced_changes(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        let Some(home) = platform::revisions::home_of(&path, self.vault.as_deref()) else {
            return;
        };
        let disk = std::fs::read_to_string(&path).unwrap_or_default();
        match platform::sidecar::discard_unplaced(&home, &path, &disk) {
            Ok(_) => {
                self.count_unplaced_changes();
                cx.notify()
            }
            Err(e) => self.say(
                Tone::Failed,
                format!("{}: {}", path.display(), crate::lang::io_reason(&e)),
                cx,
            ),
        }
    }

    /// The hint at the note's foot, left of the corner buttons: "2 changes pending", and with orphans,
    /// "1 suggestion could not be placed · Discard". A click on the changes opens the Review feed on the
    /// first change (`review_first_change`); Discard removes the orphans. It is drawn over the text like the corner buttons, so no line moves.
    pub(super) fn pending_hint(
        &self,
        count: usize,
        orphans: usize,
        corner: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let mut label = match count {
            0 => String::new(),
            1 => tr("1 change pending", "1 changement en attente").to_string(),
            _ => format!(
                "{count} {}",
                tr("changes pending", "changements en attente")
            ),
        };
        if orphans > 0 {
            let unplaced = match orphans {
                1 => tr(
                    "1 suggestion could not be placed",
                    "1 suggestion n'a pas pu être placée",
                )
                .to_string(),
                _ => format!(
                    "{orphans} {}",
                    tr(
                        "suggestions could not be placed",
                        "suggestions n'ont pas pu être placées"
                    )
                ),
            };
            if !label.is_empty() {
                label.push_str(" · ");
            }
            label.push_str(&unplaced);
        }
        let discard = orphans > 0;
        div()
            .id("pending-hint")
            .debug_selector(|| "pending-hint".into())
            .absolute()
            .bottom_4()
            .right(t.px(12. + 38. * corner as f32))
            .h(t.px(30.))
            .px(t.px(10.))
            .flex()
            .items_center()
            .rounded_full()
            .bg(t.panel)
            .border_1()
            .border_color(t.border)
            .text_color(t.soft)
            .text_size(t.px(13.))
            .cursor_pointer()
            .occlude()
            .hover(|s| s.bg(t.hover))
            .child(label)
            .when(discard, |d| {
                d.child(
                    div()
                        .id("pending-discard")
                        .debug_selector(|| "pending-discard".into())
                        .ml(t.px(6.))
                        .text_color(t.text)
                        .child(format!(" · {}", tr("Discard", "Abandonner")))
                        .on_click(cx.listener(|this, _, _, cx| {
                            cx.stop_propagation();
                            this.discard_unplaced_changes(cx);
                        })),
                )
            })
            .on_click(cx.listener(|this, _, window, cx| {
                this.review_first_change(window, cx);
                cx.notify();
            }))
    }

    /// Asks for the line of a comment on the pending change `change` (its text) of `editor`'s note.
    /// Enter writes it, Escape cancels; the note keeps the focus after either. The change is found
    /// again by its text when the prompt answers: an outside edit may have moved it meanwhile.
    pub(super) fn ask_change_comment(
        &mut self,
        editor: Entity<Editor>,
        change: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (theme, label) = (
            self.theme,
            tr("What does the comment say?", "Que dit le commentaire ?"),
        );
        let prompt = cx.new(|cx| Palette::prompt(label, "", theme, cx));
        cx.subscribe_in(&prompt, window, move |this, asked, event, window, cx| {
            // Only the prompt that asked may answer: another palette may have taken its place.
            if this.palette.as_ref() != Some(asked) {
                return;
            }
            match event {
                PaletteEvent::Submit(memo) => {
                    this.palette = None;
                    window.focus(&editor.focus_handle(cx), cx);
                    let memo = memo.clone();
                    let author = this.revision_author();
                    let time =
                        crate::logic::markdown::utc_minute(crate::platform::revisions::now());
                    let by = crate::logic::markdown::Stamp {
                        author: &author,
                        time: &time,
                    };
                    editor.update(cx, |note, cx| {
                        note.comment_on_change(&change, &memo, by, cx)
                    });
                }
                PaletteEvent::Dismiss => {
                    this.palette = None;
                    window.focus(&editor.focus_handle(cx), cx);
                }
                _ => {}
            }
            cx.notify();
        })
        .detach();
        window.focus(&prompt.focus_handle(cx), cx);
        self.palette = Some(prompt);
        cx.notify();
    }
}
