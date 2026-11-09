//! The formatting bar at the bottom of the note, and the colours of highlights.

use super::*;

/// The formatting bar of a note.
#[derive(Default)]
pub(super) struct BarState {
    /// What the formatting bar offers, with the version and selection it was computed for.
    pub(super) tools: Option<((u64, Range<usize>), Tools)>,
    /// How the bar shows (see `bar_place`).
    pub(super) at: Option<Bar>,
    /// Bar collapsed into a button (narrow note): its buttons are open above it.
    pub(super) open: bool,
    /// The highlight color badges are open.
    pub(super) swatches: bool,
}

/// Width of the formatting bar: eleven 26 px buttons, the colors chevron (14 px), two
/// separators, 1 px between each of the fourteen items, 3 px of margin and the border line.
pub(super) const BAR_WIDTH: f32 = 6. + 2. + 11. * 26. + 14. + 13. + 2. * 7.;

/// Background of the button that collapses the bar: always opaque, it goes over text (the selection tint,
/// translucent, would show what it covers).
pub fn bar_fill(t: &Theme, open: bool) -> Hsla {
    if open {
        crate::ui::theme::blend(t.accent, t.bg, 0.2)
    } else {
        t.bg
    }
}

/// Where and how the formatting bar shows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Bar {
    /// In the bottom row, its left edge at this x.
    Inline(f32),
    /// Collapsed into a button, bottom middle, which opens the bar above it.
    Compact,
}

/// What the formatting bar offers for the selection.
#[derive(Clone, Copy)]
pub(super) struct Tools {
    /// The inline styles applied (`md::STYLES`).
    pub(super) styles: u16,
    /// The list kind of the cursor's line.
    pub(super) list: Option<ListStyle>,
    /// The cursor is in a table.
    pub(super) table: bool,
}

impl Editor {
    /// A narrow note gives the shell's corner button its own row: it used to meet the centered
    /// formatting toggle at width / 2 + 15 = width - 16 - 30 (the toggle is gone from the UI,
    /// the check stays for the button's placement); rem margins scale with the UI.
    pub fn crowded_footer(&self) -> bool {
        self.theme.bar
            && self.viewport.size.width > px(0.)
            && self.viewport.size.width < self.theme.px(122.)
    }

    /// The color badges are open.
    #[cfg(test)]
    pub fn swatches_open(&self) -> bool {
        self.bar.swatches
    }

    /// Highlights the selection in color `color` (see `md::MARKS`): in the same color it already has,
    /// the highlight goes; in another, it changes color; otherwise it is applied.
    pub(super) fn paint_with(&mut self, color: usize, cx: &mut Context<Self>) {
        let (open, close) = md::mark_signs(color);
        self.aligned(cx, |this, cx| {
            let parts = this.mark_parts();
            if parts.is_empty() {
                return;
            }
            let cuts: Vec<_> = parts
                .iter()
                .map(|part| this.cut(part, md::HIGHLIGHT))
                .collect();
            this.push_undo(true);
            if cuts.iter().any(Option::is_none) {
                return this.restyle_parts(&parts, &cuts, &open, close, true, cx);
            }
            let same = cuts
                .iter()
                .flatten()
                .all(|(o, _)| this.content[o.clone()] == open);
            for (o, c) in cuts.into_iter().flatten().rev() {
                this.splice(c, if same { "" } else { close }, cx);
                this.splice(o, if same { "" } else { &open }, cx);
            }
        });
    }

    #[cfg(test)]
    /// The formatting bar's buttons are on screen: never, now — the bar is gone from the main UI
    /// (its module stays for the palette), so this stays false while placement still computes.
    pub fn bar_shown(&self) -> bool {
        false
    }

    /// The note is too narrow for the bar: it collapses into a button.
    #[cfg(test)]
    pub fn bar_folded(&self) -> bool {
        self.bar.at == Some(Bar::Compact)
    }

    /// Where to put the bar: centred on the page (the text column is centred in the pane), further left when
    /// the counter and the corner buttons would touch it, collapsed into a button, at the same centre, when
    /// it no longer has room; `None` if it is off or
    /// this is not the active pane, or as long as the note's width is not known.
    // ponytail: the counter's room is estimated (7.4 px per character at 12 px, spaces and middle dots count more than letters) rather than measured.
    pub(super) fn bar_place(&mut self) -> Option<Bar> {
        let width = f32::from(self.viewport.size.width);
        if !self.theme.bar || !self.bar_here || width == 0. {
            return None;
        }
        // At the interface's scale, like the counter and the buttons around.
        let (u, bar) = (self.theme.ui, BAR_WIDTH * self.theme.ui);
        let counts = (self.counts().chars().count() as f32 * 7.4 + 16.) * u;
        let x = (width / 2. - bar / 2.)
            .min(width - (12. + 34. * self.corner as f32) * u - counts - 8. * u - bar);
        Some(if x >= 8. * u {
            Bar::Inline(x)
        } else {
            Bar::Compact
        })
    }

    /// What the bar offers for the selection, recomputed only when the text or the selection change.
    // ponytail: nothing lights up for a selection over several lines (the gesture does handle them),
    // unless handles took cells: computing it at each mouse move would weigh on a large selection.
    pub(super) fn tools(&mut self) -> Tools {
        let key = (self.version, self.sel.clone());
        if let Some((known, tools)) = &self.bar.tools
            && *known == key
        {
            return *tools;
        }
        let line = self.line_range(self.sel.start);
        let one_line = self.sel.end <= line.end;
        let parts = if one_line || self.pick_cells().is_some() {
            self.mark_parts()
        } else {
            Vec::new()
        };
        // A button lights when every span taken has the style, as the gesture would remove it.
        let styles = if parts.is_empty() || parts.len() > 2000 {
            0
        } else {
            md::STYLES
                .into_iter()
                .filter(|&flag| parts.iter().all(|part| self.cut(part, flag).is_some()))
                .fold(0, |all, flag| all | flag)
        };
        let list = match md::classify(&self.content[line], false).0 {
            Kind::Bullet => Some(ListStyle::Bullet),
            Kind::Ordered => Some(ListStyle::Ordered),
            Kind::Task(_) => Some(ListStyle::Task),
            _ => None,
        };
        let tools = Tools {
            styles,
            list,
            table: self.in_table(),
        };
        self.bar.tools = Some((key, tools));
        tools
    }

    /// The styles applied on the selection (`md::STYLES`), to light up the formatting bar's buttons.
    #[cfg(test)]
    pub fn styles(&mut self) -> u16 {
        self.tools().styles
    }

    /// The highlight color badges, above the bar; `aligned`: under the H button.
    pub(super) fn swatches(
        &mut self,
        aligned: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let t = self.theme;
        let names = [
            tr("Yellow", "Jaune"),
            tr("Green", "Vert"),
            tr("Blue", "Bleu"),
            tr("Pink", "Rose"),
            tr("Purple", "Violet"),
        ];
        div()
            .debug_selector(|| "format-swatches".into())
            .ml(t.px(if aligned { 4. + 4. * 27. } else { 0. }))
            .flex_none()
            .h(t.px(30.))
            .px(t.px(7.))
            .flex()
            .items_center()
            .gap(t.px(5.))
            .rounded(t.large())
            .border_1()
            .border_color(t.border)
            .bg(t.panel)
            .occlude()
            .children((0..md::MARKS.len()).map(|i| {
                div()
                    .id(("swatch", i))
                    .debug_selector(move || format!("swatch-{i}"))
                    .size(t.px(18.))
                    .flex_none()
                    .rounded_full()
                    .cursor_pointer()
                    .bg(crate::ui::theme::highlight_tint(i).opacity(0.8))
                    .border_1()
                    .border_color(t.border)
                    .hover(|s| s.border_color(t.accent))
                    .tooltip(crate::ui::widgets::tip(names[i], String::new(), t))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |this, _, window, cx| {
                        window.focus(&this.focus);
                        this.bar.swatches = false;
                        this.each(cx, |this, cx| this.paint_with(i, cx));
                    }))
            }))
    }

    /// The formatting bar, no longer drawn: its module stays (placement, tools and highlight
    /// colours for the palette), but the main UI shows no bar and no toggle. Stubbed so the
    /// editor's render compiles untouched while it still calls it.
    pub(super) fn format_bar(&mut self, _mode: Bar, _cx: &mut Context<Self>) -> gpui::Div {
        div()
    }

    /// The button that collapsed the bar, no longer drawn either: formatting goes through the
    /// keys and the palette. Stubbed for the same reason.
    pub(super) fn bar_toggle(&mut self, _cx: &mut Context<Self>) -> gpui::Div {
        div()
    }
}
