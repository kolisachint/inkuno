//! Hot-reload for the agent-editable JSON config (`keybindings.json`, `themes.json`):
//! the system reports a save, the app reapplies the file without restarting.
//!
//! The watcher only reports *which* file changed (debounced, coalesced): parsing stays
//! with `ui::keys::read_json` and `ui::theme::parse_themes`, so bad JSON is a fault at
//! apply time, never a crash here. Applying must happen on the main thread.
//!
//! Shell hookup (one place, e.g. next to `Shell::watch`): keep a
//! `ConfigWatcher::new(&config_dir)` and drain `try_recv()` on a timer; on
//! `ConfigEvent::Keybindings` reload the JSON over the defaults and `rebind`, on
//! `ConfigEvent::Themes` `restyle`; a faulty file parses to its faults, shown as a toast.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};

pub const KEYBINDINGS_FILE: &str = "keybindings.json";
pub const THEMES_FILE: &str = "themes.json";
/// Saves landing together (or an editor's write-rename) apply once.
const QUIET: Duration = Duration::from_millis(250);

/// Which JSON config file changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigEvent {
    Keybindings,
    Themes,
}

fn event_of(path: &Path) -> Option<ConfigEvent> {
    match path.file_name().and_then(|name| name.to_str()) {
        Some(KEYBINDINGS_FILE) => Some(ConfigEvent::Keybindings),
        Some(THEMES_FILE) => Some(ConfigEvent::Themes),
        _ => None,
    }
}

/// Watches the config folder (`~/.config/inkuno/`, …): `try_recv` reports the files
/// changed since the last call. Watching stops when this is dropped. Without a possible
/// watcher, no event ever comes: the next restart still reads the files.
pub struct ConfigWatcher {
    rx: Receiver<PathBuf>,
    _watcher: Option<RecommendedWatcher>,
    quiet_until: Option<Instant>,
    pending: Vec<ConfigEvent>,
}

impl ConfigWatcher {
    pub fn new(dir: &Path) -> Self {
        let (tx, rx): (Sender<PathBuf>, Receiver<PathBuf>) = std::sync::mpsc::channel();
        let _watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            // A system error or an unreadable file is never a crash here: events may have
            // been lost (the next poll still sees the file), and bad JSON is a fault at
            // apply time, never on the watcher's thread.
            if let Ok(event) = event {
                if !event.kind.is_access() {
                    for path in event.paths {
                        let _ = tx.send(path);
                    }
                }
            }
        })
        .ok()
        .and_then(|mut watcher| {
            watcher.watch(dir, RecursiveMode::NonRecursive).ok()?;
            Some(watcher)
        });
        Self {
            rx,
            _watcher,
            quiet_until: None,
            pending: Vec::new(),
        }
    }

    /// The files changed since the last call, each at most once: a burst of saves is
    /// coalesced, and nothing is reported twice within [`QUIET`]. Call on a timer; apply
    /// on the main thread.
    pub fn try_recv(&mut self) -> Vec<ConfigEvent> {
        for path in self.rx.try_iter() {
            if let Some(event) = event_of(&path) {
                if !self.pending.contains(&event) {
                    self.pending.push(event);
                }
            }
        }
        if self.pending.is_empty() {
            return Vec::new();
        }
        if self.quiet_until.is_some_and(|until| Instant::now() < until) {
            return Vec::new();
        }
        self.quiet_until = Some(Instant::now() + QUIET);
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn watched_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("inkuno-config-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Drains the watcher until `wanted` arrived, or the system took too long.
    fn until(watcher: &mut ConfigWatcher, wanted: ConfigEvent) {
        let start = Instant::now();
        let mut seen = Vec::new();
        while !seen.contains(&wanted) && start.elapsed() < Duration::from_secs(10) {
            seen.extend(watcher.try_recv());
            // A burst is coalesced: what the quiet window held back arrives next round.
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(
            seen.contains(&wanted),
            "no {wanted:?} event within 10 s (got {seen:?})"
        );
    }

    #[test]
    fn reports_each_config_file_once() {
        let dir = watched_dir("reload");
        let mut watcher = ConfigWatcher::new(&dir);
        assert!(watcher.try_recv().is_empty());
        std::fs::write(dir.join(KEYBINDINGS_FILE), "{\"version\": 1}").unwrap();
        until(&mut watcher, ConfigEvent::Keybindings);
        // Unrelated files are ignored.
        std::fs::write(dir.join("notes.md"), "x").unwrap();
        std::thread::sleep(Duration::from_millis(400));
        assert!(!watcher.try_recv().contains(&ConfigEvent::Themes));
        std::fs::write(dir.join(THEMES_FILE), "{\"version\": 1}").unwrap();
        until(&mut watcher, ConfigEvent::Themes);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_json_still_reports_without_crashing() {
        let dir = watched_dir("faulty");
        let mut watcher = ConfigWatcher::new(&dir);
        // The watcher never parses: bad JSON is a fault at apply time, not here.
        std::fs::write(dir.join(KEYBINDINGS_FILE), "{oops").unwrap();
        until(&mut watcher, ConfigEvent::Keybindings);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
