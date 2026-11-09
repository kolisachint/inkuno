//! The formatting bar at the bottom of the note, and the colours of highlights.

use super::*;

/// The formatting bar of a note.
#[derive(Default)]
pub(super) struct BarState {
    /// What the formatting bar offers, with the version and selection it was computed for.
    #[cfg(test)]
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
#[cfg(test)]
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
#[cfg(test)]
pub(super) struct Tools {
    /// The inline styles applied (`md::STYLES`).
    pub(super) styles: u16,
}

impl Editor {
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
    #[cfg(test)]
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
        let tools = Tools { styles };
        self.bar.tools = Some((key, tools));
        tools
    }

    /// The styles applied on the selection (`md::STYLES`), to light up the formatting bar's buttons.
    #[cfg(test)]
    pub fn styles(&mut self) -> u16 {
        self.tools().styles
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
