//! Revisions on disk: the files of a note's history, saved, listed, read, and moved or trashed
//! with the note. The format and the paths are in `logic::revision` (`inkuno-logic`). A vault's
//! history lives in `<vault>/.inkuno/revisions/`, a scratch folder's in `<scratch>/.revisions/`;
//! both are hidden from the tree, the search and the Scratch panel.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::scratch;
use crate::logic::revision::{self, Revision};

/// Where a note lives, which decides where its history is kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Home {
    /// A note of the vault at this root.
    Vault(PathBuf),
    /// A scratch note, in the scratch folder at this path.
    Scratch(PathBuf),
}

impl Home {
    /// The folder of the revisions of `note` (or of a folder of notes), `None` when it is not in
    /// this home.
    pub fn folder(&self, note: &Path) -> Option<PathBuf> {
        match self {
            Home::Vault(root) => revision::folder(root, revision::VAULT_DIR, note),
            Home::Scratch(root) => revision::folder(root, revision::SCRATCH_DIR, note),
        }
    }

    /// The trash of this home's revisions: a deleted note's history waits here, not lost.
    pub fn bin(&self) -> PathBuf {
        match self {
            Home::Vault(root) => revision::vault_bin(root),
            Home::Scratch(root) => revision::scratch_bin(root),
        }
    }
}

/// The home of a path the app shows: a scratch note, or a file of the open vault.
pub fn home_of(path: &Path, vault: Option<&Path>) -> Option<Home> {
    if scratch::holds(path) {
        return Some(Home::Scratch(scratch::dir()));
    }
    vault
        .filter(|root| path.starts_with(root))
        .map(|root| Home::Vault(root.to_path_buf()))
}

/// What a save did: a new revision, or nothing, the text being the latest revision's.
#[derive(Debug, PartialEq, Eq)]
pub enum Saved {
    Made { number: u32, file: PathBuf },
    Unchanged { latest: u32 },
}

/// Saves the text of `note` as a new revision by `author`, with an optional note. Refused (no file)
/// when the text is the latest revision's already. `unix` is the time of the save.
pub fn save(
    home: &Home,
    note: &Path,
    author: &str,
    memo: Option<&str>,
    text: &str,
    unix: i64,
) -> io::Result<Saved> {
    save_with(home, note, author, memo, text, unix, true)
}

/// `save`, with the publish route chosen: hard links, or (`use_links` false) the exclusive copy
/// that file systems without links take. Tests force the copy route through here.
fn save_with(
    home: &Home,
    note: &Path,
    author: &str,
    memo: Option<&str>,
    text: &str,
    unix: i64,
    use_links: bool,
) -> io::Result<Saved> {
    let folder = home
        .folder(note)
        .ok_or_else(|| io::Error::other("the note is not in its vault or scratch folder"))?;
    // Another save may take the number between the listing and the publish: the listing is then
    // read again, and the revision is numbered after the new latest. No file is overwritten.
    for attempt in 0..SAVE_TRIES {
        let numbers = numbers(&folder)?;
        let latest = numbers.iter().copied().max();
        if let Some(latest) = latest {
            let last = match read(&folder.join(revision::file_name(latest))) {
                Ok(last) => last,
                // The copy route shows its file before its bytes are all written: a racing save
                // reads it again, after a moment. A file that stays unreadable is an error.
                Err(e) if e.kind() == io::ErrorKind::InvalidData && attempt + 1 < SAVE_TRIES => {
                    std::thread::sleep(Duration::from_millis(1));
                    continue;
                }
                Err(e) => return Err(e),
            };
            if last.text == text {
                return Ok(Saved::Unchanged { latest });
            }
        }
        let number = revision::next_number(numbers);
        let file = folder.join(revision::file_name(number));
        let rev = Revision {
            number,
            parent: latest,
            author: author.to_string(),
            date: revision::rfc3339(unix),
            kind: revision::Kind::Checkpoint,
            note: memo.map(str::to_string),
            text: text.to_string(),
        };
        fs::create_dir_all(&folder)?;
        match publish_with(&file, &revision::render(&rev), use_links) {
            Ok(()) => return Ok(Saved::Made { number, file }),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "the revisions changed while saving, try again",
    ))
}

/// How many times a save lists the revisions again when another save takes its number.
const SAVE_TRIES: usize = 64;

/// Creates `file` with `text` in it, and fails (`AlreadyExists`) when it exists: a revision is
/// never overwritten. The text goes to a temporary file of its own in the same folder first, then
/// is hard-linked to `file` (or, where links are refused, copied into `file` exclusively), so two
/// saves never share a temporary file. The temporary file is removed in every case.
fn publish_with(file: &Path, text: &str, use_links: bool) -> io::Result<()> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let tmp = file.with_file_name(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .and_then(|mut out| {
            out.write_all(text.as_bytes())?;
            out.sync_all()
        });
    let linked = written.and_then(|()| link_or_copy(&tmp, file, use_links));
    let _ = fs::remove_file(&tmp);
    linked
}

/// Gives `file` the bytes of `tmp`: a hard link when `use_links` and the file system has them. A
/// failure other than `AlreadyExists` (no links on this file system, a permission, a cross-device
/// link) falls back to the exclusive copy. `AlreadyExists` is returned as it is, on both routes.
fn link_or_copy(tmp: &Path, file: &Path, use_links: bool) -> io::Result<()> {
    if use_links {
        match fs::hard_link(tmp, file) {
            Err(e) if e.kind() != io::ErrorKind::AlreadyExists => {}
            done => return done,
        }
    }
    copy_exclusive(tmp, file)
}

/// Copies `from` to `to`, which must not exist (`AlreadyExists` otherwise, and nothing is touched).
/// If the copy fails after `to` was created, that partial file is removed.
fn copy_exclusive(from: &Path, to: &Path) -> io::Result<()> {
    let mut out = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(to)?;
    let copied = fs::File::open(from)
        .and_then(|mut src| io::copy(&mut src, &mut out))
        .and_then(|_| out.sync_all());
    drop(out);
    if copied.is_err() {
        let _ = fs::remove_file(to);
    }
    copied
}

/// The time now, as Unix seconds.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// The revision numbers saved in `folder` (none when it does not exist).
fn numbers(folder: &Path) -> io::Result<Vec<u32>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    Ok(entries
        .flatten()
        .filter_map(|entry| revision::number_of(&entry.file_name().to_string_lossy()))
        .collect())
}

/// Reads one revision file.
pub fn read(file: &Path) -> io::Result<Revision> {
    let text = fs::read_to_string(file)?;
    revision::parse(&text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", file.display()),
        )
    })
}

/// Every revision of `note`, oldest first.
pub fn all(home: &Home, note: &Path) -> io::Result<Vec<Revision>> {
    let Some(folder) = home.folder(note) else {
        return Ok(Vec::new());
    };
    let mut found = Vec::new();
    for number in numbers(&folder)? {
        found.push(read(&folder.join(revision::file_name(number)))?);
    }
    found.sort_by_key(|rev| rev.number);
    Ok(found)
}

/// The latest revision of `note`, `None` when it has none yet.
pub fn latest(home: &Home, note: &Path) -> io::Result<Option<Revision>> {
    let Some(folder) = home.folder(note) else {
        return Ok(None);
    };
    match numbers(&folder)?.into_iter().max() {
        None => Ok(None),
        Some(number) => read(&folder.join(revision::file_name(number))).map(Some),
    }
}

/// Revision `number` of `note`, `NotFound` when there is none.
pub fn get(home: &Home, note: &Path, number: u32) -> io::Result<Revision> {
    let folder = home
        .folder(note)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no history"))?;
    read(&folder.join(revision::file_name(number)))
}

/// Follows a note or folder that moved or was renamed: its history folder moves to the new path.
/// Nothing is overwritten: a history already at the new path is an error and nothing moves. A
/// history on another disk is copied, then removed.
pub fn move_along(vault: Option<&Path>, from: &Path, to: &Path) -> io::Result<()> {
    let (Some(src), Some(dst)) = (
        home_of(from, vault).and_then(|h| h.folder(from)),
        home_of(to, vault).and_then(|h| h.folder(to)),
    ) else {
        return Ok(());
    };
    if !src.exists() {
        return Ok(());
    }
    if dst.exists() && !super::vault::same_file(&src, &dst) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the history of the new name already exists",
        ));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    match fs::rename(&src, &dst) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_tree(&src, &dst)?;
            fs::remove_dir_all(&src)
        }
    }
}

/// A trashed note or folder: its history goes to the home's bin under the name the item got in the
/// trash (`trashed`), so the pair is found by that name alone. The bin slot of that name is always
/// cleared first: an entry there is stale (the trash picked a free name, so no live item of that
/// name is in the trash) and it is moved aside under a free name, never deleted. Without this, undo
/// of a note that had no history could bring back another note's history.
pub fn trash_along(vault: Option<&Path>, path: &Path, trashed: &Path) -> io::Result<()> {
    let Some(home) = home_of(path, vault) else {
        return Ok(());
    };
    let Some(name) = trashed.file_name() else {
        return Ok(());
    };
    let bin = home.bin();
    let slot = bin.join(name);
    if fs::symlink_metadata(&slot).is_ok() {
        let aside = free_in(&bin, &name.to_string_lossy());
        fs::rename(&slot, &aside)?;
    }
    let Some(src) = home.folder(path) else {
        return Ok(());
    };
    if !src.exists() {
        return Ok(());
    }
    fs::create_dir_all(&bin)?;
    fs::rename(&src, &slot)
}

/// The first free name `dir/name`, `dir/2 name`, `dir/3 name`… (the one the bins use).
fn free_in(dir: &Path, name: &str) -> PathBuf {
    (1..)
        .map(|n| match n {
            1 => dir.join(name),
            n => dir.join(format!("{n} {name}")),
        })
        .find(|p| fs::symlink_metadata(p).is_err())
        .unwrap()
}

/// Undo of a trash, or a restore from it: the history that went to the bin with `trashed` goes back
/// to `to`. Only the bin entry of that exact name is used, the one `trash_along` filled for that
/// item. Nothing is done when there is no such history; nothing is overwritten.
pub fn restore_along(vault: Option<&Path>, trashed: &Path, to: &Path) -> io::Result<()> {
    let Some(home) = home_of(to, vault) else {
        return Ok(());
    };
    let (Some(dst), Some(name)) = (home.folder(to), trashed.file_name()) else {
        return Ok(());
    };
    let src = home.bin().join(name);
    if !src.exists() {
        return Ok(());
    }
    if dst.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "the history of that name already exists",
        ));
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&src, &dst)
}

/// Copies a folder tree (the history: files, and one folder per note).
fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type()?.is_dir() {
            copy_tree(&src, &dst)?;
        } else {
            fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::vault;

    fn temp(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("inkuno-rev-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn saves_numbered_revisions_and_refuses_an_unchanged_text() {
        let root = temp("save");
        let home = Home::Vault(root.clone());
        let note = root.join("ideas").join("plan.md");
        let first = save(&home, &note, "ana", None, "# Plan\n", 1_700_000_000).unwrap();
        let Saved::Made { number, file } = first else {
            panic!("first save");
        };
        assert_eq!(number, 1);
        assert_eq!(file, root.join(".inkuno/revisions/ideas/plan.md/0001.md"));
        assert_eq!(
            save(&home, &note, "ana", None, "# Plan\n", 1_700_000_100).unwrap(),
            Saved::Unchanged { latest: 1 }
        );
        let second = save(
            &home,
            &note,
            "bo",
            Some("kept it"),
            "# Plan 2\n",
            1_700_000_200,
        )
        .unwrap();
        assert_eq!(
            second,
            Saved::Made {
                number: 2,
                file: root.join(".inkuno/revisions/ideas/plan.md/0002.md")
            }
        );
        let all = all(&home, &note).unwrap();
        assert_eq!(all.iter().map(|r| r.number).collect::<Vec<_>>(), [1, 2]);
        assert_eq!(all[1].parent, Some(1));
        assert_eq!(all[1].author, "bo");
        assert_eq!(all[1].note.as_deref(), Some("kept it"));
        assert_eq!(all[1].date, "2023-11-14T22:16:40Z");
        assert_eq!(all[0].text, "# Plan\n");
        assert_eq!(get(&home, &note, 2).unwrap().text, "# Plan 2\n");
        assert_eq!(
            get(&home, &note, 9).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        // The vault's own files are not touched by the history.
        assert!(!root.join("ideas").join(".inkuno").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn racing_saves_each_get_their_own_revision() {
        racing_saves(true);
    }

    #[test]
    fn racing_saves_on_the_copy_route_each_get_their_own_revision() {
        racing_saves(false);
    }

    /// Eight writers racing `save_with` on one note, on the link route or the copy route.
    fn racing_saves(use_links: bool) {
        let root = temp(if use_links { "race" } else { "race-copy" });
        let home = Home::Vault(root.clone());
        let note = root.join("plan.md");
        let writers: Vec<_> = (0..8)
            .map(|t| {
                let (home, note) = (home.clone(), note.clone());
                std::thread::spawn(move || {
                    (0..12)
                        .map(|i| {
                            let text = format!("writer {t}, save {i}\n");
                            match save_with(&home, &note, "ana", None, &text, 0, use_links) {
                                Ok(Saved::Made { number, file }) => (number, file, text),
                                other => panic!("a save failed: {other:?}"),
                            }
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let mut made: Vec<(u32, PathBuf, String)> = writers
            .into_iter()
            .flat_map(|w| w.join().unwrap())
            .collect();
        made.sort_by_key(|m| m.0);
        // Every save got its own number, 1 to 96, and every file holds the text its writer saved.
        assert_eq!(
            made.iter().map(|m| m.0).collect::<Vec<_>>(),
            (1..=96).collect::<Vec<u32>>()
        );
        for (number, file, text) in &made {
            let rev = read(file).unwrap();
            assert_eq!((rev.number, &rev.text), (*number, text));
        }
        // The folder holds exactly those revisions: no temporary file is left behind.
        let folder = root.join(".inkuno/revisions/plan.md");
        let names: Vec<String> = fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 96, "{names:?}");
        assert!(names.iter().all(|n| revision::number_of(n).is_some()));
        assert_eq!(all(&home, &note).unwrap().len(), 96);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn publishing_never_overwrites_on_either_route() {
        let root = temp("publish");
        for use_links in [true, false] {
            let file = root.join(format!("{use_links}.md"));
            publish_with(&file, "one\n", use_links).unwrap();
            assert_eq!(fs::read_to_string(&file).unwrap(), "one\n");
            let taken = publish_with(&file, "two\n", use_links).unwrap_err();
            assert_eq!(taken.kind(), io::ErrorKind::AlreadyExists);
            assert_eq!(fs::read_to_string(&file).unwrap(), "one\n");
        }
        // A copy that fails leaves no partial revision behind.
        let file = root.join("partial.md");
        assert!(copy_exclusive(&root.join("missing.tmp"), &file).is_err());
        assert!(!file.exists());
        // Nothing is left of the temporary files.
        let left: Vec<String> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".tmp"))
            .collect();
        assert!(left.is_empty(), "{left:?}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scratch_notes_keep_their_history_in_the_scratch_folder() {
        let root = temp("scratch");
        scratch::isolate(root.clone());
        let home = Home::Scratch(root.clone());
        let note = root.join("2026-11-14-103200.md");
        fs::write(&note, "idée\n").unwrap();
        save(&home, &note, "ana", None, "idée\n", 0).unwrap();
        assert!(
            root.join(".revisions/2026-11-14-103200.md/0001.md")
                .exists()
        );
        // The Scratch panel lists the notes of the folder, never its `.revisions` or `.trash`.
        assert_eq!(scratch::list(), vec![note]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_renamed_note_takes_its_history_and_a_trashed_one_keeps_it() {
        let root = temp("follow");
        let home = Home::Vault(root.clone());
        let old = root.join("plan.md");
        save(&home, &old, "ana", None, "one\n", 0).unwrap();
        let new = root.join("Idea.md");
        move_along(Some(&root), &old, &new).unwrap();
        assert!(!root.join(".inkuno/revisions/plan.md").exists());
        assert_eq!(all(&home, &new).unwrap().len(), 1);
        // A name whose history is already there is refused, and nothing moves.
        save(&home, &old, "ana", None, "other\n", 0).unwrap();
        assert!(move_along(Some(&root), &old, &new).is_err());
        assert_eq!(all(&home, &old).unwrap().len(), 1);
        // Trashing the note moves its history to the vault's trash under the trashed name.
        let trashed = root.join(".trash/Idea.md");
        trash_along(Some(&root), &new, &trashed).unwrap();
        assert!(!root.join(".inkuno/revisions/Idea.md").exists());
        assert_eq!(
            fs::read_dir(root.join(".inkuno/trash/Idea.md"))
                .unwrap()
                .count(),
            1
        );
        let _ = fs::remove_dir_all(root);
    }

    /// A revision file left in `dir` by some other, older note: `dir` is a bin slot.
    fn stale_history(dir: &Path, text: &str) {
        fs::create_dir_all(dir).unwrap();
        let rev = Revision {
            number: 1,
            parent: None,
            author: "old".into(),
            date: "2020-01-01T00:00:00Z".into(),
            kind: revision::Kind::Checkpoint,
            note: None,
            text: text.into(),
        };
        fs::write(dir.join("0001.md"), revision::render(&rev)).unwrap();
    }

    #[test]
    fn a_stale_bin_entry_never_comes_back_with_another_note() {
        let root = temp("stale");
        let home = Home::Vault(root.clone());
        let note = root.join("Idea.md");
        let bin = root.join(".inkuno/trash");
        // The new note has its own history; the bin already holds an old one under the same name,
        // whose note is no longer in the trash (the trash picks a free name, so `.trash/Idea.md` is
        // free and the new note gets it).
        fs::write(&note, "new note\n").unwrap();
        save(&home, &note, "ana", None, "new note\n", 0).unwrap();
        stale_history(&bin.join("Idea.md"), "old note\n");
        let trashed = vault::trash(&root, &note).unwrap();
        assert_eq!(trashed, root.join(".trash/Idea.md"));
        trash_along(Some(&root), &note, &trashed).unwrap();
        // The new note's history sits under the trashed name; the stale one is kept, moved aside.
        assert_eq!(
            read(&bin.join("Idea.md/0001.md")).unwrap().text,
            "new note\n"
        );
        assert_eq!(
            read(&bin.join("2 Idea.md/0001.md")).unwrap().text,
            "old note\n"
        );
        // Undo brings back the new note's history, and only that.
        restore_along(Some(&root), &trashed, &note).unwrap();
        let back = all(&home, &note).unwrap();
        assert_eq!(
            back.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
            ["new note\n"]
        );
        // A note without history, trashed while a stale entry sits under its name: undo brings
        // nothing back, the stale entry stays in the bin.
        fs::remove_file(&trashed).unwrap();
        fs::remove_dir_all(root.join(".inkuno/revisions/Idea.md")).unwrap();
        fs::write(&note, "plain\n").unwrap();
        stale_history(&bin.join("Idea.md"), "stale again\n");
        let plain = vault::trash(&root, &note).unwrap();
        assert_eq!(plain, root.join(".trash/Idea.md"));
        trash_along(Some(&root), &note, &plain).unwrap();
        restore_along(Some(&root), &plain, &note).unwrap();
        assert!(all(&home, &note).unwrap().is_empty());
        assert_eq!(
            read(&bin.join("3 Idea.md/0001.md")).unwrap().text,
            "stale again\n"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_stale_scratch_bin_entry_never_comes_back_with_another_note() {
        let root = temp("stale-scratch");
        scratch::isolate(root.clone());
        let home = Home::Scratch(root.clone());
        let note = root.join("2026-11-14-103200.md");
        fs::write(&note, "idée\n").unwrap();
        save(&home, &note, "ana", None, "idée\n", 0).unwrap();
        let bin = root.join(".revisions/.trash");
        stale_history(&bin.join("2026-11-14-103200.md"), "old idea\n");
        let trashed = scratch::trash(&note).unwrap();
        trash_along(None, &note, &trashed).unwrap();
        assert_eq!(
            read(&bin.join("2026-11-14-103200.md/0001.md"))
                .unwrap()
                .text,
            "idée\n"
        );
        restore_along(None, &trashed, &note).unwrap();
        assert_eq!(all(&home, &note).unwrap()[0].text, "idée\n");
        assert_eq!(all(&home, &note).unwrap().len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn the_history_is_invisible_to_the_vault_scan() {
        let root = temp("hidden");
        let home = Home::Vault(root.clone());
        let note = root.join("plan.md");
        fs::write(&note, "# Plan\n").unwrap();
        save(&home, &note, "ana", None, "# Plan\n", 0).unwrap();
        save(&home, &root.join("ideas/x.md"), "ana", None, "x\n", 0).unwrap();
        assert!(root.join(".inkuno/revisions/plan.md/0001.md").exists());
        let (notes, dirs, _) = vault::scan(&root);
        let shown: Vec<PathBuf> = notes.iter().map(|n| n.path.clone()).collect();
        assert_eq!(shown, vec![note.clone()]);
        assert!(
            dirs.iter()
                .all(|d| !d.to_string_lossy().contains(".inkuno"))
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn a_note_outside_its_home_has_no_history() {
        let root = temp("outside");
        let home = Home::Vault(root.clone());
        assert!(home.folder(Path::new("/elsewhere/x.md")).is_none());
        assert!(all(&home, Path::new("/elsewhere/x.md")).unwrap().is_empty());
        let _ = fs::remove_dir_all(root);
    }
}
