//! The tab bar of a pane: one quiet 38 px strip over what it shows — transparent, no pills.
//! Each tab is bare text at 13 px; the one shown sits on a flat panel fill, with a hairline under
//! the bar. Unsaved tabs carry a small accent dot; the close cross appears on hover only.

use super::*;

/// Height of the tab bar, at scale 1: a fixed 38 px strip over the note.
pub(super) const TAB_BAR: f32 = 38.;
/// Height of a tab row in the bar, at scale 1: fits 13 px text, the dot and the hover cross.
const TAB_ROW: f32 = 30.;
/// Room the traffic lights float over at the window's top left when the system draws the title
/// bar (macOS, Windows), at scale 1: an editor-area top row clears it from the window's edge
/// once (the sidebar's header does the same past the rail), so the bar never pads it twice.
const TRAFFIC_ROOM: f32 = 78.;

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
        match pane.tool {
            Some(Mode::Graph) => return tr("Graph", "Graphe").to_string(),
            Some(_) => return tr("Calendar", "Calendrier").to_string(),
            None => {}
        }
        if let Some(listing) = &pane.listing {
            return match nav::tag_of(&listing.of) {
                Some(tag) => format!("#{tag}"),
                None => vault::stem(&listing.of),
            };
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

    /// Clearance an editor-area top row leaves on its left under a native title bar, measured
    /// from the window's edge once: the traffic lights float over the window's top left, and the
    /// row already starts past the sidebar — so what the sidebar covers is not padded again. It
    /// follows the sidebar: the rail alone when collapsed, the rail plus the open panel (at
    /// whatever width it was dragged to) when expanded; zero when the lights are not there.
    pub(super) fn traffic_pad(&self) -> Pixels {
        if !self.nav.native_bar {
            return px(0.);
        }
        let t = self.theme;
        let mut sidebar = self.nav.left + t.px(nav::RAIL);
        if self.left_open() && self.nav.panel == Panel::Split {
            sidebar += self.split_width() + nav::DIVIDER;
        }
        (t.px(TRAFFIC_ROOM) - sidebar).max(px(0.))
    }

    /// The tab bar of `group`, drawn only for several tabs — a single page gets the whole pane,
    /// notepad++/zed style, so opening a second tab moves the note down by the bar plus its gap.
    /// (No overlay: the bar stays in the layout, and the 1↔2 shift is accepted as the price of the
    /// calm single-page view.) The bar is transparent with a hairline under it; tabs are bare
    /// text, the one shown on a flat panel fill (no pill, no shadow, no underline). One that is
    /// not saved yet carries an accent dot; the cross appears on hover only, for any tab; a
    /// preview tab is marked by a suffix, never italics.
    pub(super) fn render_tabs(
        &mut self,
        group: usize,
        client: bool,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let t = self.theme;
        let tabs = self.groups[group].tabs.clone();
        if tabs.len() < 2 {
            return None;
        }
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
                let slot = div()
                    .size(t.px(14.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center();
                let dirty = pane.dirty || pane.canvas_dirty;
                // The slot at the row's end: the close cross on hover only, for any tab; an
                // accent dot for a tab not saved yet otherwise; empty space to keep the row's
                // width steady.
                let slot = match (over, dirty) {
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
                    .h(t.px(TAB_ROW))
                    .pl(t.px(8.))
                    .pr(t.px(4.))
                    .flex()
                    .items_center()
                    .gap(t.px(4.))
                    .cursor_pointer()
                    .text_color(if on || over { t.text } else { t.soft })
                    // The tab shown sits on a flat panel fill — no pill, no shadow, no underline —
                    // the others are bare text that the pointer tints, on a flat hover.
                    .when(on, |d| d.bg(t.panel))
                    .when(!on, |d| d.hover(|s| s.bg(t.hover)))
                    .child(div().min_w_0().truncate().child(match pane.preview {
                        true => format!("{label} \u{2022}"),
                        false => label.clone(),
                    }))
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
        let corner = client && group == self.corner_group();
        // From the window's edge once (see `traffic_pad`): already past the sidebar.
        let pad = self.traffic_pad();
        Some(
            div()
                .id(("tabs", group))
                .debug_selector(move || format!("tabs-{group}"))
                .flex_none()
                .w_full()
                .h(t.px(TAB_BAR))
                .flex()
                .text_size(t.px(13.))
                .border_b_1()
                .border_color(t.border)
                // Under a native title bar the traffic lights float over the window's top left:
                // the first tab starts right of them, measured from the window's edge once — the
                // bar already starts past the sidebar, so it pads only what the sidebar leaves.
                .when(self.nav.native_bar, |d| d.pl(pad))
                .when(corner, |d| d.pr(t.px(render::PILL_ROOM)))
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
                        .gap(t.px(0.))
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
}
