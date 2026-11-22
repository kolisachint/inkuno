//! The context menu of the tree and the note, and what each entry does.

use super::*;
use crate::ui::assets::ICON_SET;
use crate::ui::components::{
    menu::{menu_item, menu_separator},
    popover::popover,
};
use crate::views::nav::*;
use crate::{
    lang::tr,
    logic::markdown::Link,
    platform::{revisions as history, vault},
    views::editor,
};
use gpui::{
    Action, Animation, AnimationExt, ClipboardItem, Context, Focusable, MouseButton,
    MouseDownEvent, Window, div, ease_out_quint, prelude::*, px,
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

impl Shell {
    /// Runs a context menu entry on `target` (`None`: the vault).
    pub(super) fn menu_do(
        &mut self,
        what: Do,
        target: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let menu = self.menu.take();
        if let Some(back) = menu.as_ref().and_then(|menu| menu.back.as_ref()) {
            window.focus(back);
        }
        // Without a menu (from the palette), the icon grid opens near the top of the note.
        let at = menu
            .as_ref()
            .map_or(point(px(80.), px(80.)), |menu| menu.at);
        let tab = menu
            .as_ref()
            .and_then(|menu| menu.tab)
            .filter(|tab| *tab < self.panes.len());
        let link = menu.and_then(|menu| menu.text).flatten();
        cx.notify();
        // A version of the panel is a file of the vault's hidden history: only its own gestures apply.
        if let (Mode::Versions, Some(path)) = (self.nav.mode, &target) {
            self.version_do(what, path, window, cx);
            return;
        }
        // The app's history (the vault's and the scratch folder's revisions, and their bins) is never
        // moved, renamed, copied or trashed by a file gesture, whatever the selection shows: refused.
        if target
            .as_deref()
            .is_some_and(|path| history::holds_history(path, self.vault.as_deref()))
        {
            return self.say(
                Tone::Info,
                tr(
                    "The history of the notes is kept by the app: nothing is done to it.",
                    "L'historique des notes est tenu par l'application : rien n'y est fait.",
                ),
                cx,
            );
        }
        match (what, tab) {
            (Do::CloseTab, Some(tab)) => return self.close_tab(tab, window, cx),
            (Do::CloseOtherTabs, Some(tab)) => return self.close_other_tabs(tab, window, cx),
            (Do::CloseTabsInPane, Some(tab)) => {
                return self.close_tabs_in_pane(self.group_of(tab), window, cx);
            }
            _ => {}
        }
        if let (Do::Icon, Some(path)) = (what, &target) {
            // On the item's icon if it has one, else on "no icon".
            let chosen = self
                .icons
                .get(path)
                .and_then(|mine| {
                    ICON_SET
                        .iter()
                        .position(|(file, _)| file[2..file.len() - 4] == *mine)
                })
                .map_or(0, |i| i + 1);
            self.icon_pick = Some(IconPick {
                at,
                path: path.clone(),
                chosen,
                back: window.focused(cx),
            });
            window.focus(&self.menu_focus);
            return;
        }
        // Note menu: the editor's gestures, as with the keyboard, and the targeted link.
        let action: Option<Box<dyn Action>> = match (what, link) {
            (Do::Cut, _) => Some(Box::new(editor::Cut)),
            (Do::Copy, _) => Some(Box::new(editor::Copy)),
            (Do::Paste, _) => Some(Box::new(editor::Paste)),
            (Do::Bold, _) => Some(Box::new(editor::Bold)),
            (Do::Italic, _) => Some(Box::new(editor::Italic)),
            (Do::Underline, _) => Some(Box::new(editor::Underline)),
            (Do::Strike, _) => Some(Box::new(editor::Strike)),
            (Do::Highlight, _) => Some(Box::new(editor::Highlight)),
            (Do::Code, _) => Some(Box::new(editor::InlineCode)),
            (Do::Link, _) => Some(Box::new(editor::InsertLink)),
            (Do::Comment, _) => Some(Box::new(editor::Comment)),
            (Do::AcceptChange, _) => Some(Box::new(editor::AcceptChange)),
            (Do::RejectChange, _) => Some(Box::new(editor::RejectChange)),
            (Do::CommentChange, _) => Some(Box::new(editor::CommentChange)),
            (Do::Improve, _) => Some(Box::new(AssistantImprove)),
            (Do::Bullets, _) => Some(Box::new(editor::ListBullet)),
            (Do::Numbered, _) => Some(Box::new(editor::ListOrdered)),
            (Do::Tasks, _) => Some(Box::new(editor::ListTask)),
            (Do::Plain, _) => Some(Box::new(editor::ListText)),
            (Do::Table(op), _) => Some(op.action()),
            (Do::Block(op), _) => Some(op.action()),
            (Do::OpenLink, Some(link)) => return self.follow(&link, window, cx),
            (Do::CopyAddress, Some(Link::Url(url))) => {
                return cx.write_to_clipboard(ClipboardItem::new_string(url));
            }
            // A relative address is copied as Markdown writes it: spaces as `%20`.
            (Do::CopyAddress, Some(Link::Note { path, anchor })) => {
                let address = match anchor {
                    Some(anchor) => format!("{path}#{anchor}"),
                    None => path.clone(),
                };
                let address = address.replace(' ', "%20");
                return cx.write_to_clipboard(ClipboardItem::new_string(address));
            }
            (Do::CopyAddress, Some(Link::Heading(anchor))) => {
                let address = format!("#{anchor}").replace(' ', "%20");
                return cx.write_to_clipboard(ClipboardItem::new_string(address));
            }
            _ => None,
        };
        if let Some(action) = action {
            window.focus(&self.pane().editor.focus_handle(cx));
            return window.dispatch_action(action, cx);
        }
        // The panel without a version targeted (the editor's own menu, the palette): nothing of the tree.
        if self.nav.mode == Mode::Versions {
            return;
        }
        let Some(root) = self.vault.clone() else {
            return;
        };
        // A scratch note is outside the vault: it has its own gestures.
        if let Some(path) = target.clone().filter(|p| platform::scratch::holds(p)) {
            return self.scratch_do(what, &path, window, cx);
        }
        if let Do::List = what {
            return self.open_listing(target.unwrap_or(root), window, cx);
        }
        // A tag is neither a note nor a folder: nothing to rename or trash.
        if target.as_ref().is_some_and(|t| !t.starts_with(&root)) {
            return;
        }
        let is_dir = target.as_ref().is_none_or(|t| self.dirs.contains(t));
        // Targeted folder: the target itself, or the one containing the note.
        let dir = match &target {
            Some(t) if is_dir => t.clone(),
            Some(t) => t.parent().map_or(root.clone(), Path::to_path_buf),
            None => root.clone(),
        };
        // Trash, copies and duplication apply to the whole multiple selection.
        let all = target
            .as_deref()
            .map_or_else(|| vec![root.clone()], |t| self.targets(t));
        let copy = |lines: Vec<String>, cx: &mut Context<Self>| {
            cx.write_to_clipboard(ClipboardItem::new_string(lines.join("\n")));
        };
        match (what, target) {
            (Do::MoveTo, Some(_)) => self.ask_move(all, window, cx),
            (Do::Open, Some(path)) => self.open_from_nav(&path, window, cx),
            (Do::OpenAside, Some(path)) => self.open_aside(&path, window, cx),
            (Do::NewNote, _) => {
                self.nav.open.insert(dir.clone());
                self.nav.sel = Some(dir);
                self.new_note_here(window, cx);
            }
            (Do::NewFolder, _) => {
                // The folder is created in the selection's: the field says so.
                let label = match dir.file_name().filter(|_| dir != root) {
                    Some(name) => format!(
                        "{} « {} »",
                        tr("Folder name, in", "Nom du dossier, dans"),
                        name.to_string_lossy()
                    ),
                    None => tr("Folder name", "Nom du dossier").to_string(),
                };
                self.ask(label, "", window, cx, move |this, name, _, cx| {
                    let new = dir.join(name);
                    match fs::create_dir(&new) {
                        Ok(()) => {
                            this.nav.open.insert(dir.clone());
                            this.dirs.push(new.clone());
                            this.nav.reveal(&new);
                        }
                        Err(e) => {
                            this.fail(tr("Folder not created", "Dossier non créé"), &new, e, cx)
                        }
                    }
                });
            }
            (Do::Rename, Some(path)) => {
                let label = tr("Rename: new name", "Renommer : nouveau nom");
                let current = if is_dir {
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned()
                } else {
                    vault::stem(&path)
                };
                self.ask(
                    label,
                    &current.clone(),
                    window,
                    cx,
                    move |this, name, _, cx| {
                        if name == current {
                            return;
                        }
                        this.flush(cx);
                        let image = vault::is_image(&path) || vault::is_attachment(&path);
                        let table = vault::is_table(&path);
                        // The links follow the rename. The notes that may link to it are read first,
                        // once and off the UI thread: a rename that would rewrite a link inside a
                        // pending change of a linking note is refused before anything moves. The
                        // file is renamed where `to` says (as `rename_note` and the branch below do).
                        let (old, new, to) = if is_dir {
                            (String::new(), String::new(), path.clone())
                        } else {
                            let extension = path.extension().unwrap_or_default().to_string_lossy();
                            let to = match image || table {
                                true => path.with_file_name(format!("{name}.{extension}")),
                                false => path.with_file_name(format!("{name}.md")),
                            };
                            match image {
                                true => (vault::image_name(&path), vault::image_name(&to), to),
                                false => (current.clone(), name.clone(), to),
                            }
                        };
                        let moved = (!image && !is_dir).then_some((path.as_path(), to.as_path()));
                        let scan = (!is_dir).then(|| this.link_scan(&old, &new, moved, None));
                        let (path, name) = (path.clone(), name.clone());
                        cx.spawn(async move |this, cx| {
                            let plan = match scan {
                                Some(scan) => {
                                    cx.background_executor()
                                        .spawn(async move { scan.run() })
                                        .await
                                }
                                None => super::notes::LinkPlan::default(),
                            };
                            this.update(cx, |this, cx| {
                                if this.links_refused(plan.refused, cx) {
                                    return;
                                }
                                let renamed = if is_dir {
                                    let to = path.with_file_name(&name);
                                    vault::rename(&path, &to).map(|()| to)
                                } else if image || table {
                                    // An image or a table keeps its extension.
                                    vault::rename(&path, &to).map(|()| to)
                                } else {
                                    vault::rename_note(this.vault.as_deref(), &path, &name)
                                };
                                match renamed {
                                    Ok(to) => {
                                        // The title of a renamed note may have changed: reread it.
                                        this.relocate(&path, &to, !is_dir, cx);
                                        // Links follow: by note name, or by file name for an image.
                                        if !is_dir && this.apply_links(plan, cx) {
                                            this.reload(cx);
                                        }
                                    }
                                    Err(e) => this.fail(
                                        tr("Not renamed", "Renommage impossible"),
                                        &path,
                                        e,
                                        cx,
                                    ),
                                }
                            })
                            .ok();
                        })
                        .detach();
                    },
                );
            }
            (Do::Trash, Some(_)) => {
                all.iter().for_each(|path| self.trash(&root, path, cx));
                self.nav.marked.clear();
            }
            (Do::CopyLink, Some(_)) => {
                let files = all.iter().filter(|p| !self.dirs.contains(p));
                let link = |path: &PathBuf| match (vault::is_image(path), vault::is_table(path)) {
                    (true, _) => format!("![[{}]]", vault::image_name(path)),
                    // A table has no link: its file name.
                    (_, true) => vault::image_name(path),
                    // An attachment is targeted by its file name, extension included.
                    _ if vault::is_attachment(path) => format!("[[{}]]", vault::image_name(path)),
                    _ => format!("[[{}]]", vault::stem(path)),
                };
                copy(files.map(link).collect(), cx);
            }
            (Do::CopyPath, _) => copy(all.iter().map(|p| p.display().to_string()).collect(), cx),
            (Do::CopyRelative, _) => {
                let relative =
                    |p: &PathBuf| p.strip_prefix(&root).unwrap_or(p).display().to_string();
                copy(all.iter().map(relative).collect(), cx);
            }
            (Do::Reveal, _) => cx.reveal_path(&all[0]),
            // ponytail: only files are duplicated; copying a whole folder
            // needs a recursive walk, to write if the need arises.
            (Do::Duplicate, Some(_)) => {
                // The copy starts from what is on screen, not from a lagging file.
                self.flush(cx);
                let mut last = None;
                for path in all.iter().filter(|p| p.is_file()) {
                    let extension = path.extension().unwrap_or_default().to_string_lossy();
                    let twin = vault::free_path(
                        path.parent().unwrap_or(&root),
                        &vault::stem(path),
                        &extension,
                    );
                    match fs::copy(path, &twin) {
                        Ok(_) => last = Some(twin),
                        Err(e) => {
                            return self.fail(
                                tr("Not duplicated", "Duplication impossible"),
                                path,
                                e,
                                cx,
                            );
                        }
                    }
                }
                if let Some(twin) = last {
                    let (notes, dirs, images) = vault::rescan(&root, &self.notes);
                    self.sync(notes, dirs, images, cx);
                    self.nav.marked.clear();
                    self.nav.reveal(&twin);
                }
            }
            _ => {}
        }
    }

    /// A row menu's entry on the file of the active pane; "Show as a list" lists its folder.
    pub(super) fn file_do(&mut self, what: Do, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.pane().path.clone() else {
            return;
        };
        let target = if matches!(what, Do::List) {
            path.parent().map(Path::to_path_buf)
        } else {
            Some(path)
        };
        self.menu_do(what, target, window, cx)
    }

    /// The folders of the vault in a list, the vault first; the one chosen receives `items`. A folder is
    /// not offered for itself, nor what is inside it.
    fn ask_move(&mut self, items: Vec<PathBuf>, window: &mut Window, cx: &mut Context<Self>) {
        let Some(root) = self.vault.clone() else {
            return;
        };
        let mut dirs: Vec<PathBuf> = self
            .dirs
            .iter()
            .filter(|d| !items.iter().any(|item| d.starts_with(item)))
            .cloned()
            .collect();
        dirs.sort();
        dirs.insert(0, root.clone());
        let names = dirs
            .iter()
            .map(|d| match d.strip_prefix(&root) {
                Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().into_owned(),
                _ => format!("{} ({})", vault::shown(&root), tr("the vault", "le coffre")),
            })
            .collect();
        let back = window.focused(cx);
        let theme = self.theme;
        let palette =
            cx.new(|cx| Palette::choose(tr("Move to", "Déplacer vers"), names, "", theme, cx));
        cx.subscribe_in(&palette, window, move |this, palette, event, window, cx| {
            if matches!(event, PaletteEvent::Preview(_)) {
                return;
            }
            let chosen = palette
                .read(cx)
                .chosen()
                .and_then(|i| dirs.get(i))
                .cloned()
                .filter(|_| matches!(event, PaletteEvent::Submit(_)));
            this.palette = None;
            if let Some(back) = &back {
                window.focus(back);
            }
            if let Some(dir) = chosen {
                this.move_all(&items, &dir, cx);
            }
            cx.notify();
        })
        .detach();
        window.focus(&palette.focus_handle(cx));
        self.palette = Some(palette);
        cx.notify();
    }

    /// Opens `menu`; it takes the keys until it closes.
    pub(super) fn show_menu(&mut self, menu: Menu, window: &mut Window, cx: &mut Context<Self>) {
        let back = self
            .menu
            .take()
            .and_then(|old| old.back)
            .or_else(|| window.focused(cx));
        self.menu = Some(Menu { back, ..menu });
        window.focus(&self.menu_focus);
        cx.notify();
    }

    /// Closes the menu, if one is open, and gives the focus back to what had it.
    pub(super) fn close_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.menu.take().and_then(|menu| menu.back) {
            window.focus(&back);
        }
        cx.notify();
    }

    /// Shift+F10 or the Menu key: the menu of the selected row in the panel, or of the cursor in the note.
    pub(super) fn menu_from_keys(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.nav.focus.is_focused(window) {
            let Some(target) = self.nav.sel.clone() else {
                return;
            };
            let at = self
                .nav
                .sel_at
                .get()
                .map_or(point(px(60.), px(60.)), |row| {
                    row.bottom_left() + point(px(24.), px(0.))
                });
            if self
                .vault
                .as_ref()
                .is_some_and(|root| target.starts_with(root))
                || tag_of(&target).is_some()
            {
                self.show_menu(
                    Menu {
                        at,
                        target: Some(target),
                        chosen: Some(0),
                        ..Default::default()
                    },
                    window,
                    cx,
                );
            }
        } else if self.pane().editor.focus_handle(cx).is_focused(window) {
            self.pane()
                .editor
                .clone()
                .update(cx, |editor, cx| editor.menu_at_cursor(cx));
        }
    }

    /// Arrows, Home, End, Enter and Escape in the open menu.
    fn menu_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let entries: Vec<Do> = self
            .menu_groups(cx)
            .into_iter()
            .flatten()
            .map(|(_, what)| what)
            .collect();
        let (Some(menu), n) = (self.menu.as_mut(), entries.len()) else {
            return false;
        };
        if n == 0 {
            self.close_menu(window, cx);
            return true;
        }
        let at = menu.chosen;
        menu.chosen = match key {
            "down" => Some(at.map_or(0, |i| (i + 1) % n)),
            "up" => Some(at.map_or(n - 1, |i| (i + n - 1) % n)),
            "home" | "pageup" => Some(0),
            "end" | "pagedown" => Some(n - 1),
            "enter" | "space" => {
                if let Some(what) = at.and_then(|i| entries.get(i)) {
                    let target = menu.target.clone();
                    self.menu_do(*what, target, window, cx);
                }
                return true;
            }
            "escape" => {
                self.close_menu(window, cx);
                return true;
            }
            _ => return false,
        };
        cx.notify();
        true
    }

    /// The entries of the open menu, by group; a line separates the groups.
    pub(super) fn menu_groups(&self, cx: &App) -> Vec<Vec<(&'static str, Do)>> {
        let Some(menu) = self.menu.as_ref() else {
            return Vec::new();
        };
        let is_dir = menu.target.as_ref().is_none_or(|p| self.dirs.contains(p));
        let tree = self.nav.mode == Mode::Tree;
        let many = menu
            .target
            .as_ref()
            .is_some_and(|p| self.nav.marked.contains(p))
            && self.nav.marked.len() > 1;
        // Groups of entries; a line separates those that have any.
        let mut groups: Vec<Vec<(&'static str, Do)>> = vec![Vec::new(); 7];
        let scratch =
            menu.tab.is_none() && menu.target.as_deref().is_some_and(platform::scratch::holds);
        // A version of the panel: compared with the note now, its text copied, or restored. A version
        // whose file cannot be read has no restore: there is no text to propose.
        let version =
            self.nav.mode == Mode::Versions && menu.tab.is_none() && menu.target.is_some();
        if version {
            groups[0].push((
                tr(
                    "Compare with the note now",
                    "Comparer avec la note maintenant",
                ),
                Do::CompareVersion,
            ));
            groups[1].push((
                tr(
                    "Copy this version's text",
                    "Copier le texte de cette version",
                ),
                Do::CopyVersion,
            ));
            let readable = menu
                .target
                .as_ref()
                .and_then(|path| self.version_of(path))
                .is_some_and(|item| item.head.is_some());
            if readable {
                groups[1].push((
                    tr("Restore this version", "Restaurer cette version"),
                    Do::RestoreVersion,
                ));
            }
        } else if scratch {
            groups[0].push((tr("Open", "Ouvrir"), Do::Open));
            groups[0].push((tr("Open to the side", "Ouvrir à côté"), Do::OpenAside));
            if self.vault.is_some() {
                groups[1].push((
                    tr("Keep in vault…", "Garder dans le coffre…"),
                    Do::KeepInVault,
                ));
            }
            groups[2].push((tr("Copy path", "Copier le chemin"), Do::CopyPath));
            groups[2].push((reveal_label(), Do::Reveal));
            groups[3].push((tr("Delete", "Supprimer"), Do::DeleteScratch));
        } else if menu.tab.is_some() {
            groups[0].push((tr("Close", "Fermer"), Do::CloseTab));
            groups[0].push((
                tr("Close other tabs", "Fermer les autres onglets"),
                Do::CloseOtherTabs,
            ));
            groups[0].push((
                tr(
                    "Close all tabs in this pane",
                    "Fermer tous les onglets de ce volet",
                ),
                Do::CloseTabsInPane,
            ));
        } else if menu.block {
            // A block's handle: what it can become, move or duplicate it, copy it, delete it.
            use editor::BlockOp;
            groups[0].extend(
                BlockOp::ALL
                    .into_iter()
                    .filter(|op| matches!(op, BlockOp::Turn(_)))
                    .map(|op| (op.label(), Do::Block(op))),
            );
            groups[1].extend(
                [BlockOp::Duplicate, BlockOp::Up, BlockOp::Down]
                    .map(|op| (op.label(), Do::Block(op))),
            );
            groups[2].push((tr("Copy", "Copier"), Do::Copy));
            groups[2].push((tr("Cut", "Couper"), Do::Cut));
            groups[2].push((tr("Comment", "Commenter"), Do::Comment));
            groups[3].push((BlockOp::Delete.label(), Do::Block(BlockOp::Delete)));
        } else if let Some(link) = &menu.text {
            let web = matches!(
                link,
                Some(Link::Url(_) | Link::Note { .. } | Link::Heading(_))
            );
            // Inside a pending change: its gestures, on the change under the pointer.
            if menu.change {
                groups[0].push((
                    tr("Accept the change", "Accepter le changement"),
                    Do::AcceptChange,
                ));
                groups[0].push((
                    tr("Reject the change", "Refuser le changement"),
                    Do::RejectChange,
                ));
                groups[0].push((
                    tr("Comment on the change", "Commenter le changement"),
                    Do::CommentChange,
                ));
            }
            if link.is_some() {
                groups[0].push((tr("Open the link", "Ouvrir le lien"), Do::OpenLink));
            }
            if web {
                groups[0].push((tr("Copy the address", "Copier l'adresse"), Do::CopyAddress));
            }
            groups[1].push((tr("Cut", "Couper"), Do::Cut));
            groups[1].push((tr("Copy", "Copier"), Do::Copy));
            groups[1].push((tr("Paste", "Coller"), Do::Paste));
            if self.pane().editor.read(cx).in_table() {
                // In a table: its rows, its columns, then the table.
                use editor::TableOp::*;
                for (group, ops) in [
                    (
                        2,
                        [
                            RowAbove,
                            RowBelow,
                            DuplicateRow,
                            MoveRowUp,
                            MoveRowDown,
                            DeleteRow,
                        ]
                        .as_slice(),
                    ),
                    (
                        3,
                        [
                            ColLeft,
                            ColRight,
                            DuplicateCol,
                            MoveColLeft,
                            MoveColRight,
                            DeleteCol,
                        ]
                        .as_slice(),
                    ),
                    (4, [SortAsc, SortDesc, Transpose, DeleteTable].as_slice()),
                ] {
                    groups[group].extend(ops.iter().map(|&op| (op.label(), Do::Table(op))));
                }
                groups[5].push((tr("Comment", "Commenter"), Do::Comment));
            } else {
                groups[2].push((tr("Bold", "Gras"), Do::Bold));
                groups[2].push((tr("Italic", "Italique"), Do::Italic));
                groups[2].push((tr("Underline", "Souligné"), Do::Underline));
                groups[2].push((tr("Strikethrough", "Barré"), Do::Strike));
                groups[2].push((tr("Highlight", "Surligné"), Do::Highlight));
                groups[2].push((tr("Code", "Code"), Do::Code));
                groups[2].push((
                    if web {
                        tr("Edit the link", "Modifier le lien")
                    } else {
                        tr("Make a link", "Faire un lien")
                    },
                    Do::Link,
                ));
                groups[2].push((
                    if menu.commented {
                        tr("Resolve the comment", "Résoudre le commentaire")
                    } else {
                        tr("Comment", "Commenter")
                    },
                    Do::Comment,
                ));
                // The targeted lines, from one kind of list to another.
                groups[3].push((tr("Bulleted list", "Liste à puces"), Do::Bullets));
                groups[3].push((tr("Numbered list", "Liste numérotée"), Do::Numbered));
                groups[3].push((tr("Task list", "Liste de tâches"), Do::Tasks));
                groups[3].push((tr("Plain text", "Texte simple"), Do::Plain));
            }
            // The Assistant improves the selection (the whole note is the panel's button).
            if !self.pane().editor.read(cx).span().is_empty() {
                groups[6].push((
                    tr("Improve selection", "Améliorer la sélection"),
                    Do::Improve,
                ));
            }
        } else if menu.target.as_deref().and_then(tag_of).is_some() {
            // A tag is neither a note nor a folder: only its notes can be listed.
            groups[0].push((tr("Show as a list", "Afficher en liste"), Do::List));
        } else {
            if !is_dir && !many {
                groups[0].push((tr("Open", "Ouvrir"), Do::Open));
                groups[0].push((tr("Open to the side", "Ouvrir à côté"), Do::OpenAside));
            }
            if tree && !many {
                groups[1].push((tr("New note here", "Nouvelle note ici"), Do::NewNote));
                groups[1].push((tr("New folder", "Nouveau dossier"), Do::NewFolder));
            }
            if !is_dir || many {
                groups[2].push((tr("Copy the [[link]]", "Copier le [[lien]]"), Do::CopyLink));
            }
            groups[2].push((tr("Copy path", "Copier le chemin"), Do::CopyPath));
            if menu.target.is_some() {
                groups[2].push((
                    tr("Copy relative path", "Copier le chemin relatif"),
                    Do::CopyRelative,
                ));
            }
            if !many {
                groups[2].push((reveal_label(), Do::Reveal));
            }
            if is_dir && !many && tree {
                groups[0].push((tr("Show as a list", "Afficher en liste"), Do::List));
            }
            if menu.target.is_some() {
                if !is_dir || many {
                    groups[3].push((tr("Duplicate", "Dupliquer"), Do::Duplicate));
                }
                if !many {
                    groups[3].push((tr("Rename", "Renommer"), Do::Rename));
                    groups[3].push((tr("Move to…", "Déplacer vers…"), Do::MoveTo));
                    groups[3].push((tr("Icon…", "Icône…"), Do::Icon));
                }
                groups[3].push((tr("Move to the trash", "Mettre à la corbeille"), Do::Trash));
            }
        }
        groups.retain(|g| !g.is_empty());
        groups
    }

    /// The label colour of a menu entry that is not the default: deleting is the caution red in Quiet,
    /// and the trash is the accent in the other themes (which keep their colours otherwise).
    pub(super) fn ink_of(what: &Do, t: Theme) -> Option<gpui::Hsla> {
        let gone = matches!(
            what,
            Do::Trash
                | Do::DeleteScratch
                | Do::Block(editor::BlockOp::Delete)
                | Do::Table(
                    editor::TableOp::DeleteRow
                        | editor::TableOp::DeleteCol
                        | editor::TableOp::DeleteTable
                )
        );
        match () {
            _ if gone && t.quiet => Some(t.caution),
            _ if matches!(what, Do::Trash) => Some(t.accent),
            _ => None,
        }
    }

    /// Context menu, over the whole window: a click elsewhere closes it.
    pub(super) fn render_menu(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let menu = self.menu.as_ref()?;
        let t = self.theme;
        let groups = self.menu_groups(cx);
        let chosen = menu.chosen;
        let at = menu.at;
        let target = menu.target.clone();
        let entry = |i: usize, (label, what): (&'static str, Do), cx: &mut Context<Self>| {
            let target = target.clone();
            let on = chosen == Some(i);
            // The whole label, then its shortcut: the menu takes the width they need.
            menu_item(label, &what.keys(cx), Self::ink_of(&what, t), on, t)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.menu_do(what, target.clone(), window, cx)
                    }),
                )
                .into_any_element()
        };
        let mut rows = Vec::new();
        let mut i = 0;
        for (g, group) in groups.into_iter().enumerate() {
            if g > 0 {
                rows.push(menu_separator(t).into_any_element());
            }
            for item in group {
                rows.push(entry(i, item, cx));
                i += 1;
            }
        }
        // Never larger than the window minus the anchor margin: long labels are ellipsized, the list scrolls.
        let room = window.viewport_size() - gpui::size(t.px(16.), t.px(16.));
        let close = || {
            cx.listener(|this: &mut Self, _: &MouseDownEvent, window, cx| {
                this.close_menu(window, cx)
            })
        };
        Some(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, close())
                .on_mouse_down(MouseButton::Right, close())
                // The menu is as wide as its longest label with its shortcut, whatever
                // the font, and stays in the window, even opened near an edge.
                .child(
                    gpui::anchored()
                        .position(at)
                        .snap_to_window_with_margin(t.px(8.))
                        .child(
                            popover(t)
                                .debug_selector(|| "menu".into())
                                .track_focus(&self.menu_focus)
                                .key_context("Menu")
                                // Escape is the window's `CloseHelp` before it is a key.
                                .on_action(cx.listener(|this, _: &CloseHelp, window, cx| {
                                    this.close_menu(window, cx)
                                }))
                                .on_key_down(cx.listener(
                                    |this, e: &gpui::KeyDownEvent, window, cx| {
                                        if e.keystroke.modifiers.modified() {
                                            return;
                                        }
                                        if this.menu_key(&e.keystroke.key, window, cx) {
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .id("menu-list")
                                .min_w(t.px(230.).min(room.width))
                                .max_w(room.width)
                                .max_h(room.height)
                                .overflow_y_scroll()
                                .py(t.px(if t.quiet { 5. } else { 6. }))
                                .text_size(t.px(13.))
                                .children(rows)
                                .with_animation(
                                    "menu-in",
                                    Animation::new(Duration::from_millis(110))
                                        .with_easing(ease_out_quint()),
                                    |menu, delta| menu.opacity(delta),
                                ),
                        ),
                ),
        )
    }
}
