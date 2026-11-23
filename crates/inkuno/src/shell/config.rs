//! Live reload of the agent-editable config files (`keybindings.json`, `themes.json`,
//! `settings`): the window watches the configuration folder and applies a change while it runs.
//! A file that does not parse keeps the state in force and says so in a toast.

use super::*;
use crate::ui::theme::parse_themes;
use std::{sync::Arc, time::Duration};

impl Shell {
    /// Starts the watch of the configuration folder (see `vault::watch_config`). Not in tests:
    /// their folder is the user's own.
    pub(super) fn watch_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if cfg!(test) {
            return;
        }
        let changed = Arc::new(vault::Signal::default());
        let raised = changed.clone();
        let Some(watcher) = vault::watch_config(move || raised.raise()) else {
            return;
        };
        cx.spawn_in(window, async move |this, cx| {
            // The watcher lives as long as the task: it stops when the window closes.
            let _watcher = watcher;
            loop {
                let raised = changed
                    .wait(cx.background_executor().timer(Duration::from_secs(60)))
                    .await;
                if raised {
                    // An editor saves in several steps: the last one counts.
                    cx.background_executor()
                        .timer(Duration::from_millis(150))
                        .await;
                    changed.take();
                }
                // Each wake asks the shell whether it is still there: a closed window ends the task.
                let alive = this.update_in(cx, |this, window, cx| {
                    if raised {
                        this.reload_config(window, cx);
                    }
                });
                if alive.is_err() {
                    return;
                }
            }
        })
        .detach();
    }

    /// Writes the settings and remembers the text, so the watch does not reload the app's own write.
    pub(super) fn save_prefs(&mut self) {
        let text = self.prefs.to_text();
        vault::save_settings(&text);
        self.seen.settings = text;
    }

    /// Rereads the config files that changed since the app last read or wrote them, and applies
    /// them: the shortcuts, then the appearance (the settings and the themes). A file that does not
    /// parse is not applied, and a faulty theme leaves the others in force; each fault is a toast.
    fn reload_config(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let keys = vault::load_keybindings();
        let themes = vault::load_themes();
        let settings = vault::load_settings();
        let (keys_moved, themes_moved, settings_moved) = (
            keys != self.seen.keys,
            themes != self.seen.themes,
            settings != self.seen.settings,
        );
        if !(keys_moved || themes_moved || settings_moved) {
            return;
        }
        if keys_moved {
            self.seen.keys = keys.clone();
            if let Some(fault) = json_fault(&keys) {
                self.say(
                    Tone::Failed,
                    format!(
                        "{} {fault}",
                        tr(
                            "keybindings.json not reloaded:",
                            "keybindings.json non rechargé :"
                        )
                    ),
                    cx,
                );
            } else {
                self.read_keys(cx);
            }
        }
        if themes_moved {
            self.seen.themes = themes.clone();
            // The valid themes apply; each faulty one is named in a toast.
            if let Some(fault) = parse_themes(&themes).1.first() {
                self.say(
                    Tone::Failed,
                    format!(
                        "{} {fault}",
                        tr("themes.json not reloaded:", "themes.json non rechargé :")
                    ),
                    cx,
                );
            }
        }
        if settings_moved {
            self.seen.settings = settings.clone();
            self.prefs = Prefs::parse(&settings);
        }
        if settings_moved || themes_moved {
            self.restyle(window, cx);
        }
    }

    /// Reads the shortcuts (`keybindings.json`, else the legacy `keys` file) and installs them.
    /// A faulty line is set aside and reported in a toast; the default stays for it.
    pub(super) fn read_keys(&mut self, cx: &mut Context<Self>) {
        let open = editable();
        let (changes, faults) = keys::read(&keys::user_lines(), &defaults(), &|action| {
            open.contains(action)
        });
        self.changes = changes;
        self.rebind(cx);
        if let Some(first) = faults.first() {
            let more = if faults.len() > 1 {
                format!(" (+{})", faults.len() - 1)
            } else {
                String::new()
            };
            self.say(
                Tone::Failed,
                format!(
                    "{} {first}{more}",
                    tr(
                        "Shortcut ignored in the keys file:",
                        "Raccourci ignoré dans le fichier keys :"
                    )
                ),
                cx,
            );
        }
    }
}

/// The text of the config files at the last read or write (see `Shell::seen`).
#[derive(Default)]
pub(super) struct Seen {
    pub(super) keys: String,
    pub(super) themes: String,
    pub(super) settings: String,
}

impl Seen {
    pub(super) fn read() -> Self {
        Self {
            keys: vault::load_keybindings(),
            themes: vault::load_themes(),
            settings: vault::load_settings(),
        }
    }
}

/// The parse fault of a non-empty `keybindings.json`, if any.
fn json_fault(text: &str) -> Option<String> {
    if text.trim().is_empty() {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(text)
        .err()
        .map(|fault| fault.to_string())
}

#[cfg(test)]
mod tests {
    use super::json_fault;

    #[test]
    fn json_fault_names_only_broken_json() {
        assert_eq!(json_fault(""), None);
        assert_eq!(json_fault(r#"{"version": 1, "bindings": {}}"#), None);
        assert!(json_fault("{not json").is_some());
    }
}
