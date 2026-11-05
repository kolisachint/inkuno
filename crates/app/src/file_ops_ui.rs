//! File operations in the sidebar (task 9.3): new note, inline rename, and delete with a
//! confirmation. The file system work is done by `workspace::ops`. This module decides what the
//! user sees and keeps the open note in step with what was done on disk.
//!
//! Keys. New note is Cmd+N on macOS and Ctrl+N on Linux, and the "+" button at the top of the
//! sidebar does the same. Rename is F2 on the open note, or a double-click on a row. Delete is
//! a right-click on a row, or Cmd+Backspace and Ctrl+Backspace while the sidebar has keyboard
//! focus. In the editor, Ctrl+Backspace still deletes a word, so the open note is deleted from
//! the keyboard only after a click in the sidebar. While renaming, Enter commits, Escape cancels.
//! While confirming a delete, Enter deletes and Escape keeps the file.
//!
//! The pure helpers ([`untitled_stem`], [`rename_input`], [`NameEdit`], [`next_note_after`])
//! take plain values, so they are unit-tested without a window.

use std::path::{Path, PathBuf};

use gpui::{actions, AppContext, Context, Focusable, KeyDownEvent, Window};
use workspace::ops::OpenNotes;

use crate::file_io::{read_file, Document};
use crate::search_panel::note_label;
use crate::workspace_view::WorkspaceView;

actions!(file_ops_ui, [NewNote, RenameNote, DeleteNote]);

/// Name used for a new note before a number is added.
pub const UNTITLED: &str = "Untitled";

/// Registers the file operation actions and their key bindings. Called from `main`.
pub fn init(cx: &mut gpui::App) {
    use gpui::KeyBinding;
    // Cmd on macOS, Ctrl on Linux. Ctrl+Backspace is bound in the sidebar context only,
    // because the editor binds it to delete a word.
    #[cfg(target_os = "macos")]
    let (new_key, delete_key) = ("cmd-n", "cmd-backspace");
    #[cfg(not(target_os = "macos"))]
    let (new_key, delete_key) = ("ctrl-n", "ctrl-backspace");
    cx.bind_keys([
        KeyBinding::new(new_key, NewNote, None),
        KeyBinding::new("f2", RenameNote, None),
        KeyBinding::new(delete_key, DeleteNote, Some("Sidebar")),
    ]);
}

/// The first free name `Untitled`, `Untitled 2`, `Untitled 3`, ... `taken` says whether a stem
/// is already used in the folder.
pub fn untitled_stem(taken: impl Fn(&str) -> bool) -> String {
    if !taken(UNTITLED) {
        return UNTITLED.to_owned();
    }
    (2u32..)
        .map(|n| format!("{UNTITLED} {n}"))
        .find(|stem| !taken(stem))
        .unwrap_or_else(|| UNTITLED.to_owned())
}

/// Why a typed name cannot be used. Other problems are left to `workspace::ops`, which
/// reports them on the rename.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    Separator,
    Hidden,
}

impl NameError {
    pub fn message(self) -> &'static str {
        match self {
            NameError::Empty => "Name is empty",
            NameError::Separator => "Name cannot contain / or \\",
            NameError::Hidden => "Name cannot start with a dot",
        }
    }
}

/// The name to hand to `workspace::ops::rename`, from what was typed. Surrounding spaces are
/// dropped. For a note the `.md` is kept or added by ops, so `a` and `a.md` both work.
pub fn rename_input(typed: &str) -> Result<String, NameError> {
    let name = typed.trim();
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.contains(['/', '\\']) {
        return Err(NameError::Separator);
    }
    if name.starts_with('.') {
        return Err(NameError::Hidden);
    }
    Ok(name.to_owned())
}

/// The line shown while a delete waits for an answer.
pub fn delete_prompt(name: &str) -> String {
    format!("Delete {name}? Enter = yes, Esc = no")
}

/// The note that takes the place of a deleted open note: the one after `deleted` in path
/// order, else the one before it. `None` when no note is left.
pub fn next_note_after(notes: &[PathBuf], deleted: &Path) -> Option<PathBuf> {
    let mut sorted = notes.to_vec();
    sorted.sort();
    let index = sorted.partition_point(|note| note.as_path() < deleted);
    sorted
        .get(index)
        .or_else(|| index.checked_sub(1).and_then(|i| sorted.get(i)))
        .cloned()
}

/// Text being typed into a name. It starts selected, so the first key replaces it, as in a
/// file manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEdit {
    pub text: String,
    pub replace_all: bool,
}

impl NameEdit {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            replace_all: !text.is_empty(),
        }
    }

    /// Appends typed text, replacing the selection first.
    pub fn insert(&mut self, typed: &str) {
        if self.replace_all {
            self.text.clear();
            self.replace_all = false;
        }
        self.text.push_str(typed);
    }

    /// Deletes the selection, or the last character.
    pub fn backspace(&mut self) {
        if self.replace_all {
            self.text.clear();
            self.replace_all = false;
        } else {
            self.text.pop();
        }
    }
}

/// A file operation waiting for the user. The sidebar shows it until it is finished or cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEdit {
    /// An inline rename of the row at `path`. `label` is the name as the sidebar shows it, which
    /// is what the user started from. `error` is the last failure, shown under the row.
    Rename {
        path: PathBuf,
        label: String,
        edit: NameEdit,
        is_note: bool,
        error: Option<String>,
    },
    /// Waiting for Enter (delete) or Escape (keep).
    ConfirmDelete { path: PathBuf, name: String },
}

/// Modifier keys that make a key press a shortcut, not text.
fn has_shortcut_modifier(event: &KeyDownEvent) -> bool {
    let m = event.keystroke.modifiers;
    m.control || m.alt || m.platform || m.function
}

impl WorkspaceView {
    /// Makes sure the sidebar keyboard focus is held, so the sidebar sees the typed keys.
    fn focus_sidebar(&self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.sidebar_focus, cx);
    }

    /// Gives keyboard focus back to the open note's editor.
    pub(crate) fn focus_editor(&self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.document.read(cx).editor.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
    }

    /// Reads the folder tree from disk again, and reruns the search when it is open.
    pub(crate) fn refresh_tree(&mut self) {
        if let Some(sidebar) = self.sidebar.as_mut() {
            match sidebar.vault.tree() {
                Ok(tree) => sidebar.tree = tree,
                Err(err) => eprintln!(
                    "noteapp: cannot list vault {}: {err}",
                    sidebar.vault.root().display()
                ),
            }
        }
        if self.search_open {
            self.search.refresh(self.sidebar.as_ref().map(|s| &s.vault));
        }
    }

    /// Ends any rename or delete question without changing the disk.
    pub(crate) fn cancel_file_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.file_edit = None;
        self.focus_editor(window, cx);
        cx.notify();
    }

    /// Creates `Untitled.md` in the open note's folder (or the vault root), opens it and starts
    /// an inline rename. Without a vault there is nowhere to create a note.
    pub(crate) fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(sidebar) = self.sidebar.as_ref() else {
            eprintln!("noteapp: open a folder to create a note");
            return;
        };
        let root = sidebar.vault.root().to_path_buf();
        let parent = self
            .document
            .read(cx)
            .path
            .as_deref()
            .and_then(Path::parent)
            .filter(|dir| dir.starts_with(&root))
            .map(Path::to_path_buf)
            .unwrap_or(root);
        let stem =
            untitled_stem(|stem| parent.join(format!("{stem}.md")).symlink_metadata().is_ok());
        let created = sidebar.vault.create_note(&parent, &stem);
        match created {
            Ok(path) => {
                self.refresh_tree();
                if self.open_note(path.clone(), window, cx) {
                    self.start_rename(path, stem, true, window, cx);
                }
            }
            Err(err) => eprintln!(
                "noteapp: cannot create a note in {}: {err}",
                parent.display()
            ),
        }
        cx.notify();
    }

    /// Starts an inline rename of `path`, showing `label` in its row.
    pub(crate) fn start_rename(
        &mut self,
        path: PathBuf,
        label: String,
        is_note: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.sidebar.is_none() {
            return;
        }
        self.file_edit = Some(FileEdit::Rename {
            path,
            edit: NameEdit::new(&label),
            label,
            is_note,
            error: None,
        });
        self.focus_sidebar(window, cx);
        cx.notify();
    }

    /// F2: renames the open note.
    pub(crate) fn rename_open_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.document.read(cx).path.clone() else {
            return;
        };
        let label = note_label(&path);
        self.start_rename(path, label, true, window, cx);
    }

    /// Asks before deleting `path`. The question is shown in the sidebar until answered.
    pub(crate) fn ask_delete(
        &mut self,
        path: PathBuf,
        name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.sidebar.is_none() {
            return;
        }
        self.file_edit = Some(FileEdit::ConfirmDelete { path, name });
        self.focus_sidebar(window, cx);
        cx.notify();
    }

    /// Ctrl/Cmd+Backspace: asks before deleting the open note. Ignored during a rename.
    pub(crate) fn ask_delete_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if matches!(self.file_edit, Some(FileEdit::Rename { .. })) {
            return;
        }
        let Some(path) = self.document.read(cx).path.clone() else {
            return;
        };
        let name = note_label(&path);
        self.ask_delete(path, name, window, cx);
    }

    /// Keys while the sidebar has focus and a rename or delete question is open.
    pub(crate) fn file_edit_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        match self.file_edit.clone() {
            None => {}
            Some(FileEdit::ConfirmDelete { .. }) => match key {
                "enter" => self.delete_confirmed(window, cx),
                "escape" => self.cancel_file_edit(window, cx),
                _ => {}
            },
            Some(FileEdit::Rename { .. }) => match key {
                "enter" => self.commit_rename(window, cx),
                "escape" => self.cancel_file_edit(window, cx),
                "backspace" => self.edit_name(|edit| edit.backspace(), cx),
                _ => {
                    if has_shortcut_modifier(event) {
                        return;
                    }
                    let Some(text) = event.keystroke.key_char.as_deref() else {
                        return;
                    };
                    if text.chars().any(char::is_control) {
                        return;
                    }
                    self.edit_name(|edit| edit.insert(text), cx);
                }
            },
        }
    }

    /// Changes the typed name of the open rename, and clears the last error.
    fn edit_name(&mut self, change: impl FnOnce(&mut NameEdit), cx: &mut Context<Self>) {
        if let Some(FileEdit::Rename { edit, error, .. }) = self.file_edit.as_mut() {
            change(edit);
            *error = None;
            cx.notify();
        }
    }

    /// Enter in a rename: renames the file on disk. A failure keeps the rename open with the
    /// reason under the row.
    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(FileEdit::Rename {
            path, label, edit, ..
        }) = self.file_edit.clone()
        else {
            return;
        };
        let name = match rename_input(&edit.text) {
            Ok(name) => name,
            Err(err) => return self.set_rename_error(err.message().to_owned(), cx),
        };
        if name == label {
            return self.cancel_file_edit(window, cx);
        }
        let Some(sidebar) = self.sidebar.as_ref() else {
            return self.cancel_file_edit(window, cx);
        };
        match sidebar.vault.rename(&path, &name) {
            Ok(to) => {
                self.file_edit = None;
                self.after_move(&path, &to, cx);
                self.focus_editor(window, cx);
                cx.notify();
            }
            Err(err) => self.set_rename_error(err.to_string(), cx),
        }
    }

    fn set_rename_error(&mut self, message: String, cx: &mut Context<Self>) {
        if let Some(FileEdit::Rename { error, .. }) = self.file_edit.as_mut() {
            *error = Some(message);
        }
        cx.notify();
    }

    /// Follows a rename or move of `from` to `to`. The tree is read again. An open note under
    /// `from` takes its new path, so it stays open. Its text and known hash do not change, so
    /// the watcher's events for our own rename do not show a "changed on disk" banner.
    fn after_move(&mut self, from: &Path, to: &Path, cx: &mut Context<Self>) {
        self.refresh_tree();
        let Some(open) = self.document.read(cx).path.clone() else {
            return;
        };
        let mut open_notes = OpenNotes {
            paths: vec![open.clone()],
        };
        open_notes.on_rename(from, to);
        let Some(new_path) = open_notes.paths.pop() else {
            return;
        };
        if new_path != open {
            self.document
                .update(cx, |doc, cx| doc.moved_to(new_path, cx));
        }
    }

    /// Enter on the delete question: deletes the file. If it was the open note, another note
    /// opens, or an empty untitled document when none is left. Unsaved edits to the deleted
    /// note are dropped, since the user asked for the delete.
    fn delete_confirmed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(FileEdit::ConfirmDelete { path, .. }) = self.file_edit.take() else {
            return;
        };
        let Some(sidebar) = self.sidebar.as_ref() else {
            return;
        };
        if let Err(err) = sidebar.vault.delete(&path) {
            eprintln!("noteapp: cannot delete {}: {err}", path.display());
            return self.cancel_file_edit(window, cx);
        }
        self.refresh_tree();
        let open = self.document.read(cx).path.clone();
        if open.is_some_and(|open| open.starts_with(&path)) {
            self.show_note_after_delete(&path, window, cx);
        }
        self.focus_editor(window, cx);
        cx.notify();
    }

    /// Replaces the open document, which was just deleted, with the next note or an empty one.
    fn show_note_after_delete(
        &mut self,
        deleted: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let notes = self
            .sidebar
            .as_ref()
            .and_then(|s| s.vault.notes().ok())
            .unwrap_or_default();
        let next = next_note_after(&notes, deleted)
            .and_then(|path| read_file(&path).ok().map(|text| (path, text)));
        let document = match next {
            Some((path, text)) => cx.new(|cx| Document::with_text(cx, &text, Some(path))),
            None => cx.new(|cx| Document::with_text(cx, "", None)),
        };
        let focus = document.read(cx).editor.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.document = document;
    }
}

/// A rename row's text and whether it is the one being renamed.
pub(crate) fn renaming_text(edit: &Option<FileEdit>, path: &Path) -> Option<String> {
    match edit {
        Some(FileEdit::Rename {
            path: renamed,
            edit,
            ..
        }) if renamed == path => Some(edit.text.clone()),
        _ => None,
    }
}

/// The line under the header while a rename or delete question is open: the prompt, or the
/// rename error or hint.
pub(crate) fn edit_message(edit: &Option<FileEdit>) -> Option<String> {
    match edit {
        None => None,
        Some(FileEdit::ConfirmDelete { name, .. }) => Some(delete_prompt(name)),
        Some(FileEdit::Rename { error, .. }) => Some(match error {
            Some(error) => error.clone(),
            None => "Enter = rename, Esc = cancel".to_owned(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    #[test]
    fn untitled_takes_the_first_free_number() {
        assert_eq!(untitled_stem(|_| false), "Untitled");
        let taken = |stem: &str| stem == "Untitled";
        assert_eq!(untitled_stem(taken), "Untitled 2");
        let taken = |stem: &str| matches!(stem, "Untitled" | "Untitled 2");
        assert_eq!(untitled_stem(taken), "Untitled 3");
        // A gap is reused.
        let taken = |stem: &str| matches!(stem, "Untitled" | "Untitled 3");
        assert_eq!(untitled_stem(taken), "Untitled 2");
    }

    #[test]
    fn rename_input_trims_and_rejects_bad_names() {
        assert_eq!(rename_input("  plan  "), Ok("plan".to_owned()));
        assert_eq!(rename_input("plan.md"), Ok("plan.md".to_owned()));
        assert_eq!(rename_input(""), Err(NameError::Empty));
        assert_eq!(rename_input("   "), Err(NameError::Empty));
        assert_eq!(rename_input("a/b"), Err(NameError::Separator));
        assert_eq!(rename_input("a\\b"), Err(NameError::Separator));
        assert_eq!(rename_input(".hidden"), Err(NameError::Hidden));
    }

    #[test]
    fn name_error_messages_are_short_sentences() {
        assert_eq!(NameError::Empty.message(), "Name is empty");
        assert!(NameError::Separator.message().contains('/'));
        assert!(NameError::Hidden.message().contains("dot"));
    }

    #[test]
    fn typing_over_a_selected_name_replaces_it() {
        let mut edit = NameEdit::new("Untitled");
        assert!(edit.replace_all);
        edit.insert("p");
        assert_eq!(edit.text, "p");
        assert!(!edit.replace_all);
        edit.insert("lan");
        assert_eq!(edit.text, "plan");
    }

    #[test]
    fn backspace_on_a_selection_clears_it_then_deletes_characters() {
        let mut edit = NameEdit::new("draft");
        edit.backspace();
        assert_eq!(edit.text, "");
        edit.insert("x");
        edit.insert("y");
        edit.backspace();
        assert_eq!(edit.text, "x");
        let mut empty = NameEdit::new("");
        assert!(!empty.replace_all);
        empty.backspace();
        assert_eq!(empty.text, "");
    }

    #[test]
    fn delete_prompt_names_the_file_and_the_keys() {
        assert_eq!(
            delete_prompt("draft"),
            "Delete draft? Enter = yes, Esc = no"
        );
    }

    #[test]
    fn next_note_is_the_one_after_or_before_the_deleted_one() {
        let notes = vec![p("/v/a.md"), p("/v/c.md"), p("/v/d.md")];
        assert_eq!(
            next_note_after(&notes, Path::new("/v/b.md")),
            Some(p("/v/c.md"))
        );
        assert_eq!(
            next_note_after(&notes[..2], Path::new("/v/d.md")),
            Some(p("/v/c.md"))
        );
        assert_eq!(
            next_note_after(&notes, Path::new("/v/z.md")),
            Some(p("/v/d.md"))
        );
        // The deleted note is already gone from `notes` when this runs.
        assert_eq!(
            next_note_after(&[p("/v/b.md")], Path::new("/v/a.md")),
            Some(p("/v/b.md"))
        );
        assert_eq!(next_note_after(&[], Path::new("/v/a.md")), None);
    }

    #[test]
    fn rename_message_and_text_follow_the_edit_state() {
        let rename = Some(FileEdit::Rename {
            path: p("/v/a.md"),
            label: "a".into(),
            edit: NameEdit::new("a"),
            is_note: true,
            error: None,
        });
        assert_eq!(
            renaming_text(&rename, Path::new("/v/a.md")),
            Some("a".into())
        );
        assert_eq!(renaming_text(&rename, Path::new("/v/b.md")), None);
        assert_eq!(
            edit_message(&rename),
            Some("Enter = rename, Esc = cancel".into())
        );

        let failed = Some(FileEdit::Rename {
            path: p("/v/a.md"),
            label: "a".into(),
            edit: NameEdit::new("b"),
            is_note: true,
            error: Some("exists".into()),
        });
        assert_eq!(edit_message(&failed), Some("exists".into()));
    }

    #[test]
    fn delete_question_is_shown_and_takes_no_text() {
        let ask = Some(FileEdit::ConfirmDelete {
            path: p("/v/a.md"),
            name: "a".into(),
        });
        assert_eq!(
            edit_message(&ask),
            Some("Delete a? Enter = yes, Esc = no".into())
        );
        assert_eq!(renaming_text(&ask, Path::new("/v/a.md")), None);
        assert_eq!(edit_message(&None), None);
    }
}
