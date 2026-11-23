//! The tab bar of a pane: one quiet band over what it shows — transparent, no pills.
//! Each tab is bare text at 13 px; the one shown sits on a flat panel fill, with a hairline under
//! the bar. Unsaved tabs carry a small accent dot; the close cross appears on hover only.

use super::*;
use crate::ui::theme::{BAND, BAND_HIDDEN, LIGHTS_ROOM};

/// Height of the tab bar, at scale 1: the band (`app-ui.md` §20.1), the same as the sidebars' top rows.
pub(super) const TAB_BAR: f32 = BAND;
/// A tab being dragged: a small card with its name follows the pointer.
#[derive(Clone)]
pub(super) struct TabDragged {
    pub tab: usize,
    name: String,
    theme: Theme,
}

impl Render for TabDragged {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let t = self.theme;
        crate::ui::widgets::floating(t)
            .px(t.px(10.))
            .py(t.px(5.))
            .max_w(t.px(220.))
            .truncate()
            .rounded(t.tab())
            .text_size(t.px(13.))
            .text_color(t.text)
            .child(self.name.clone())
    }
}

impl Shell {
    /// What a tab is called: its file, its list page, or a new note's first line.
    pub(super) fn tab_name(&self, tab: usize, cx: &App) -> String {
        let pane = &self.panes[tab];
        // A comparison is named by what it compares: `<note> · #12 ↔ now`.
        if let Some(compare) = &pane.compare {
            return compare.title.clone();
        }
        // The Settings tab is named by what it is.
        if pane.settings.is_some() {
            return tr("Settings", "Réglages").to_string();
        }
        if let Some(listing) = &pane.listing {
            return match nav::tag_of(&listing.of) {
                Some(tag) => format!("#{tag}"),
                None => vault::stem(&listing.of),
            };
        }
        // A scratch note is named by its title: its file name is only its moment.
        if pane.scratch && pane.file.is_none() {
            return logic::scratch::title(pane.editor.read(cx).text());
        }
        if let Some(path) = pane.file_path().or(pane.path.as_ref()) {
            return vault::stem(path);
        }
        let first = pane
            .editor
            .read(cx)
            .text()
            .lines()
            .next()
            .unwrap_or_default()
            .trim_start_matches('#')
            .trim();
        match first.is_empty() {
            true => tr("New note", "Nouvelle note").to_string(),
            false => first.chars().take(80).collect(),
        }
    }

    /// What a tab shows of its label: a preview is marked by a dot, a scratch note by the word
    /// after its title (a folder reads the same way: as a second word). Quiet marks a preview in
    /// italics instead (`tab_italic`), so its title is bare. The whole caption, for the tests; the tab draws the
    /// two parts (`caption_parts`).
    #[cfg(test)]
    pub(super) fn caption(&self, tab: usize, label: &str) -> String {
        let (title, rest) = self.caption_parts(tab, label);
        format!("{title}{rest}")
    }

    /// The title of a tab and what follows it (the dot, the scratch word). A long title is cut; what follows
    /// is kept whole, so the scratch marker stays in view (C07).
    pub(super) fn caption_parts(&self, tab: usize, label: &str) -> (String, String) {
        let pane = &self.panes[tab];
        let rest = match (pane.preview, pane.scratch) {
            (true, _) => String::new(),
            (false, true) => format!(" · {}", tr("Scratch", "Brouillon")),
            (false, false) => String::new(),
        };
        (label.to_string(), rest)
    }

    /// A preview tab's title is set in italics.
    pub(super) fn tab_italic(&self, tab: usize) -> bool {
        self.panes[tab].preview
    }

    /// The focus cue on a tab, as `render_tabs` paints it: the shown tab carries a wash and
    /// the tabs of a pane not in focus dim to 55 %. Returns (washed, opacity).
    pub(super) fn tab_cue(&self, tab: usize) -> (bool, f32) {
        let focused = self.group_of(self.active) == self.group_of(tab);
        let shown = self.groups[self.group_of(tab)].shown == tab;
        (shown, if !focused { 0.55 } else { 1.0 })
    }

    /// The labels of a pane's tabs, in order; two tabs of the same name (two folders) say their folder.
    pub(super) fn tab_labels(&self, group: usize, cx: &App) -> Vec<String> {
        let tabs = &self.groups[group].tabs;
        let names: Vec<String> = tabs.iter().map(|&tab| self.tab_name(tab, cx)).collect();
        tabs.iter()
            .zip(&names)
            .map(|(&tab, name)| {
                let pane = &self.panes[tab];
                let twin = names.iter().filter(|other| *other == name).count() > 1;
                let folder = pane
                    .file_path()
                    .or(pane.path.as_ref())
                    .and_then(|p| p.parent())
                    .and_then(|p| p.file_name());
                match folder.filter(|_| twin) {
                    Some(folder) => format!("{name} · {}", folder.to_string_lossy()),
                    None => name.clone(),
                }
            })
            .collect()
    }

    /// Where a dragged tab would land in a bar; the pane under it is not aimed at meanwhile.
    fn aim_tab(&mut self, aim: Option<(usize, usize)>, cx: &mut Context<Self>) {
        if aim != self.tab_aim {
            self.tab_aim = aim;
            if aim.is_some() {
                self.pane_aim = None;
            }
            cx.notify();
        }
    }

    /// Ctrl+W or the cross of a tab: that tab closes; input stays where it was unless it was there.
    pub(super) fn close_tab(&mut self, tab: usize, window: &mut Window, cx: &mut Context<Self>) {
        let keep = (self.active != tab).then_some(self.active);
        self.active = tab;
        if !self.close_pane(window, cx) {
            return;
        }
        if let Some(keep) = keep {
            self.focus_pane(if keep > tab { keep - 1 } else { keep }, window, cx);
        }
    }

    /// "Close other tabs": the other tabs of `tab`'s pane close; it stays, shown, with input.
    pub(super) fn close_other_tabs(
        &mut self,
        tab: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let others: Vec<usize> = self.groups[self.group_of(tab)]
            .tabs
            .iter()
            .copied()
            .filter(|t| *t != tab)
            .collect();
        self.close_tabs(&others, Some(tab), window, cx);
    }

    /// "Close all tabs in this pane": the other panes keep theirs.
    pub(super) fn close_tabs_in_pane(
        &mut self,
        group: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tabs = self.groups[group].tabs.clone();
        let keep = (self.group_of(self.active) != group).then_some(self.active);
        self.close_tabs(&tabs, keep, window, cx);
    }

    /// "Close all files": every tab of every pane; one pane is left, on a new empty note.
    pub(super) fn close_all_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let all: Vec<usize> = (0..self.panes.len()).collect();
        self.close_tabs(&all, None, window, cx);
    }

    /// The left edge of a top row of `group` clear of what the window draws there: where the sidebar is
    /// hidden, the top-left pane's row starts past the sidebar toggle, and past the traffic lights on
    /// macOS (`LIGHTS_ROOM`). Zero everywhere else: an open sidebar already leaves the row its place.
    pub(super) fn strip_edge(&self, group: usize) -> Pixels {
        if !self.band_shown() {
            return px(0.);
        }
        self.band_edge(group)
    }

    /// `strip_edge` as the band of `group` takes it when drawn, whether or not the band is shown now:
    /// the peeked band of a hidden title bar starts at the same edge.
    pub(super) fn band_edge(&self, group: usize) -> Pixels {
        if self.left_open() || self.layout.panes().first() != Some(&group) {
            return px(0.);
        }
        self.theme
            .px(if self.nav.lights { LIGHTS_ROOM } else { 8. } + 32.)
    }

    /// The tab bar of `group`, drawn only for several tabs — a single page gets the whole pane,
    /// notepad++/zed style, so opening a second tab moves the note down by the bar plus its gap.
    /// (No overlay: the bar stays in the layout, and the 1↔2 shift is accepted as the price of the
    /// calm single-page view.) The bar is transparent with a hairline under it; tabs are bare
    /// text, the one shown on a flat panel fill (no pill, no shadow, no underline). One that is
    /// not saved yet carries an accent dot; the cross appears on hover only, for any tab; a
    /// preview tab is marked by a suffix, never italics.
    ///
    /// The band draws on the ground, 28 px pills of radius 8 with the shown tab on a 7 % wash, a cross
    /// of 18 px on the shown tab too, and a pane not in focus dimmed to 55 %; a preview tab is italic. `peek` draws the same bar as the overlay of a hidden title bar.
    pub(super) fn render_tabs(
        &mut self,
        group: usize,
        client: bool,
        peek: bool,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let t = self.theme;
        let tabs = self.groups[group].tabs.clone();
        // The bar is drawn for every pane while the band is shown, one tab or many (v2 #3); with the
        // title bar hidden only the peeked band draws it (the strip is the pane's top row until then).
        if self.focus_mode || !(self.band_shown() || peek) {
            return None;
        }
        let row = 28.;
        let shown = self.groups[group].shown;
        // Brought into view once when it changes, so the wheel stays free afterwards. gpui scrolls with
        // the places of the last frame: a bar not drawn yet (at startup) asks for one more frame.
        if self.groups[group].seen != Some(shown)
            && let Some(at) = tabs.iter().position(|tab| *tab == shown)
        {
            let scroll = &self.groups[group].scroll;
            scroll.scroll_to_item(at);
            if scroll.bounds_for_item(at).is_some() {
                self.groups[group].seen = Some(shown);
            } else {
                let me = cx.entity();
                cx.defer(move |cx| me.update(cx, |_, cx| cx.notify()));
            }
        }
        let labels = self.tab_labels(group, cx);
        // Sidebar hidden: the bar holds the sidebar toggle after the lights (`strip_edge`); a peeked
        // band takes the same edge while the title bar is hidden (`band_edge`).
        let edge = if peek {
            self.band_edge(group)
        } else {
            self.strip_edge(group)
        };
        let toggle = (edge > px(0.)).then(|| {
            div()
                .absolute()
                .top(t.px((TAB_BAR - 28.) / 2.))
                .left(t.px(if self.nav.lights { LIGHTS_ROOM } else { 8. }))
                .child(self.sidebar_toggle(false, cx))
        });
        // Each tab is a box and its text; the cross and the dot are drawn only where they show (the
        // tab shown, the one under the pointer), so that a long bar stays cheap to draw.
        // ponytail: every tab is laid out at each frame (about 20 µs a tab: 2 ms for a hundred, measured
        // by `hundred_tabs`); draw only those in view, or cache the bar as its own view, if that matters.
        let hovered = self.tab_hover;
        // While a tab is dragged over this bar: a line where it would land.
        let aim = self
            .tab_aim
            .filter(|(over, _)| *over == group)
            .map(|(_, at)| at);
        let count = tabs.len();
        let line = move || {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .w(t.px(2.))
                .bg(t.accent)
                .debug_selector(|| "tab-aim".into())
        };
        let cells = tabs
            .iter()
            .zip(labels)
            .enumerate()
            .map(|(at, (&tab, label))| {
                let pane = &self.panes[tab];
                let (on, over) = (tab == shown, hovered == Some(tab));
                let (wash, dim) = self.tab_cue(tab);
                // The cross shows on the shown tab too, in an 18 px box.
                let box_px = 18.;
                let slot = div()
                    .size(t.px(box_px))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center();
                let dirty = pane.dirty;
                // The tab's tooltip is its note's folder (v2 `title=` on a tab; README #4).
                let folder = pane
                    .displayed_path()
                    .and_then(|path| path.parent())
                    .map(|dir| dir.display().to_string());
                // The slot at the row's end: the close cross on hover only, for any tab; an
                // accent dot for a tab not saved yet otherwise; empty space to keep the row's
                // width steady.
                let slot = match (over || on, dirty) {
                    // The cross appears on hover only; an unsaved tab keeps its dot until then.
                    (true, _) => slot.child(
                        div()
                            .id(("tab-close", tab))
                            .debug_selector(move || format!("tab-close-{tab}"))
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(t.small())
                            .hover(|s| s.bg(t.hover))
                            // The shown tab's cross is softer than the one under the pointer (v2 `.tab.on .x`).
                            .when(on, |d| d.opacity(0.7))
                            .child(svg().path("close.svg").size(t.px(12.)).text_color(t.soft))
                            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.close_tab(tab, window, cx)
                            })),
                    ),
                    // Not saved yet: an accent dot, which turns into the cross under the pointer.
                    (false, true) => slot.child(div().size(t.px(6.)).rounded_full().bg(t.accent)),
                    (false, false) => slot,
                };
                div()
                    .id(("tab", tab))
                    .debug_selector(move || format!("tab-{tab}"))
                    .relative()
                    .flex_none()
                    .max_w(t.px(200.))
                    .when_some(folder, |d, dir| {
                        d.tooltip(ui::widgets::tip(dir, String::new(), t))
                    })
                    .h(t.px(row))
                    .pl(t.px(10.))
                    .pr(t.px(6.))
                    .rounded(t.large())
                    .flex()
                    .items_center()
                    .gap(t.px(6.))
                    .cursor_pointer()
                    .text_color(if on || over { t.text } else { t.soft })
                    // The tab shown sits on a flat panel fill — no pill, no shadow, no underline —
                    // the others are bare text that the pointer tints, on a flat hover. Quiet's
                    // shown tab is a neutral wash of the text (the sidebar's selected row).
                    .when(wash, |d| d.bg(t.text.opacity(0.072)))
                    .when(!on, |d| d.hover(|s| s.bg(t.hover)))
                    // Quiet: the pane not in focus is dimmed as a whole.
                    .when(dim < 1., |d| d.opacity(dim))
                    // A preview is marked by a dot, a scratch note by its word after the title; in
                    // Quiet, by italics.
                    .child({
                        // The title is cut first; the marker after it keeps its whole word (C07).
                        let (title, rest) = self.caption_parts(tab, &label);
                        div()
                            .flex()
                            .min_w_0()
                            .when(self.tab_italic(tab), |d| d.italic())
                            .child(div().min_w_0().truncate().child(title))
                            .when(!rest.is_empty(), |d| {
                                d.child(div().flex_none().whitespace_nowrap().child(rest))
                            })
                    })
                    .child(slot)
                    .when(aim == Some(at), |d| d.child(line().left_0()))
                    .when(aim == Some(count) && at + 1 == count, |d| {
                        d.child(line().right_0())
                    })
                    // Taken, it follows the pointer; over a tab, the half under the pointer says on
                    // which side of it it would land.
                    .on_drag(
                        TabDragged {
                            tab,
                            name: label,
                            theme: t,
                        },
                        |dragged, _, _, cx| cx.new(|_| dragged.clone()),
                    )
                    .on_drag_move(cx.listener(
                        move |this, e: &gpui::DragMoveEvent<TabDragged>, _, cx| {
                            let p = e.event.position;
                            if e.bounds.contains(&p) {
                                this.aim_tab(
                                    Some((group, at + usize::from(p.x > e.bounds.center().x))),
                                    cx,
                                );
                            }
                        },
                    ))
                    .on_hover(cx.listener(move |this, hover: &bool, _, cx| {
                        let now = if *hover {
                            Some(tab)
                        } else {
                            this.tab_hover.filter(|t| *t != tab)
                        };
                        if now != this.tab_hover {
                            this.tab_hover = now;
                            cx.notify();
                        }
                    }))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| this.focus_pane(tab, window, cx)),
                    )
                    .on_mouse_down(
                        MouseButton::Middle,
                        cx.listener(move |this, _, window, cx| this.close_tab(tab, window, cx)),
                    )
                    // Right click: what to close, this tab, the others or the whole pane.
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, e: &gpui::MouseDownEvent, window, cx| {
                            cx.stop_propagation();
                            this.show_menu(
                                nav::Menu {
                                    at: e.position,
                                    tab: Some(tab),
                                    ..Default::default()
                                },
                                window,
                                cx,
                            );
                        }),
                    )
            });
        // The window buttons sit over the top right corner: the right pane's bar stops before them,
        // so that no tab scrolls under them.
        // The band's right end keeps clear of the window buttons (client) or the right-panel toggle.
        let corner = group == self.corner_group() && (self.band_shown() || peek);
        // Where the sidebar is hidden, past the toggle (and the lights): see `strip_edge`.
        let pad = edge;
        Some(
            div()
                .id(if peek {
                    ("tabs-peek", group)
                } else {
                    ("tabs", group)
                })
                .debug_selector(move || {
                    if peek {
                        format!("tabs-peek-{group}")
                    } else {
                        format!("tabs-{group}")
                    }
                })
                .when(!peek, |d| d.relative())
                .flex_none()
                .w_full()
                .h(t.px(TAB_BAR))
                .flex()
                .text_size(t.px(13.))
                // The band is the sheet's colour (v2 `.band`), with no hairline under it. A peeked band is opaque over the note.
                .bg(t.bg)
                // Under a native title bar the traffic lights float over the window's top left:
                // the first tab starts right of them, measured from the window's edge once — the
                // bar already starts past the sidebar, so it pads only what the sidebar leaves.
                .pl(pad)
                .children(toggle)
                .when(corner, |d| {
                    // Open, the panel is beside the band's end: the toggle is there, and no window buttons.
                    d.pr(t.px(if self.nav.native_bar || self.right_dock_drawn() {
                        render::TOGGLE_ROOM
                    } else {
                        render::NOTE_ROOM
                    }))
                })
                // Past the last tab, it lands at the end; out of the bar, nowhere in it.
                .on_drag_move(cx.listener(
                    move |this, e: &gpui::DragMoveEvent<TabDragged>, _, cx| {
                        let mine = this.tab_aim.is_some_and(|(over, _)| over == group);
                        match e.bounds.contains(&e.event.position) {
                            true if !mine => this.aim_tab(Some((group, count)), cx),
                            false if mine => this.aim_tab(None, cx),
                            _ => {}
                        }
                    },
                ))
                .on_drop(cx.listener(move |this, dragged: &TabDragged, window, cx| {
                    if let Some((_, at)) = this.tab_aim.take().filter(|(over, _)| *over == group) {
                        this.place_tab(dragged.tab, pane::Place::Bar(group, at), window, cx);
                    }
                }))
                .child(
                    div()
                        .id(("tabs-row", group))
                        .debug_selector(move || format!("tabs-row-{group}"))
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .flex()
                        .items_center()
                        .gap(t.px(4.))
                        .overflow_x_scroll()
                        .track_scroll(&self.groups[group].scroll)
                        .children(cells)
                        // Past the last tab: a handle to move the window, where the app draws its title bar.
                        .child(
                            div()
                                .debug_selector(move || format!("tabs-free-{group}"))
                                .flex_1()
                                .h_full()
                                .when(client, |d| d.map(crate::ui::widgets::drag_window)),
                        ),
                ),
        )
    }

    /// Title bar hidden: the 12 px strip at the top of a pane. Pointing at it peeks the band over the note
    /// (`render_peek`), until the pointer leaves that band.
    pub(super) fn render_peek_strip(
        &self,
        group: usize,
        client: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let t = self.theme;
        div()
            .id(("band-strip", group))
            .debug_selector(move || format!("band-strip-{group}"))
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(t.px(BAND_HIDDEN))
            // Drawn by the app (Linux), the strip is the handle that moves the window. Not while the band
            // is peeked over it: the band is the pointer's then, and a press there starts no move.
            .when(client && !self.band_peek, |d| {
                d.map(crate::ui::widgets::drag_window)
            })
            .on_hover(cx.listener(move |this, hover: &bool, _, cx| {
                if *hover && !this.band_peek {
                    this.band_peek = true;
                    cx.notify();
                }
            }))
            .into_any_element()
    }

    /// The band drawn over the note while the hidden title bar is peeked: the tab row of `group`. The
    /// pointer leaving it takes the peek back.
    pub(super) fn render_peek(
        &mut self,
        group: usize,
        client: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let t = self.theme;
        let band = match self.render_tabs(group, client, true, cx) {
            Some(bar) => bar.into_any_element(),
            None => div().into_any_element(),
        };
        div()
            .id(("band-peek", group))
            .debug_selector(move || format!("band-peek-{group}"))
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(t.px(BAND))
            .bg(t.bg)
            .shadow(t.floating())
            // The band is on top: what is under it (the strip that moves the window) gets no press.
            .occlude()
            .on_hover(cx.listener(move |this, hover: &bool, _, cx| {
                if !*hover && this.band_peek {
                    this.band_peek = false;
                    cx.notify();
                }
            }))
            .child(band)
            .into_any_element()
    }
}
