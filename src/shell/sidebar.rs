//! The side panel as the shell drives it: rail, tree, recent notes, outline, search and list
//! pages, context menu and file operations. (Graph, tags, backlinks and calendar views are cut.)

use crate::{
    lang::tr,
    logic::markdown::{self},
    logic::note::Note,
    platform::vault,
    ui::widgets::{button, tip},
    views::palette::{Palette, PaletteEvent},
};
use gpui::{
    Action, ClickEvent, Context, CursorStyle, Div, Focusable, MouseButton, MouseDownEvent, Pixels,
    ScrollStrategy, Stateful, Window, div, point, prelude::*, px, svg, uniform_list,
};
use std::{
    collections::HashSet,
    hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher},
    ops::Range,
    path::{Path, PathBuf},
};

use super::*;
use crate::ui::assets::ICON_SET;
use crate::views::nav::*;

/// Room the native traffic lights float over at the window's top left: what the sidebar
/// header leaves free on that side, at scale 1 (mirrors `PILL_ROOM` on the right under the drawn bar).
const TRAFFIC_ROOM: f32 = 78.;

impl Shell {
    /// Shows the tool where it lives (its dock, alone unless it is stacked there already, or its
    /// tab) and gives it the keys. Asking again for a tool in sight hides it (from the keyboard:
    /// only if it already has the keys); the last one of the left dock folds the panel.
    pub(super) fn show_nav(
        &mut self,
        mode: Mode,
        toggle: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.tool_tab(mode) {
            return self.focus_pane(tab, window, cx);
        }
        let keys = self.nav.focus.contains_focused(window, cx) && self.nav.mode == mode;
        if self.tool_shown(mode) && (toggle || keys) {
            return self.hide_tool(mode, window, cx);
        }
        self.put_tool(mode);
        self.nav.reveal = true;
        window.focus(&self.nav.focus);
        self.settle_nav(window, cx);
    }

    pub(super) fn toggle_full(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.nav.panel = match self.nav.panel {
            Panel::Full => Panel::Split,
            _ => Panel::Full,
        };
        self.settle_nav(window, cx);
    }

    /// After a panel change: focus leaves what is no longer
    /// shown, and the layout is saved.
    pub(super) fn settle_nav(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        (self.nav.dragging, self.nav.right_dragging) = (false, false);
        let editor = self.pane().editor.focus_handle(cx);
        match self.nav.panel {
            Panel::Rail if self.nav.focus.contains_focused(window, cx) => window.focus(&editor),
            Panel::Full if editor.is_focused(window) => window.focus(&self.nav.focus),
            _ => {}
        }
        self.nav.save();
        cx.notify();
    }

    /// Width of the panel open beside the note: the one it was given, as long as the note
    /// (`NOTE_MIN`) and what is reserved to its right keep their room.
    pub(super) fn split_width(&self) -> Pixels {
        let room = self.nav.total - self.theme.px(RAIL) - DIVIDER - self.nav.aside;
        let floor = px(120.);
        match self.nav.list_w {
            w if w == px(0.) => room / 2.,
            w => w.min(room - NOTE_MIN).max(floor),
        }
    }

    /// The separator follows the pointer; at the ends, one side collapses.
    pub(super) fn drag_nav(&mut self, x: Pixels, cx: &mut Context<Self>) {
        let x = x - self.nav.left - self.theme.px(RAIL);
        let room = self.nav.total - self.theme.px(RAIL);
        self.nav.panel = if x < px(100.) {
            Panel::Rail
        } else if x > room - px(140.) {
            Panel::Full
        } else {
            self.nav.list_w = x;
            Panel::Split
        };
        cx.notify();
    }

    /// Hash of everything the panel's rows depend on: recomputing them
    /// costs ten times more than checking they are still right.
    pub(super) fn rows_source(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        let open = self
            .nav
            .open
            .iter()
            .map(|dir| BuildHasherDefault::<DefaultHasher>::default().hash_one(dir))
            .fold(0u64, u64::wrapping_add);
        (
            self.nav.mode,
            self.nav.panel == Panel::Rail,
            &self.vault,
            &self.dirs,
            &self.images,
            open,
        )
            .hash(&mut hasher);
        if self.nav.mode == Mode::Recent {
            self.recent.hash(&mut hasher);
        }
        for note in &self.notes {
            (&note.path, &note.name).hash(&mut hasher);
        }
        hasher.finish()
    }

    pub(super) fn nav_rows(&self) -> Vec<Row> {
        let Some(root) = &self.vault else {
            return Vec::new();
        };
        match self.nav.mode {
            Mode::Tree => tree_rows(root, &self.notes, &self.dirs, &self.images, &self.nav.open),
            Mode::Search | Mode::Outline => Vec::new(),
            Mode::Recent => self
                .by_recency()
                .into_iter()
                .map(|n| Row {
                    path: n.path.clone(),
                    name: n.name.clone(),
                    depth: 0,
                    dir: None,
                })
                .collect(),
        }
    }

    pub(super) fn selected_row(&self) -> Option<usize> {
        let sel = self.nav.sel.as_ref()?;
        self.nav.rows.iter().position(|r| r.path == *sel)
    }

    /// Selects the row; a note shows as a preview if it has room.
    pub(super) fn nav_select(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.nav.rows.get(ix) else {
            return;
        };
        let (path, is_note) = (row.path.clone(), row.dir.is_none());
        self.nav.marked.clear();
        self.nav.scroll.scroll_to_item(ix, ScrollStrategy::Center);
        if is_note && self.nav.panel == Panel::Split {
            self.preview_note(&path, window, cx);
        }
        self.nav.sel = Some(path);
        self.nav.reveal = false;
        cx.notify();
    }

    /// Ctrl+click adds the row to the selection or removes it; Shift+click
    /// selects everything since the last chosen row.
    pub(super) fn nav_mark(&mut self, ix: usize, extend: bool, cx: &mut Context<Self>) {
        let Some(path) = self.nav.rows.get(ix).map(|r| r.path.clone()) else {
            return;
        };
        let marked = &mut self.nav.marked;
        if extend {
            let from = self
                .nav
                .sel
                .as_ref()
                .and_then(|sel| self.nav.rows.iter().position(|r| r.path == *sel))
                .unwrap_or(ix);
            *marked = self.nav.rows[from.min(ix)..=from.max(ix)]
                .iter()
                .map(|r| r.path.clone())
                .collect();
            self.nav.sel.get_or_insert(path);
        } else {
            // The row already selected is part of the batch.
            if marked.is_empty() {
                marked.extend(self.nav.sel.take());
            }
            if !marked.remove(&path) {
                marked.insert(path.clone());
            }
            // Removed, it leaves its chosen-row place to another of the batch.
            let other = self
                .nav
                .rows
                .iter()
                .map(|r| &r.path)
                .find(|p| marked.contains(*p))
                .cloned();
            self.nav.sel = if marked.contains(&path) {
                Some(path)
            } else {
                other.or(Some(path))
            };
        }
        cx.notify();
    }

    /// What an action on `target` aims at: the whole multiple selection if it
    /// is part of it, in panel order, without what a targeted folder already contains.
    pub(super) fn targets(&self, target: &Path) -> Vec<PathBuf> {
        let marked = &self.nav.marked;
        if !marked.contains(target) {
            return vec![target.to_path_buf()];
        }
        let mut seen = HashSet::new();
        let rows = self.nav.rows.iter().map(|r| &r.path);
        rows.filter(|p| {
            marked.contains(*p)
                && !p.ancestors().skip(1).any(|a| marked.contains(a))
                && seen.insert(*p)
        })
        .cloned()
        .collect()
    }

    pub(super) fn nav_step(&mut self, down: bool, window: &mut Window, cx: &mut Context<Self>) {
        let last = self.nav.rows.len().saturating_sub(1);
        let ix = match self.selected_row() {
            Some(ix) if down => (ix + 1).min(last),
            Some(ix) => ix.saturating_sub(1),
            None => 0,
        };
        self.nav_select(ix, window, cx);
    }

    /// Enter or double-click: opens the note for good, or toggles the folder.
    pub(super) fn nav_activate(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(row) = self.nav.rows.get(ix) else {
            return;
        };
        let path = row.path.clone();
        self.nav.marked.clear();
        if row.dir.is_some() {
            if !self.nav.open.remove(&path) {
                self.nav.unfold(path.clone());
            }
            self.nav.sel = Some(path);
            cx.notify();
        } else {
            self.open_from_nav(&path, window, cx);
        }
    }

    /// Opens the note and puts focus in it; the note gets its place back if the
    /// panel took the whole window.
    pub(super) fn open_from_nav(
        &mut self,
        path: &Path,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open(path, window, cx);
        if self.nav.panel == Panel::Full {
            self.nav.panel = Panel::Split;
        }
        // A table takes focus so one can write in it; an image shows in place
        // of the note, and the panel keeps control.
        match self.shown_sheet() {
            Some(sheet) => window.focus(&sheet.focus_handle(cx)),
            None if self.pane().file.is_none() => {
                window.focus(&self.pane().editor.focus_handle(cx))
            }
            None => {}
        }
        self.settle_nav(window, cx);
    }

    /// Left/right in the tree: collapse or expand, otherwise go up or down.
    pub(super) fn nav_fold(&mut self, unfold: bool, window: &mut Window, cx: &mut Context<Self>) {
        let Some(ix) = self.selected_row() else {
            return self.nav_step(true, window, cx);
        };
        let row = self.nav.rows[ix].clone();
        match (row.dir, unfold) {
            (Some(false), true) => {
                self.nav.unfold(row.path);
            }
            (Some(true), false) => {
                self.nav.open.remove(&row.path);
            }
            (_, true) => return self.nav_step(true, window, cx),
            (_, false) => {
                let parent = row.path.parent();
                if let Some(up) = self
                    .nav
                    .rows
                    .iter()
                    .position(|r| Some(r.path.as_path()) == parent)
                {
                    return self.nav_select(up, window, cx);
                }
            }
        }
        cx.notify();
    }

    /// Collapses all the tree's folders; if they already are, expands them all.
    pub(super) fn fold_all(&mut self) {
        if self.all_folded() {
            self.nav.open = self.dirs.iter().cloned().collect();
        } else {
            self.nav.open.clear();
        }
    }

    /// No vault folder is expanded. `open` also keeps the parents of the
    /// vault itself, hence the folder-by-folder test.
    pub(super) fn all_folded(&self) -> bool {
        !self.dirs.iter().any(|d| self.nav.open.contains(d))
    }

    /// Asks for a name in an input field, then calls `then` with that name cleaned up.
    pub(crate) fn ask(
        &mut self,
        label: impl Into<String>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
        then: impl Fn(&mut Self, String, &mut Window, &mut Context<Self>) + 'static,
    ) {
        let theme = self.theme;
        let prompt = cx.new(|cx| Palette::prompt(label, text, theme, cx));
        cx.subscribe_in(&prompt, window, move |this, _, event, window, cx| {
            this.palette = None;
            window.focus(&this.nav.focus);
            if let PaletteEvent::Submit(text) = event
                && let Some(name) = vault::clean_name(text)
            {
                then(this, name, window, cx);
            }
            cx.notify();
        })
        .detach();
        window.focus(&prompt.focus_handle(cx));
        self.palette = Some(prompt);
        cx.notify();
    }

    /// Closes the icon grid, if it is open, and gives the focus back.
    pub(super) fn close_icons(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(back) = self.icon_pick.take().and_then(|pick| pick.back) {
            window.focus(&back);
        }
        cx.notify();
    }

    /// Gives cell `i` of the grid (0: no icon) to the item, and closes the grid.
    fn pick_icon(&mut self, i: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.icon_pick.as_ref().map(|pick| pick.path.clone()) else {
            return;
        };
        self.close_icons(window, cx);
        let name = i
            .checked_sub(1)
            .and_then(|i| ICON_SET.get(i))
            .map(|(file, _)| &file[2..file.len() - 4]);
        self.set_icon(path, name, cx)
    }

    /// Arrows (eight cells a row), Home, End, Enter and Space in the icon grid.
    fn icons_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let n = ICON_SET.len() + 1;
        let Some(pick) = self.icon_pick.as_mut() else {
            return false;
        };
        let at = pick.chosen;
        pick.chosen = match key {
            "right" => (at + 1) % n,
            "left" => (at + n - 1) % n,
            "down" => (at + 8).min(n - 1),
            "up" => at.saturating_sub(8),
            "home" => 0,
            "end" => n - 1,
            "enter" | "space" => {
                self.pick_icon(at, window, cx);
                return true;
            }
            _ => return false,
        };
        cx.notify();
        true
    }

    /// Icon grid, opened from the menu: a click or Enter gives the icon, the cross removes it,
    /// a click elsewhere or Esc closes it.
    pub(super) fn render_icons(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<impl IntoElement + use<>> {
        let IconPick {
            at, path, chosen, ..
        } = self.icon_pick.as_ref()?;
        let (at, chosen) = (*at, *chosen);
        let t = self.theme;
        let mine = self.icons.get(path).cloned();
        let pick =
            |i: usize, file: &'static str, name: Option<&'static str>, cx: &mut Context<Self>| {
                button(
                    ("icon", i),
                    file,
                    name.is_some() && name == mine.as_deref(),
                    t,
                )
                .when(i == chosen, |d| d.border_2().border_color(t.accent))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.pick_icon(i, window, cx)
                    }),
                )
            };
        let mut cells = vec![pick(0, "close.svg", None, cx)];
        cells.extend(
            ICON_SET
                .iter()
                .enumerate()
                .map(|(i, (file, _))| pick(i + 1, file, Some(&file[2..file.len() - 4]), cx)),
        );
        // Eight per row; the grid stays in the window, like the menu.
        let view = window.viewport_size();
        let (width, height) = (
            px(8. * 30. - 2. + 12.),
            px(cells.len().div_ceil(8) as f32 * 30. - 2. + 12.),
        );
        let at = point(
            (at.x - self.nav.left).min(view.width - self.nav.left * 2. - width - t.px(8.)),
            (at.y - self.nav.left).min(view.height - self.nav.left * 2. - height - t.px(8.)),
        );
        let close = cx.listener(|this: &mut Self, _: &MouseDownEvent, window, cx| {
            this.close_icons(window, cx)
        });
        Some(
            div()
                .absolute()
                .inset_0()
                .occlude()
                .on_mouse_down(MouseButton::Left, close)
                .child(
                    div()
                        .debug_selector(|| "icons".into())
                        .track_focus(&self.menu_focus)
                        .key_context("Menu")
                        .on_action(cx.listener(|this, _: &CloseHelp, window, cx| {
                            this.close_icons(window, cx)
                        }))
                        .on_key_down(cx.listener(|this, e: &gpui::KeyDownEvent, window, cx| {
                            if !e.keystroke.modifiers.modified()
                                && this.icons_key(&e.keystroke.key, window, cx)
                            {
                                cx.stop_propagation();
                            }
                        }))
                        .absolute()
                        .left(at.x)
                        .top(at.y)
                        .w(width)
                        .p(t.px(5.))
                        .bg(t.panel)
                        .border_1()
                        .border_color(t.border)
                        .rounded(t.large())
                        .shadow(t.floating())
                        .flex()
                        .flex_wrap()
                        .gap(t.px(2.))
                        .children(cells),
                ),
        )
    }

    /// The width the rows of the panel's list have: the dock it is in, the whole window when it is alone.
    fn list_room(&self) -> Pixels {
        let (right, alone) = (
            self.dock_tools(true).contains(&self.nav.mode),
            self.nav.panel == Panel::Full,
        );
        let width = match (right, alone) {
            (true, _) => self.right_width(),
            (_, true) => self.nav.total - self.theme.px(RAIL),
            _ => self.split_width(),
        };
        // The list's own margin (`px_2`) on both sides.
        width - px(16.)
    }

    /// The outline in the right dock: the note's headings, indented by level, the one of the
    /// cursor's section lit; parents fold. A click jumps the cursor there and gives the note the
    /// keys back; Escape in the list does the same (`nav::Close`).
    fn render_outline(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let t = self.theme;
        let (text, pos) = match self.vault.is_some() && self.pane().file.is_none() {
            true => {
                let editor = self.pane().editor.read(cx);
                (editor.text().to_string(), editor.position())
            }
            // An image, a table, a diagram: no headings to list.
            false => (String::new(), 0),
        };
        let (rows, _) =
            outline_visible(&markdown::headings(&text), pos, &self.nav.outline_collapsed);
        if rows.is_empty() {
            return div()
                .flex_1()
                .min_h_0()
                .flex()
                .items_start()
                .justify_center()
                .p_3()
                .child(
                    div()
                        .debug_selector(|| "outline-empty".into())
                        .text_size(t.px(13.))
                        .text_color(t.soft)
                        .child(tr(
                            "This note has no headings",
                            "Cette note n'a pas de titres",
                        )),
                )
                .into_any_element();
        }
        let count = rows.len();
        let rows = cx.processor(move |this, range: std::ops::Range<usize>, _window, cx| {
            let (text, pos) = match this.vault.is_some() && this.pane().file.is_none() {
                true => {
                    let editor = this.pane().editor.read(cx);
                    (editor.text().to_string(), editor.position())
                }
                false => (String::new(), 0),
            };
            let (rows, active) =
                outline_visible(&markdown::headings(&text), pos, &this.nav.outline_collapsed);
            rows.into_iter()
                .enumerate()
                .skip(range.start)
                .take(range.end - range.start)
                .map(|(ix, row)| this.outline_item(ix, row, active == Some(ix), cx))
                .collect()
        });
        uniform_list(("nav-rows", Mode::Outline as usize), count, rows)
            .track_scroll(self.nav.scroll.clone())
            .flex_1()
            .min_h_0()
            .into_any_element()
    }

    /// One row of the outline: its fold chevron (when it parents deeper headings) and its label,
    /// indented by level. A click jumps there; the chevron folds without jumping.
    fn outline_item(
        &mut self,
        ix: usize,
        row: OutlineRow,
        active: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let t = self.theme;
        let at = row.at;
        let indent = t.px(INDENT * row.level.saturating_sub(1) as f32);
        let mut slot = div()
            .flex_none()
            .w(t.px(20.))
            .h_full()
            .flex()
            .items_center()
            .justify_center();
        if row.kids {
            let icon = match row.shut {
                true => "chevron-right.svg",
                false => "chevron-down.svg",
            };
            slot = slot.child(
                button(("outline-fold", ix), icon, false, t)
                    .debug_selector(move || format!("outline-fold-{ix}"))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.nav.outline_collapsed.remove(&at) {
                            this.nav.outline_collapsed.insert(at);
                        }
                        cx.notify();
                    })),
            );
        }
        div()
            .id(("outline-row", ix))
            .debug_selector(move || format!("outline-row-{ix}"))
            .w_full()
            .h(t.px(ROW))
            .flex()
            .items_center()
            .pl(t.px(8.))
            .pr_2()
            .gap_1()
            .cursor_pointer()
            .text_size(t.px(13.))
            .text_color(match active {
                true => t.accent,
                false => t.text,
            })
            .when(active, |d| d.bg(t.hover))
            .hover(|s| s.bg(t.hover))
            .on_click(cx.listener(move |this, _, window, cx| this.outline_go(at, window, cx)))
            .child(slot)
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .pl(indent)
                    .child(row.label),
            )
            .into_any_element()
    }

    pub(super) fn render_rows(
        &mut self,
        range: Range<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<Stateful<Div>> {
        let t = self.theme;
        let focused = self.nav.focus.is_focused(window);
        let tree = self.nav.mode == Mode::Tree;
        let marked = &self.nav.marked;
        // Dragging a row of the multiple selection carries the whole batch.
        let lot = marked
            .iter()
            .next()
            .map(|any| self.targets(any))
            .unwrap_or_default();
        let room = self.list_room();
        range
            .filter_map(|ix| {
                let row = self.nav.rows.get(ix)?;
                let selected = match marked.is_empty() {
                    true => self.nav.sel.as_ref() == Some(&row.path),
                    false => marked.contains(&row.path),
                };
                let current = self.pane().displayed_path() == Some(&row.path);
                // In the recent list, the note's folder serves as detail.
                let detail = match (&self.vault, row.path.parent()) {
                    (Some(root), Some(dir)) if !tree => dir
                        .strip_prefix(root)
                        .map(|d| d.display().to_string())
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                // A folder of the tree that cannot be read says so and why, instead of looking empty.
                let closed = self.unreadable.iter().find(|(dir, _)| tree && *dir == row.path);
                let detail = match closed {
                    Some((_, why)) => format!("{} {}", tr("unreadable:", "illisible :"), crate::lang::io_reason(&std::io::Error::from(*why))),
                    None => detail,
                };
                let wide = closed.is_some();
                let said = closed.map(|_| detail.clone());
                let foldable = self.nav.mode != Mode::Recent;
                let chevron = match row.dir {
                    Some(true) => "chevron-down.svg",
                    Some(false) => "chevron-right.svg",
                    None => "",
                };
                // Name and detail share the row by measure: a text is cut with an ellipsis only when it
                // is given its width, so the shares are worked out here.
                let (stem, ext) = row.split();
                let shown = {
                    let used = 8. + nav::INDENT * row.depth as f32 + 8. + 8.;
                    let has_icon = tree || self.icons.contains_key(&row.path);
                    let fixed = t.px(used + if foldable { 21. } else { 0. } + if has_icon { 21. } else { 0. }) ;
                    let avail = f32::from(room - fixed);
                    let measure = |text: &str, size: f32| -> f32 {
                        if text.is_empty() {
                            return 0.;
                        }
                        let run = gpui::TextRun { len: text.len(), font: gpui::font(crate::ui::assets::sans()), color: t.text, background_color: None, underline: None, strikethrough: None };
                        let line = window.text_system().shape_line(gpui::SharedString::from(text.to_string()), t.px(size), &[run], None);
                        f32::from(line.width).ceil() + 1.
                    };
                    let (name_w, detail_w) = (measure(&row.name, 13.), measure(&detail, 11.5));
                    let (name_room, detail_room) = nav::share(avail, name_w, detail_w, f32::from(t.px(4.)), f32::from(t.px(nav::DETAIL_MIN)));
                    // The ellipsis goes into the name; its extension stays whole.
                    let stem_room = if name_w <= name_room { f32::MAX } else { name_room - measure(ext, 13.) };
                    (
                        nav::ellipsize(stem, stem_room, |s| measure(s, 13.)),
                        nav::ellipsize(&detail, detail_room, |s| measure(s, 11.5)),
                    )
                };
                let line = div()
                    .when(current, |d| d.debug_selector(|| "nav-current".into()))
                    .size_full()
                    .pr_2()
                        .pl(px(8. + nav::INDENT * row.depth as f32))
                        .flex()
                        .items_center()
                        .gap(t.px(7.))
                        .rounded(t.large())
                        .text_size(t.px(13.))
                        .when(!selected, |d| d.hover(|s| s.bg(t.hover)))
                        // The displayed file: accent text and icon, no second tint; the selection wins on the same row.
                        .when(selected, |d| {
                            let at = self.nav.sel_at.clone();
                            d.bg(if focused { t.selection } else { t.selection.opacity(0.5) })
                                .relative()
                                .child(gpui::canvas(move |bounds, _, _| at.set(Some(bounds)), |_, _, _, _| {}).absolute().size_full())
                        })
                        .when(foldable, |d| {
                            d.child(div().size(t.px(14.)).flex_none().when(row.dir.is_some(), |d| {
                                d.child(svg().path(chevron).size(t.px(14.)).text_color(if selected { t.text } else { t.soft }))
                            }))
                        })
                        // What the row is, in one 14 px column for every row so the names line up: the icon
                        // the user gave it (a star is the accent's), else, in the tree, the one of its kind.
                        .children({
                            let given = self.icons.get(&row.path).and_then(|name| ICON_SET.iter().find(|(file, _)| file[2..file.len() - 4] == *name));
                            let (file, tint) = match given {
                                Some((file, _)) => (*file, if *file == "i-star.svg" || current { t.accent } else { t.text }),
                                None => (row.icon(), if current { t.accent } else { t.soft }),
                            };
                            let selector = format!("nav-icon-{}", row.name);
                            (given.is_some() || tree).then(|| svg().debug_selector(move || selector.clone()).path(file).size(t.px(14.)).flex_none().text_color(tint))
                        })
                        .child(
                            div()
                                .debug_selector({
                                    let name = format!("nav-name-{}", row.name);
                                    move || name.clone()
                                })
                                .overflow_hidden()
                                .flex()
                                // The name goes first, in full as long as it fits; the link's line, or the
                                // reason a folder cannot be read, takes what is left.
                                .map(|d| match (closed.is_some(), wide) {
                                    // The reason a folder cannot be read is said in full; its name gives way.
                                    (true, _) => d.flex_1().min_w(t.px(40.)),
                                    (_, true) => d.flex_none().max_w(gpui::relative(0.7)),
                                    _ => d.flex_none().whitespace_nowrap(),
                                })
                                .when(current, |d| d.text_color(t.accent))
                                .child(div().when(wide, |d| d.min_w_0().truncate()).child(if wide { stem.to_string() } else { shown.0.clone() }))
                                .when(!ext.is_empty(), |d| {
                                    let selector = format!("nav-ext-{}", row.name);
                                    d.child(div().debug_selector(move || selector.clone()).flex_none().text_color(t.soft).child(ext.to_string()))
                                }),
                        )
                        .when(!wide, |d| d.child(div().flex_1().min_w_0()))
                        .child(
                            div()
                                .debug_selector({
                                    let why = format!("nav-why-{}", row.name);
                                    move || why.clone()
                                })
                                .map(|d| match (closed.is_some(), wide) {
                                    // ponytail: in a panel too narrow for it, the reason is cut last, in full in the row's tooltip.
                                    (true, _) => d.flex_initial().min_w_0().truncate(),
                                    (_, true) => d.flex_1().min_w_0().truncate(),
                                    // At the right, in the width the name leaves it (none: hidden).
                                    _ => d.flex_none().whitespace_nowrap(),
                                })
                                .text_size(t.px(11.5))
                                .text_color(if selected { t.text } else { t.soft })
                                .child(if wide { detail.clone() } else { shown.1.clone() }),
                        );
                let (path, name) = (row.path.clone(), row.name.clone());
                // Dropping on a note files into that note's folder.
                let into = match row.dir {
                    Some(_) => Some(path.clone()),
                    None => path.parent().map(Path::to_path_buf),
                };
                let (target, drop_into) = (path.clone(), into.clone());
                let (paths, name) = match marked.contains(&path) {
                    true => (lot.clone(), format!("{} {}", lot.len(), tr("items", "éléments"))),
                    false => (vec![path], name),
                };
                let selector = format!("nav-row-{}", row.name);
                Some(
                    div()
                        .id(ix)
                        .debug_selector(move || selector.clone())
                        .w_full()
                        .h(t.px(ROW))
                        .px_2()
                        .cursor_pointer()
                        .child(match self.nav.fading(&row.path).filter(|_| tree && t.motion) {
                            Some(seq) => crate::ui::widgets::fade_in(line, (gpui::SharedString::from(format!("row-in-{seq}")), ix), &t),
                            None => line.into_any_element(),
                        })
                        .when_some(said, |d, said| d.tooltip(tip(said, String::new(), t)))
                        .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                            let held = e.modifiers();
                            if held.secondary() || held.shift {
                                return this.nav_mark(ix, held.shift, cx);
                            }
                            // A click opens the note, in its tab; a second click finds it there.
                            this.nav_activate(ix, window, cx)
                        }))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, e: &MouseDownEvent, window, cx| {
                                // Outside the multiple selection, right-click replaces it.
                                if !this.nav.marked.contains(&target) {
                                    this.nav.marked.clear();
                                    this.nav.sel = Some(target.clone());
                                }
                                window.focus(&this.nav.focus);
                                if this.vault.as_ref().is_some_and(|root| target.starts_with(root)) {
                                    this.show_menu(Menu { at: e.position, target: Some(target.clone()), ..Default::default() }, window, cx);
                                }
                                cx.stop_propagation();
                                cx.notify();
                            }),
                        )
                        // Files coming from the system: they go into the row's folder, or into the
                        // note's, and the row tints to say so.
                        .when(tree, |d| {
                            let aim_into = drop_into.clone();
                            d.drag_over::<gpui::ExternalPaths>(move |style, _, _, _| style.bg(t.selection))
                                .on_drag_move(cx.listener(move |this, e: &gpui::DragMoveEvent<gpui::ExternalPaths>, _, cx| {
                                    let Some(dir) = &aim_into else { return };
                                    let mine = matches!(&this.drop_aim, Some(super::DropAim { aim: super::Aim::Folder(d), .. }) if d == dir);
                                    if e.bounds.contains(&e.event.position) {
                                        let aim = super::DropAim { aim: super::Aim::Folder(dir.clone()), items: e.drag(cx).paths().len(), pointer: e.event.position };
                                        this.aim_drop(Some(aim), cx);
                                    } else if mine {
                                        this.aim_drop(None, cx);
                                    }
                                }))
                                .on_drop(cx.listener(move |this, paths: &gpui::ExternalPaths, _, cx| {
                                    if let Some(dir) = &drop_into {
                                        this.drop_paths(paths.paths().to_vec(), super::Aim::Folder(dir.clone()), cx);
                                    }
                                }))
                        })
                        .when(tree, |d| {
                            d.on_drag(Dragged { paths, name, theme: t }, |dragged, _, _, cx| {
                                cx.new(|_| dragged.clone())
                            })
                            .when(row.dir.is_some(), |d| {
                                d.drag_over::<Dragged>(move |style, _, _, _| style.bg(t.selection))
                            })
                            .on_drop(cx.listener(move |this, dragged: &Dragged, _, cx| {
                                if let Some(into) = &into {
                                    this.move_all(&dragged.paths, into, cx)
                                }
                            }))
                        }),
                )
            })
            .collect()
    }

    /// The rail, then the left dock and its separator if it is open.
    pub(super) fn render_nav(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let current = self.pane().displayed_path().cloned();
        if current != self.nav.followed {
            if let Some(path) = &current {
                self.nav.reveal(path);
            }
            self.nav.followed = current.clone();
        }
        // The keys stay with a tool in sight: the active one hidden, the first one shown takes them.
        if !self.docked_now(self.nav.mode)
            && let Some(first) = self
                .dock_tools(false)
                .into_iter()
                .filter(|_| self.left_open())
                .chain(
                    self.dock_tools(true)
                        .into_iter()
                        .filter(|_| self.right_open()),
                )
                .next()
        {
            self.nav.activate(first);
        }
        let panel = self.nav.panel;
        let unfolded = self.left_open();

        // The view in sight is a small sheet laid on the rail, its icon in the accent.
        let on_sheet = |b: Stateful<Div>, on: bool| {
            b.rounded(t.large())
                .when(on, |d| d.bg(t.bg).shadow(t.lifted()))
        };
        let shown: Vec<Mode> = Mode::ALL
            .into_iter()
            .filter(|m| self.tool_shown(*m))
            .collect();
        let mode_button = |id: &'static str, icon, m: Mode, label: &'static str, keys: String| {
            on_sheet(button(id, icon, shown.contains(&m), t), shown.contains(&m))
                .debug_selector(move || id.to_string())
                .tooltip(tip(label, keys, t))
                // Shift+click: with the others of its dock, one above the other.
                // A plain click never hides: the sidebar stays shown (hiding is on the keys
                // and the palette; bring back by restoring `true`).
                .on_click(cx.listener(move |this, e: &ClickEvent, window, cx| {
                    match e.modifiers().shift {
                        true => this.stack_tool(m, window, cx),
                        false => this.show_nav(m, false, window, cx),
                    }
                }))
        };
        let keys = |action: &dyn Action| crate::ui::keys::of(cx, action);
        let rail = div()
            .id("rail")
            .flex_none()
            .w(t.px(RAIL))
            .h_full()
            // A short window: the rail scrolls, its last buttons stay reachable.
            .overflow_y_scroll()
            .py_2()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            // The app logo is the commands button, at the top of the rail on every system.
            .child(
                div()
                    .id("nav-commands")
                    .debug_selector(|| "nav-commands".into())
                    .size(t.px(28.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(t.small())
                    .cursor_pointer()
                    .tab_index(0)
                    .focus(|s| s.border_2().border_color(t.accent))
                    .hover(|s| s.bg(t.hover))
                    .active(|s| s.bg(t.pressed))
                    .child(
                        div()
                            .debug_selector(|| "rail-logo".into())
                            .child(crate::ui::assets::logo(t)),
                    )
                    .tooltip(tip(
                        tr("Commands", "Commandes"),
                        keys(&crate::actions::ToggleCommands),
                        t,
                    ))
                    .on_click(cx.listener(|this, _, window, cx| this.open_commands(window, cx))),
            )
            // Two groups, one button's height apart: the vault tree, then find and new.
            // Everything else left the rail for the palette (back/forward, trash, sync,
            // backup, theme, help all stay one `› word` away, chords unchanged).
            .child(
                div()
                    .mt(t.px(28.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    // Calm default: the rail shows the vault tree only. Recent notes and vault
                    // search stay as modes (Mode::ALL) and reopen on demand through the palette
                    // or the dock tool gate (put_tool/stack_tool/place_tool).
                    .child(mode_button(
                        "nav-tree",
                        "tree.svg",
                        Mode::Tree,
                        tr("Vault tree", "Arbre du coffre"),
                        keys(&ShowTree),
                    )),
            )
            .child(
                div()
                    .mt(t.px(28.))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .child(
                        button("nav-search", "search.svg", false, t)
                            .debug_selector(|| "nav-search".into())
                            .tooltip(tip(
                                tr("Find or create a note", "Chercher ou créer une note"),
                                keys(&crate::actions::OpenPalette),
                                t,
                            ))
                            .on_click(
                                cx.listener(|this, _, window, cx| {
                                    this.open_palette("", window, cx)
                                }),
                            ),
                    )
                    .child(
                        button("nav-new", "plus.svg", false, t)
                            .debug_selector(|| "nav-new".into())
                            .tooltip(tip(
                                tr("New note", "Nouvelle note"),
                                keys(&crate::actions::NewNote),
                                t,
                            ))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.new_note_here(window, cx)),
                            ),
                    ),
            )
            .child(
                div()
                    .debug_selector(|| "rail-free".into())
                    .flex_1()
                    .w_full()
                    .when(!self.nav.native_bar, |d| {
                        d.map(crate::ui::widgets::drag_window)
                    }),
            );

        let wrapper = div().h_full().flex().min_w_0();
        // A header dragged from the right dock: the rail of a folded panel takes it.
        let rail = rail
            .on_drag_move(cx.listener(
                |this, e: &gpui::DragMoveEvent<docks::ToolDragged>, _, cx| {
                    this.tool_drag = true;
                    if !this.left_open() {
                        let inside = e.bounds.contains(&e.event.position);
                        match inside {
                            true => this.aim_tool(Some((false, 0)), cx),
                            false if this.tool_aim == Some((false, 0)) => this.aim_tool(None, cx),
                            false => {}
                        }
                    }
                },
            ))
            .when(!unfolded, |d| {
                d.on_drop(
                    cx.listener(|this, dragged: &docks::ToolDragged, window, cx| {
                        if this.tool_aim.take() == Some((false, 0)) {
                            this.place_tool(dragged.mode, false, 0, window, cx);
                        }
                    }),
                )
                .when(self.tool_aim == Some((false, 0)), |d| {
                    d.bg(t.accent.opacity(0.12))
                })
            });
        if !unfolded {
            return wrapper.flex_none().child(rail);
        }
        let full = panel == Panel::Full;
        let content = self.render_stack(false, window, cx);
        // 5 px handle, invisible until the pointer is over it; double-click: original width.
        let divider = div()
            .id("nav-divider")
            .debug_selector(|| "nav-divider".into())
            .flex_none()
            .w(DIVIDER)
            .h_full()
            .flex()
            .justify_center()
            .cursor(CursorStyle::ResizeLeftRight)
            .group("nav-divider")
            .child(
                div()
                    .w_full()
                    .h_full()
                    .when(self.nav.dragging, |d| d.bg(t.hover))
                    .group_hover("nav-divider", |s| s.bg(t.hover)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    if e.click_count >= 2 {
                        this.nav.list_w = LIST_W;
                        this.nav.panel = Panel::Split;
                        this.settle_nav(window, cx);
                    } else {
                        this.nav.dragging = true;
                    }
                    cx.stop_propagation();
                }),
            );
        wrapper
            .map(|d| if full { d.flex_1() } else { d.flex_none() })
            .child(rail)
            .child(crate::ui::widgets::fade_in(content, "nav-in", &t))
            .child(divider)
    }

    /// The right dock, if it is open and the window leaves the note its room; while a tool's
    /// header is dragged and it is closed, a strip on the right edge to drop it there.
    pub(super) fn render_right_dock(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let t = self.theme;
        if !self.right_open() || self.right_width() <= px(0.) {
            if !self.tool_drag || self.nav.panel == Panel::Full {
                return None;
            }
            let aimed = self.tool_aim == Some((true, 0));
            return Some(
                div()
                    .id("dock-right-drop")
                    .debug_selector(|| "dock-right-drop".into())
                    .flex_none()
                    .w(t.px(48.))
                    .h_full()
                    .border_l_1()
                    .border_color(if aimed { t.accent } else { t.border })
                    .rounded(t.sheet())
                    .bg(if aimed {
                        t.accent.opacity(0.12)
                    } else {
                        t.panel
                    })
                    .on_drag_move(cx.listener(
                        |this, e: &gpui::DragMoveEvent<docks::ToolDragged>, _, cx| {
                            match e.bounds.contains(&e.event.position) {
                                true => this.aim_tool(Some((true, 0)), cx),
                                false if this.tool_aim == Some((true, 0)) => {
                                    this.aim_tool(None, cx)
                                }
                                false => {}
                            }
                        },
                    ))
                    .on_drop(
                        cx.listener(|this, dragged: &docks::ToolDragged, window, cx| {
                            if this.tool_aim.take() == Some((true, 0)) {
                                this.place_tool(dragged.mode, true, 0, window, cx);
                            }
                        }),
                    )
                    .into_any_element(),
            );
        }
        let content = self.render_stack(true, window, cx);
        let divider = div()
            .id("dock-divider")
            .debug_selector(|| "dock-divider".into())
            .flex_none()
            .w(DIVIDER)
            .h_full()
            .flex()
            .justify_center()
            .cursor(CursorStyle::ResizeLeftRight)
            .group("dock-divider")
            .child(
                div()
                    .w_full()
                    .h_full()
                    .when(self.nav.right_dragging, |d| d.bg(t.hover))
                    .group_hover("dock-divider", |s| s.bg(t.hover)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, e: &MouseDownEvent, window, cx| {
                    if e.click_count >= 2 {
                        this.nav.right_w = LIST_W;
                        this.settle_nav(window, cx);
                    } else {
                        this.nav.right_dragging = true;
                    }
                    cx.stop_propagation();
                }),
            );
        Some(
            div()
                .h_full()
                .flex()
                .flex_none()
                .child(divider)
                .child(crate::ui::widgets::fade_in(content, "dock-in", &t))
                .into_any_element(),
        )
    }

    /// The tool is drawn in a dock this frame.
    fn docked_now(&self, mode: Mode) -> bool {
        self.left_open() && self.dock_tools(false).contains(&mode)
            || self.right_open()
                && self.right_width() > px(0.)
                && self.dock_tools(true).contains(&mode)
    }

    /// A dock: its tools one above the other, each under its header, the folded ones reduced to it;
    /// a header dragged over it shows by a line where it would land.
    fn render_stack(
        &mut self,
        right: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let tools = self.dock_tools(right);
        let (count, stacked) = (tools.len(), tools.len() > 1);
        let active = self.nav.mode;
        let aim = self.tool_aim.filter(|(r, _)| *r == right).map(|(_, at)| at);
        let full = !right && self.nav.panel == Panel::Full;
        let width = if right {
            self.right_width()
        } else {
            self.split_width()
        };
        let line = move || {
            div()
                .debug_selector(|| "tool-aim".into())
                .absolute()
                .left_0()
                .right_0()
                .h(t.px(2.))
                .bg(t.accent)
        };
        let mut sections = Vec::new();
        for (at, mode) in tools.into_iter().enumerate() {
            self.nav.activate(mode);
            let section = self
                .render_dock(right, stacked, mode == active, window, cx)
                .when(aim == Some(at), |d| d.child(line().top_0()))
                .when(aim == Some(count) && at + 1 == count, |d| {
                    d.child(line().bottom_0())
                })
                .on_drag_move(cx.listener(
                    move |this, e: &gpui::DragMoveEvent<docks::ToolDragged>, _, cx| {
                        let p = e.event.position;
                        if e.bounds.contains(&p) {
                            this.aim_tool(
                                Some((right, at + usize::from(p.y > e.bounds.center().y))),
                                cx,
                            );
                        }
                    },
                ));
            sections.push(section);
        }
        self.nav.activate(active);
        // Flat on the ground, not a sheet: a hairline border instead of a card and its shadow.
        div()
            .bg(t.bg)
            .border_1()
            .border_color(t.border)
            .overflow_hidden()
            .debug_selector(move || {
                if right {
                    "dock-right".into()
                } else {
                    "nav-panel".into()
                }
            })
            .id(if right { "dock-right" } else { "nav-panel" })
            .h_full()
            .min_w_0()
            .flex()
            .flex_col()
            .map(|d| {
                if full {
                    d.flex_1()
                } else {
                    d.flex_none().w(width)
                }
            })
            .children(sections)
            .on_drag_move(cx.listener(
                move |this, e: &gpui::DragMoveEvent<docks::ToolDragged>, _, cx| {
                    this.tool_drag = true;
                    let mine = this.tool_aim.is_some_and(|(r, _)| r == right);
                    match e.bounds.contains(&e.event.position) {
                        true if !mine => this.aim_tool(Some((right, count)), cx),
                        false if mine => this.aim_tool(None, cx),
                        _ => {}
                    }
                },
            ))
            .on_drop(
                cx.listener(move |this, dragged: &docks::ToolDragged, window, cx| {
                    if let Some((_, at)) = this.tool_aim.take().filter(|(r, _)| *r == right) {
                        this.place_tool(dragged.mode, right, at, window, cx);
                    }
                }),
            )
    }

    /// Header and content of the tool whose view is in `Nav`, in the right dock or the left one;
    /// `stacked` with others, `keys`: the one that takes the keys of the panel.
    fn render_dock(
        &mut self,
        right: bool,
        stacked: bool,
        keys_here: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = self.theme;
        let (mode, panel) = (self.nav.mode, self.nav.panel);
        let shut = stacked && self.nav.folded.contains(&mode);
        let keys = |action: &dyn Action| crate::ui::keys::of(cx, action);
        let source = self.rows_source();
        if source != self.nav.rows_from {
            self.nav.rows = self.nav_rows();
            self.nav.rows_from = source;
        }
        if std::mem::take(&mut self.nav.reveal)
            && let Some(ix) = self.selected_row()
        {
            self.nav.scroll.scroll_to_item(ix, ScrollStrategy::Center);
        }
        let full = !right && panel == Panel::Full;
        // Under the window buttons: the top tool of the right dock, or of the left one on the whole window.
        let first = self.dock_tools(right).first() == Some(&mode);
        let folded = self.all_folded();
        let title = match (mode, &self.vault) {
            (Mode::Tree, Some(root)) => vault::stem(root),
            (Mode::Search, _) => tr("Search", "Recherche").to_string(),
            (Mode::Outline, _) => tr("Outline", "Plan").to_string(),
            _ => tr("Recent", "Récents").to_string(),
        };
        let header = div()
            .flex_none()
            .h(t.px(32.))
            // Under the native bar the traffic lights float over the window's left edge, rail
            // included: the header clears them from its own edge, so its title starts exactly at
            // the traffic inset from the window (TRAFFIC_ROOM past the rail), in line with every
            // other top row, instead of the whole inset twice.
            .pl(if !right && first && self.nav.native_bar {
                (t.px(TRAFFIC_ROOM) - t.px(RAIL)).max(t.px(8.))
            } else {
                t.px(8.)
            })
            // Over the whole window, the window buttons float over the right end of the header.
            .pr(if (full || right) && first && !self.nav.native_bar {
                t.px(super::render::PILL_ROOM)
            } else {
                t.px(4.)
            })
            // A narrow panel at a large scale: its buttons are cut at its edge, never drawn over the note
            // (they are in the palette too).
            .overflow_hidden()
            .flex()
            .items_center()
            .when(stacked, |d| d.border_t_1().border_color(t.border))
            // Stacked, a tool folds down to its header.
            .when(stacked, |d| {
                d.child(
                    button(
                        ("tool-fold", mode as usize),
                        if shut {
                            "chevron-right.svg"
                        } else {
                            "chevron-down.svg"
                        },
                        false,
                        t,
                    )
                    .debug_selector(move || format!("tool-fold-{}", mode.name()))
                    .tooltip(tip(
                        if shut {
                            tr("Unfold", "Déplier")
                        } else {
                            tr("Fold", "Replier")
                        },
                        String::new(),
                        t,
                    ))
                    .on_click(cx.listener(move |this, _, _, cx| this.fold_tool(mode, cx))),
                )
            })
            // The title is the handle: dragged, the tool goes to the other dock or another place in
            // its stack. The empty part of the header (and the title, on the whole window) moves the
            // window, as a title bar does.
            .child(
                div()
                    .debug_selector(move || format!("tool-head-{}", mode.name()))
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .items_center()
                    .when(!self.nav.native_bar, |d| {
                        d.map(crate::ui::widgets::drag_window)
                    })
                    .child(
                        div()
                            .id(("tool-title", mode as usize))
                            .debug_selector(move || format!("tool-title-{}", mode.name()))
                            .min_w_0()
                            .h_full()
                            .flex()
                            .items_center()
                            .truncate()
                            .when(!full, |d| {
                                d.cursor_grab().on_drag(
                                    docks::ToolDragged {
                                        mode,
                                        name: title.clone(),
                                        theme: t,
                                    },
                                    |dragged, _, _, cx| cx.new(|_| dragged.clone()),
                                )
                            })
                            // Quiet sans-semibold title (13 px) for a 32 px header: no serif at this size.
                            .map(|d| {
                                d.font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_size(t.px(13.))
                                    .text_color(t.text)
                            })
                            .child(title.clone()),
                    ),
            )
            .when(mode == Mode::Tree, |d| {
                // A note at the vault root, whatever the selection.
                d.child(
                    button("nav-file", "file-plus.svg", false, t)
                        .debug_selector(|| "nav-file".into())
                        .tooltip(tip(
                            tr("New note at the root", "Nouvelle note à la racine"),
                            String::new(),
                            t,
                        ))
                        .on_click(
                            cx.listener(|this, _, window, cx| this.new_note_in(None, window, cx)),
                        ),
                )
                .child(
                    button("nav-folder", "folder-plus.svg", false, t)
                        .debug_selector(|| "nav-folder".into())
                        .tooltip(tip(
                            tr("New folder", "Nouveau dossier"),
                            keys(&NewFolder),
                            t,
                        ))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.menu_do(Do::NewFolder, this.nav.sel.clone(), window, cx)
                        })),
                )
                // Collapse all; if all already is, expand all.
                .child(
                    button(
                        "nav-fold",
                        if folded { "unfold.svg" } else { "fold.svg" },
                        false,
                        t,
                    )
                    .debug_selector(|| "nav-fold".into())
                    .tooltip(tip(
                        if folded {
                            tr("Unfold every folder", "Déplier tous les dossiers")
                        } else {
                            tr("Fold every folder", "Replier tous les dossiers")
                        },
                        String::new(),
                        t,
                    ))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.fold_all();
                        cx.notify();
                    })),
                )
            })
            // The tool elsewhere: in the other dock. Not from the
            // panel on the whole window, where it is not in sight.
            .when(!full, |d| {
                d.child(
                    button(
                        if right {
                            "dock-move-right"
                        } else {
                            "dock-move"
                        },
                        if right { "c-left.svg" } else { "c-right.svg" },
                        false,
                        t,
                    )
                    .debug_selector(move || {
                        if right {
                            "dock-move-right".into()
                        } else {
                            "dock-move".into()
                        }
                    })
                    .tooltip(tip(
                        if right {
                            tr("Move to the left dock", "Déplacer dans le dock de gauche")
                        } else {
                            tr("Move to the right dock", "Déplacer dans le dock de droite")
                        },
                        keys(&crate::actions::MoveTool),
                        t,
                    ))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.nav.activate(mode);
                        this.move_tool(window, cx)
                    })),
                )
            })
            // No hide button in the header: the sidebar stays shown; hiding is on the keys
            // and the palette (`hide_tool` unchanged, bring the button back below to restore it).
            .when(!right && first, |d| {
                d.child(
                    button(
                        "nav-full",
                        if full { "shrink.svg" } else { "expand.svg" },
                        false,
                        t,
                    )
                    .debug_selector(|| "nav-full".into())
                    .tooltip(tip(
                        if full {
                            tr("Back beside the note", "Revenir à côté de la note")
                        } else {
                            tr("Panel on the whole window", "Panneau en pleine fenêtre")
                        },
                        keys(&ToggleFull),
                        t,
                    ))
                    .on_click(cx.listener(|this, _, window, cx| this.toggle_full(window, cx))),
                )
            });
        // A vault that cannot be read is not an empty one: whatever the mode, the panel says so, why and
        // where, and offers another vault rather than a note that could not be written.
        let unreadable = self.vault.as_ref().and_then(|root| {
            self.unreadable
                .iter()
                .find(|(dir, _)| dir == root)
                .map(|(_, why)| (root.clone(), *why))
        });
        let nothing = self.nav.rows.is_empty().then(|| {
            if let Some((root, why)) = &unreadable {
                return div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_4()
                    .text_center()
                    .text_size(t.px(13.))
                    .text_color(t.soft)
                    .child(
                        div()
                            .debug_selector(|| "nav-unreadable".into())
                            .max_w_full()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_1()
                            .child(format!(
                                "{} {}\n« {} »",
                                tr(
                                    "This vault cannot be read:",
                                    "Ce coffre ne peut pas être lu :"
                                ),
                                crate::lang::io_reason(&std::io::Error::from(*why)),
                                vault::shown(root)
                            ))
                            .child(
                                div()
                                    .id("nav-other-vault")
                                    .debug_selector(|| "nav-other-vault".into())
                                    .px_2()
                                    .py_1()
                                    .rounded(t.small())
                                    .cursor_pointer()
                                    .text_color(t.accent)
                                    .hover(|s| s.bg(t.hover))
                                    .child(tr("Open another vault…", "Ouvrir un autre coffre…"))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.choose_vault(window, cx)
                                    })),
                            ),
                    );
            }
            let said = match mode {
                Mode::Tree => match keys(&crate::actions::NewNote) {
                    key if key.is_empty() => {
                        tr("Nothing here yet", "Rien ici pour l'instant").to_string()
                    }
                    key => format!(
                        "{}\n{} {key}",
                        tr("Nothing here yet", "Rien ici pour l'instant"),
                        tr("New note:", "Nouvelle note :")
                    ),
                },
                _ => tr("No recent note", "Aucune note récente").to_string(),
            };
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .px_4()
                .text_center()
                .text_size(t.px(13.))
                .text_color(t.soft)
                .child(
                    div()
                        .debug_selector(|| "nav-empty".into())
                        .max_w_full()
                        .child(said),
                )
        });
        // Set again by the selected row if it is drawn.
        self.nav.sel_at.set(None);
        // The rows are drawn later, at layout: they bring their dock's view back for that time.
        let rows = cx.processor(move |this, range, window, cx| {
            let was = this.nav.mode;
            this.nav.activate(mode);
            let rows = this.render_rows(range, window, cx);
            this.nav.activate(was);
            rows
        });
        let list = uniform_list(("nav-rows", mode as usize), self.nav.rows.len(), rows)
            .track_scroll(self.nav.scroll.clone())
            .flex_1()
            .min_h_0()
            // Outside any row: a drop goes to the root, right-click targets the vault.
            .when(mode == Mode::Tree, |d| {
                // System files dropped outside any row: at the vault root.
                d.on_drag_move(cx.listener(
                    |this, e: &gpui::DragMoveEvent<gpui::ExternalPaths>, _, cx| {
                        let root = matches!(
                            &this.drop_aim,
                            Some(super::DropAim {
                                aim: super::Aim::Root,
                                ..
                            })
                        );
                        if !e.bounds.contains(&e.event.position) {
                            if root {
                                this.aim_drop(None, cx);
                            }
                        } else if this.drop_aim.is_none() || root {
                            let aim = super::DropAim {
                                aim: super::Aim::Root,
                                items: e.drag(cx).paths().len(),
                                pointer: e.event.position,
                            };
                            this.aim_drop(Some(aim), cx);
                        }
                    },
                ))
                .on_drop(cx.listener(|this, paths: &gpui::ExternalPaths, _, cx| {
                    this.drop_paths(paths.paths().to_vec(), super::Aim::Root, cx);
                }))
                .on_drop(cx.listener(|this, dragged: &Dragged, _, cx| {
                    if let Some(root) = this.vault.clone() {
                        this.move_all(&dragged.paths, &root, cx)
                    }
                }))
                .on_mouse_down(
                    MouseButton::Right,
                    cx.listener(|this, e: &MouseDownEvent, window, cx| {
                        this.show_menu(
                            Menu {
                                at: e.position,
                                ..Default::default()
                            },
                            window,
                            cx,
                        );
                    }),
                )
            });
        div()
            .id(("tool", mode as usize))
            .debug_selector(move || format!("tool-{}", mode.name()))
            .relative()
            .w_full()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            // Its share of the dock's height, or its header alone when folded.
            .map(|d| {
                if shut {
                    d.flex_none()
                } else {
                    d.flex_1().flex_basis(px(0.))
                }
            })
            // A press in a tool brings its view into `Nav` before any of its gestures read it.
            .capture_any_mouse_down(cx.listener(move |this, _, _, cx| {
                if this.nav.mode != mode {
                    this.nav.activate(mode);
                    cx.notify();
                }
            }))
            .key_context("Nav")
            .when(keys_here, |d| d.track_focus(&self.nav.focus))
            .on_action(cx.listener(|this, _: &Prev, window, cx| this.nav_step(false, window, cx)))
            .on_action(cx.listener(|this, _: &Next, window, cx| this.nav_step(true, window, cx)))
            .on_action(cx.listener(|this, _: &Fold, window, cx| this.nav_fold(false, window, cx)))
            .on_action(cx.listener(|this, _: &Unfold, window, cx| this.nav_fold(true, window, cx)))
            .on_action(cx.listener(|this, _: &Open, window, cx| {
                if let Some(ix) = this.selected_row() {
                    this.nav_activate(ix, window, cx)
                }
            }))
            .on_action(cx.listener(|this, _: &Rename, window, cx| {
                this.menu_do(Do::Rename, this.nav.sel.clone(), window, cx)
            })
            .on_action(cx.listener(|this, _: &Duplicate, window, cx| {
                this.menu_do(Do::Duplicate, this.nav.sel.clone(), window, cx)
            })
            .on_action(cx.listener(|this, _: &Trash, window, cx| {
                this.menu_do(Do::Trash, this.nav.sel.clone(), window, cx)
            })
            .on_action(cx.listener(|this, _: &Close, window, cx| {
                if this.menu.is_some() {
                    return this.close_menu(window, cx);
                }
                if this.icon_pick.is_some() {
                    return this.close_icons(window, cx);
                }
                if this.nav.panel == Panel::Full {
                    this.nav.panel = Panel::Split;
                }
                window.focus(&this.pane().editor.focus_handle(cx));
                this.settle_nav(window, cx);
            }))
            .child(header)
            .map(|d| match mode {
                _ if shut => d,
                Mode::Search => d.child(self.render_vault_find(window, cx)),
                Mode::Outline => d.child(self.render_outline(cx)),
                _ => d.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .relative()
                        .flex()
                        .flex_col()
                        .child(list)
                        .children(nothing),
                ),
            })
    }
}
