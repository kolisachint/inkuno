//! Settings persistence (task 10.3).
//!
//! The settings are one JSON file, `settings.json`, in the config directory. This is a
//! provisional decision for iteration 2 (DESIGN §16 item 9):
//!
//! - Linux: `$XDG_CONFIG_HOME/noteapp/settings.json`, or `~/.config/noteapp/settings.json` when
//!   `XDG_CONFIG_HOME` is unset, empty or not an absolute path.
//! - macOS: `~/Library/Application Support/noteapp/settings.json`.
//! - `NOTEAPP_CONFIG_DIR` overrides the directory (used by tests and manual checks).
//!
//! Loading never fails. A missing file gives the defaults. A corrupt file gives the defaults
//! for the bad fields, with a warning on stderr, and the file is left as it is: it is only
//! rewritten when the user changes a setting. Saving writes a temporary file and renames it
//! over the old one, so a crash never leaves a half-written settings file.
//!
//! The settings live in a GPUI global ([`SettingsStore`]). [`change`] updates it and saves
//! when a value really changed. Unknown fields are ignored.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use gpui::{App, Global};
use md_editor::modes::Modes;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::theme::ThemeMode;

/// The settings file name inside the config directory.
pub const FILE_NAME: &str = "settings.json";
/// The temporary file a save writes before the rename.
const TEMP_NAME: &str = "settings.json.tmp";

/// What the app remembers between runs. Source Mode is not stored: it starts off.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    /// The theme mode: Auto, Light or Dark.
    pub theme: ThemeMode,
    /// The vault that was open last. Reopened at start-up when no file is given.
    pub last_vault: Option<PathBuf>,
    /// Focus Mode is on.
    pub focus: bool,
    /// Typewriter Mode is on.
    pub typewriter: bool,
}

impl Settings {
    /// The mode flags to start a document with: Focus and Typewriter as stored, Source off.
    pub fn restored_modes(&self) -> Modes {
        Modes {
            source: false,
            focus: self.focus,
            typewriter: self.typewriter,
        }
    }
}

/// The config directory for this platform, or `None` when no home directory is known and
/// `NOTEAPP_CONFIG_DIR` is not set.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("NOTEAPP_CONFIG_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(dir));
    }
    let home = std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/noteapp"));
    }
    let xdg = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .filter(|d| d.is_absolute());
    xdg.or_else(|| home.map(|h| h.join(".config")))
        .map(|dir| dir.join("noteapp"))
}

/// The settings file inside `dir`.
pub fn settings_path(dir: &Path) -> PathBuf {
    dir.join(FILE_NAME)
}

/// Reads settings from `text`. Never fails: bad input gives defaults, with a warning for each
/// bad field. Each field is read on its own, so one bad value keeps the others.
pub fn parse(text: &str) -> Settings {
    let value: Value = match serde_json::from_str(text) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("noteapp: settings are not valid JSON, using defaults: {err}");
            return Settings::default();
        }
    };
    let Value::Object(map) = value else {
        eprintln!("noteapp: settings are not a JSON object, using defaults");
        return Settings::default();
    };
    let mut settings = Settings::default();
    read_field(&map, "theme", &mut settings.theme);
    read_field(&map, "last_vault", &mut settings.last_vault);
    read_field(&map, "focus", &mut settings.focus);
    read_field(&map, "typewriter", &mut settings.typewriter);
    settings
}

/// Sets `slot` from `map[key]` when the key is present and valid. Otherwise keeps the default.
fn read_field<T: DeserializeOwned>(map: &Map<String, Value>, key: &str, slot: &mut T) {
    let Some(value) = map.get(key) else {
        return;
    };
    match serde_json::from_value(value.clone()) {
        Ok(parsed) => *slot = parsed,
        Err(err) => eprintln!("noteapp: ignoring setting {key}: {err}"),
    }
}

/// Loads the settings in `dir`. Never fails, see [`parse`].
pub fn load(dir: &Path) -> Settings {
    match fs::read_to_string(settings_path(dir)) {
        Ok(text) => parse(&text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Settings::default(),
        Err(err) => {
            eprintln!(
                "noteapp: cannot read {}, using defaults: {err}",
                settings_path(dir).display()
            );
            Settings::default()
        }
    }
}

/// Saves `settings` in `dir`, creating the directory. The file is written under a temporary
/// name and renamed into place, so the old file survives a failed write.
pub fn save(dir: &Path, settings: &Settings) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    text.push('\n');
    let temp = dir.join(TEMP_NAME);
    let written = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()
    })();
    if let Err(err) = written {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    if let Err(err) = fs::rename(&temp, settings_path(dir)) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

/// The settings in memory, and the directory they are saved to. `dir` is `None` when there is
/// no config directory, so changes are kept for the run but not saved.
pub struct SettingsStore {
    dir: Option<PathBuf>,
    settings: Settings,
}

impl Global for SettingsStore {}

/// Loads the settings into a GPUI global. Call once at start-up, before the window opens.
pub fn init(cx: &mut App) {
    let dir = config_dir();
    let settings = dir.as_deref().map(load).unwrap_or_default();
    cx.set_global(SettingsStore { dir, settings });
}

/// The current settings. The defaults when [`init`] has not run.
pub fn current(cx: &App) -> Settings {
    cx.try_global::<SettingsStore>()
        .map(|store| store.settings.clone())
        .unwrap_or_default()
}

/// The Focus and Typewriter flags to start a document with (task 10.3).
pub fn restored_modes(cx: &App) -> Modes {
    current(cx).restored_modes()
}

/// Applies `edit` to the settings and saves them when a value changed. A failed save is
/// reported on stderr and the change is kept for the run.
pub fn change(cx: &mut App, edit: impl FnOnce(&mut Settings)) {
    if !cx.has_global::<SettingsStore>() {
        return;
    }
    let store = cx.global_mut::<SettingsStore>();
    let before = store.settings.clone();
    edit(&mut store.settings);
    if store.settings == before {
        return;
    }
    if let Some(dir) = store.dir.as_deref() {
        if let Err(err) = save(dir, &store.settings) {
            eprintln!("noteapp: cannot save settings in {}: {err}", dir.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    fn sample() -> Settings {
        Settings {
            theme: ThemeMode::Dark,
            last_vault: Some(PathBuf::from("/notes/vault")),
            focus: true,
            typewriter: true,
        }
    }

    #[test]
    fn round_trip_keeps_every_field() {
        let dir = scratch();
        save(dir.path(), &sample()).expect("save");
        assert_eq!(load(dir.path()), sample());
    }

    #[test]
    fn missing_file_gives_defaults() {
        let dir = scratch();
        assert_eq!(load(dir.path()), Settings::default());
        assert_eq!(load(&dir.path().join("not/yet/here")), Settings::default());
    }

    #[test]
    fn defaults_are_auto_and_everything_off() {
        let settings = Settings::default();
        assert_eq!(settings.theme, ThemeMode::Auto);
        assert_eq!(settings.last_vault, None);
        assert!(!settings.focus);
        assert!(!settings.typewriter);
    }

    #[test]
    fn corrupt_file_gives_defaults_and_is_not_touched() {
        let dir = scratch();
        let path = settings_path(dir.path());
        let garbage = b"{\"theme\": \"dark\", ";
        fs::write(&path, garbage).unwrap();
        assert_eq!(load(dir.path()), Settings::default());
        assert_eq!(fs::read(&path).unwrap(), garbage);
    }

    #[test]
    fn non_object_json_gives_defaults() {
        assert_eq!(parse("[1, 2, 3]"), Settings::default());
        assert_eq!(parse("\"dark\""), Settings::default());
    }

    #[test]
    fn partial_json_keeps_the_valid_fields() {
        let settings = parse(r#"{"theme": "light", "typewriter": true}"#);
        assert_eq!(settings.theme, ThemeMode::Light);
        assert!(settings.typewriter);
        assert!(!settings.focus);
        assert_eq!(settings.last_vault, None);
    }

    #[test]
    fn a_bad_field_does_not_drop_the_others() {
        let settings = parse(r#"{"theme": 42, "focus": true, "last_vault": "/v"}"#);
        assert_eq!(settings.theme, ThemeMode::Auto);
        assert!(settings.focus);
        assert_eq!(settings.last_vault, Some(PathBuf::from("/v")));
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let settings = parse(r#"{"theme": "dark", "font_size": 14, "future": {"a": [1]}}"#);
        assert_eq!(settings.theme, ThemeMode::Dark);
        assert_eq!(settings.last_vault, None);
    }

    #[test]
    fn theme_is_stored_in_lowercase() {
        let text = serde_json::to_string(&sample()).unwrap();
        assert!(text.contains(r#""theme":"dark""#), "{text}");
    }

    #[test]
    fn save_leaves_no_temp_file() {
        let dir = scratch();
        save(dir.path(), &sample()).unwrap();
        save(dir.path(), &Settings::default()).unwrap();
        let names: Vec<String> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec![FILE_NAME.to_string()]);
    }

    #[test]
    fn save_creates_the_directory() {
        let dir = scratch();
        let nested = dir.path().join("a").join("b");
        save(&nested, &sample()).unwrap();
        assert_eq!(load(&nested), sample());
    }

    #[test]
    fn restored_modes_never_turn_source_on() {
        let modes = sample().restored_modes();
        assert_eq!(
            modes,
            Modes {
                source: false,
                focus: true,
                typewriter: true
            }
        );
    }
}
