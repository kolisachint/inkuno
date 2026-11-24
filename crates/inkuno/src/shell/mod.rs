//! The window: it owns the panes and the views, routes their events, runs the commands, and
//! keeps the vault and the settings. One `impl Shell` per subject, in the files of
//! this folder.

mod assistant;
mod changes;
mod comments;
mod config;
mod files;
mod help;
pub(crate) mod keymap;
mod library;
mod listing;
mod menu;
mod notes;
mod palette_open;
mod pane;
mod render;
mod review;
mod revisions;
mod scratch;
mod settings;
mod settings_tab;
mod sidebar;
mod tabs;
mod toasts;
mod versions;

use help::*;
use keymap::*;
use pane::*;
use settings_tab::*;
use toasts::*;

use crate::actions::*;
use crate::lang::{date_name, tr};
use crate::logic::{
    self, commands,
    layout::{self, Node, Side, Zone},
    markdown,
    prefs::Prefs,
    recall, wiki,
};
use crate::platform::{self, clock::today, vault};
use crate::ui::keys::{self, MOD};
use crate::ui::{
    self,
    assets::{Assets, FAMILIES, apply_fonts, init_fonts, logo, mono, sans},
    components::{
        icon_button::{icon_button, icon_button_over},
        kbd::keycaps,
    },
    line,
    theme::{Page, THEMES, Theme},
    widgets::drag_window,
};
use crate::views::{editor, field, nav, palette, viewer};

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use gpui::{
    Animation, AnimationExt, App, Bounds, ClipboardItem, Context, CursorStyle, Decorations, Div,
    Entity, FocusHandle, Focusable, KeyBinding, MouseButton, MouseMoveEvent, ObjectFit,
    PathPromptOptions, Pixels, Point, ResizeEdge, Size, Stateful, Subscription, TitlebarOptions,
    Transformation, Window, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowOptions, div, img, percentage, point, prelude::*, px, size, svg,
};

use editor::{Editor, EditorEvent};

use logic::note::Note;
use markdown::Link;
use nav::{Mode, Nav, Panel};
use palette::{Entry, Palette, PaletteEvent, Setting};

/// The icon grid: where it opens, for which item, the cell the keys are on (0: no icon), and what
/// had the focus before it.
struct IconPick {
    at: Point<Pixels>,
    path: PathBuf,
    chosen: usize,
    back: Option<FocusHandle>,
}

struct Shell {
    /// Every tab of the window, shown or not; `active` is the one that last got focus.
    panes: Vec<Pane>,
    active: usize,
    /// The panes as the user sees them: one, or two side by side, each with its tabs.
    groups: Vec<Group>,
    /// Files of the tabs closed, to reopen them (most recent last).
    closed: Vec<PathBuf>,
    /// The tab under the pointer: it shows its cross.
    tab_hover: Option<usize>,
    /// How the panes (`groups`, by index) are arranged: rows and columns with their shares.
    layout: Node,
    /// The border being dragged: the split (its path in `layout`) and the child it follows.
    pane_drag: Option<(Vec<usize>, usize)>,
    /// The pane on the whole window; the others keep what they hold, out of sight.
    zoomed: Option<usize>,
    /// Space the panes take, recorded when they are drawn.
    pane_box: std::rc::Rc<std::cell::Cell<Bounds<Pixels>>>,
    /// While a tab or a file from the tree is dragged over a pane: that pane, and where in it.
    pane_aim: Option<(usize, Zone)>,
    /// While a tab is dragged over a tab bar: that pane, and the place it would take.
    tab_aim: Option<(usize, usize)>,
    /// Test: the attachments the system was asked to open.
    #[cfg(test)]
    opened: Vec<PathBuf>,
    /// Test: the HTML pages the viewer was asked to show.
    #[cfg(test)]
    viewed: Vec<PathBuf>,
    /// The vault the config points to when it is no longer there: the welcome page says so, until a vault is opened.
    lost: Option<PathBuf>,
    /// Test: the window buttons are drawn by the app, as on Linux, whatever the test platform says.
    #[cfg(test)]
    client: bool,
    /// The active pane changed without input following: the next render gives it focus.
    grab: bool,

    /// The pane arrangement as saved (file `panes`).
    panes_saved: String,
    focus: FocusHandle,
    nav: Nav,
    palette: Option<Entity<Palette>>,
    vault: Option<PathBuf>,
    notes: Vec<Note>,
    /// Vault folders, including those that contain no note.
    dirs: Vec<PathBuf>,
    /// Vault files that are not notes: images and tables (CSV, TSV).
    images: Vec<PathBuf>,
    /// Folders of the vault that could not be read at the last scan, and why; the root itself
    /// when the whole vault cannot be read.
    unreadable: vault::Unreadable,
    /// Tree context menu, if open.
    menu: Option<nav::Menu>,
    /// The open menu has the keys: arrows, Enter, Escape.
    menu_focus: FocusHandle,
    /// Scroll of the open menu's entries: a long menu scrolls, and the entry the keys choose stays in view.
    menu_scroll: gpui::ScrollHandle,
    /// Open notes, most recent first.
    recent: Vec<PathBuf>,
    save_gen: usize,
    /// Icons of the vault's notes, files and folders (see `vault::load_icons`).
    icons: std::collections::HashMap<PathBuf, String>,
    /// Icon picker grid, if open.
    icon_pick: Option<IconPick>,
    /// The Review feed of the note as the right panel lists it, built while the view is shown (`review.rs`).
    review_items: Vec<review::Item>,
    /// What `review_items` was built from, and the open items of the note (shown with the view hidden too)
    /// with what they were counted from (`refresh_review`).
    review_items_source: Option<review::Source>,
    review_total: usize,
    review_total_source: Option<review::Source>,
    /// The one card of the feed that is expanded, if any.
    review_open: Option<review::Key>,
    /// The reply being written under the expanded comment: its key, its field, and the field's events.
    review_reply: Option<(review::Key, Entity<field::Field>, Subscription)>,
    /// The comment being written, its draft card at the top of the feed (`comments.rs`).
    comment_draft: Option<comments::Draft>,
    /// Scroll of the Review feed.
    review_scroll: gpui::ScrollHandle,
    /// The author the editors write their comments by: the revisions' author, as last pushed to them.
    comment_author: String,
    /// Status messages shown, oldest to newest.
    toasts: Vec<Toast>,
    /// Number of the last message, to remove the right one when it expires.
    toasted: usize,
    title: String,
    prefs: Prefs,
    /// The last text of each config file read or written by the app: a reload of the same text is skipped.
    seen: config::Seen,
    theme: Theme,
    /// Hovered window edge (resizing without system decorations).
    edge: Option<ResizeEdge>,
    copied: bool,
    /// Focus mode (F9): the note alone, edge to edge. Not saved. Nothing of the chrome is changed
    /// to hide it, so leaving the mode shows exactly what was there.
    focus_mode: bool,
    /// The hidden title bar is peeked: the pointer is in its 12 px strip, or on the band it drew over the note.
    band_peek: bool,
    /// The macOS traffic lights' hidden state last applied to the window (`None` before the first frame).
    lights_hidden: Option<bool>,
    /// The scratch note whose close waits for its choice (Keep, Keep in vault, Discard), by its editor.
    scratch_ask: Option<Entity<Editor>>,
    /// Counts the changes of the scratch notes: the Scratch panel's rows are read again when it moves.
    scratch_rev: u64,
    /// The Versions panel's list: the versions of the active tab's note, read from disk
    /// (`versions.rs`), and whether it must be read again (a save, a proposal, the panel shown).
    versions: Option<versions::Versions>,
    versions_stale: bool,
    /// Counts the reads of the list: the panel's rows are built again when it moves.
    versions_rev: u64,
    /// The shortcuts page of the Settings tab (its search, the keys being changed).
    help: Help,
    /// The assistant's panel and its state (the process side is the seam in `assistant.rs`).
    assistant: assistant::Assistant,
    /// The bindings in effect, and what the user changed in the default shortcuts.
    bound: Vec<keys::Bind>,
    changes: keys::Changes,
}

/// The active pane is the one the palette, the tree and the shortcuts act on (`pane`,
/// `pane_mut`). What holds for every pane goes through `each_pane`, what targets one through
/// `in_pane`.
impl Shell {
    fn pane(&self) -> &Pane {
        &self.panes[self.active]
    }

    fn pane_mut(&mut self) -> &mut Pane {
        &mut self.panes[self.active]
    }
}

impl Shell {
    fn new(
        vault: Option<PathBuf>,
        recent: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let prefs = Prefs::parse(&vault::load_settings());
        apply_fonts(&prefs);
        let theme = Theme::of(&prefs, window.appearance());
        let pane = Self::new_pane(theme, Self::author_named(&prefs.author), window, cx);
        cx.observe_window_appearance(window, |this, window, cx| this.restyle(window, cx))
            .detach();
        let shell = cx.entity().downgrade();
        window.on_window_should_close(cx, move |window, cx| {
            shell
                .update(cx, |this, cx| this.may_close_window(window, cx))
                .unwrap_or(true)
        });
        cx.on_app_quit(|this, cx| {
            this.each_pane(|this| {
                this.leave(cx);
                this.drop_blank_scratch(cx);
            });
            this.assistant_halt(cx);
            async {}
        })
        .detach();

        let nav = Nav::new(cx.focus_handle());
        let assistant = assistant::Assistant::new(theme, window, cx);
        // Scratch notes left blank by a crash go; the others come back with the tabs.
        platform::scratch::clean_blank();

        let mut this = Self {
            panes: vec![pane],
            active: 0,
            groups: vec![Group::of(0)],
            closed: Vec::new(),
            tab_hover: None,
            layout: Node::default(),
            pane_drag: None,
            zoomed: None,
            pane_box: Default::default(),
            pane_aim: None,
            tab_aim: None,
            #[cfg(test)]
            opened: Vec::new(),
            #[cfg(test)]
            viewed: Vec::new(),
            lost: None,
            #[cfg(test)]
            client: false,
            grab: false,
            panes_saved: String::new(),

            focus: cx.focus_handle(),
            menu_focus: cx.focus_handle(),
            menu_scroll: gpui::ScrollHandle::new(),
            nav,
            palette: None,
            vault: None,
            notes: Vec::new(),
            dirs: Vec::new(),
            images: Vec::new(),
            unreadable: Vec::new(),
            menu: None,
            recent: Vec::new(),
            save_gen: 0,
            icons: Default::default(),
            icon_pick: None,
            // Calm UI (ui.md §12): the comments column starts collapsed to its dot;
            // the palette (`› review`) reopens it via `open_review`.
            review_items: Vec::new(),
            review_items_source: None,
            review_total: 0,
            review_total_source: None,
            review_open: None,
            review_reply: None,
            comment_draft: None,
            review_scroll: gpui::ScrollHandle::new(),
            comment_author: String::new(),
            toasts: Vec::new(),
            toasted: 0,
            title: String::new(),
            seen: config::Seen::read(),
            prefs,
            theme,
            edge: None,
            copied: false,
            focus_mode: false,
            band_peek: false,
            lights_hidden: None,
            scratch_ask: None,
            scratch_rev: 0,
            versions: None,
            versions_stale: true,
            versions_rev: 0,
            help: Help::default(),
            assistant,
            bound: defaults(),
            changes: keys::Changes::new(),
        };
        match vault {
            Some(root) => {
                this.set_vault(root.clone(), window, cx);
                this.recent = recent
                    .into_iter()
                    .filter(|p| p.is_file() && !vault::is_app_file(p))
                    .collect();
                // The tabs of last time, those whose file is still there; the older formats left
                // the first pane to the most recent note.
                // `restore=off`: the window starts with one empty pane, as if nothing was saved.
                let restore = this.prefs.restore != "off";
                let panes = vault::load_file("panes");
                let (tree, saved) = if restore {
                    read_panes(&panes)
                } else {
                    (None, Vec::new())
                };
                this.restore_panes(
                    tree,
                    saved,
                    !restore || panes.starts_with("v3 "),
                    Some(&root),
                    window,
                    cx,
                );
                this.panes_saved = this.panes_state();
                if this.nav.panel == Panel::Full {
                    window.focus(&this.nav.focus, cx);
                }
            }
            None => {
                // No vault: only the scratch notes of the last layout come back (they need none).
                let panes = vault::load_file("panes");
                let (tree, saved) = read_panes(&panes);
                let scratch = saved
                    .iter()
                    .any(|(tabs, _)| tabs.iter().any(|tab| platform::scratch::holds(tab)));
                if scratch && this.prefs.restore != "off" {
                    this.restore_panes(tree, saved, panes.starts_with("v3 "), None, window, cx);
                } else {
                    window.focus(&this.focus, cx);
                }
            }
        }
        this.watch(cx);
        this.listen(window, cx);
        this.assistant_listen(window, cx);
        this.assistant_check(cx);
        let weak = cx.weak_entity();
        this.help.key_tap = Some(cx.intercept_keystrokes(move |event, window, cx| {
            // Escape in the middle of a drag: nothing moves (before any binding takes the key).
            if event.keystroke.key == "escape" && cx.has_active_drag() {
                cx.stop_active_drag(window);
                let _ = weak.update(cx, |this, cx| {
                    (this.pane_aim, this.tab_aim) = (None, None);
                    cx.notify();
                });
                return cx.stop_propagation();
            }
            if weak
                .update(cx, |this, cx| this.key_pressed(&event.keystroke, cx))
                .unwrap_or(false)
            {
                cx.stop_propagation();
            }
        }));
        // Changed shortcuts; a faulty line of the file is dropped and reported.
        this.read_keys(cx);
        this.watch_config(window, cx);
        this
    }
}

/// Opens the main window.
pub fn run() {
    #[cfg(target_os = "linux")]
    platform::skip_absent_nvidia_driver();
    gpui_platform::application()
        .with_assets(Assets)
        .run(|cx: &mut App| {
            bind_keys(cx);
            init_fonts(cx);
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.on_window_closed(|cx, _| cx.quit()).detach();

            let (vault, recent) = vault::load_config();
            let lost = vault.clone().filter(|v| !v.is_dir());
            let vault = vault.filter(|v| v.is_dir());
            let bounds = Bounds::centered(None, size(px(860.), px(720.)), cx);
            // Zed-style: the folder first, then the app — the first frame updates it with the open
            // note, if any (see `render`).
            let title = match vault.as_ref().map(|v| vault::stem(v)) {
                Some(name) => format!("{name} — Inkuno"),
                None => "Inkuno".into(),
            };
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(title.into()),
                        // macOS: the content runs under the transparent bar, which `app::ToggleTitleBar`
                        // and focus mode hide with the traffic lights (`platform::mac_chrome`). gpui reads
                        // this at creation only, and Windows keeps its own bar.
                        appears_transparent: cfg!(target_os = "macos"),
                        ..Default::default()
                    }),
                    // Must match the .desktop file name for the desktop to associate the icon.
                    app_id: Some("com.inkuno.Inkuno".into()),
                    // On Linux the app draws its title bar. Asking for
                    // `Server` does not work on GNOME/Wayland, which provides none
                    // while gpui 0.2 still believes it is decorated. macOS and Windows
                    // ignore the request and keep their native bar.
                    window_decorations: Some(WindowDecorations::Client),
                    window_background: WindowBackgroundAppearance::Transparent,
                    window_min_size: Some(size(px(360.), px(240.))),
                    ..Default::default()
                },
                |window, cx| {
                    cx.new(|cx| {
                        let mut shell = Shell::new(vault, recent, window, cx);
                        shell.lost = lost;
                        shell
                    })
                },
            )
            .unwrap();
            cx.activate(true);
        });
}

#[cfg(test)]
mod tests;
