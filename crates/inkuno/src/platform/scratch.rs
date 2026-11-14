//! Scratch notes on disk: plain Markdown files in the app's own config folder, next to the
//! layout and the keys. They are never in a vault: the vault's scan, search and links do not see
//! them, and a scratch note moves into a vault only when the person keeps it there.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use super::{clock, vault};
use crate::logic::scratch;

/// The folder of the scratch notes: `inkuno/scratch` under the config folder of the platform.
pub fn dir() -> PathBuf {
    #[cfg(test)]
    {
        if let Some(dir) = TEST_DIR.with(|dir| dir.borrow().clone()) {
            return dir;
        }
    }
    vault::config_dir().join("inkuno").join("scratch")
}

#[cfg(test)]
thread_local! {
    static TEST_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// Tests: this thread's scratch folder, whatever the config folder becomes meanwhile. The app
/// still writes at teardown, after the test has released its config folder (another test may hold
/// it by then), so the folder is pinned per test thread.
#[cfg(test)]
pub fn isolate(dir: PathBuf) {
    TEST_DIR.with(|cell| *cell.borrow_mut() = Some(dir));
}

/// Whether `path` is a scratch note (a file of the scratch folder).
pub fn holds(path: &Path) -> bool {
    path.starts_with(dir())
}

/// A free file for a new scratch note, named after the moment it is made.
pub fn new_path() -> PathBuf {
    let stem = scratch::stem(clock::today(), clock::hms());
    vault::free_path(&dir(), &stem, "md")
}

/// Writes the scratch note whole, atomically, creating the folder when it is missing.
pub fn write(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    vault::write(path, text)
}

/// The scratch notes on disk, newest first.
pub fn list() -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir()) else {
        return Vec::new();
    };
    let mut stems: Vec<String> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "md"))
        .filter_map(|path| path.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    scratch::newest_first(&mut stems);
    stems
        .into_iter()
        .map(|stem| dir().join(format!("{stem}.md")))
        .collect()
}

/// Removes the scratch notes left blank (a crash before their tab closed): nothing written is lost.
pub fn clean_blank() {
    for path in list() {
        if fs::read_to_string(&path).is_ok_and(|text| scratch::is_blank(&text)) {
            let _ = fs::remove_file(&path);
        }
    }
}
