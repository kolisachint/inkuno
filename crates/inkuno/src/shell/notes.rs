//! Notes: opening, previewing, creating, saving, renaming with their title, links and wikilinks.

use super::*;
use std::{
    io,
    sync::{Arc, Condvar, Mutex},
};

/// A note a rename may rewrite the links of: its path on disk now, its path after the rename, and
/// whether it links to the old name by `[[name]]`.
pub(super) struct LinkedNote {
    disk: PathBuf,
    post: PathBuf,
    wiki: bool,
}

/// What a rename does to the notes that link to it (see `Shell::link_scan`). Owned, so that it can run
/// off the UI thread.
#[derive(Default)]
pub(super) struct LinkScan {
    root: Option<PathBuf>,
    notes: Vec<LinkedNote>,
    old: String,
    new: String,
    moved: Option<(PathBuf, PathBuf)>,
    shown: Option<(PathBuf, String)>,
}

/// The result of a scan: whether a pending change refuses the rename, and the notes that hold a link to
/// rewrite (indexes into `scan.notes`). The rewrites are planned again when they are applied, from the
/// text each note holds then (see `Shell::apply_links`); the scan keeps what it needs for that.
#[derive(Default)]
pub(super) struct LinkPlan {
    pub(super) refused: bool,
    scan: LinkScan,
    notes: Vec<usize>,
}

impl LinkScan {
    /// The text `text` of a note (its file with its pending changes composed in, or the editor's text)
    /// with its links rewritten, or None when it holds none.
    fn rewrite(&self, note: &LinkedNote, text: &str) -> Option<String> {
        let moved = self
            .moved
            .as_ref()
            .map(|(from, to)| (from.as_path(), to.as_path()));
        logic::wiki::rewrite_links(
            text,
            self.root.as_deref(),
            note.post.as_path(),
            &self.old,
            &self.new,
            note.wiki,
            moved,
        )
    }

    /// Reads each note once and chooses the notes whose links the rename rewrites. The rename is refused
    /// when a rewrite would change a pending change.
    pub(super) fn run(self) -> LinkPlan {
        let mut plan = LinkPlan::default();
        for (index, note) in self.notes.iter().enumerate() {
            let Ok(disk) = fs::read_to_string(&note.disk) else {
                continue;
            };
            let current = match &self.shown {
                Some((path, text)) if *path == note.disk => text.clone(),
                _ => {
                    let home = platform::revisions::home_of(&note.disk, self.root.as_deref());
                    platform::sidecar::display(home.as_ref(), &note.disk, &disk)
                }
            };
            if let Some(fresh) = self.rewrite(note, &current) {
                if logic::pending::rewrite_refused(&current, &fresh) {
                    plan.refused = true;
                }
                plan.notes.push(index);
            }
        }
        plan.scan = self;
        plan
    }
}

impl Shell {
    /// Shows a Mermaid figure or a picture of the note large, in its place.
    pub(super) fn show_figure(
        &mut self,
        source: viewer::Source,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let theme = self.theme;
        let figure = cx.new(|cx| viewer::Viewer::new(source, theme, cx));
        window.focus(&figure.focus_handle(cx), cx);
        self.pane_mut().figure = Some(figure);
        cx.notify();
    }

    /// Opens an HTML page in `inkuno view`, a window of its own. Its root is the page's own folder,
    /// so the viewer reads nothing outside it. The viewer is a process of its own, started and not
    /// waited for. False when this build has no viewer (Windows has none), the page cannot be
    /// served from its folder, or the process did not start: the caller then hands the page to the
    /// system.
    pub(super) fn open_html_in_viewer(&mut self, file: &Path) -> bool {
        let Some(root) = file.parent() else {
            return false;
        };
        if platform::web::url_for(root, file).is_none() {
            return false;
        }
        let Some(args) = platform::viewer::view_args(file, root) else {
            return false;
        };
        #[cfg(test)]
        {
            // The test process must not run itself as a viewer: the request is recorded instead.
            let _ = args;
            self.viewed.push(file.to_path_buf());
            true
        }
        #[cfg(all(not(test), feature = "viewer", not(windows)))]
        {
            std::env::current_exe()
                .and_then(|exe| {
                    platform::command(exe)
                        .args(&args)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .spawn()
                        .map(|mut child| {
                            std::thread::spawn(move || {
                                let _ = child.wait();
                            });
                        })
                })
                .is_ok()
        }
        #[cfg(all(not(test), not(all(feature = "viewer", not(windows)))))]
        {
            let _ = args;
            false
        }
    }

    /// Copies the whole note to the clipboard.
    /// Copies the note; with `block`, only the code block the cursor is in,
    /// if there is one.
    pub(super) fn copy_all(&mut self, block: bool, cx: &mut Context<Self>) {
        if self.pane().file.is_some()
            || self.pane().listing.is_some()
            || self.pane().figure.is_some()
        {
            return;
        }
        let editor = self.pane().editor.read(cx);
        let code = editor.code_at_cursor().filter(|_| block);
        // The note's text without its pending marks: what it will say once every change is accepted.
        let text = match code {
            Some(code) => code.to_string(),
            None => logic::pending::plain(editor.text()),
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
        self.copied = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(1500))
                .await;
            this.update(cx, |this, cx| {
                this.copied = false;
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Loads the note into the editor; false if the file is unreadable.
    pub(super) fn load_note(&mut self, path: &Path, cx: &mut Context<Self>) -> bool {
        let shown = self.load_note_from_disk(path, cx);
        self.count_unplaced_changes();
        shown
    }

    fn load_note_from_disk(&mut self, path: &Path, cx: &mut Context<Self>) -> bool {
        // Already in the other pane: two editors on the same file would overwrite each other.
        // Input moves there; a mere preview does not move it.
        if let Some(other) = self
            .pane_showing(path)
            .filter(|other| *other != self.active)
        {
            if !self.pane().peek {
                self.active = other;
                if self.pane().path.as_deref() == Some(path) {
                    self.set_file(None, cx);
                    self.pane_mut().listing = None;
                }
                self.grab = true;
                cx.notify();
            }
            return false;
        }
        self.pane_mut().listing = None;
        // An attachment (PDF, video, archive…) opens in the system's application; the note stays.
        // An HTML page opens in the viewer instead, when this build has one.
        if vault::is_attachment(path) {
            if platform::viewer::is_html(path) && self.open_html_in_viewer(path) {
                return false;
            }
            // The test platform opens nothing: it records what it would have been asked.
            #[cfg(test)]
            self.opened.push(path.to_path_buf());
            #[cfg(not(test))]
            cx.open_with_system(path);
            return false;
        }
        // A picture takes the note's place; the note stays loaded underneath.
        // An SVG file opens in the viewer: zoom in, move around.
        if vault::is_image(path) {
            let svg = path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
            let open = self
                .pane()
                .file
                .as_ref()
                .filter(|file| file.path == path)
                .map(|file| file.kind.clone());
            let theme = self.theme;
            let kind = match open {
                _ if !svg => FileKind::Image,
                Some(FileKind::Svg(viewer)) => FileKind::Svg(viewer),
                _ => {
                    let source = viewer::Source::File(path.to_path_buf(), None);
                    FileKind::Svg(cx.new(|cx| viewer::Viewer::new(source, theme, cx)))
                }
            };
            self.set_file(
                Some(FileView {
                    path: path.to_path_buf(),
                    kind,
                }),
                cx,
            );
            cx.notify();
            return false;
        }
        if !self.leave(cx) {
            return false;
        }
        self.set_file(None, cx);
        cx.notify();
        match fs::read_to_string(path) {
            Ok(disk) => {
                // The note's pending changes beside the file are shown as marked areas.
                let home = platform::revisions::home_of(path, self.vault.as_deref());
                let text = platform::sidecar::display(home.as_ref(), path, &disk);
                // Second line of defence: a note never loads into a comparison (the routes open a
                // tab of their own first). If one did, the tab becomes the note: editable, no comparison.
                self.pane_mut().compare = None;
                // A scratch note keeps its moment as its name: its title never renames it, and no
                // link points at it.
                let scratch = platform::scratch::holds(path);
                self.pane_mut().scratch = scratch;
                self.pane_mut().synced = !scratch && vault::stem(path) == vault::title_of(&text);
                self.pane_mut().fresh = false;
                self.pane_mut().h1 = vault::h1_of(&text);
                self.pane_mut().origin = (!scratch).then(|| vault::stem(path));
                self.pane_mut().path = Some(path.to_path_buf());
                self.pane_mut().base = text.clone();
                self.calm();
                let cursor = markdown::body_start(&text);
                self.pane()
                    .editor
                    .update(cx, |e, cx| e.load(text, cursor, cx));
                self.push_dirs(cx);
                self.nav.reveal(path);
                true
            }
            Err(e) => {
                let what = tr("Cannot open", "Impossible d'ouvrir");
                self.say(
                    Tone::Failed,
                    format!("{what} {} : {}", path.display(), crate::lang::io_reason(&e)),
                    cx,
                );
                false
            }
        }
    }

    pub(super) fn open_note(&mut self, path: &Path, cx: &mut Context<Self>) {
        self.pane_mut().peek = false;
        if self.load_note(path, cx) {
            self.pane_mut().preview = false;
            self.touch(path);
        }
    }

    /// Shows the note without counting it as opened.
    pub(super) fn preview_note(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Already open: its tab is shown, input stays where it is, and the history does not count it.
        if let Some(tab) = self.pane_showing(path) {
            self.active = tab;
            self.show_active(cx);
            self.pane_mut().peek = true;
            return cx.notify();
        }
        // One preview tab per pane: each note browsed takes the place of the last one.
        let group = self.group();
        match self.groups[group]
            .tabs
            .iter()
            .copied()
            .find(|&tab| self.panes[tab].preview)
        {
            Some(tab) => {
                self.active = tab;
                self.show_active(cx);
            }
            None => self.fresh_tab(window, cx),
        }
        self.pane_mut().peek = true;
        // The note already loaded, hidden by a picture: removing the picture is enough.
        if self.pane().path.as_deref() == Some(path) {
            if self.pane().file.is_some() {
                self.set_file(None, cx);
                cx.notify();
            }
        } else if self.load_note(path, cx) {
            self.pane_mut().preview = true;
        }
    }

    /// The preview becomes an opened note.
    pub(super) fn keep_preview(&mut self) {
        self.pane_mut().peek = false;
        if std::mem::take(&mut self.pane_mut().preview)
            && let Some(path) = self.pane().path.clone()
        {
            self.touch(&path);
        }
    }

    /// New note, filed in the selected folder when the tree is shown.
    pub(super) fn new_note_here(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.new_note_in(self.nav.target_dir(), window, cx);
    }

    /// New note filed in `dir`; `None`: at the vault root.
    pub(super) fn new_note_in(
        &mut self,
        dir: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.vault.is_none() {
            return;
        }
        self.fresh_tab(window, cx);
        self.new_note(String::new(), cx);
        self.pane_mut().new_dir = dir;
        if self.nav.panel == Panel::Full {
            self.nav.panel = Panel::Split;
        }
        window.focus(&self.pane().editor.focus_handle(cx), cx);
        self.settle_nav(window, cx);
    }

    pub(super) fn new_note(&mut self, text: String, cx: &mut Context<Self>) {
        self.pane_mut().peek = false;
        if !self.leave(cx) {
            return;
        }
        self.pane_mut().listing = None;
        self.set_file(None, cx);
        self.pane_mut().path = None;
        self.pane_mut().origin = None;
        self.pane_mut().synced = true;
        self.pane_mut().preview = false;
        self.pane_mut().new_dir = None;
        self.pane_mut().scratch = false;
        self.pane_mut().compare = None;
        self.pane_mut().fresh = true;
        self.pane_mut().dirty = !text.is_empty();
        let cursor = text.len();
        self.pane()
            .editor
            .update(cx, |e, cx| e.load(text, cursor, cx));
        self.push_dirs(cx);
        self.flush(cx);
        cx.notify();
    }

    /// Before leaving the shown note: it is saved and, if its title
    /// renamed it, links to its old name follow. Waiting for this moment
    /// avoids rewriting the links at every intermediate title while typing.
    /// False means the current view must stay alive, with its dirty state and undo history.
    pub(super) fn leave(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.flush_one(cx) {
            return false;
        }
        self.flush(cx);
        self.follow_title(cx);
        self.drop_if_empty(cx);
        true
    }

    /// The links to the note follow the name its title gave it since it was opened.
    pub(super) fn follow_title(&mut self, cx: &mut Context<Self>) {
        if let (Some(old), Some(path)) = (self.pane_mut().origin.take(), self.pane().path.clone()) {
            let new = vault::stem(&path);
            // Only a note that changed its name moved: opening a note never rewrites another.
            if old != new {
                let from = path.with_file_name(format!("{old}.md"));
                self.relink(&old, &new, Some((&from, &path)), cx);
            }
            // Still open in its tab: a later title follows from this name.
            self.pane_mut().origin = Some(new);
        }
    }

    /// A note created here then emptied leaves no empty file behind, nor in the
    /// recent list. Only an empty file on disk is removed: nothing written is lost.
    pub(super) fn drop_if_empty(&mut self, cx: &mut Context<Self>) {
        if !std::mem::take(&mut self.pane_mut().fresh)
            || !self.pane_mut().editor.read(cx).text().trim().is_empty()
        {
            return;
        }
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        if fs::read_to_string(&path).is_ok_and(|text| text.trim().is_empty())
            && fs::remove_file(&path).is_ok()
        {
            self.notes.retain(|n| n.path != path);
            self.recent.retain(|p| *p != path);
            if let Some(root) = &self.vault {
                vault::save_config(root, &self.recent);
            }
            self.pane_mut().path = None;
            self.push_names(cx);
        }
    }

    /// `[[links]]` to `old` now point to `new` in every note, and the relative links to a note
    /// moved from `moved.0` to `moved.1`; true if the shown note was rewritten. No effect on the
    /// `[[links]]` if another note is still called `old`: the links may be meant for it.
    // ponytail: synchronous rewrite, the `[[links]]` limited to the notes the index says link to
    // `old`; a note moved reads every note of the vault, as a relative link names its file. The
    // rename menu runs the same scan off the UI thread (see `link_scan`).
    pub(super) fn relink(
        &mut self,
        old: &str,
        new: &str,
        moved: Option<(&Path, &Path)>,
        cx: &mut Context<Self>,
    ) -> bool {
        let plan = self.link_scan(old, new, moved, None).run();
        self.apply_links(plan, cx)
    }

    /// The scan of the notes that a rename of `old` to `new` rewrites links in, to run off the UI
    /// thread (see `LinkScan::run`). `moved`: a note file that moves from its path to the other, whose
    /// relative addresses any note may hold, so every note is read; otherwise only the notes the index
    /// says link to `old`. `shown`: the text an editor holds for a note, read in place of its file.
    /// Note that the moved note may still bear its old name here (a rename not done yet).
    pub(super) fn link_scan(
        &self,
        old: &str,
        new: &str,
        moved: Option<(&Path, &Path)>,
        shown: Option<(&Path, &str)>,
    ) -> LinkScan {
        let moved = moved.filter(|(from, to)| from != to);
        let key = old.to_lowercase();
        // As `logic::wiki::relink_set`: nothing is rewritten when the names match ignoring case, or
        // when another note is still called `old`.
        let kept = key == new.to_lowercase()
            || self.notes.iter().any(|n| {
                n.name.to_lowercase() == key && moved.is_none_or(|(from, _)| n.path != from)
            });
        let wiki = |n: &Note| !kept && n.links.contains(&key);
        let notes = self
            .notes
            .iter()
            .filter(|n| moved.is_some() || wiki(n))
            .map(|n| LinkedNote {
                post: match moved {
                    Some((from, to)) if n.path == from => to.to_path_buf(),
                    _ => n.path.clone(),
                },
                wiki: wiki(n),
                disk: n.path.clone(),
            })
            .collect();
        LinkScan {
            root: self.vault.clone(),
            notes,
            old: old.to_string(),
            new: new.to_string(),
            moved: moved.map(|(from, to)| (from.to_path_buf(), to.to_path_buf())),
            shown: shown.map(|(path, text)| (path.to_path_buf(), text.to_string())),
        }
    }

    /// Writes the rewrites of a scan and reloads the other pane when it shows a rewritten note. True
    /// when the shown note was rewritten: the caller decides what its editor does (it may be leaving).
    /// The scan ran off the UI thread, so each note is read again here, at its path after the rename,
    /// and its rewrite is planned from the text it holds now: an edit made since (an autosave, an
    /// agent's write) is kept. A note whose link is gone, or whose rewrite would change a pending
    /// change, is left as it is and reported.
    pub(super) fn apply_links(&mut self, plan: LinkPlan, cx: &mut Context<Self>) -> bool {
        let mut changed = Vec::new();
        let mut missed = false;
        for index in plan.notes {
            let note = &plan.scan.notes[index];
            let path = &note.post;
            let Ok(raw) = fs::read_to_string(path) else {
                continue;
            };
            let home = platform::revisions::home_of(path, self.vault.as_deref());
            let current = platform::sidecar::display(home.as_ref(), path, &raw);
            let fresh = plan.scan.rewrite(note, &current);
            let refused = fresh
                .as_ref()
                .is_some_and(|fresh| logic::pending::rewrite_refused(&current, fresh));
            let Some(text) = fresh.filter(|_| !refused) else {
                missed = true;
                continue;
            };
            // The text as the editor shows it goes back as the file and its pending changes beside it.
            if platform::sidecar::save(home.as_ref(), path, &text, "").is_ok() {
                if let Some(note) = self.notes.iter_mut().find(|n| n.path == *path) {
                    let disk = fs::read_to_string(path).unwrap_or_default();
                    (note.tags, note.links) = Note::index(&disk);
                }
                changed.push(path.clone());
            }
        }
        if missed {
            self.say(
                Tone::Failed,
                tr(
                    "A note that links here changed while the rename ran: its link was not rewritten.",
                    "Une note qui pointe ici a changé pendant le renommage : son lien n'a pas été réécrit.",
                ),
                cx,
            );
        }
        // The other pane's note is reloaded here; the active pane's is up to the caller
        // (it may be leaving it).
        let showing = |pane: &Pane| {
            pane.path
                .as_ref()
                .is_some_and(|path| changed.contains(path))
        };
        for pane in 0..self.panes.len() {
            if pane != self.active && showing(&self.panes[pane]) {
                self.in_pane(pane, |this| this.reload(cx));
            }
        }
        showing(&self.panes[self.active])
    }

    /// The check before the note (or image) at `moved` is renamed from `old` to `new`: when a link
    /// rewrite would change a pending change of a linking note, nothing may be renamed, and a notice
    /// says so. True then. `text` is the text `moved` is about to hold, when it is not on disk yet;
    /// a linking note is read from disk, as `relink` reads it.
    pub(super) fn links_pending(
        &mut self,
        moved: &Path,
        old: &str,
        new: &str,
        text: Option<&str>,
        cx: &mut Context<Self>,
    ) -> bool {
        let renamed = moved.with_file_name(format!("{new}.md"));
        let scan = self.link_scan(
            old,
            new,
            Some((moved, renamed.as_path())),
            text.map(|t| (moved, t)),
        );
        self.links_refused(scan.run().refused, cx)
    }

    /// The notice of a refused rename: true when `refused`.
    pub(super) fn links_refused(&mut self, refused: bool, cx: &mut Context<Self>) -> bool {
        if refused {
            self.say(
                Tone::Failed,
                tr(
                    "A note that links here has a pending change on that link: accept or reject it first.",
                    "Une note qui pointe ici a un changement en attente sur ce lien : acceptez-le ou refusez-le d'abord.",
                ),
                cx,
            );
        }
        refused
    }

    /// A heading of the note was renamed (the same headings but one, which reads differently): the links
    /// of the other notes that point at it, `[[Note#old]]` and `![[Note#old]]`, now point at the new one.
    // ponytail: synchronous rewrite, limited to the notes the index says link to this one, like `relink`;
    // typing a heading rewrites them at each save, which is as often as the note is written.
    fn follow_heading(&mut self, path: &Path, before: &str, after: &str, cx: &mut Context<Self>) {
        let (old, new): (Vec<_>, Vec<_>) = (markdown::headings(before), markdown::headings(after));
        let changed: Vec<_> = old
            .iter()
            .zip(&new)
            .filter(|(a, b)| a.0 != b.0 || a.1 != b.1)
            .collect();
        let [(was, now)] = changed.as_slice() else {
            return;
        };
        if old.len() != new.len()
            || was.0 != now.0
            || now.1.trim().is_empty()
            || was.1.trim().is_empty()
        {
            return;
        }
        let (name, key) = (vault::stem(path), vault::stem(path).to_lowercase());
        let mut rewritten = Vec::new();
        for note in self
            .notes
            .iter_mut()
            .filter(|n| n.path != path && n.links.contains(&key))
        {
            // Written as the editor shows the note, so its pending changes stay placed. A rewrite that
            // would change a pending change is left out, as `apply_links` leaves it.
            let home = platform::revisions::home_of(&note.path, self.vault.as_deref());
            let saved = platform::sidecar::edit(home.as_ref(), &note.path, "", |shown| {
                let (text, count) = wiki::rename_heading(shown, &name, was.1.trim(), now.1.trim());
                let refused = logic::pending::rewrite_refused(shown, &text)
                    || platform::sidecar::rewrite_unplaces(home.as_ref(), &note.path, shown, &text);
                (count > 0 && !refused).then_some(text)
            });
            if let Ok(true) = saved {
                let disk = fs::read_to_string(&note.path).unwrap_or_default();
                (note.tags, note.links) = Note::index(&disk);
                rewritten.push(note.path.clone());
            }
        }
        for pane in 0..self.panes.len() {
            if pane != self.active
                && self.panes[pane]
                    .path
                    .as_ref()
                    .is_some_and(|p| rewritten.contains(p))
            {
                self.in_pane(pane, |this| this.reload(cx));
            }
        }
    }

    /// Reloads the shown note from disk, rewritten outside the editor.
    pub(super) fn reload(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.pane().path.clone() {
            // Reloading is not opening: a preview stays a preview, the list page stays shown.
            // Nor does it move the reader: an agent writing at the end leaves cursor and scroll
            // where they were.
            let (preview, listing) = (self.pane_mut().preview, self.pane_mut().listing.take());
            let (at, scroll) = self.pane().editor.read(cx).view();
            if self.load_note(&path, cx) {
                self.pane()
                    .editor
                    .update(cx, |e, cx| e.set_view(at, scroll, cx));
            }
            (self.pane_mut().preview, self.pane_mut().listing) = (preview, listing);
        }
    }

    /// Brings the disk up to date: each pane writes what is waiting in it.
    pub(super) fn flush(&mut self, cx: &mut Context<Self>) {
        self.each_pane(|this| {
            this.flush_one(cx);
        });
    }

    /// Another program (an agent, a sync tool) wrote the note while it was typed in: its changes
    /// join the typed text as one undo step. When both changed the same lines, its version is
    /// written next to the note, as a sync conflict is. False if that copy could not be written:
    /// the note is then not saved, so that nothing is lost.
    pub(super) fn take_outside(&mut self, path: &Path, cx: &mut Context<Self>) -> bool {
        let Ok(raw) = fs::read_to_string(path) else {
            return true;
        };
        let home = platform::revisions::home_of(path, self.vault.as_deref());
        let disk = platform::sidecar::display(home.as_ref(), path, &raw);
        if disk == self.pane().base {
            return true;
        }
        let ours = self.pane().editor.read(cx).text().to_string();
        if disk != ours {
            match logic::merge::onto(&self.pane().base, &ours, &disk) {
                Some(edits) => self.pane().editor.update(cx, |e, cx| e.absorb(&edits, cx)),
                None => {
                    let date = date_name(today());
                    let copy = free_conflict_path(path, &date);
                    // The copy is the other version: its own text, and its pending changes beside it.
                    let (text, entries) = platform::sidecar::split(home.as_ref(), path, &disk, "");
                    let written = vault::write(&copy, &text).and_then(|()| match &home {
                        Some(home) => platform::sidecar::store(home, &copy, &entries),
                        None => Ok(()),
                    });
                    if let Err(e) = written {
                        let what = tr(
                            "Changed here and elsewhere, and the other version could not be kept:",
                            "Modifiée ici et ailleurs, et l'autre version n'a pas pu être gardée :",
                        );
                        self.say(
                            Tone::Failed,
                            format!(
                                "{what} « {} » : {}",
                                path.display(),
                                crate::lang::io_reason(&e)
                            ),
                            cx,
                        );
                        return false;
                    }
                    let what = tr(
                        "Changed here and elsewhere on the same lines: the other version is in",
                        "Modifiée ici et ailleurs sur les mêmes lignes : l'autre version est dans",
                    );
                    self.say(Tone::Info, format!("{what} {}", vault::stem(&copy)), cx);
                }
            }
        }
        self.pane_mut().base = disk;
        true
    }

    /// The pane's text to save. A comment typed in the earlier form is written in the new one, and
    /// the editor takes the new form too (one undo step).
    pub(super) fn migrated_text(&mut self, cx: &mut Context<Self>) -> String {
        let shown = self.pane().editor.read(cx).text().to_string();
        // One edit per comment: the cursors keep their places around the ones that do not move.
        let edits = markdown::migration_edits(&shown);
        if edits.is_empty() {
            return shown;
        }
        self.pane().editor.update(cx, |e, cx| e.absorb(&edits, cx));
        markdown::apply_edits(&shown, &edits)
    }

    /// Writes the pane's note to disk if it changed, on the UI thread. Debounced saves of an existing
    /// note run in the background (`flush_debounced`); this path remains for close, switch, rename,
    /// new notes and conflicts.
    pub(super) fn flush_one(&mut self, cx: &mut Context<Self>) -> bool {
        let done = self.flush_one_text(cx);
        self.count_unplaced_changes();
        done
    }

    fn flush_one_text(&mut self, cx: &mut Context<Self>) -> bool {
        if self.pane().scratch {
            return self.flush_scratch(cx);
        }
        // A background save of this note is settled first, so that the file read below is the one it left.
        self.settle_saving(cx);
        let Some(root) = self.vault.clone() else {
            return !self.pane().dirty;
        };
        if !self.pane().dirty {
            return true;
        }
        // Held until the note is written: a background save of it waits for this one.
        let guard = vault::save_lock();
        if let Some(path) = self.pane().path.clone()
            && !self.take_outside(&path, cx)
        {
            return false;
        }
        let shown = self.migrated_text(cx);
        if self.pane().path.is_none() && shown.trim().is_empty() {
            self.pane_mut().dirty = false;
            return true;
        }
        // The file's text, and the pending changes that go beside it (none in the file itself).
        let home = self
            .pane()
            .path
            .as_deref()
            .and_then(|path| platform::revisions::home_of(path, Some(root.as_path())));
        let old_path = self.pane().path.clone();
        let (content, entries) = match &old_path {
            Some(path) => {
                platform::sidecar::split(home.as_ref(), path, &shown, &self.revision_author())
            }
            None => (shown.clone(), Vec::new()),
        };
        // A `#` title added or changed at the top: the file now bears its name.
        if vault::h1_of(&content).is_some_and(|h1| self.pane().h1.as_ref() != Some(&h1)) {
            self.pane_mut().synced = true;
        }
        // A title that renames the file (as `vault::save` decides it) is refused, with nothing
        // written, when the links to the old name would change a pending change.
        if let Some(current) = self.pane().path.clone()
            && self.pane().synced
        {
            let title = vault::title_of(&content);
            let old = vault::stem(&current);
            let renamed = current.with_file_name(format!("{title}.md"));
            if old != title
                && !renamed.exists()
                && self.links_pending(&current, &old, &title, Some(&shown), cx)
            {
                return false;
            }
        }
        let dir = self.pane().new_dir.as_deref().unwrap_or(&root);
        match vault::save(
            dir,
            self.pane().path.as_deref(),
            self.pane().synced,
            &content,
        ) {
            Ok(path) => {
                // The changes follow the note to its name; the old list goes when the name changed.
                if let Some(home) = &home {
                    if old_path.as_ref().is_some_and(|old| *old != path) {
                        let _ = platform::sidecar::store(home, old_path.as_deref().unwrap(), &[]);
                    }
                    if let Err(e) = platform::sidecar::store(home, &path, &entries) {
                        self.say(
                            Tone::Failed,
                            format!("{}: {}", path.display(), crate::lang::io_reason(&e)),
                            cx,
                        );
                    }
                }
                self.pane_mut().dirty = false;
                drop(guard);
                self.saved(old_path, path, shown, content, cx);
            }
            Err(e) => {
                let name = self
                    .pane()
                    .path
                    .as_deref()
                    .map_or_else(|| vault::title_of(&content), vault::stem);
                self.say(
                    Tone::Failed,
                    format!(
                        "{} « {name} » : {}",
                        tr("Note not saved", "Note non enregistrée"),
                        crate::lang::io_reason(&e)
                    ),
                    cx,
                )
            }
        }
        cx.notify();
        !self.pane().dirty
    }

    /// Starts the 400 ms wait before the next debounced save; a newer change starts it again.
    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.save_gen += 1;
        let generation = self.save_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(400))
                .await;
            this.update(cx, |this, cx| {
                if this.save_gen == generation {
                    this.flush_debounced(cx);
                }
            })
            .ok();
        })
        .detach();
    }

    /// The debounced save: an existing note is written in the background (`start_quiet_save`), the other
    /// panes are flushed as before. A note whose background save is still running is left to it: when
    /// that ends, it saves what was typed meanwhile.
    fn flush_debounced(&mut self, cx: &mut Context<Self>) {
        self.each_pane(|this| {
            if this.pane().saving.is_some() {
                return;
            }
            match this.quiet_root(cx) {
                Some(root) => this.start_quiet_save(root, cx),
                None => {
                    this.flush_one(cx);
                }
            }
        });
    }

    /// The vault's root, when the pane's note can be written in the background: a dirty note of the vault
    /// that has a file, whose title keeps its file name and that needs no comment migrated. Anything else
    /// takes the synchronous path.
    // ponytail: `pane.h1` is set when the note loads, so a note that gains a title saves on the UI
    // thread until it is reopened.
    fn quiet_root(&self, cx: &Context<Self>) -> Option<PathBuf> {
        let pane = self.pane();
        let root = self.vault.clone()?;
        let path = pane.path.as_ref()?;
        if pane.scratch || !pane.dirty || pane.new_dir.is_some() {
            return None;
        }
        let text = pane.editor.read(cx).text();
        if !markdown::migration_edits(text).is_empty()
            || vault::h1_of(text) != pane.h1
            || (pane.synced && vault::title_of(text) != vault::stem(path))
        {
            return None;
        }
        Some(root)
    }

    /// Writes the pane's note in the background (`platform::vault::save_quiet`). Its outcome comes back to
    /// `quiet_done` on the UI thread.
    fn start_quiet_save(&mut self, root: PathBuf, cx: &mut Context<Self>) {
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        let editor = self.pane().editor.clone();
        let shown = editor.read(cx).text().to_string();
        let version = editor.read(cx).version();
        let base = self.pane().base.clone();
        let home = platform::revisions::home_of(&path, Some(root.as_path()));
        let author = self.revision_author();
        let slot: Arc<Slot> = Arc::new((Mutex::new(Outcome::Queued), Condvar::new()));
        self.pane_mut().saving = Some(Saving {
            slot: slot.clone(),
            path: path.clone(),
            shown: shown.clone(),
            version,
        });
        let work = {
            let slot = slot.clone();
            cx.background_executor().spawn(async move {
                run_quiet(&slot, || {
                    platform::vault::save_quiet(home.as_ref(), &path, &shown, &base, &author)
                });
            })
        };
        cx.spawn(async move |this, cx| {
            work.await;
            this.update(cx, |this, cx| this.quiet_done(&editor, &slot, cx))
                .ok();
        })
        .detach();
    }

    /// The UI side of a background save: its outcome is applied to the pane that holds the note. A file
    /// another program changed meanwhile is merged here, as the synchronous save does.
    fn quiet_done(&mut self, editor: &Entity<Editor>, slot: &Arc<Slot>, cx: &mut Context<Self>) {
        let Some(index) = self.panes.iter().position(|pane| pane.editor == *editor) else {
            return;
        };
        // Taken already by a save on the UI thread: nothing is left to apply.
        let Some(saving) = self.panes[index]
            .saving
            .take_if(|saving| Arc::ptr_eq(&saving.slot, slot))
        else {
            return;
        };
        let Some(result) = saving.settle() else {
            return;
        };
        self.in_pane(index, |this| {
            match this.applied_quiet(&saving, result, cx) {
                Applied::Saved => {
                    this.count_unplaced_changes();
                    if this.pane().dirty {
                        this.schedule_save(cx);
                    }
                }
                Applied::Outside => {
                    this.flush_one(cx);
                }
                Applied::Failed | Applied::Stale => {}
            }
        });
        cx.notify();
    }

    /// Applies what a background save did to the pane's note. Nothing is applied when the note has been
    /// renamed or closed meanwhile: its own save takes it from there.
    // ponytail: the count of unplaced changes and the link follow of a renamed heading still read or
    // write the disk on the UI thread; each is one small file.
    fn applied_quiet(
        &mut self,
        saving: &Saving,
        result: io::Result<platform::vault::Quiet>,
        cx: &mut Context<Self>,
    ) -> Applied {
        if self.pane().path.as_ref() != Some(&saving.path) {
            return Applied::Stale;
        }
        match result {
            Ok(platform::vault::Quiet::Saved { content }) => {
                // Typing went on meanwhile: the note stays dirty, and the next save takes the rest.
                if self.pane().editor.read(cx).version() == saving.version {
                    self.pane_mut().dirty = false;
                }
                self.saved(
                    Some(saving.path.clone()),
                    saving.path.clone(),
                    saving.shown.clone(),
                    content,
                    cx,
                );
                Applied::Saved
            }
            Ok(platform::vault::Quiet::Outside) => Applied::Outside,
            Err(e) => {
                let name = vault::stem(&saving.path);
                self.say(
                    Tone::Failed,
                    format!(
                        "{} « {name} » : {}",
                        tr("Note not saved", "Note non enregistrée"),
                        crate::lang::io_reason(&e)
                    ),
                    cx,
                );
                Applied::Failed
            }
        }
    }

    /// A save on the UI thread needs the file: the background save of the pane's note, if one is running,
    /// is settled first (`Saving::settle`), and its outcome applied.
    fn settle_saving(&mut self, cx: &mut Context<Self>) {
        if let Some(saving) = self.pane_mut().saving.take()
            && let Some(result) = saving.settle()
        {
            self.applied_quiet(&saving, result, cx);
        }
    }

    /// After a note is written, its text as saved (`shown`, and `content` in the file): the list, the
    /// index and the links follow it. `old_path` is the file it had, which differs when it was renamed.
    fn saved(
        &mut self,
        old_path: Option<PathBuf>,
        path: PathBuf,
        shown: String,
        content: String,
        cx: &mut Context<Self>,
    ) {
        let old = old_path;
        self.pane_mut().base = shown;
        self.calm();
        let (tags, links) = markdown::index(&content);
        // The text as it was last saved, to see which heading was renamed.
        let before = self
            .notes
            .iter()
            .find(|n| Some(&n.path) == old.as_ref())
            .map(|n| n.body.clone());
        self.notes
            .retain(|n| Some(&n.path) != old.as_ref() && n.path != path);
        let body: Arc<str> = Arc::from(content);
        self.notes.insert(
            0,
            Note {
                name: vault::stem(&path),
                tags: tags.into(),
                links: links.into(),
                aliases: markdown::aliases(&body).into(),
                mtime: SystemTime::now(),
                path: path.clone(),
                lower: logic::note::lowered(&body),
                body,
            },
        );
        if old.as_ref() != Some(&path) {
            // Renamed by its title, the note keeps its icon.
            if let Some(icon) = old.as_ref().and_then(|old| self.icons.remove(old)) {
                self.icons.insert(path.clone(), icon);
                self.save_icons(cx);
            }
            self.recent.retain(|p| Some(p) != old.as_ref());
            self.touch(&path);
            self.nav.reveal(&path);
            self.pane_mut().path = Some(path.clone());
            self.push_names(cx);
            self.push_dirs(cx);
        }
        if let Some(before) = before {
            self.follow_heading(&path, &before, &self.notes[0].body.clone(), cx);
        }
    }

    pub(super) fn on_editor_event(
        &mut self,
        editor: &Entity<Editor>,
        event: &EditorEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(pane) = self.panes.iter().position(|pane| pane.editor == *editor) else {
            return;
        };
        // A gesture made in a note (followed link, menu, replacement) applies to its pane;
        // typing, though, may come from elsewhere (a board card, a list cell).
        if !matches!(event, EditorEvent::Changed) {
            self.active = pane;
        }
        match event {
            EditorEvent::CommentAt(at) => self.comment_clicked(*at, window, cx),
            EditorEvent::CommentStarted { inside, noted } => {
                self.comment_started(editor, *inside, noted.clone(), window, cx)
            }
            EditorEvent::Changed => {
                self.in_pane(pane, |this| {
                    this.keep_preview();
                    this.pane_mut().dirty = true;
                });
                self.schedule_save(cx);
            }
            EditorEvent::Open(link) => self.follow(link, window, cx),
            EditorEvent::OpenAside(link) => match link {
                Link::Wiki(name) => match self.wiki_path(name) {
                    Some(path) => self.open_aside(&path, window, cx),
                    None => self.follow(link, window, cx),
                },
                _ => self.follow(link, window, cx),
            },
            EditorEvent::OpenAt(target, aside) => self.open_at(target, *aside, window, cx),
            EditorEvent::Menu(at, link, commented, keys, change) => {
                let chosen = keys.then_some(0);
                self.show_menu(
                    nav::Menu {
                        at: *at,
                        text: Some(link.clone()),
                        commented: *commented,
                        change: *change,
                        chosen,
                        ..Default::default()
                    },
                    window,
                    cx,
                );
            }
            EditorEvent::BlockMenu(at) => {
                self.show_menu(
                    nav::Menu {
                        at: *at,
                        text: Some(None),
                        block: true,
                        ..Default::default()
                    },
                    window,
                    cx,
                );
            }
            // The diagram, formula or picture takes the note's place; Escape returns to it.
            EditorEvent::View(source) => self.show_figure(source.clone(), window, cx),
            // A message of the note, shown once while it is not already on screen.
            EditorEvent::Notice(text) => {
                if !self.toasts.iter().any(|toast| toast.text == *text) {
                    self.say(Tone::Info, text.clone(), cx);
                }
            }
            // The comment's line is asked by a prompt of the shell, which then writes it in the note.
            EditorEvent::AskChangeComment(change) => {
                self.ask_change_comment(editor.clone(), change.clone(), window, cx)
            }
        }
    }

    /// Follows a link in the note: the address in the browser, the tag in the palette, the note.
    /// Footnotes navigate inside the note (see `Editor::footnote_target`) and never reach this.
    pub(super) fn follow(&mut self, link: &Link, window: &mut Window, cx: &mut Context<Self>) {
        match link {
            Link::Footnote(_) => {}
            Link::Url(url) => cx.open_url(url),
            Link::Tag(tag) => self.open_palette(&format!("#{tag}"), window, cx),
            // A folder reads as a list page, when no note bears its name.
            Link::Wiki(name)
                if self.wiki_path(name).is_none() && self.wiki_file(name).is_none() =>
            {
                match self.wiki_dir(name) {
                    Some(dir) => self.open_listing(dir, window, cx),
                    None => self.open_wiki(name, window, cx),
                }
            }
            Link::Wiki(name) => self.open_wiki(name, window, cx),
            Link::Note { path, anchor } => self.follow_note(path, anchor.as_deref(), window, cx),
            Link::Heading(anchor) => self.jump_to_heading(anchor, cx),
        }
    }

    /// Opens the note a relative link names, at its heading when it has one. Resolved from the note
    /// shown (the vault root for an unsaved one); a note that is not in the vault is not created.
    fn follow_note(
        &mut self,
        address: &str,
        anchor: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.vault.clone() else {
            return;
        };
        let from = self
            .pane()
            .path
            .clone()
            .unwrap_or_else(|| root.join("New.md"));
        let target = logic::wiki::note_at(&root, &from, address);
        if !self.notes.iter().any(|n| n.path == target) {
            let shown = address.to_string();
            return self.say(
                Tone::Failed,
                format!(
                    "{} « {shown} »",
                    tr("Note not found:", "Note introuvable :")
                ),
                cx,
            );
        }
        self.open(&target, window, cx);
        if let Some(anchor) = anchor {
            self.jump_to_heading(anchor, cx);
        }
    }

    /// Scrolls the shown note to its heading `anchor` (GitHub's slug, see `markdown::heading_at`).
    fn jump_to_heading(&mut self, anchor: &str, cx: &mut Context<Self>) {
        let at = markdown::heading_at(self.pane().editor.read(cx).text(), anchor);
        if let Some(at) = at {
            self.pane().editor.update(cx, |e, cx| e.jump(at, true, cx));
        }
    }

    /// Opens the note of a link to a heading, scrolled to what it points at: the heading,
    /// the block (a note comes back from there with Alt+Left, like after any link).
    pub(super) fn open_at(
        &mut self,
        target: &wiki::Target,
        aside: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match (aside, self.wiki_path(&target.note)) {
            (true, Some(path)) => self.open_aside(&path, window, cx),
            _ => self.open_wiki(&target.note, window, cx),
        }
        let anchor = target.anchor.clone();
        let at = self.pane().editor.read(cx).text();
        let at = wiki::position(at, &anchor).filter(|_| anchor != wiki::Anchor::Whole);
        if let Some(at) = at {
            self.pane().editor.update(cx, |e, cx| e.jump(at, true, cx));
        }
    }

    /// The vault file (picture, table, attachment) that `[[name.ext]]` points to, case-insensitive.
    pub(super) fn wiki_file(&self, name: &str) -> Option<PathBuf> {
        let wanted = name.to_lowercase();
        self.images
            .iter()
            .find(|path| {
                path.file_name()
                    .is_some_and(|n| n.to_string_lossy().to_lowercase() == wanted)
            })
            .cloned()
    }

    /// The vault folder that `[[Name]]` points to, except the templates folder.
    pub(super) fn wiki_dir(&self, name: &str) -> Option<PathBuf> {
        let (root, wanted) = (self.vault.as_ref()?, name.to_lowercase());
        self.dirs
            .iter()
            .find(|dir| {
                !vault::in_templates(root, &dir.join("x"))
                    && dir
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().to_lowercase() == wanted)
            })
            .cloned()
    }

    /// Daily note: the note named after the local date, wherever it is in the
    /// vault; created with that date as title if it does not exist yet.
    /// (No template anymore: the cut keeps plain Markdown editing only.)
    pub(super) fn open_today(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.vault.is_none() {
            return;
        }
        self.open_wiki(&date_name(today()), window, cx);
    }

    /// The note a `[[link]]` points to: by its name first, otherwise by an alias in its
    /// front matter.
    pub(super) fn wiki_path(&self, name: &str) -> Option<PathBuf> {
        logic::wiki::resolve(&self.notes, name, |_| false).map(|note| note.path.clone())
    }

    pub(super) fn open_wiki(&mut self, name: &str, window: &mut Window, cx: &mut Context<Self>) {
        match self.wiki_path(name).or_else(|| self.wiki_file(name)) {
            Some(path) => self.open(&path, window, cx),
            None => {
                self.fresh_tab(window, cx);
                self.new_note(format!("# {name}\n\n"), cx);
                window.focus(&self.pane().editor.focus_handle(cx), cx);
            }
        }
    }
}

/// A debounced save running off the UI thread (`Shell::start_quiet_save`): what it was given, and its
/// outcome, which whoever comes first takes.
pub(super) struct Saving {
    slot: Arc<Slot>,
    pub(super) path: PathBuf,
    pub(super) shown: String,
    pub(super) version: u64,
}

type Slot = (Mutex<Outcome>, Condvar);

/// Where a background save stands. `Done` is a save that was cancelled, or whose outcome was taken.
enum Outcome {
    Queued,
    Running,
    Finished(io::Result<platform::vault::Quiet>),
    Done,
}

/// What applying a background save came to.
enum Applied {
    Saved,
    /// The file changed since it was read: the caller merges it.
    Outside,
    /// The error is shown already.
    Failed,
    /// The note was renamed or closed meanwhile.
    Stale,
}

impl Saving {
    /// The outcome of the save, for its caller to apply. A save that has not started is cancelled and
    /// writes nothing (`None`); one that is running is waited for, which ends: the worker never waits on
    /// the UI thread.
    pub(super) fn settle(&self) -> Option<io::Result<platform::vault::Quiet>> {
        let (lock, done) = &*self.slot;
        let mut outcome = lock.lock().unwrap_or_else(|e| e.into_inner());
        loop {
            match &*outcome {
                Outcome::Running => {
                    outcome = done.wait(outcome).unwrap_or_else(|e| e.into_inner());
                }
                Outcome::Queued => {
                    *outcome = Outcome::Done;
                    return None;
                }
                Outcome::Done => return None,
                Outcome::Finished(_) => break,
            }
        }
        match std::mem::replace(&mut *outcome, Outcome::Done) {
            Outcome::Finished(result) => Some(result),
            _ => None,
        }
    }
}

/// Runs `save` for `slot`, unless the UI thread cancelled it first. The lock is not held while it runs.
fn run_quiet(slot: &Slot, save: impl FnOnce() -> io::Result<platform::vault::Quiet>) {
    let (lock, done) = slot;
    {
        let mut outcome = lock.lock().unwrap_or_else(|e| e.into_inner());
        if !matches!(*outcome, Outcome::Queued) {
            return;
        }
        *outcome = Outcome::Running;
    }
    let result = save();
    let mut outcome = lock.lock().unwrap_or_else(|e| e.into_inner());
    *outcome = Outcome::Finished(result);
    done.notify_all();
}

/// Where to write the other side's version of a conflicting note: next to it, as Dropbox
/// and Syncthing do (plain-folder sync keeps both versions, no markers in the note).
fn conflict_path(path: &Path, date: &str) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let name = match path.extension() {
        Some(extension) => format!("{stem} (conflict {date}).{}", extension.to_string_lossy()),
        None => format!("{stem} (conflict {date})"),
    };
    path.with_file_name(name)
}

/// `conflict_path`, numbered when that name is taken: nothing is overwritten.
fn free_conflict_path(path: &Path, date: &str) -> PathBuf {
    let named = conflict_path(path, date);
    let mut other = named.clone();
    let mut n = 1;
    while other.exists() {
        n += 1;
        let stem = named
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        other = named.with_file_name(match named.extension() {
            Some(extension) => format!("{stem} {n}.{}", extension.to_string_lossy()),
            None => format!("{stem} {n}"),
        });
    }
    other
}
