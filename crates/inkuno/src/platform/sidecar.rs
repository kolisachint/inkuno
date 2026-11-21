//! Pending changes beside their note. A note's file holds its own text only: its pending changes
//! wait in `.inkuno/pending/<note>.json` (a scratch note: `<scratch>/.inkuno/pending/<name>.json`),
//! as a list of `{id, author, at, old, new}`. The editor shows them as marked areas (`pending.rs`):
//! `display` composes the marks from the file and the list, and `split` takes them off again on
//! save. A note file with marks written before this (CriticMarkup) is shown as it is, and its
//! marks go to the list on the next save. Comments written in the earlier form are rewritten to
//! the new one when read (`markdown::migrate_comments`), so the editor and the file agree.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use super::{revisions::Home, vault};
use crate::logic::{
    markdown,
    pending::{self, Entry},
};

/// The file that holds the pending changes of `note`, `None` for a note outside its home.
fn file(home: &Home, note: &Path) -> Option<PathBuf> {
    let (root, rel) = match home {
        Home::Vault(root) => (root, note.strip_prefix(root).ok()?.to_path_buf()),
        Home::Scratch(root) => (root, PathBuf::from(note.file_name()?)),
    };
    let mut name = rel.into_os_string();
    name.push(".json");
    Some(root.join(".inkuno").join("pending").join(name))
}

/// The pending changes of `note`, as they were last saved. A missing or unreadable list is none.
pub fn read(home: &Home, note: &Path) -> Vec<Entry> {
    let Some(file) = file(home, note) else {
        return Vec::new();
    };
    let Ok(text) = fs::read_to_string(file) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    let Some(list) = value.get("changes").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|change| {
            Some(Entry {
                id: u32::try_from(change.get("id")?.as_u64()?).ok()?,
                author: change.get("author")?.as_str()?.to_string(),
                at: usize::try_from(change.get("at")?.as_u64()?).ok()?,
                old: change.get("old")?.as_str()?.to_string(),
                new: change.get("new")?.as_str()?.to_string(),
            })
        })
        .collect()
}

/// Writes the pending changes of `note`: the list, or no file when there are none.
pub fn store(home: &Home, note: &Path, entries: &[Entry]) -> io::Result<()> {
    let Some(file) = file(home, note) else {
        return Ok(());
    };
    if entries.is_empty() {
        return match fs::remove_file(&file) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let changes: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "author": e.author,
                "at": e.at,
                "old": e.old,
                "new": e.new,
            })
        })
        .collect();
    let text = serde_json::to_string_pretty(&json!({ "version": 1, "changes": changes }))
        .map_err(io::Error::other)?;
    vault::write(&file, text + "\n")
}

/// The text the editor shows for `note`, given the text on disk. Without a home (a note outside
/// the vault) and without changes, the text is shown as it is. Comments are in the new form.
pub fn display(home: Option<&Home>, note: &Path, disk: &str) -> String {
    let base = markdown::migrate_comments(disk);
    let Some(home) = home else {
        return base;
    };
    // A file with marks of its own is shown as it is: its changes are split on the next save.
    if pending::has_pending(&base) {
        return base;
    }
    match read(home, note) {
        entries if entries.is_empty() => base,
        entries => pending::compose(&base, &entries),
    }
}

/// The text of `display` as the note's file holds it, and its pending changes, for saving. New
/// changes take `author` when they have none. Without a home, the display is the file.
pub fn split(
    home: Option<&Home>,
    note: &Path,
    display: &str,
    author: &str,
) -> (String, Vec<Entry>) {
    let known = home.map(|home| read(home, note)).unwrap_or_default();
    let (base, mut entries) = pending::split(display, &known);
    for entry in &mut entries {
        if entry.author.is_empty() {
            entry.author = author.to_string();
        }
    }
    (base, entries)
}

/// Saves `display` for `note` at `path` (the file's own text, and its pending changes beside it).
/// Used by the command line and the assistant; the app's save of a note goes through `split`.
pub fn save(home: Option<&Home>, path: &Path, display: &str, author: &str) -> io::Result<()> {
    match home {
        Some(home) => {
            let (base, entries) = split(Some(home), path, display, author);
            vault::write(path, base)?;
            store(home, path, &entries)
        }
        None => vault::write(path, display),
    }
}
