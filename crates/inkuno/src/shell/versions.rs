//! The Versions panel and the compare (`app-ui.md` §19): the saved versions of the note of the active
//! tab (or of the note a compare belongs to), newest first, and a compare of one of them with the note
//! as it is now. The compare opens in a read-only tab of its own, where the difference is drawn as
//! pending changes (`logic::revision::compare`).

use super::*;
use crate::logic::revision::{self, CompareError, Kind, Revision};
use crate::platform::revisions::{self as history, Listed};
use std::collections::HashMap;

/// A version of the panel: the file it is saved in (its row's key), its number, and the revision, or
/// `None` when its file cannot be read (the row says so, and cannot be compared).
pub(super) struct Version {
    pub(super) path: PathBuf,
    pub(super) number: u32,
    pub(super) rev: Option<Revision>,
}

/// The panel's list: the versions of the note of the active tab, newest first, and what the panel
/// says in place of the rows when there are none to list.
#[derive(Default)]
pub(super) struct Versions {
    /// The note the list is of; `None` when the active tab shows no note.
    pub(super) note: Option<PathBuf>,
    pub(super) items: Vec<Version>,
    /// The place in `items` of each row, by the row's file: a lookup per row, not a scan.
    index: HashMap<PathBuf, usize>,
    /// The quiet text of the panel when the list is empty (or cannot be read).
    pub(super) said: String,
}

/// A comparison tab: the version it compares with the note, and the name its tab shows.
pub(super) struct Compare {
    pub(super) note: PathBuf,
    pub(super) number: u32,
    pub(super) title: String,
}

impl Shell {
    /// Reads the versions of the note of the active tab again when the list is not of that note, or
    /// is stale (a save, a proposal, the panel shown, a revision written from a terminal). Called as
    /// the panel is drawn: the disk is read only then, never on a frame that changes nothing.
    pub(super) fn sync_versions(&mut self, cx: &App) {
        let note = self.versions_note();
        let current = self.versions.as_ref().is_some_and(|list| list.note == note);
        if current && !self.versions_stale {
            return;
        }
        self.versions_stale = false;
        self.versions = Some(self.read_versions(note, cx));
        self.versions_rev += 1;
    }

    /// A revision was written outside the panel's own saves (`inkuno rev save`, another window): the
    /// list is read again when the panel is drawn. Only the list: the vault is not reread for it.
    pub(super) fn versions_moved(&mut self, cx: &mut Context<Self>) {
        self.versions_stale = true;
        cx.notify();
    }

    /// The note the panel lists: the one a comparison belongs to when a comparison is shown (its tab
    /// shows no note of its own), else the note of the active tab.
    pub(super) fn versions_note(&self) -> Option<PathBuf> {
        let pane = self.pane();
        pane.compare
            .as_ref()
            .map(|compare| compare.note.clone())
            .or_else(|| pane.path.clone())
    }

    /// The versions of `note` (`None`: the tab shows no note), newest first, and what the panel says.
    /// A history that cannot be read says why, instead of looking empty; a version whose file cannot
    /// be read is listed as a row of its own, the others are not hidden by it.
    fn read_versions(&self, note: Option<PathBuf>, cx: &App) -> Versions {
        let said = |text: &str| Versions {
            note: note.clone(),
            items: Vec::new(),
            index: HashMap::new(),
            said: text.to_string(),
        };
        let Some(path) = note.clone() else {
            return said(tr("No note is open.", "Aucune note n'est ouverte."));
        };
        let Some(home) = history::home_of(&path, self.vault.as_deref()) else {
            return said(tr(
                "This note has no history.",
                "Cette note n'a pas d'historique.",
            ));
        };
        let revs = match history::listed(&home, &path) {
            Ok(revs) => revs,
            Err(e) => {
                return said(&format!(
                    "{} {}",
                    tr(
                        "The versions cannot be read:",
                        "Les versions ne peuvent pas être lues :"
                    ),
                    crate::lang::io_reason(&e)
                ));
            }
        };
        if revs.is_empty() {
            let key = crate::ui::keys::of(cx, &SaveRevision);
            let key = match key.is_empty() {
                true => tr("Save a version", "Enregistrer une version").to_string(),
                false => key,
            };
            return said(&format!(
                "{} {key}.",
                tr(
                    "No version yet. Save one with",
                    "Aucune version encore. Enregistrez-en une avec"
                )
            ));
        }
        // Each version is saved in its own file, in the note's history folder.
        let folder = home.folder(&path);
        let items: Vec<Version> = revs
            .into_iter()
            .rev()
            .filter_map(|listed: Listed| {
                let number = match &listed {
                    Ok(rev) => rev.number,
                    Err((number, _)) => *number,
                };
                let file = folder.as_ref()?.join(revision::file_name(number));
                Some(Version {
                    path: file,
                    number,
                    rev: listed.ok(),
                })
            })
            .collect();
        let index = items
            .iter()
            .enumerate()
            .map(|(at, item)| (item.path.clone(), at))
            .collect();
        Versions {
            note,
            items,
            index,
            said: String::new(),
        }
    }

    /// The panel's rows: one per version, newest first, each keyed by its file. A version that cannot
    /// be read is one quiet row that says so.
    pub(super) fn version_rows(&self) -> Vec<nav::Row> {
        let Some(list) = &self.versions else {
            return Vec::new();
        };
        list.items
            .iter()
            .map(|item| nav::Row {
                path: item.path.clone(),
                name: match &item.rev {
                    Some(rev) => revision::row_name(rev, tr("auto", "auto")),
                    None => format!("#{} · {}", item.number, tr("cannot be read", "illisible")),
                },
                depth: 0,
                dir: None,
            })
            .collect()
    }

    /// The version a panel row is for, by its file.
    pub(super) fn version_of(&self, path: &Path) -> Option<&Version> {
        let list = self.versions.as_ref()?;
        list.index.get(path).and_then(|&at| list.items.get(at))
    }

    /// Whether a panel row is drawn quietly: an automatic version, or one that cannot be read.
    pub(super) fn version_quiet(&self, path: &Path) -> bool {
        self.version_of(path)
            .is_some_and(|item| item.rev.as_ref().is_none_or(|rev| rev.kind == Kind::Auto))
    }

    /// Whether a panel row is the version the shown comparison compares: the one marked as current.
    pub(super) fn version_current(&self, path: &Path) -> bool {
        let Some(compare) = &self.pane().compare else {
            return false;
        };
        self.version_of(path)
            .is_some_and(|item| item.number == compare.number)
            && self.versions.as_ref().and_then(|list| list.note.as_ref()) == Some(&compare.note)
    }

    /// What the panel says in place of its rows, when it has none to show.
    pub(super) fn versions_said(&self) -> String {
        self.versions
            .as_ref()
            .map(|list| list.said.clone())
            .unwrap_or_default()
    }

    /// The text of `note` as it is now: the tab that shows it (its unsaved text included), else the
    /// file on disk. `None` when neither can be read.
    fn note_now(&self, note: &Path, cx: &App) -> Option<String> {
        let shown = self
            .panes
            .iter()
            .find(|pane| pane.compare.is_none() && pane.path.as_deref() == Some(note));
        match shown {
            Some(pane) => Some(pane.editor.read(cx).text().to_string()),
            None => std::fs::read_to_string(note).ok(),
        }
    }

    /// Enter, a click, or the menu's compare: the version compared with the note as it is now, in a
    /// read-only tab of its own, titled `<note> · #12 ↔ now`. The tab that already shows that version
    /// comes to the front (its text is replaced when the note has changed since). Refused, with a
    /// message and no tab, when the version cannot be read, the two texts are the same, or either
    /// holds pending changes.
    pub(super) fn compare_version(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(item) = self.version_of(path) else {
            return;
        };
        let (number, rev) = (item.number, item.rev.clone());
        let Some(note) = self.versions.as_ref().and_then(|list| list.note.clone()) else {
            return;
        };
        let Some(rev) = rev else {
            let said = format!(
                "#{number} · {}",
                tr(
                    "this version cannot be read, so it cannot be compared.",
                    "cette version ne peut pas être lue, elle ne peut pas être comparée."
                )
            );
            return self.say(Tone::Failed, said, cx);
        };
        let Some(now) = self.note_now(&note, cx) else {
            return self.say(
                Tone::Failed,
                tr(
                    "The note cannot be read: nothing to compare.",
                    "La note ne peut pas être lue : rien à comparer.",
                ),
                cx,
            );
        };
        let text = match revision::compare(&rev.text, &now) {
            Ok(text) => text,
            Err(CompareError::Unchanged) => {
                let said = format!(
                    "{} {}.",
                    tr(
                        "Nothing changed since version",
                        "Rien n'a changé depuis la version"
                    ),
                    rev.number
                );
                return self.say(Tone::Info, said, cx);
            }
            Err(CompareError::Pending) => {
                return self.say(
                    Tone::Info,
                    tr(
                        "This version or the note holds pending changes: accept or reject them first.",
                        "Cette version ou la note contient des changements en attente : acceptez-les ou refusez-les d'abord.",
                    ),
                    cx,
                );
            }
        };
        let title = format!(
            "{} · #{} ↔ {}",
            vault::stem(&note),
            rev.number,
            tr("now", "maintenant")
        );
        let shown = self.panes.iter().position(|pane| {
            pane.compare
                .as_ref()
                .is_some_and(|c| c.note == note && c.number == rev.number)
        });
        let tab = match shown {
            Some(tab) => tab,
            None => {
                self.add_tab(window, cx);
                self.active
            }
        };
        let pane = &mut self.panes[tab];
        pane.compare = Some(Compare {
            note,
            number: rev.number,
            title,
        });
        pane.editor.update(cx, |editor, cx| {
            if editor.text() != text {
                editor.load(text, 0, cx);
            }
            editor.set_readonly(true);
        });
        self.focus_pane(tab, window, cx);
        cx.notify();
    }

    /// A gesture on a version of the panel (its menu, or a key the panel has): compared with the note
    /// now, or its text copied. A version is history: no other gesture applies to it.
    pub(super) fn version_do(
        &mut self,
        what: nav::Do,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match what {
            nav::Do::Open | nav::Do::CompareVersion => self.compare_version(path, window, cx),
            nav::Do::CopyVersion => {
                if let Some(rev) = self.version_of(path).and_then(|item| item.rev.as_ref()) {
                    let text = rev.text.clone();
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// The date a panel row shows on its right: the version's, to the minute.
    pub(super) fn version_date(&self, path: &Path) -> String {
        self.version_of(path)
            .and_then(|item| item.rev.as_ref())
            .map(|rev| revision::short_date(&rev.date))
            .unwrap_or_default()
    }
}
