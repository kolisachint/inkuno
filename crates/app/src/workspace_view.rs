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

use std::path::{Path, PathBuf};

use gpui::{div, prelude::*, px, rgb, Context, Entity, Focusable, IntoElement, Render, Window};
use workspace::tree::{visible_rows, Expansion, Node, Vault};

use crate::file_io::{read_file, Document};

/// Width of the sidebar in pixels.
const SIDEBAR_WIDTH: f32 = 240.;
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
struct Sidebar {
    vault: Vault,
    tree: Node,
    expansion: Expansion,
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
        Some(Self {
            vault,
            tree,
            expansion: Expansion::new(),
        })
    }
}

/// The root view: the sidebar (when there is a vault) and the open document.
pub struct WorkspaceView {
    sidebar: Option<Sidebar>,
    document: Entity<Document>,
}

impl WorkspaceView {
    /// Builds the start-up state from the command line (see the module docs).
    pub fn new(cx: &mut Context<Self>) -> Self {
        let arg = std::env::args_os().nth(1).map(PathBuf::from);
        let Some(arg) = arg else {
            return Self {
                sidebar: None,
                document: cx.new(Document::sample),
            };
        };
        if arg.is_dir() {
            return Self {
                sidebar: Sidebar::open(&arg),
                document: cx.new(|cx| Document::with_text(cx, "", None)),
            };
        }
        let dir = match arg.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let mut sidebar = Sidebar::open(dir);
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
        Self { sidebar, document }
    }

    /// Flips a folder row between expanded and collapsed.
    fn toggle_folder(&mut self, folder: &Path, cx: &mut Context<Self>) {
        if let Some(sidebar) = self.sidebar.as_mut() {
            sidebar.expansion.toggle(folder);
            cx.notify();
        }
    }

    /// Opens `path` in place of the current document. Unsaved edits in the current document
    /// are saved first. If that save fails, the switch is refused.
    fn open_note(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if self.document.read(cx).path.as_deref() == Some(path.as_path()) {
            return;
        }
        if self.document.read(cx).is_dirty(cx) {
            if let Err(err) = self.document.update(cx, |doc, cx| doc.save(None, cx)) {
                eprintln!("noteapp: not switching notes, save failed: {err}");
                return;
            }
        }
        let text = match read_file(&path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("noteapp: cannot open {}: {err}", path.display());
                return;
            }
        };
        let document = cx.new(|cx| Document::with_text(cx, &text, Some(path)));
        let focus = document.read(cx).editor.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        self.document = document;
        cx.notify();
    }

    fn render_sidebar(&self, sidebar: &Sidebar, cx: &mut Context<Self>) -> impl IntoElement {
        let open_path = self.document.read(cx).path.clone();
        let rows = sidebar_items(&sidebar.tree, &sidebar.expansion)
            .into_iter()
            .enumerate()
            .map(|(index, item)| {
                let selected =
                    !item.is_folder() && open_path.as_deref() == Some(item.path.as_path());
                let path = item.path.clone();
                let is_folder = item.is_folder();
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
                    .cursor_pointer()
                    .text_color(rgb(0x1f2937))
                    .hover(|style| style.bg(rgb(0xe5e7eb)))
                    .when(selected, |style| style.bg(rgb(0xdbeafe)))
                    .child(div().w(px(12.)).child(item.marker()))
                    .child(
                        div()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(item.label),
                    );
                let row = if is_folder {
                    row.on_click(cx.listener(move |this, _event, _window, cx| {
                        this.toggle_folder(&path, cx);
                    }))
                } else {
                    row.on_click(cx.listener(move |this, _event, window, cx| {
                        this.open_note(path.clone(), window, cx);
                    }))
                };
                row
            })
            .collect::<Vec<_>>();

        div()
            .id("sidebar")
            .h_full()
            .w(px(SIDEBAR_WIDTH))
            .flex_none()
            .overflow_y_scroll()
            .py_2()
            .bg(rgb(0xf3f4f6))
            .border_r_1()
            .border_color(rgb(0xd1d5db))
            .children(rows)
    }
}

impl Render for WorkspaceView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let root = div().size_full().flex().flex_row();
        let root = match self.sidebar.as_ref() {
            Some(sidebar) => root.child(self.render_sidebar(sidebar, cx)),
            None => root,
        };
        root.child(
            div()
                .flex_1()
                .min_w_0()
                .h_full()
                .child(self.document.clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
