//! Editor modes (phase 8). Modes are views over the same buffer (DESIGN §5).
//!
//! 8.4 adds the skeleton: the [`Modes`] flags on `Editor`, the three toggle actions, and the
//! Typewriter Mode implementation in [`typewriter`]. Only Typewriter does anything yet. Source
//! Mode (8.1) and Focus Mode (8.3) only flip their flags. Key bindings come in 8.5, so the
//! toggles are not bound to any key here.
//!
//! Debug aid until 8.5: `NOTEAPP_TYPEWRITER=1` starts the editor with Typewriter Mode on
//! ([`startup_modes`]). Remove it when 8.5 binds the key.

pub mod typewriter;

use gpui::{actions, Div, Entity};

use crate::editor::Editor;
use crate::keys;

actions!(
    md_editor_modes,
    [ToggleSourceMode, ToggleFocusMode, ToggleTypewriterMode]
);

/// The mode flags. Each mode is independent, so Focus and Typewriter can be on together.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Modes {
    pub source: bool,
    pub focus: bool,
    pub typewriter: bool,
}

/// The modes an editor starts with. All off, except Typewriter when `NOTEAPP_TYPEWRITER=1`.
pub fn startup_modes() -> Modes {
    Modes {
        typewriter: std::env::var_os("NOTEAPP_TYPEWRITER").is_some_and(|v| v.to_str() == Some("1")),
        ..Modes::default()
    }
}

impl Editor {
    /// The mode flags currently set.
    pub fn modes(&self) -> Modes {
        self.modes
    }

    /// Set the mode flags and redraw. Turning Typewriter on or off resets its scroll state, so
    /// the caret is centred again on the next frame.
    pub fn set_modes(&mut self, modes: Modes, cx: &mut gpui::Context<Self>) {
        if self.modes.typewriter != modes.typewriter {
            self.typewriter = typewriter::TypewriterState::default();
        }
        self.modes = modes;
        cx.notify();
    }
}

/// Wire the mode toggle actions to the editor root. Called from `Editor::render` (HOOK 8.4).
pub fn attach(root: Div, editor: &Entity<Editor>) -> Div {
    let root = keys::on::<ToggleSourceMode>(root, editor, |e, cx| {
        let modes = Modes {
            source: !e.modes.source,
            ..e.modes
        };
        e.set_modes(modes, cx);
    });
    let root = keys::on::<ToggleFocusMode>(root, editor, |e, cx| {
        let modes = Modes {
            focus: !e.modes.focus,
            ..e.modes
        };
        e.set_modes(modes, cx);
    });
    keys::on::<ToggleTypewriterMode>(root, editor, |e, cx| {
        let modes = Modes {
            typewriter: !e.modes.typewriter,
            ..e.modes
        };
        e.set_modes(modes, cx);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_start_off() {
        assert_eq!(
            Modes::default(),
            Modes {
                source: false,
                focus: false,
                typewriter: false
            }
        );
    }
}
