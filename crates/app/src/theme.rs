//! Theme mode: Auto, Light or Dark (task 10.2).
//!
//! The palettes are tokens in `md_editor::theme` (task 10.1). This module chooses one of them.
//! The mode is Auto by default. Auto follows the window's appearance: the window's appearance is
//! read when the window opens, and the window's appearance observer applies a change at once, so
//! the window redraws on its next frame. The Cycle Theme key (`cmd-k cmd-t` on macOS and
//! `ctrl-k ctrl-t` on Linux, see `keymap.rs`) steps Auto, Light, Dark, Auto.
//!
//! Settings persistence is task 10.3. The mode lives in memory until then, so it resets to Auto
//! on each start.

use gpui::{actions, App, Global, Window, WindowAppearance};
use md_editor::theme::{self as tokens, Theme};

actions!(app_theme, [CycleTheme]);

/// What the user chose. Auto follows the window appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Auto,
    Light,
    Dark,
}

impl ThemeMode {
    /// The mode after this one in the cycle: Auto, Light, Dark, Auto.
    pub fn next(self) -> Self {
        match self {
            ThemeMode::Auto => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Auto,
        }
    }
}

/// The palette that is in use, once the mode is resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    Light,
    Dark,
}

/// The palette for `mode`, given the window's `appearance`. Light and Dark ignore the
/// appearance. Auto is Dark when the appearance is dark, vibrant or not.
pub fn resolve(mode: ThemeMode, appearance: WindowAppearance) -> ThemeKind {
    match mode {
        ThemeMode::Light => ThemeKind::Light,
        ThemeMode::Dark => ThemeKind::Dark,
        ThemeMode::Auto => match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => ThemeKind::Dark,
            WindowAppearance::Light | WindowAppearance::VibrantLight => ThemeKind::Light,
        },
    }
}

/// The tokens of a palette.
pub fn palette(kind: ThemeKind) -> Theme {
    match kind {
        ThemeKind::Light => Theme::light(),
        ThemeKind::Dark => Theme::dark(),
    }
}

/// The mode and the last appearance seen, as a GPUI global.
#[derive(Clone, Copy, Debug)]
struct ThemeState {
    mode: ThemeMode,
    appearance: WindowAppearance,
}

impl Global for ThemeState {}

/// Resolves the mode and makes the palette in use, then redraws the windows.
fn apply(cx: &mut App) {
    let state = *cx.global::<ThemeState>();
    let kind = resolve(state.mode, state.appearance);
    tokens::set(palette(kind), cx);
    cx.refresh_windows();
}

/// Moves to the next mode and applies it.
pub fn cycle(cx: &mut App) {
    if let Some(state) = cx.try_global::<ThemeState>().copied() {
        cx.set_global(ThemeState {
            mode: state.mode.next(),
            ..state
        });
    }
    apply(cx);
}

/// Start-up for a window: reads its appearance, applies the palette, and follows later changes
/// of the appearance. Call it once, from the window's build closure. The observer lives as long
/// as the window.
pub fn watch_window(window: &mut Window, cx: &mut App) {
    let appearance = window.appearance();
    if cx.try_global::<ThemeState>().is_none() {
        cx.set_global(ThemeState {
            mode: ThemeMode::default(),
            appearance,
        });
    }
    set_appearance(appearance, cx);
    window
        .observe_window_appearance(|window, cx| {
            set_appearance(window.appearance(), cx);
        })
        .detach();
}

/// Records a new window appearance and applies it, unless it is the same as before.
fn set_appearance(appearance: WindowAppearance, cx: &mut App) {
    let changed = cx
        .try_global::<ThemeState>()
        .is_none_or(|state| state.appearance != appearance);
    if changed {
        if let Some(state) = cx.try_global::<ThemeState>().copied() {
            cx.set_global(ThemeState {
                appearance,
                ..state
            });
        }
        apply(cx);
    }
}

/// Registers the Cycle Theme action. Called from `main`.
pub fn init(cx: &mut App) {
    cx.on_action(|_: &CycleTheme, cx| cycle(cx));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_cycles_auto_light_dark_auto() {
        assert_eq!(ThemeMode::default(), ThemeMode::Auto);
        assert_eq!(ThemeMode::Auto.next(), ThemeMode::Light);
        assert_eq!(ThemeMode::Light.next(), ThemeMode::Dark);
        assert_eq!(ThemeMode::Dark.next(), ThemeMode::Auto);
    }

    #[test]
    fn auto_follows_the_window_appearance() {
        assert_eq!(
            resolve(ThemeMode::Auto, WindowAppearance::Light),
            ThemeKind::Light
        );
        assert_eq!(
            resolve(ThemeMode::Auto, WindowAppearance::VibrantLight),
            ThemeKind::Light
        );
        assert_eq!(
            resolve(ThemeMode::Auto, WindowAppearance::Dark),
            ThemeKind::Dark
        );
        assert_eq!(
            resolve(ThemeMode::Auto, WindowAppearance::VibrantDark),
            ThemeKind::Dark
        );
    }

    #[test]
    fn manual_modes_ignore_the_window_appearance() {
        for appearance in [WindowAppearance::Light, WindowAppearance::Dark] {
            assert_eq!(resolve(ThemeMode::Light, appearance), ThemeKind::Light);
            assert_eq!(resolve(ThemeMode::Dark, appearance), ThemeKind::Dark);
        }
    }

    #[test]
    fn palettes_are_the_light_and_dark_tokens() {
        assert_eq!(palette(ThemeKind::Light), Theme::light());
        assert_eq!(palette(ThemeKind::Dark), Theme::dark());
    }
}
