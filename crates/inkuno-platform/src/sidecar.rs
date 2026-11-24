//! Pending changes beside their note. A note's file holds its own text only: its pending changes
//! wait in `.inkuno/pending/<note>.json` (a scratch note: `<scratch>/.inkuno/pending/<name>.json`),
//! as a list of `{id, author, at, old, new}`. The editor shows them as marked areas (`pending.rs`):
//! `display` composes the marks from the file and the list, and `split` takes them off again on
//! save. A change the note's text does not place (an orphan) waits in the list with `orphan_since`:
//! it is shown nowhere, the pill counts it, and it is dropped at the first save after 30 days. An
//! orphan the text places again is shown, and loses its stamp at that save. A note file with marks
//! written before this (CriticMarkup) is shown as it is, and its
//! marks go to the list on the next save. Comments written in the earlier form are rewritten to
//! the new one when read (`markdown::migrate_comments`), so the editor and the file agree.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    revisions::{Home, home_of},
    vault,
};
use inkuno_logic::{
    markdown,
    pending::{self, Entry},
    sidecar,
};

/// How long an orphan waits beside its note before it is dropped on load: 30 days, in seconds.
const ORPHAN_LIFETIME: u64 = 30 * 24 * 60 * 60;

/// The time now, in seconds since the Unix epoch (0 when the clock is before it).
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

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

/// Whether an orphan stamped `since` is past its lifetime at `now` (it is dropped at the next save).
fn expired(since: Option<u64>, now: u64) -> bool {
    since.is_some_and(|t| now.saturating_sub(t) > ORPHAN_LIFETIME)
}

/// The pending changes of `note`, as they were last saved. A missing or unreadable list is none.
/// Expired orphans are read too: a change the text places again is shown, whatever its stamp.
pub fn read(home: &Home, note: &Path) -> Vec<Entry> {
    let Some(file) = file(home, note) else {
        return Vec::new();
    };
    match fs::read_to_string(file) {
        Ok(text) => sidecar::parse(&text),
        Err(_) => Vec::new(),
    }
}

/// The pending changes of `note` that its file text `disk` does not place (`pending::placed`): they
/// are shown nowhere, and wait beside the note as orphans. An expired one is not counted: the next
/// save drops it.
pub fn unplaced(home: &Home, note: &Path, disk: &str) -> Vec<Entry> {
    let known = read(home, note);
    let placed = pending::placed(&markdown::migrate_comments(disk), &known);
    let now = now_secs();
    known
        .into_iter()
        .zip(placed)
        .filter_map(|(e, placed)| (!placed && !expired(e.orphan_since, now)).then_some(e))
        .collect()
}

/// Discards the orphans of `note` (see `unplaced`): the list keeps the changes that are placed.
/// Returns how many were discarded. The note's text is not touched.
pub fn discard_unplaced(home: &Home, note: &Path, disk: &str) -> io::Result<usize> {
    let known = read(home, note);
    let placed = pending::placed(&markdown::migrate_comments(disk), &known);
    let kept: Vec<Entry> = known
        .iter()
        .zip(&placed)
        .filter(|(_, placed)| **placed)
        .map(|(e, _)| e.clone())
        .collect();
    let dropped = known.len() - kept.len();
    if dropped > 0 {
        store(home, note, &kept)?;
    }
    Ok(dropped)
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
    vault::write(&file, sidecar::render(entries))
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
/// changes take `author` when they have none. Without a home, the display is the file, marks and
/// all: there is no list beside it.
///
/// A change of the list that the note did not show when it was read (its text is not in the note
/// any more, so `display` cannot have it) is kept as it is: it waits beside the note, shown once
/// its text is back. A change that was shown, and is not in `display` now, was settled: it goes.
pub fn split(
    home: Option<&Home>,
    note: &Path,
    display: &str,
    author: &str,
) -> (String, Vec<Entry>) {
    let Some(home) = home else {
        return (display.to_string(), Vec::new());
    };
    let known = read(home, note);
    let (base, mut entries) = pending::split(display, &known);
    for entry in &mut entries {
        if entry.author.is_empty() {
            entry.author = author.to_string();
        }
    }
    // A change the list did not have is proposed now; one it had keeps its time (none in an older list).
    let now = markdown::utc_minute(now_secs() as i64);
    for entry in &mut entries {
        entry.created = match known
            .iter()
            .find(|k| k.old == entry.old && k.new == entry.new)
        {
            Some(k) => k.created.clone(),
            None => Some(now.clone()),
        };
    }
    if !known.is_empty() {
        let now = now_secs();
        // The text the editor was given: the note as it is on disk, with comments in the new form.
        let disk = fs::read_to_string(note)
            .map(|text| markdown::migrate_comments(&text))
            .unwrap_or_default();
        for (entry, shown) in known.iter().zip(pending::placed(&disk, &known)) {
            let kept = entries
                .iter()
                .any(|e| e.old == entry.old && e.new == entry.new);
            if !shown && !kept && !expired(entry.orphan_since, now) {
                // An orphan: the time it was first left out is kept, and a new one is stamped.
                let orphan_since = entry.orphan_since.unwrap_or(now);
                entries.push(Entry {
                    orphan_since: Some(orphan_since),
                    ..entry.clone()
                });
            }
        }
    }
    (base, entries)
}

/// Whether rewriting the shown text of `note` from `shown` into `fresh` would take a pending change off
/// its place: one that the text places, by its context lines, and that `fresh` no longer places. The
/// rewrite of a link on such a line is refused, as the rewrite inside a change is (`rewrite_refused`).
pub fn rewrite_unplaces(home: Option<&Home>, note: &Path, shown: &str, fresh: &str) -> bool {
    let Some(home) = home else {
        return false;
    };
    let known = read(home, note);
    if known.is_empty() {
        return false;
    }
    let placed = |text: &str| pending::placed(&pending::split(text, &known).0, &known);
    placed(shown)
        .into_iter()
        .zip(placed(fresh))
        .any(|(before, after)| before && !after)
}

/// Edits the text `path` shows, as `display` composes it, and saves the result as `save` does: the
/// pending changes keep their place. `edit` gets the shown text and returns the new one, or `None`
/// to leave the note as it is. Returns whether the note was written. The app and the command line
/// both edit notes this way.
pub fn edit(
    home: Option<&Home>,
    path: &Path,
    author: &str,
    edit: impl FnOnce(&str) -> Option<String>,
) -> io::Result<bool> {
    let disk = fs::read_to_string(path)?;
    let shown = display(home, path, &disk);
    match edit(&shown) {
        Some(next) => save(home, path, &next, author).map(|()| true),
        None => Ok(false),
    }
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

/// The folder of the pending lists of a home: a list mirrors its note's path (`<rel>.json`, or a
/// folder `<rel>` for the notes under it). A note's list is found by its path alone.
fn lists_root(home: &Home) -> PathBuf {
    match home {
        Home::Vault(root) | Home::Scratch(root) => root.join(".inkuno").join("pending"),
    }
}

/// The bin of trashed lists: a trashed note's list waits here under the name it got in the trash,
/// as its history does in the revisions bin, and comes back with it (`restore_along`).
fn bin_root(home: &Home) -> PathBuf {
    match home {
        Home::Vault(root) | Home::Scratch(root) => root.join(".inkuno").join("pending-trash"),
    }
}

/// The path of `note`'s list, without the `.json`: a file's list is `<base>.json`, a folder's is
/// `<base>`. `None` outside the home.
fn base(home: &Home, note: &Path) -> Option<PathBuf> {
    let rel = match home {
        Home::Vault(root) => note.strip_prefix(root).ok()?.to_path_buf(),
        Home::Scratch(_) => PathBuf::from(note.file_name()?),
    };
    Some(lists_root(home).join(rel))
}

/// `path` with `.json` added to its name.
fn json_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".json");
    PathBuf::from(name)
}

/// Moves `path` aside, under a free name next to it: a list in the way of a move is stale (no note
/// of that name is there to own it), so it is kept, never deleted.
fn aside(path: &Path) -> io::Result<()> {
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let target = (1..)
        .map(|n| path.with_file_name(format!("{name}.stale-{n}")))
        .find(|p| fs::symlink_metadata(p).is_err())
        .unwrap();
    fs::rename(path, target)
}

/// Moves one list (or folder of lists) from `src` to `dst`. A list already at `dst` is stale and is
/// moved aside first (`aside`). A list on another disk is copied, then removed.
fn move_one(src: &Path, dst: &Path) -> io::Result<()> {
    if !src.exists() {
        return Ok(());
    }
    if fs::symlink_metadata(dst).is_ok() {
        aside(dst)?;
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    if fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_path(src, dst)?;
    if src.is_dir() {
        fs::remove_dir_all(src)
    } else {
        fs::remove_file(src)
    }
}

/// Copies a file or a folder tree.
fn copy_path(src: &Path, dst: &Path) -> io::Result<()> {
    if src.is_dir() {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            copy_path(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        fs::copy(src, dst).map(|_| ())
    }
}

/// Moves the lists of `src` to `dst` (both the note's own and a folder's).
fn move_lists(src: &Path, dst: &Path) -> io::Result<()> {
    move_one(src, dst)?;
    move_one(&json_of(src), &json_of(dst))
}

/// Follows a note or folder that moved or was renamed: its pending lists move with it, as its
/// history does (`revisions::move_along` calls this).
pub fn move_along(vault: Option<&Path>, from: &Path, to: &Path) -> io::Result<()> {
    let (Some(hf), Some(ht)) = (home_of(from, vault), home_of(to, vault)) else {
        return Ok(());
    };
    let (Some(src), Some(dst)) = (base(&hf, from), base(&ht, to)) else {
        return Ok(());
    };
    move_lists(&src, &dst)
}

/// A trashed note or folder: its pending lists go to the bin under the name it got in the trash
/// (`trashed`). A stale entry of that name in the bin is moved aside, never deleted.
pub fn trash_along(vault: Option<&Path>, path: &Path, trashed: &Path) -> io::Result<()> {
    let (Some(home), Some(name)) = (home_of(path, vault), trashed.file_name()) else {
        return Ok(());
    };
    let Some(src) = base(&home, path) else {
        return Ok(());
    };
    let bin = bin_root(&home);
    for slot in [bin.join(name), json_of(&bin.join(name))] {
        if fs::symlink_metadata(&slot).is_ok() {
            let mut n = 1;
            let aside = loop {
                let candidate = bin.join(format!("{}.stale-{n}", name.to_string_lossy()));
                if fs::symlink_metadata(&candidate).is_err() {
                    break candidate;
                }
                n += 1;
            };
            fs::rename(&slot, &aside)?;
        }
    }
    move_lists(&src, &bin.join(name))
}

/// Undo of a trash, or a restore from it: the lists that went to the bin with `trashed` go back to
/// `to`. Nothing is done when there are none; nothing is overwritten.
pub fn restore_along(vault: Option<&Path>, trashed: &Path, to: &Path) -> io::Result<()> {
    let (Some(home), Some(name)) = (home_of(to, vault), trashed.file_name()) else {
        return Ok(());
    };
    let Some(dst) = base(&home, to) else {
        return Ok(());
    };
    move_lists(&bin_root(&home).join(name), &dst)
}

#[cfg(test)]
mod tests {
    use super::*;
    use inkuno_logic::wiki;

    #[test]
    fn without_a_home_the_marks_stay_in_the_text() {
        // A note outside the vault has no list beside it: its marks are its text, shown and saved as
        // they are, the same for the app and the command line.
        let text = "keep\n{++\nnew\n++}\n";
        let note = Path::new("/elsewhere/Note.md");
        assert_eq!(display(None, note, text), text);
        assert_eq!(
            split(None, note, text, "me"),
            (text.to_string(), Vec::new())
        );
    }

    #[test]
    fn the_lists_follow_a_move_a_trash_and_a_restore() {
        let root = std::env::temp_dir().join(format!("inkuno-sidecar-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a")).unwrap();
        let note = root.join("a").join("Plan.md");
        fs::write(&note, "# Plan\n").unwrap();
        let entry = Entry {
            created: None,
            id: 1,
            author: "assistant".into(),
            at: 7,
            old: "old\n".into(),
            new: "new\n".into(),
            before: String::new(),
            after: String::new(),
            orphan_since: None,
        };
        let home = Home::Vault(root.clone());
        store(&home, &note, std::slice::from_ref(&entry)).unwrap();
        // A rename of the folder carries the list of its notes.
        let moved = root.join("b");
        fs::rename(root.join("a"), &moved).unwrap();
        move_along(Some(&root), &root.join("a"), &moved).unwrap();
        let note = moved.join("Plan.md");
        assert_eq!(read(&home, &note), vec![entry.clone()]);
        // Trashed: the list leaves the note's place and waits in the bin under the trashed name.
        let trashed = root.join(".trash").join("Plan.md");
        trash_along(Some(&root), &note, &trashed).unwrap();
        assert!(read(&home, &note).is_empty());
        assert!(bin_root(&home).join("Plan.md.json").is_file());
        // Restored: back at the note's place.
        restore_along(Some(&root), &trashed, &note).unwrap();
        assert_eq!(read(&home, &note), vec![entry]);
        let _ = fs::remove_dir_all(root);
    }

    /// A vault folder of the test, empty.
    fn vault_dir(name: &str) -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("inkuno-sidecar-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn a_stale_list_at_the_new_name_does_not_block_a_move() {
        let root = vault_dir("stale-move");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\n").unwrap();
        let entry = Entry {
            created: None,
            id: 1,
            author: "assistant".into(),
            at: 2,
            old: String::new(),
            new: "x\n".into(),
            before: "a\n".into(),
            after: "b\n".into(),
            orphan_since: None,
        };
        store(&home, &note, std::slice::from_ref(&entry)).unwrap();
        // A list left at the new name by a note deleted outside the app.
        let other = root.join("Other.md");
        store(&home, &other, std::slice::from_ref(&entry)).unwrap();
        move_along(Some(&root), &note, &other).unwrap();
        assert_eq!(read(&home, &other), vec![entry]);
        assert!(read(&home, &note).is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_list_of_a_deleted_note_does_not_attach_to_a_new_note_of_the_same_name() {
        let root = vault_dir("new-note");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "# Plan\nold line\nend\n").unwrap();
        let marked = "# Plan\n{--\nold line\n--}\n{++\nnew line\n++}\nend\n";
        save(Some(&home), &note, marked, "assistant").unwrap();
        assert_eq!(read(&home, &note).len(), 1);
        // The note was deleted outside the app, and a new one of that name written.
        fs::remove_file(&note).unwrap();
        fs::write(&note, "# Plan\nsomething else\nend\n").unwrap();
        let shown = display(Some(&home), &note, "# Plan\nsomething else\nend\n");
        assert_eq!(shown, "# Plan\nsomething else\nend\n");
        // A new note whose first context line matches, and whose second does not: not inherited.
        fs::write(&note, "# Plan\nold line\nother\n").unwrap();
        assert_eq!(
            display(Some(&home), &note, "# Plan\nold line\nother\n"),
            "# Plan\nold line\nother\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_unplaced_change_is_kept_as_an_orphan_with_its_first_time() {
        let root = vault_dir("orphan");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nold\nb\n").unwrap();
        save(
            Some(&home),
            &note,
            "a\n{--\nold\n--}\n{++\nnew\n++}\nb\n",
            "assistant",
        )
        .unwrap();
        // The line is edited away outside the app: the change is an orphan, shown nowhere.
        fs::write(&note, "a\nz\nb\n").unwrap();
        assert_eq!(unplaced(&home, &note, "a\nz\nb\n").len(), 1);
        assert_eq!(display(Some(&home), &note, "a\nz\nb\n"), "a\nz\nb\n");
        // A save keeps it, stamped with the time it was first left out.
        let (text, kept) = split(Some(&home), &note, "a\nz\nb edited\n", "");
        assert_eq!(text, "a\nz\nb edited\n");
        assert_eq!(kept.len(), 1);
        let stamp = kept[0].orphan_since.expect("stamped");
        assert!(stamp > 0);
        store(&home, &note, &kept).unwrap();
        assert_eq!(read(&home, &note)[0].orphan_since, Some(stamp));
        // A later save keeps the first time, not the new one.
        let (_, again) = split(Some(&home), &note, "a\nz\nb\n", "");
        assert_eq!(again[0].orphan_since, Some(stamp));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_list_without_orphan_fields_reads_its_changes_as_placed() {
        let root = vault_dir("old-list");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\n").unwrap();
        let list = lists_root(&home).join("Plan.md.json");
        fs::create_dir_all(list.parent().unwrap()).unwrap();
        fs::write(
            &list,
            r#"{"version": 2, "changes": [{"id": 1, "author": "", "at": 2, "old": "", "new": "x\n", "before": "a\n", "after": "b\n"}]}"#,
        )
        .unwrap();
        let read_back = read(&home, &note);
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].orphan_since, None);
        assert!(unplaced(&home, &note, "a\nb\n").is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_list_without_created_reads_its_changes_with_no_time_and_keeps_it_so() {
        let root = vault_dir("old-created");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\n").unwrap();
        let list = lists_root(&home).join("Plan.md.json");
        fs::create_dir_all(list.parent().unwrap()).unwrap();
        fs::write(
            &list,
            r#"{"version": 2, "changes": [{"id": 1, "author": "", "at": 2, "old": "", "new": "x\n", "before": "a\n", "after": "b\n"}]}"#,
        )
        .unwrap();
        let read_back = read(&home, &note);
        assert_eq!(read_back.len(), 1);
        assert_eq!(read_back[0].created, None);
        // Stored again without a time, the key is not written.
        store(&home, &note, &read_back).unwrap();
        let text = fs::read_to_string(&list).unwrap();
        assert!(!text.contains("created"), "{text}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_new_change_is_stamped_with_its_creation_time_and_the_stamp_is_read_back() {
        let root = vault_dir("created-stamp");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\n").unwrap();
        let change = Entry {
            id: 1,
            author: "assistant".into(),
            at: 2,
            new: "x\n".into(),
            before: "a\n".into(),
            after: "b\n".into(),
            ..Entry::default()
        };
        let shown = pending::compose("a\nb\n", std::slice::from_ref(&change));
        let (base, entries) = split(Some(&home), &note, &shown, "Ana");
        assert_eq!(base, "a\nb\n");
        assert_eq!(entries.len(), 1);
        let created = entries[0].created.clone().expect("a new change has a time");
        assert_eq!(created.len(), 17, "{created}");
        store(&home, &note, &entries).unwrap();
        let read_back = read(&home, &note);
        assert_eq!(read_back[0].created.as_deref(), Some(created.as_str()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_orphan_older_than_thirty_days_is_dropped_at_the_next_save() {
        let root = vault_dir("expiry");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\n").unwrap();
        let old = Entry {
            created: None,
            id: 1,
            author: String::new(),
            at: 0,
            old: "gone\n".into(),
            new: "x\n".into(),
            before: String::new(),
            after: "a\n".into(),
            orphan_since: Some(1),
        };
        let fresh = Entry {
            id: 2,
            orphan_since: Some(now_secs() - 29 * 24 * 60 * 60),
            ..old.clone()
        };
        store(&home, &note, &[old, fresh.clone()]).unwrap();
        // Only the young orphan is counted, and only it is kept by a save.
        assert_eq!(unplaced(&home, &note, "a\nb\n"), vec![fresh.clone()]);
        assert_eq!(split(Some(&home), &note, "a\nb\n", "").1, vec![fresh]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn an_expired_orphan_the_text_places_again_is_shown_and_loses_its_stamp() {
        let root = vault_dir("placed-again");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\n").unwrap();
        save(Some(&home), &note, "a\n{++\nx\n++}\nb\n", "assistant").unwrap();
        // Stamped as an orphan long ago, then the text has its context back (a checkout, an undo).
        let mut list = read(&home, &note);
        list[0].orphan_since = Some(1);
        store(&home, &note, &list).unwrap();
        let shown = display(Some(&home), &note, "a\nb\n");
        assert_ne!(shown, "a\nb\n", "the change is shown again");
        assert!(unplaced(&home, &note, "a\nb\n").is_empty());
        // The next save settles it: placed, so it keeps no stamp.
        let (_, kept) = split(Some(&home), &note, &shown, "");
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].orphan_since, None);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn edit_saves_the_shown_text_and_keeps_the_pending_change_in_place() {
        let root = vault_dir("edit");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "# Plan\ntext\n").unwrap();
        save(
            Some(&home),
            &note,
            "# Plan\n{++\nnew\n++}\ntext\n",
            "assistant",
        )
        .unwrap();
        // A property set on the note (the list's `set_front`): the change under the heading stays placed.
        let wrote = edit(Some(&home), &note, "", |shown| {
            let new = markdown::set_front(shown, "title", "x");
            (new != shown).then_some(new)
        })
        .unwrap();
        assert!(wrote);
        let disk = fs::read_to_string(&note).unwrap();
        assert!(
            disk.starts_with("---\n") && disk.ends_with("# Plan\ntext\n"),
            "{disk}"
        );
        assert!(unplaced(&home, &note, &disk).is_empty());
        assert_eq!(read(&home, &note).len(), 1);
        // Declined by the closure: nothing is written.
        assert!(!edit(Some(&home), &note, "", |_| None).unwrap());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_heading_rename_that_would_move_a_pending_change_is_refused() {
        // The link to a renamed heading is rewritten in the shown text, unless it is the line a pending
        // change is placed by: then the rewrite is left out, and the note is not written.
        let root = vault_dir("heading");
        let home = Home::Vault(root.clone());
        let other = root.join("Other.md");
        fs::write(&other, "# Other\n\nsee [[Plan#Old]]\nend\n").unwrap();
        save(
            Some(&home),
            &other,
            "# Other\n\nsee [[Plan#Old]]\n{++\nnew\n++}\nend\n",
            "assistant",
        )
        .unwrap();
        let before = fs::read_to_string(&other).unwrap();
        let done = edit(Some(&home), &other, "", |shown| {
            let (text, count) = wiki::rename_heading(shown, "Plan", "Old", "New");
            let refused = pending::rewrite_refused(shown, &text)
                || rewrite_unplaces(Some(&home), &other, shown, &text);
            (count > 0 && !refused).then_some(text)
        })
        .unwrap();
        assert!(!done);
        assert_eq!(fs::read_to_string(&other).unwrap(), before);
        assert_eq!(read(&home, &other).len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn discard_drops_the_orphans_and_keeps_the_placed_changes() {
        let root = vault_dir("discard");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nb\nc\n").unwrap();
        let placed = Entry {
            created: None,
            id: 1,
            author: String::new(),
            at: 2,
            old: String::new(),
            new: "x\n".into(),
            before: "a\n".into(),
            after: "b\n".into(),
            orphan_since: None,
        };
        let orphan = Entry {
            id: 2,
            old: "gone\n".into(),
            new: "y\n".into(),
            before: "q\n".into(),
            after: "r\n".into(),
            orphan_since: Some(now_secs()),
            ..placed.clone()
        };
        store(&home, &note, &[placed.clone(), orphan]).unwrap();
        assert_eq!(discard_unplaced(&home, &note, "a\nb\nc\n").unwrap(), 1);
        assert_eq!(read(&home, &note), vec![placed]);
        assert_eq!(discard_unplaced(&home, &note, "a\nb\nc\n").unwrap(), 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_change_the_note_does_not_show_waits_through_a_save() {
        let root = vault_dir("waits");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nold\nb\n").unwrap();
        save(
            Some(&home),
            &note,
            "a\n{--\nold\n--}\n{++\nnew\n++}\nb\n",
            "assistant",
        )
        .unwrap();
        assert_eq!(read(&home, &note).len(), 1);
        // The line is gone outside the app: the change is not shown, and a save keeps it.
        fs::write(&note, "a\nb\nc\n").unwrap();
        assert_eq!(display(Some(&home), &note, "a\nb\nc\n"), "a\nb\nc\n");
        let (text, kept) = split(Some(&home), &note, "a\nb\nc edited\n", "");
        assert_eq!(text, "a\nb\nc edited\n");
        assert_eq!(kept.len(), 1, "{kept:?}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_settled_change_is_dropped_on_save() {
        let root = vault_dir("settled");
        let home = Home::Vault(root.clone());
        let note = root.join("Plan.md");
        fs::write(&note, "a\nold\nb\n").unwrap();
        let marked = "a\n{--\nold\n--}\n{++\nnew\n++}\nb\n";
        save(Some(&home), &note, marked, "assistant").unwrap();
        assert_eq!(read(&home, &note).len(), 1);
        // Shown, then accepted: the saved text has the new line and no change left.
        let accepted = pending::accept_all(&display(Some(&home), &note, marked));
        let (text, kept) = split(Some(&home), &note, &accepted, "");
        assert_eq!(text, "a\nnew\nb\n");
        assert!(kept.is_empty(), "{kept:?}");
        let _ = fs::remove_dir_all(root);
    }
}
