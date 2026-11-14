//! Revisions on disk: the files of a note's history, saved, listed, read, and moved or trashed
//! with the note. The format and the paths are in `logic::revision` (`inkuno-logic`). A vault's
//! history lives in `<vault>/.inkuno/revisions/`, a scratch folder's in `<scratch>/.revisions/`;
//! both are hidden from the tree, the search and the Scratch panel.

// ponytail: nothing calls the saving, listing and reading functions until the checkpoint key and
// the command line land (next commits); this line goes then.
#![allow(dead_code)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use super::{scratch, vault};
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
    let folder = home
        .folder(note)
        .ok_or_else(|| io::Error::other("the note is not in its vault or scratch folder"))?;
    let numbers = numbers(&folder)?;
    let latest = numbers.iter().copied().max();
    if let Some(latest) = latest {
        let last = read(&folder.join(revision::file_name(latest)))?;
        if last.text == text {
            return Ok(Saved::Unchanged { latest });
        }
    }
    let number = revision::next_number(numbers);
    let file = folder.join(revision::file_name(number));
    if file.exists() {
        // Another save took this number since the folder was listed: nothing is overwritten.
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("revision {number} was saved meanwhile, try again"),
        ));
    }
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
    vault::write(&file, revision::render(&rev))?;
    Ok(Saved::Made { number, file })
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
    if dst.exists() {
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

/// A trashed note or folder: its history goes to the home's trash under the same name the trashed
/// item got there (numbered when taken), so nothing is lost and the pair can be found.
pub fn trash_along(vault: Option<&Path>, path: &Path, trashed: &Path) -> io::Result<()> {
    let Some(home) = home_of(path, vault) else {
        return Ok(());
    };
    let Some(src) = home.folder(path) else {
        return Ok(());
    };
    if !src.exists() {
        return Ok(());
    }
    let bin = home.bin();
    fs::create_dir_all(&bin)?;
    let name = trashed
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let free = (1..)
        .map(|n| match n {
            1 => bin.join(&name),
            n => bin.join(format!("{n} {name}")),
        })
        .find(|p| !p.exists())
        .unwrap();
    fs::rename(&src, &free)
}

/// Undo of a trash, or a restore from it: the history that went to the bin with `trashed` goes back
/// to `to`. Nothing is done when there is no such history; nothing is overwritten.
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
