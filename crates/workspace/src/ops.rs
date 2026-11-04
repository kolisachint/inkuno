//! File operations on the workspace (task 9.3).
//!
//! Every operation takes absolute paths and must stay inside the vault root.
//! Paths outside the root fail with `ErrorKind::InvalidInput`.

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::tree::{is_note_path, Vault};

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

/// A single file or folder name: non-empty, no separators, not hidden.
fn check_name(name: &str) -> io::Result<()> {
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\', '\0']) {
        return Err(invalid("invalid file or folder name"));
    }
    Ok(())
}

fn has_md_suffix(name: &str) -> bool {
    name.to_ascii_lowercase().ends_with(".md")
}

/// Resolves `.` and `..` without touching the file system.
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn ensure_dir(path: &Path) -> io::Result<()> {
    if fs::metadata(path)?.is_dir() {
        Ok(())
    } else {
        Err(invalid("parent is not a folder"))
    }
}

impl Vault {
    /// Returns the normalized path if it is absolute and lies inside the vault.
    /// Symbolic links and missing trailing components are resolved for the check only.
    fn contain(&self, path: &Path) -> io::Result<PathBuf> {
        if !path.is_absolute() {
            return Err(invalid("path must be absolute"));
        }
        let normalized = lexical_normalize(path);
        let root = fs::canonicalize(self.root())?;

        // Canonicalize the longest existing ancestor, then re-append the rest.
        let mut existing = normalized.as_path();
        let mut missing = Vec::new();
        while fs::symlink_metadata(existing).is_err() {
            match (existing.file_name(), existing.parent()) {
                (Some(name), Some(parent)) => {
                    missing.push(name);
                    existing = parent;
                }
                _ => return Err(invalid("path is outside the vault")),
            }
        }
        let mut real = fs::canonicalize(existing)?;
        real.extend(missing.iter().rev());
        if !real.starts_with(&root) {
            return Err(invalid("path is outside the vault"));
        }
        Ok(normalized)
    }

    fn is_vault_root(&self, path: &Path) -> bool {
        match (fs::canonicalize(path), fs::canonicalize(self.root())) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }

    /// Creates a note in `parent`. Appends `.md` when `name` lacks it.
    /// Fails with `AlreadyExists` if the file is already there.
    pub fn create_note(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        check_name(name)?;
        let parent = self.contain(parent)?;
        ensure_dir(&parent)?;
        let file_name = if has_md_suffix(name) {
            name.to_owned()
        } else {
            format!("{name}.md")
        };
        let path = parent.join(file_name);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        Ok(path)
    }

    /// Creates a folder in `parent`. Fails with `AlreadyExists` if it is already there.
    pub fn create_folder(&self, parent: &Path, name: &str) -> io::Result<PathBuf> {
        check_name(name)?;
        let parent = self.contain(parent)?;
        ensure_dir(&parent)?;
        let path = parent.join(name);
        fs::create_dir(&path)?;
        Ok(path)
    }

    /// Renames a note or folder within its parent and returns the new path.
    /// For notes, `.md` is kept or appended. Renaming to the same path is a no-op.
    /// Fails with `AlreadyExists` if the target exists. This includes a
    /// case-only rename on a case-insensitive file system.
    pub fn rename(&self, from: &Path, new_name: &str) -> io::Result<PathBuf> {
        check_name(new_name)?;
        let from = self.contain(from)?;
        if self.is_vault_root(&from) {
            return Err(invalid("cannot rename the vault root"));
        }
        let meta = fs::symlink_metadata(&from)?;
        let file_name = if meta.is_file() && is_note_path(&from) && !has_md_suffix(new_name) {
            format!("{new_name}.md")
        } else {
            new_name.to_owned()
        };
        let parent = from.parent().ok_or_else(|| invalid("path has no parent"))?;
        let to = parent.join(file_name);
        if to == from {
            return Ok(to);
        }
        if fs::symlink_metadata(&to).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "a file or folder with that name already exists",
            ));
        }
        fs::rename(&from, &to)?;
        Ok(to)
    }

    /// Deletes a note or folder. Folders are deleted recursively.
    /// A symbolic link is removed itself, not its target.
    pub fn delete(&self, path: &Path) -> io::Result<()> {
        let path = self.contain(path)?;
        if self.is_vault_root(&path) {
            return Err(invalid("cannot delete the vault root"));
        }
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        }
    }
}

/// Tracks open notes so renames keep buffers open under the new path.
#[derive(Default, Debug)]
pub struct OpenNotes {
    pub paths: Vec<PathBuf>,
}

impl OpenNotes {
    /// Rewrites open paths after `from` was renamed to `to`.
    /// Works for a note and for a folder that contains open notes.
    pub fn on_rename(&mut self, from: &Path, to: &Path) {
        for open in &mut self.paths {
            let Ok(rest) = open.strip_prefix(from) else {
                continue;
            };
            let updated = if rest.as_os_str().is_empty() {
                to.to_path_buf()
            } else {
                to.join(rest)
            };
            *open = updated;
        }
    }

    /// Forgets open notes at or below `path` after it was deleted.
    pub fn on_delete(&mut self, path: &Path) {
        self.paths.retain(|open| !open.starts_with(path));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::Node;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn all_paths(node: &Node, out: &mut Vec<PathBuf>) {
        out.push(node.path().to_path_buf());
        if let Node::Folder { children, .. } = node {
            for child in children {
                all_paths(child, out);
            }
        }
    }

    fn tree_paths(vault: &Vault) -> Vec<PathBuf> {
        let mut out = Vec::new();
        all_paths(&vault.tree().unwrap(), &mut out);
        out
    }

    fn fixture() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        (dir, vault)
    }

    #[test]
    fn create_note_appends_md_and_reflects_in_tree() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let a = vault.create_note(r, "first").unwrap();
        assert_eq!(a, r.join("first.md"));
        assert!(a.is_file());
        let b = vault.create_note(r, "second.md").unwrap();
        assert_eq!(b, r.join("second.md"));
        let c = vault.create_note(r, "third.MD").unwrap();
        assert_eq!(c, r.join("third.MD"));

        assert_eq!(
            vault.notes().unwrap(),
            vec![a.clone(), b.clone(), c.clone()]
        );
        assert!(tree_paths(&vault).contains(&a));
        assert_eq!(
            vault.create_note(r, "first").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
    }

    #[test]
    fn create_folder_and_note_inside_it() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let folder = vault.create_folder(r, "projects").unwrap();
        assert!(folder.is_dir());
        assert_eq!(
            vault.create_folder(r, "projects").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        // An empty folder is not shown in the tree.
        assert!(!tree_paths(&vault).contains(&folder));

        let note = vault.create_note(&folder, "plan").unwrap();
        assert!(tree_paths(&vault).contains(&folder));
        assert!(tree_paths(&vault).contains(&note));
    }

    #[test]
    fn rename_note_keeps_or_appends_md() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let a = vault.create_note(r, "a").unwrap();

        let b = vault.rename(&a, "b").unwrap();
        assert_eq!(b, r.join("b.md"));
        assert!(!a.exists());
        assert!(b.is_file());

        let c = vault.rename(&b, "c.md").unwrap();
        assert_eq!(c, r.join("c.md"));
        assert!(tree_paths(&vault).contains(&c));
        assert!(!tree_paths(&vault).contains(&b));

        // Renaming to the same name is a no-op.
        assert_eq!(vault.rename(&c, "c").unwrap(), c);
    }

    #[test]
    fn rename_folder_moves_notes_below_it() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let folder = vault.create_folder(r, "old").unwrap();
        let note = vault.create_note(&folder, "n").unwrap();

        let renamed = vault.rename(&folder, "new").unwrap();
        assert_eq!(renamed, r.join("new"));
        assert!(!folder.exists());
        assert_eq!(vault.notes().unwrap(), vec![r.join("new/n.md")]);
        assert!(!note.exists());
    }

    #[test]
    fn rename_onto_existing_target_fails() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let a = vault.create_note(r, "a").unwrap();
        vault.create_note(r, "b").unwrap();
        assert_eq!(
            vault.rename(&a, "b").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert!(a.exists());
    }

    #[test]
    fn delete_note_and_folder() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let note = vault.create_note(r, "gone").unwrap();
        let folder = vault.create_folder(r, "dir").unwrap();
        vault.create_note(&folder, "inner").unwrap();
        write(&folder.join("sub/deep.md"), "");

        vault.delete(&note).unwrap();
        assert!(!note.exists());
        assert!(!tree_paths(&vault).contains(&note));

        vault.delete(&folder).unwrap();
        assert!(!folder.exists());
        assert!(vault.notes().unwrap().is_empty());
        assert_eq!(tree_paths(&vault), vec![r.to_path_buf()]);
    }

    #[test]
    fn paths_outside_the_vault_are_rejected() {
        let (dir, vault) = fixture();
        let outside = tempfile::tempdir().unwrap();
        let r = dir.path();
        let existing = vault.create_note(r, "keep").unwrap();
        let other = outside.path().join("other.md");
        write(&other, "");

        let kind = |res: io::Result<PathBuf>| res.unwrap_err().kind();
        assert_eq!(
            kind(vault.create_note(outside.path(), "x")),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            kind(vault.create_note(&r.join("../escape"), "x")),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            kind(vault.create_folder(outside.path(), "x")),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            kind(vault.rename(&other, "moved")),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            vault.delete(&other).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            vault.delete(&r.join("../")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            vault.delete(Path::new("relative.md")).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            vault.delete(r).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            vault.rename(&existing, "../../escape").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            vault.create_note(r, "a/b").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(other.exists());
        assert!(existing.exists());
    }

    #[test]
    fn open_notes_follow_note_rename() {
        let mut open = OpenNotes {
            paths: vec![PathBuf::from("/v/a.md"), PathBuf::from("/v/other.md")],
        };
        open.on_rename(Path::new("/v/a.md"), Path::new("/v/b.md"));
        assert_eq!(
            open.paths,
            vec![PathBuf::from("/v/b.md"), PathBuf::from("/v/other.md")]
        );
    }

    #[test]
    fn open_notes_follow_folder_rename() {
        let mut open = OpenNotes {
            paths: vec![
                PathBuf::from("/v/dir/a.md"),
                PathBuf::from("/v/dir/sub/b.md"),
                PathBuf::from("/v/dirx/c.md"),
            ],
        };
        open.on_rename(Path::new("/v/dir"), Path::new("/v/renamed"));
        assert_eq!(
            open.paths,
            vec![
                PathBuf::from("/v/renamed/a.md"),
                PathBuf::from("/v/renamed/sub/b.md"),
                PathBuf::from("/v/dirx/c.md"),
            ]
        );
    }

    #[test]
    fn open_notes_drop_deleted_paths() {
        let mut open = OpenNotes {
            paths: vec![
                PathBuf::from("/v/dir/a.md"),
                PathBuf::from("/v/dir"),
                PathBuf::from("/v/keep.md"),
            ],
        };
        open.on_delete(Path::new("/v/dir"));
        assert_eq!(open.paths, vec![PathBuf::from("/v/keep.md")]);
    }

    #[test]
    fn open_notes_rename_through_vault_ops() {
        let (dir, vault) = fixture();
        let r = dir.path();
        let folder = vault.create_folder(r, "dir").unwrap();
        let note = vault.create_note(&folder, "a").unwrap();
        let mut open = OpenNotes {
            paths: vec![note.clone()],
        };
        let renamed = vault.rename(&folder, "moved").unwrap();
        open.on_rename(&folder, &renamed);
        assert_eq!(open.paths, vec![r.join("moved/a.md")]);
        assert!(open.paths[0].is_file());
    }
}
