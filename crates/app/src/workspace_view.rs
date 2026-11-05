//! The window's root view (task 9.1): a vault sidebar beside the open note.
//!
//! Start-up. With no argument there is no vault, so the window shows the sample document
//! with no sidebar, as before. With a directory argument the directory is the vault and the
//! document starts empty. With a file argument the file's parent directory is the vault and
//! the file is opened and revealed in the sidebar.
//!
//! Sidebar. Folders start collapsed. A click on a folder row toggles it. A click on a note
//! row opens that note in place of the current one. The open note is highlighted. If the
//! open note has unsaved edits, it is saved first; when it cannot be saved (for example, it
//! has no file path), the switch is refused and the reason goes to stderr.
//!
//! Vault changes (9.2). The sidebar watches the vault. The window polls the watcher every
//! [`POLL_INTERVAL`] and applies what arrived: the tree is rebuilt on a create, remove or rename,
//! the search is rerun, and the open note reloads, keeps its edits behind a banner, or is marked
//! deleted.

use std::path::{Path, PathBuf};

use gpui::{
    div, prelude::*, px, App, ClickEvent, Context, Entity, EntityId, FocusHandle, Focusable,
    IntoElement, KeyDownEvent, MouseButton, MouseDownEvent, Render, Subscription, Window,
};
use md_editor::modes::Modes;
use md_editor::selection::Selection;
use workspace::tree::{visible_rows, Expansion, Node, Vault};
use workspace::watcher::{plan_buffer_updates, OpenBuffer};

use crate::file_io::{read_file, Document};
use crate::file_ops_ui::{
    edit_message, renaming_text, DeleteNote, FileEdit, NewFolder, NewNote, RenameNote,
};
use crate::search_panel::{self, hit_offset, SearchPanel, ToggleSearch};
use crate::settings;
use crate::text_input;
use crate::watch::{changes_for_open_note, needs_tree_refresh, WatchFeed, POLL_INTERVAL};

/// Width of the sidebar in pixels.
pub(crate) const SIDEBAR_WIDTH: f32 = 240.;
/// Height of one sidebar row in pixels.
const ROW_HEIGHT: f32 = 24.;
/// Left padding of a depth-0 row, and the extra indent per depth level, in pixels.
const BASE_PADDING: f32 = 8.;
const INDENT_STEP: f32 = 14.;

/// One sidebar row as plain data, so the mapping from the tree can be tested without a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarItem {
    /// The node's name. A note's name has no `.md` extension.
    pub label: String,
    /// 0 for the vault's top level.
    pub depth: usize,
    pub path: PathBuf,
    /// `None` for a note. `Some(expanded)` for a folder.
    pub expanded: Option<bool>,
}

impl SidebarItem {
    /// True for a folder row.
    pub fn is_folder(&self) -> bool {
        self.expanded.is_some()
    }

    /// The disclosure marker: `▸` for a collapsed folder, `▾` for an expanded one, and
    /// nothing for a note.
    pub fn marker(&self) -> &'static str {
        match self.expanded {
            None => "",
            Some(false) => "\u{25B8}",
            Some(true) => "\u{25BE}",
        }
    }
}

/// Left padding in pixels for a row at `depth`.
pub fn indent_px(depth: usize) -> f32 {
    BASE_PADDING + depth as f32 * INDENT_STEP
}

/// The bottom-right status text for the modes that are on, such as `Source · Focus`, or `None`
/// when every mode is off (task 8.5). Pure, unit-tested.
pub fn mode_status(modes: Modes) -> Option<String> {
    let names: Vec<&str> = [
        (modes.source, "Source"),
        (modes.focus, "Focus"),
        (modes.typewriter, "Typewriter"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect();
    (!names.is_empty()).then(|| names.join(" \u{00B7} "))
}

/// The sidebar rows for `root`, top to bottom, as owned data (see [`visible_rows`]).
pub fn sidebar_items(root: &Node, expansion: &Expansion) -> Vec<SidebarItem> {
    visible_rows(root, expansion)
        .into_iter()
        .map(|row| SidebarItem {
            label: row.node.name().to_owned(),
            depth: row.depth,
            path: row.node.path().to_path_buf(),
            expanded: matches!(row.node, Node::Folder { .. }).then_some(row.expanded),
        })
        .collect()
}

/// The vault shown in the sidebar, with its tree and which folders are open.
pub(crate) struct Sidebar {
    pub(crate) vault: Vault,
    pub(crate) tree: Node,
    expansion: Expansion,
    /// Changes on disk under the vault (9.2). `None` when the watch could not start.
    watch: Option<WatchFeed>,
}

impl Sidebar {
    /// Opens the vault at `dir`. Failures are reported on stderr and leave no sidebar.
    fn open(dir: &Path) -> Option<Self> {
        let vault = match Vault::open(dir) {
            Ok(vault) => vault,
            Err(err) => {
                eprintln!("noteapp: cannot open vault {}: {err}", dir.display());
                return None;
            }
        };
        let tree = match vault.tree() {
            Ok(tree) => tree,
            Err(err) => {
                eprintln!(
                    "noteapp: cannot list vault {}: {err}",
                    vault.root().display()
                );
                return None;
            }
        };
        let watch = match WatchFeed::start(&vault) {
            Ok(feed) => Some(feed),
            Err(err) => {
                eprintln!(
                    "noteapp: cannot watch vault {}: {err}",
                    vault.root().display()
                );
                None
            }
        };
        Some(Self {
            vault,
            tree,
            expansion: Expansion::new(),
            watch,
        })
    }
}

/// The root view: the sidebar (when there is a vault) and the open document.
pub struct WorkspaceView {
    pub(crate) sidebar: Option<Sidebar>,
    pub(crate) document: Entity<Document>,
    /// Search state (task 9.4). Kept between opens so the query survives a close.
    pub(crate) search: SearchPanel,
    /// True while the sidebar shows the search panel instead of the folder tree.
    pub(crate) search_open: bool,
    /// The rename or Trash question in the sidebar, if one is open (task 9.3).
    pub(crate) file_edit: Option<FileEdit>,
    /// Keyboard focus for the sidebar, so it sees the keys of a rename or Trash move (task 9.3).
    pub(crate) sidebar_focus: FocusHandle,
    /// The editor whose mode changes this view observes, and the subscription (task 8.5). The
    /// mode status text reads the modes, so the view redraws when they change. It follows the
    /// open document, see [`WorkspaceView::watch_modes`].
    mode_watch: Option<(EntityId, Subscription)>,
}

impl WorkspaceView {
    /// Builds the start-up state from the command line (see the module docs) and starts
    /// applying vault changes.
    pub fn new(cx: &mut Context<Self>) -> Self {
        let view = Self::from_args(cx);
        Self::start_polling(cx);
        view
    }

    /// Redraws this view when the open document's editor changes, such as when a mode is
    /// toggled. The subscription is kept for the editor it observes, so it is replaced when the
    /// open document is replaced (task 8.5).
    fn watch_modes(&mut self, cx: &mut Context<Self>) {
        let editor = self.document.read(cx).editor.clone();
        if self
            .mode_watch
            .as_ref()
            .is_some_and(|(seen, _)| *seen == editor.entity_id())
        {
            return;
        }
        // Focus and Typewriter are saved when they change (task 10.3). Source Mode is not.
        let subscription = cx.observe(&editor, |_, editor, cx| {
            let modes = editor.read(cx).modes();
            settings::change(cx, |s| {
                s.focus = modes.focus;
                s.typewriter = modes.typewriter;
            });
            cx.notify();
        });
        self.mode_watch = Some((editor.entity_id(), subscription));
    }

    /// Polls the vault watcher on the window thread every [`POLL_INTERVAL`]. The loop ends
    /// when the view is dropped.
    fn start_polling(cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(POLL_INTERVAL).await;
            if this
                .update(cx, |view, cx| view.apply_watch_changes(cx))
                .is_err()
            {
                break;
            }
        })
        .detach();
    }

    /// Applies the changes the watcher reported since the last poll (9.2).
    fn apply_watch_changes(&mut self, cx: &mut Context<Self>) {
        let Some(sidebar) = self.sidebar.as_mut() else {
            return;
        };
        let Some(feed) = sidebar.watch.as_ref() else {
            return;
        };
        let batch = feed.drain();
        if batch.is_empty() {
            return;
        }
        if needs_tree_refresh(&batch) {
            // The expansion state is kept; only the tree is rebuilt from disk.
            match sidebar.vault.tree() {
                Ok(tree) => sidebar.tree = tree,
                Err(err) => eprintln!(
                    "noteapp: cannot list vault {}: {err}",
                    sidebar.vault.root().display()
                ),
            }
        }
        if self.search_open {
            self.refresh_search(cx);
        }
        let (path, dirty) = {
            let document = self.document.read(cx);
            (document.path.clone(), document.is_dirty(cx))
        };
        if let Some(path) = path {
            let open = [OpenBuffer {
                path: path.clone(),
                dirty,
            }];
            let plan = plan_buffer_updates(&changes_for_open_note(&batch, &path), &open);
            if !plan.is_empty() {
                self.document.update(cx, |doc, cx| {
                    for update in &plan {
                        doc.apply_disk_update(update, cx);
                    }
                });
            }
        }
        cx.notify();
    }

    /// Opens the vault at `dir` and remembers it as the last vault (task 10.3).
    fn open_vault(dir: &Path, cx: &mut Context<Self>) -> Option<Sidebar> {
        let sidebar = Sidebar::open(dir)?;
        let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
        settings::change(cx, |s| s.last_vault = Some(dir));
        Some(sidebar)
    }

    /// Builds the start-up state from the command line. With no argument, the last vault is
    /// reopened when it still exists (task 10.3).
    fn from_args(cx: &mut Context<Self>) -> Self {
        let arg = std::env::args_os().nth(1).map(PathBuf::from);
        let Some(arg) = arg else {
            let last = settings::current(cx).last_vault.filter(|dir| dir.is_dir());
            if let Some(sidebar) = last.and_then(|dir| Self::open_vault(&dir, cx)) {
                return Self {
                    sidebar: Some(sidebar),
                    document: cx.new(|cx| Document::with_text(cx, "", None)),
                    search: SearchPanel::new(cx.focus_handle()),
                    search_open: false,
                    file_edit: None,
                    sidebar_focus: cx.focus_handle(),
                    mode_watch: None,
                };
            }
            return Self {
                sidebar: None,
                document: cx.new(Document::sample),
                search: SearchPanel::new(cx.focus_handle()),
                search_open: false,
                file_edit: None,
                sidebar_focus: cx.focus_handle(),
                mode_watch: None,
            };
        };
        if arg.is_dir() {
            return Self {
                sidebar: Self::open_vault(&arg, cx),
                document: cx.new(|cx| Document::with_text(cx, "", None)),
                search: SearchPanel::new(cx.focus_handle()),
                search_open: false,
                file_edit: None,
                sidebar_focus: cx.focus_handle(),
                mode_watch: None,
            };
        }
        let dir = match arg.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let mut sidebar = Self::open_vault(dir, cx);
        let document = match read_file(&arg) {
            Ok(text) => {
                if let Some(sidebar) = sidebar.as_mut() {
                    sidebar.expansion.reveal(&sidebar.vault, &arg);
                }
                cx.new(|cx| Document::with_text(cx, &text, Some(arg)))
            }
            Err(err) => {
                eprintln!("noteapp: cannot open {}: {err}", arg.display());
                cx.new(Document::sample)
            }
        };
        Self {
            sidebar,
            document,
            search: SearchPanel::new(cx.focus_handle()),
            search_open: false,
            file_edit: None,
            sidebar_focus: cx.focus_handle(),
            mode_watch: None,
        }
    }

    /// Flips a folder row between expanded and collapsed.
    pub(crate) fn toggle_folder(&mut self, folder: &Path, cx: &mut Context<Self>) {
        if let Some(sidebar) = self.sidebar.as_mut() {
            sidebar.expansion.toggle(folder);
            cx.notify();
        }
    }

    /// Opens `path` in place of the current document. Unsaved edits in the current document
    /// are saved first. If that save fails, the switch is refused. Returns true when `path` is
    /// the open note afterwards.
    pub(crate) fn open_note(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.document.read(cx).path.as_deref() == Some(path.as_path()) {
            return true;
        }
        if self.document.read(cx).is_dirty(cx) {
            if let Err(err) = self.document.update(cx, |doc, cx| doc.save(None, cx)) {
                eprintln!("noteapp: not switching notes, save failed: {err}");
                return false;
            }
        }
        let text = match read_file(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("noteapp: cannot open {}: {err}", path.display());
                return false;
            }
        };
        let document = cx.new(|cx| Document::with_text(cx, &text, Some(path)));
        let focus = document.read(cx).editor.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.document = document;
        cx.notify();
        true
    }

    /// The open note's path and buffer text, so the search scans the buffer and
    /// its hits land on the right offset while the note has unsaved edits.
    /// `None` when the open document has no file (the sample document).
    fn open_buffer(&self, cx: &App) -> Option<(PathBuf, String)> {
        let document = self.document.read(cx);
        let path = document.path.clone()?;
        let text = document.editor.read(cx).text();
        Some((path, text))
    }

    /// Reruns the search over the vault, with the open note scanned from its
    /// buffer instead of from disk (see [`WorkspaceView::open_buffer`]).
    pub(crate) fn refresh_search(&mut self, cx: &mut Context<Self>) {
        let buffer = self.open_buffer(cx);
        let buffer = buffer
            .as_ref()
            .map(|(path, text)| (path.as_path(), text.as_str()));
        self.search
            .refresh_with_buffer(self.sidebar.as_ref().map(|s| &s.vault), buffer);
    }

    /// Shows or hides the search panel (ToggleSearch). Opening it focuses the query input.
    fn toggle_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_open {
            self.close_search(window, cx);
            return;
        }
        self.search_open = true;
        self.refresh_search(cx);
        window.focus(&self.search.focus, cx);
        cx.notify();
    }

    /// Hides the search panel and gives keyboard focus back to the open note's editor.
    pub(crate) fn close_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.search_open = false;
        let focus = self.document.read(cx).editor.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    /// Keys typed while the search panel has focus. Printable text (without Ctrl, Alt or Cmd)
    /// goes to the query. Backspace deletes a character, Enter opens the first result, and
    /// Escape closes the panel.
    pub(crate) fn search_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        match keystroke.key.as_str() {
            "escape" => return self.close_search(window, cx),
            "enter" => return self.open_search_hit(0, window, cx),
            "backspace" => {
                self.search.query.pop();
            }
            _ => {
                let m = keystroke.modifiers;
                if m.control || m.alt || m.platform || m.function {
                    return;
                }
                let Some(text) = keystroke.key_char.as_deref() else {
                    return;
                };
                if text.chars().any(char::is_control) {
                    return;
                }
                self.search.query.push_str(text);
            }
        }
        self.refresh_search(cx);
        cx.notify();
    }

    /// Opens the note of search result `index` with the caret on the match, then closes the
    /// panel. A result whose note cannot be opened keeps the panel open.
    pub(crate) fn open_search_hit(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(hit) = self.search.hits.get(index).cloned() else {
            return;
        };
        if !self.open_note(hit.path.clone(), window, cx) {
            return;
        }
        let offset = hit_offset(&hit);
        let editor = self.document.read(cx).editor.clone();
        editor.update(cx, |editor, cx| {
            editor.set_selection(Selection::caret(offset), cx);
        });
        self.close_search(window, cx);
    }

    fn render_sidebar(&self, sidebar: &Sidebar, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = md_editor::theme::current(cx);
        let open_path = self.document.read(cx).path.clone();
        let rows = sidebar_items(&sidebar.tree, &sidebar.expansion)
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let selected =
                    !item.is_folder() && open_path.as_deref() == Some(item.path.as_path());
                let renaming = renaming_text(&self.file_edit, &item.path);
                let label = match &renaming {
                    Some(text) => text_input::field(&theme, "sidebar-rename", text, "", 20.)
                        .flex_1()
                        .min_w_0()
                        .into_any_element(),
                    None => div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(item.label.clone())
                        .into_any_element(),
                };
                let row = div()
                    .id(("sidebar-row", index))
                    .h(px(ROW_HEIGHT))
                    .w_full()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .pl(px(indent_px(item.depth)))
                    .pr(px(BASE_PADDING))
                    .text_color(theme.ui_text)
                    .when(selected, |style| style.bg(theme.ui_selected))
                    .child(div().w(px(12.)).child(item.marker()))
                    .child(label);
                if renaming.is_some() {
                    // The row being renamed takes keys and clicks in its field only.
                    return row.into_any_element();
                }
                let is_folder = item.is_folder();
                let path = item.path.clone();
                let label = item.label.clone();
                let delete_path = item.path.clone();
                let delete_name = item.label.clone();
                row.cursor_pointer()
                    .hover(|style| style.bg(theme.ui_hover))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _: &MouseDownEvent, window, cx| {
                            this.ask_delete(delete_path.clone(), delete_name.clone(), window, cx);
                        }),
                    )
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        // A double-click starts a rename. The first click has already opened
                        // the note or toggled the folder, so the second one changes nothing else.
                        if event.click_count() >= 2 {
                            this.start_rename(path.clone(), label.clone(), !is_folder, window, cx);
                            return;
                        }
                        this.file_edit = None;
                        if is_folder {
                            this.toggle_folder(&path, cx);
                        } else {
                            this.open_note(path.clone(), window, cx);
                        }
                    }))
                    .into_any_element()
            })
            .collect::<Vec<_>>();

        // The header holds the title and the buttons for a new note ("+") and a new
        // folder ("Folder") (task 9.3).
        let header = div()
            .flex_none()
            .h(px(28.))
            .px_2()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .child(div().text_xs().text_color(theme.ui_muted).child("Files"))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("sidebar-new-folder")
                            .h(px(20.))
                            .px_2()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.ui_text)
                            .hover(|style| style.bg(theme.ui_hover))
                            .child("Folder")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.new_folder(window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id("sidebar-new-note")
                            .w(px(20.))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .cursor_pointer()
                            .text_color(theme.ui_text)
                            .hover(|style| style.bg(theme.ui_hover))
                            .child("+")
                            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                                this.new_note(window, cx);
                            })),
                    ),
            );
        // The rename hint, the Trash question, or the rename error (task 9.3).
        let message = edit_message(&self.file_edit).map(|text| {
            div()
                .flex_none()
                .px_2()
                .pb_1()
                .text_xs()
                .text_color(theme.ui_muted)
                .child(text)
        });

        div()
            .id("sidebar")
            .h_full()
            .w(px(SIDEBAR_WIDTH))
            .flex_none()
            .flex()
            .flex_col()
            .bg(theme.ui_surface)
            .border_r_1()
            .border_color(theme.ui_border)
            .key_context("Sidebar")
            .track_focus(&self.sidebar_focus)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    window.focus(&this.sidebar_focus, cx);
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.file_edit_key_down(event, window, cx);
            }))
            .child(header)
            .children(message)
            .child(
                div()
                    .id("sidebar-rows")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .py_2()
                    .children(rows),
            )
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch_modes(cx);
        let modes = self.document.read(cx).editor.read(cx).modes();
        let theme = md_editor::theme::current(cx);
        let root = div()
            .size_full()
            .flex()
            .flex_row()
            .on_action(cx.listener(|this, _: &ToggleSearch, window, cx| {
                this.toggle_search(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| {
                this.new_note(window, cx);
            }))
            .on_action(cx.listener(|this, _: &NewFolder, window, cx| {
                this.new_folder(window, cx);
            }))
            .on_action(cx.listener(|this, _: &RenameNote, window, cx| {
                this.rename_open_note(window, cx);
            }))
            .on_action(cx.listener(|this, _: &DeleteNote, window, cx| {
                this.ask_delete_open(window, cx);
            }));
        let root = if self.search_open {
            let has_vault = self.sidebar.is_some();
            root.child(search_panel::render_panel(&self.search, has_vault, cx))
        } else {
            match self.sidebar.as_ref() {
                Some(sidebar) => root.child(self.render_sidebar(sidebar, cx)),
                None => root,
            }
        };
        let root = root.child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(self.document.clone()),
        );
        // Muted status for the modes that are on, at the window's bottom right (task 8.5).
        match mode_status(modes) {
            Some(text) => root.child(
                div()
                    .absolute()
                    .bottom(px(6.))
                    .right(px(12.))
                    .text_xs()
                    .text_color(theme.ui_faint)
                    .child(text),
            ),
            None => root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_status_lists_the_modes_that_are_on() {
        let off = Modes::default();
        assert_eq!(mode_status(off), None);
        let all = Modes {
            source: true,
            focus: true,
            typewriter: true,
        };
        assert_eq!(
            mode_status(all).as_deref(),
            Some("Source \u{00B7} Focus \u{00B7} Typewriter")
        );
        let focus_only = Modes { focus: true, ..off };
        assert_eq!(mode_status(focus_only).as_deref(), Some("Focus"));
    }

    fn note(name: &str, path: &str) -> Node {
        Node::Note {
            name: name.into(),
            path: PathBuf::from(path),
        }
    }

    /// `/v` with folder `Alpha` (holding note `a` and folder `sub` with note `inner`) and
    /// note `top`.
    fn fixture() -> Node {
        Node::Folder {
            name: "v".into(),
            path: PathBuf::from("/v"),
            children: vec![
                Node::Folder {
                    name: "Alpha".into(),
                    path: PathBuf::from("/v/Alpha"),
                    children: vec![
                        Node::Folder {
                            name: "sub".into(),
                            path: PathBuf::from("/v/Alpha/sub"),
                            children: vec![note("inner", "/v/Alpha/sub/inner.md")],
                        },
                        note("a", "/v/Alpha/a.md"),
                    ],
                },
                note("top", "/v/top.md"),
            ],
        }
    }

    fn labels(items: &[SidebarItem]) -> Vec<(usize, &str)> {
        items.iter().map(|i| (i.depth, i.label.as_str())).collect()
    }

    #[test]
    fn collapsed_vault_shows_only_top_level_with_closed_markers() {
        let items = sidebar_items(&fixture(), &Expansion::new());
        assert_eq!(labels(&items), vec![(0, "Alpha"), (0, "top")]);
        assert_eq!(items[0].marker(), "\u{25B8}");
        assert_eq!(items[0].expanded, Some(false));
        // Notes have no disclosure marker and no `.md` in their label.
        assert_eq!(items[1].marker(), "");
        assert_eq!(items[1].expanded, None);
    }

    #[test]
    fn expanded_folder_indents_its_children_one_level_deeper() {
        let mut expansion = Expansion::new();
        expansion.set_expanded(Path::new("/v/Alpha"), true);
        let items = sidebar_items(&fixture(), &expansion);
        assert_eq!(
            labels(&items),
            vec![(0, "Alpha"), (1, "sub"), (1, "a"), (0, "top")]
        );
        assert_eq!(items[0].marker(), "\u{25BE}");
        assert_eq!(items[0].path, PathBuf::from("/v/Alpha"));
        assert_eq!(items[2].path, PathBuf::from("/v/Alpha/a.md"));
    }

    #[test]
    fn indent_grows_by_one_step_per_depth() {
        assert_eq!(indent_px(0), BASE_PADDING);
        assert_eq!(indent_px(1) - indent_px(0), INDENT_STEP);
        assert_eq!(indent_px(3), BASE_PADDING + 3.0 * INDENT_STEP);
    }
}
