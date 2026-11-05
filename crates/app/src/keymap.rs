//! The app's key bindings in one table (task 8.5).
//!
//! Every app-level binding and every mode toggle is listed in [`bindings`]. [`init`] registers
//! the table at start-up, so a key is changed in one place. The editor's own bindings (movement,
//! editing, clipboard) stay in `md_editor` (`keys.rs`, `edit_keys.rs`), in the `MdEditor`
//! context.
//!
//! Keys. Save is Cmd+S on macOS and Ctrl+S on Linux. Source Mode is Cmd+/ or Ctrl+/. Focus Mode
//! is F8 and Typewriter Mode is F9 on both platforms. The other keys follow the same
//! Cmd-on-macOS, Ctrl-on-Linux rule. DESIGN §16 item 1 lists the mode keys as proposals.
//!
//! The unit tests check the table: no keystroke is bound twice in the same context on either
//! platform, and each mode toggle has exactly one binding.

use std::rc::Rc;

use gpui::{Action, App, KeyBinding, KeyBindingContextPredicate};
use md_editor::modes::{ToggleFocusMode, ToggleSourceMode, ToggleTypewriterMode};

use crate::file_io::Save;
use crate::file_ops_ui::{DeleteNote, NewNote, RenameNote};
use crate::search_panel::ToggleSearch;

/// One key binding: the keystroke on each platform, the action it runs, and the key context it
/// applies in (`None` for the whole window).
pub struct Binding {
    /// Keystroke on macOS, in GPUI syntax (`cmd-s`).
    pub mac: &'static str,
    /// Keystroke on Linux (`ctrl-s`).
    pub linux: &'static str,
    /// Builds the action the keystroke runs.
    pub action: fn() -> Box<dyn Action>,
    /// The key context the binding needs, such as `Sidebar`. `None` applies everywhere.
    pub context: Option<&'static str>,
}

impl Binding {
    /// The keystroke for the platform this build runs on.
    pub fn keystroke(&self) -> &'static str {
        if cfg!(target_os = "macos") {
            self.mac
        } else {
            self.linux
        }
    }
}

/// Every app-level and mode binding. Order does not matter.
pub fn bindings() -> Vec<Binding> {
    vec![
        Binding {
            mac: "cmd-s",
            linux: "ctrl-s",
            action: || Box::new(Save),
            context: None,
        },
        Binding {
            mac: "cmd-n",
            linux: "ctrl-n",
            action: || Box::new(NewNote),
            context: None,
        },
        Binding {
            mac: "f2",
            linux: "f2",
            action: || Box::new(RenameNote),
            context: None,
        },
        // Ctrl+Backspace is bound in the sidebar only, because the editor deletes a word with it.
        Binding {
            mac: "cmd-backspace",
            linux: "ctrl-backspace",
            action: || Box::new(DeleteNote),
            context: Some("Sidebar"),
        },
        Binding {
            mac: "cmd-shift-f",
            linux: "ctrl-shift-f",
            action: || Box::new(ToggleSearch),
            context: None,
        },
        Binding {
            mac: "cmd-/",
            linux: "ctrl-/",
            action: || Box::new(ToggleSourceMode),
            context: None,
        },
        Binding {
            mac: "f8",
            linux: "f8",
            action: || Box::new(ToggleFocusMode),
            context: None,
        },
        Binding {
            mac: "f9",
            linux: "f9",
            action: || Box::new(ToggleTypewriterMode),
            context: None,
        },
    ]
}

/// Registers every binding in [`bindings`] with GPUI. Called from `main`.
pub fn init(cx: &mut App) {
    let mut keys = Vec::new();
    for binding in bindings() {
        let context = binding
            .context
            .map(|ctx| Rc::new(KeyBindingContextPredicate::parse(ctx).expect("valid context")));
        let key = KeyBinding::load(
            binding.keystroke(),
            (binding.action)(),
            context,
            false,
            None,
            cx,
        )
        .expect("valid keystroke in the keymap table");
        keys.push(key);
    }
    cx.bind_keys(keys);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    type Platform = fn(&Binding) -> &'static str;

    fn mac(b: &Binding) -> &'static str {
        b.mac
    }

    fn linux(b: &Binding) -> &'static str {
        b.linux
    }

    const PLATFORMS: [(&str, Platform); 2] = [("macOS", mac), ("Linux", linux)];

    /// Every binding on one platform, as (context, keystroke, action name).
    fn rows(keystroke: Platform) -> Vec<(Option<&'static str>, &'static str, &'static str)> {
        bindings()
            .iter()
            .map(|b| (b.context, keystroke(b), (b.action)().name()))
            .collect()
    }

    /// The keystrokes bound to the action named `name` on one platform.
    fn keys_of(name: &str, keystroke: Platform) -> Vec<&'static str> {
        rows(keystroke)
            .into_iter()
            .filter(|(_, _, action)| *action == name)
            .map(|(_, key, _)| key)
            .collect()
    }

    #[test]
    fn no_keystroke_is_bound_twice_in_one_context() {
        for (platform, keystroke) in PLATFORMS {
            let mut seen = HashSet::new();
            for (context, key, action) in rows(keystroke) {
                assert!(
                    seen.insert((context, key)),
                    "{platform}: {key} is bound twice in context {context:?} (action {action})"
                );
            }
        }
    }

    #[test]
    fn each_mode_toggle_has_one_binding() {
        for name in [
            ToggleSourceMode.name(),
            ToggleFocusMode.name(),
            ToggleTypewriterMode.name(),
        ] {
            for (platform, keystroke) in PLATFORMS {
                assert_eq!(
                    keys_of(name, keystroke).len(),
                    1,
                    "{platform}: {name} should have exactly one binding"
                );
            }
        }
    }

    #[test]
    fn save_is_bound_on_both_platforms() {
        assert_eq!(keys_of(Save.name(), mac), vec!["cmd-s"]);
        assert_eq!(keys_of(Save.name(), linux), vec!["ctrl-s"]);
    }

    #[test]
    fn mode_keys_match_the_proposals() {
        assert_eq!(keys_of(ToggleSourceMode.name(), mac), vec!["cmd-/"]);
        assert_eq!(keys_of(ToggleSourceMode.name(), linux), vec!["ctrl-/"]);
        assert_eq!(keys_of(ToggleFocusMode.name(), mac), vec!["f8"]);
        assert_eq!(keys_of(ToggleFocusMode.name(), linux), vec!["f8"]);
        assert_eq!(keys_of(ToggleTypewriterMode.name(), mac), vec!["f9"]);
        assert_eq!(keys_of(ToggleTypewriterMode.name(), linux), vec!["f9"]);
    }
}
