//! Versions of a note (`Save a version`, `Ctrl+Alt+S`): the text as it is now becomes a revision of
//! the note's history (`platform/revisions.rs`), with the author and an optional line on what
//! changed. Nothing is saved when the text is the latest revision's already.

use super::*;
use crate::logic::revision;
use crate::platform::revisions::{self as history, Saved};

impl Shell {
    /// `Save a version`: asks for a line on what changed (Enter skips it, Escape cancels), then saves
    /// the note's text as a new revision. When the text is the latest revision's, says so and asks
    /// nothing.
    pub(super) fn save_revision(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.flush_one(cx) {
            return;
        }
        let Some(path) = self.pane().path.clone() else {
            return self.say(
                Tone::Info,
                tr(
                    "Nothing to save yet: write something first",
                    "Rien à enregistrer : écrivez d'abord",
                ),
                cx,
            );
        };
        let Some(home) = history::home_of(&path, self.vault.as_deref()) else {
            return self.say(
                Tone::Failed,
                tr(
                    "This note has no history: it is not in the vault",
                    "Cette note n'a pas d'historique : elle n'est pas dans le coffre",
                ),
                cx,
            );
        };
        let text = self.pane().editor.read(cx).text().to_string();
        match history::latest(&home, &path) {
            Ok(Some(last)) if last.text == text => {
                return self.say(
                    Tone::Info,
                    format!(
                        "{} {}",
                        tr(
                            "Nothing changed since revision",
                            "Rien n'a changé depuis la version"
                        ),
                        last.number
                    ),
                    cx,
                );
            }
            Ok(_) => {}
            Err(e) => return self.fail(tr("Not saved", "Non enregistré"), &path, e, cx),
        }
        let (theme, label) = (
            self.theme,
            tr(
                "What changed or was decided? (Enter to skip)",
                "Qu'est-ce qui a changé ou été décidé ? (Entrée pour passer)",
            ),
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
                    window.focus(&this.pane().editor.focus_handle(cx));
                    this.write_revision(&path, memo, cx);
                }
                PaletteEvent::Dismiss => {
                    this.palette = None;
                    window.focus(&this.pane().editor.focus_handle(cx));
                }
                _ => {}
            }
            cx.notify();
        })
        .detach();
        window.focus(&prompt.focus_handle(cx));
        self.palette = Some(prompt);
        cx.notify();
    }

    /// Writes the revision of the note at `path`, its text as its tab shows it now, and says what
    /// became of it.
    fn write_revision(&mut self, path: &Path, memo: &str, cx: &mut Context<Self>) {
        let Some(home) = history::home_of(path, self.vault.as_deref()) else {
            return;
        };
        let text = match self.pane_showing(path) {
            Some(tab) => self.panes[tab].editor.read(cx).text().to_string(),
            None => match fs::read_to_string(path) {
                Ok(text) => text,
                Err(e) => return self.fail(tr("Not saved", "Non enregistré"), path, e, cx),
            },
        };
        let memo = Some(memo.trim()).filter(|m| !m.is_empty());
        let author = self.revision_author();
        match history::save(&home, path, &author, memo, &text, history::now()) {
            Ok(Saved::Made { number, .. }) => self.say(
                Tone::Done,
                format!(
                    "{} {number} {}",
                    tr("Revision", "Version"),
                    tr("saved", "enregistrée")
                ),
                cx,
            ),
            Ok(Saved::Unchanged { latest }) => self.say(
                Tone::Info,
                format!(
                    "{} {latest}",
                    tr(
                        "Nothing changed since revision",
                        "Rien n'a changé depuis la version"
                    )
                ),
                cx,
            ),
            Err(e) => self.fail(tr("Not saved", "Non enregistré"), path, e, cx),
        }
    }

    /// The author of the revisions made here: the setting, else `INKUNO_AUTHOR`, else the system
    /// user name, else `unknown`.
    pub(super) fn revision_author(&self) -> String {
        match revision::clean_author(&self.prefs.author) {
            name if !name.is_empty() => name,
            _ => revision::default_author(|key| std::env::var(key).ok()),
        }
    }
}
