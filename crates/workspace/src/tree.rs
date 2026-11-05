//! Folder tree model (task 9.1).
//!
//! A vault is a folder of plain `.md` files. The tree shows folders and notes.
//! A note's `name` is its file name without the `.md` extension.
//! Hidden entries (names starting with `.`), symbolic links and anything matched
//! by a `.gitignore` are skipped. Full-text search uses the same walker
//! (`note_walker`), so the tree and search always list the same notes.

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;

/// One entry in the vault tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Folder {
        name: String,
        path: PathBuf,
        children: Vec<Node>,
    },
    Note {
        name: String,
        path: PathBuf,
    },
}

impl Node {
    pub fn name(&self) -> &str {
        match self {
            Node::Folder { name, .. } | Node::Note { name, .. } => name,
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Node::Folder { path, .. } | Node::Note { path, .. } => path,
        }
    }
}

/// A folder of notes. The root is stored as an absolute path, as given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    root: PathBuf,
}

impl Vault {
    /// Opens a vault. Fails if `root` does not exist or is not a directory.
    pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = std::path::absolute(root.into())?;
        if !fs::metadata(&root)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "vault root is not a directory",
            ));
        }
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Tree of the vault. The root is always present, even when it holds no notes.
    /// Empty folders are shown, so a just-created folder has a row to rename.
    pub fn tree(&self) -> io::Result<Node> {
        let mut top = Dir::new(self.root.clone());
        for dir in dir_paths(&self.root)? {
            top.insert_dir(&dir);
        }
        for note in note_files(&self.root)? {
            top.insert(&note);
        }
        Ok(Node::Folder {
            name: folder_name(&self.root),
            path: self.root.clone(),
            children: top.into_children(),
        })
    }

    /// Every `.md` file below the root, sorted by path. Same set as `tree()` and search.
    pub fn notes(&self) -> io::Result<Vec<PathBuf>> {
        note_files(&self.root)
    }
}

/// True when the path has a `.md` extension (case-insensitive).
pub(crate) fn is_note_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// The walker for vault notes. Both the tree and full-text search use it, so they
/// agree on which files are notes. Hidden entries are skipped, and `.gitignore`
/// files apply at every level below `root`, also without a `.git` directory.
/// `.ignore`, git-global and `.git/info/exclude` rules are not used, and symbolic
/// links are not followed.
pub(crate) fn note_walker(root: &Path) -> ignore::Walk {
    WalkBuilder::new(root)
        .hidden(true)
        .require_git(false)
        .git_global(false)
        .git_exclude(false)
        .ignore(false)
        .parents(false)
        .build()
}

/// Every `.md` file below `root` that `note_walker` yields, sorted by path.
/// Files with a non-UTF-8 name in their path are skipped, since the tree cannot show them.
pub(crate) fn note_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in note_walker(root) {
        let entry = entry.map_err(io::Error::other)?;
        let is_file = entry.file_type().is_some_and(|t| t.is_file());
        if is_file && is_note_path(entry.path()) && has_utf8_names(root, entry.path()) {
            files.push(entry.into_path());
        }
    }
    files.sort();
    Ok(files)
}

/// Every directory below `root` that [`note_walker`] yields, sorted by path. The
/// walker already skips hidden and ignored directories and never follows symbolic
/// links, so the same rules as for notes apply. Directories with a non-UTF-8 name
/// in their path are skipped, since the tree cannot show them.
fn dir_paths(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    for entry in note_walker(root) {
        let entry = entry.map_err(io::Error::other)?;
        if entry.file_type().is_some_and(|t| t.is_dir())
            && entry.path() != root
            && has_utf8_names(root, entry.path())
        {
            dirs.push(entry.into_path());
        }
    }
    dirs.sort();
    Ok(dirs)
}

/// True when every component of `path` below `root` is valid UTF-8.
fn has_utf8_names(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .is_ok_and(|rel| rel.iter().all(|c| c.to_str().is_some()))
}

/// A folder while the tree is built from the flat note list. Folders come from
/// directories on disk (even empty ones, so a new folder has a row) and from the
/// parents of notes.
struct Dir {
    path: PathBuf,
    folders: BTreeMap<String, Dir>,
    notes: Vec<Node>,
}

impl Dir {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            folders: BTreeMap::new(),
            notes: Vec::new(),
        }
    }

    /// Adds `note`, an absolute path below this folder.
    fn insert(&mut self, note: &Path) {
        let Ok(rest) = note.strip_prefix(&self.path) else {
            return;
        };
        if rest.components().count() == 1 {
            let name = note
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.notes.push(Node::Note {
                name,
                path: note.to_path_buf(),
            });
            return;
        }
        let Some(folder) = rest.iter().next().and_then(OsStr::to_str) else {
            return;
        };
        let child_path = self.path.join(folder);
        self.folders
            .entry(folder.to_owned())
            .or_insert_with(|| Dir::new(child_path))
            .insert(note);
    }

    /// Adds `dir`, an absolute path below this folder. Intermediate folders are
    /// created as needed, so empty folders (and chains of them) appear in the tree.
    fn insert_dir(&mut self, dir: &Path) {
        let Ok(rest) = dir.strip_prefix(&self.path) else {
            return;
        };
        if rest.as_os_str().is_empty() {
            return;
        }
        let mut current = self;
        for component in rest.iter() {
            let Some(name) = component.to_str() else {
                return;
            };
            let child_path = current.path.join(name);
            current = current
                .folders
                .entry(name.to_owned())
                .or_insert_with(|| Dir::new(child_path));
        }
    }

    /// Children of this folder: subfolders first, then notes. Each group is sorted
    /// case-insensitively by name.
    fn into_children(self) -> Vec<Node> {
        let mut folders: Vec<Node> = self
            .folders
            .into_iter()
            .map(|(name, dir)| {
                let path = dir.path.clone();
                Node::Folder {
                    name,
                    path,
                    children: dir.into_children(),
                }
            })
            .collect();
        let mut notes = self.notes;
        folders.sort_by_cached_key(sort_key);
        notes.sort_by_cached_key(sort_key);
        folders.extend(notes);
        folders
    }
}

fn sort_key(node: &Node) -> (String, String) {
    (node.name().to_lowercase(), node.name().to_owned())
}

fn folder_name(root: &Path) -> String {
    root.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

/// Which folders are expanded in the sidebar. Folders are keyed by path.
/// Folders start collapsed. The state is pure data, so the sidebar can keep it
/// across tree rescans.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Expansion {
    expanded: HashSet<PathBuf>,
}

impl Expansion {
    /// A state with every folder collapsed.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when `folder` has been marked expanded. This is a raw lookup that does not
    /// consult the tree, so ask it about folders only. `visible_rows` reports notes as collapsed.
    pub fn is_expanded(&self, folder: &Path) -> bool {
        self.expanded.contains(folder)
    }

    /// Sets the expanded state of a folder.
    pub fn set_expanded(&mut self, folder: &Path, expanded: bool) {
        if expanded {
            self.expanded.insert(folder.to_path_buf());
        } else {
            self.expanded.remove(folder);
        }
    }

    /// Flips a folder's state and returns the new state.
    pub fn toggle(&mut self, folder: &Path) -> bool {
        let now_expanded = !self.is_expanded(folder);
        self.set_expanded(folder, now_expanded);
        now_expanded
    }

    /// Expands every folder between the vault root and `note`, so that the note
    /// is visible in the sidebar. Paths outside the vault change nothing.
    pub fn reveal(&mut self, vault: &Vault, note: &Path) {
        if !note.starts_with(vault.root()) {
            return;
        }
        for ancestor in note.ancestors().skip(1) {
            if ancestor == vault.root() || !ancestor.starts_with(vault.root()) {
                break;
            }
            self.expanded.insert(ancestor.to_path_buf());
        }
    }
}

/// One line of the sidebar: a node and how deep it sits below the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row<'a> {
    /// 0 for the root's children.
    pub depth: usize,
    pub node: &'a Node,
    /// Always false for notes.
    pub expanded: bool,
}

/// The sidebar rows for a tree, top to bottom. The root is not a row. A folder's
/// children appear only when that folder is expanded. Sibling order is the tree's order.
pub fn visible_rows<'a>(root: &'a Node, expansion: &Expansion) -> Vec<Row<'a>> {
    let mut out = Vec::new();
    if let Node::Folder { children, .. } = root {
        push_rows(children, 0, expansion, &mut out);
    }
    out
}

fn push_rows<'a>(nodes: &'a [Node], depth: usize, expansion: &Expansion, out: &mut Vec<Row<'a>>) {
    for node in nodes {
        let expanded = matches!(node, Node::Folder { .. }) && expansion.is_expanded(node.path());
        out.push(Row {
            depth,
            node,
            expanded,
        });
        if let Node::Folder { children, .. } = node {
            if expanded {
                push_rows(children, depth + 1, expansion, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    #[test]
    fn nested_folders_produce_expected_tree() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join("b.md"), "");
        write(&r.join("A.md"), "");
        write(&r.join("zeta/deep.md"), "");
        write(&r.join("Alpha/a.md"), "");
        write(&r.join("Alpha/sub/inner.md"), "");

        let vault = Vault::open(r).unwrap();
        let expected = Node::Folder {
            name: folder_name(r),
            path: r.to_path_buf(),
            children: vec![
                Node::Folder {
                    name: "Alpha".into(),
                    path: r.join("Alpha"),
                    children: vec![
                        Node::Folder {
                            name: "sub".into(),
                            path: r.join("Alpha/sub"),
                            children: vec![Node::Note {
                                name: "inner".into(),
                                path: r.join("Alpha/sub/inner.md"),
                            }],
                        },
                        Node::Note {
                            name: "a".into(),
                            path: r.join("Alpha/a.md"),
                        },
                    ],
                },
                Node::Folder {
                    name: "zeta".into(),
                    path: r.join("zeta"),
                    children: vec![Node::Note {
                        name: "deep".into(),
                        path: r.join("zeta/deep.md"),
                    }],
                },
                Node::Note {
                    name: "A".into(),
                    path: r.join("A.md"),
                },
                Node::Note {
                    name: "b".into(),
                    path: r.join("b.md"),
                },
            ],
        };
        assert_eq!(vault.tree().unwrap(), expected);
    }

    #[test]
    fn non_markdown_files_are_excluded() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join("note.md"), "");
        write(&r.join("NOTE2.MD"), "");
        write(&r.join("image.png"), "");
        write(&r.join("draft.markdown"), "");
        write(&r.join("md"), "");
        write(&r.join("sub/readme.txt"), "");
        write(&r.join("sub/deep.md"), "");

        let vault = Vault::open(r).unwrap();
        let expected = vec![r.join("NOTE2.MD"), r.join("note.md"), r.join("sub/deep.md")];
        assert_eq!(vault.notes().unwrap(), {
            let mut e = expected.clone();
            e.sort();
            e
        });
        let tree = vault.tree().unwrap();
        let Node::Folder { children, .. } = tree else {
            panic!("root must be a folder");
        };
        let names: Vec<&str> = children.iter().map(Node::name).collect();
        assert_eq!(names, vec!["sub", "note", "NOTE2"]);
    }

    #[test]
    fn empty_folders_are_shown_but_root_is_kept() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join("empty/also-empty")).unwrap();
        write(&r.join("empty/also-empty/image.png"), "");
        write(&r.join("full/n.md"), "");

        let vault = Vault::open(r).unwrap();
        let Node::Folder { children, path, .. } = vault.tree().unwrap() else {
            panic!("root must be a folder");
        };
        assert_eq!(path, r);
        // Both the empty chain and the folder with a note are listed.
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name(), "empty");
        assert_eq!(children[1].name(), "full");
        // The empty chain has no notes, only nested folders down to the file's parent.
        let Node::Folder { children, path, .. } = &children[0] else {
            panic!("empty must be a folder");
        };
        assert_eq!(path, &r.join("empty"));
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name(), "also-empty");
        let Node::Folder { children, path, .. } = &children[0] else {
            panic!("also-empty must be a folder");
        };
        assert_eq!(path, &r.join("empty/also-empty"));
        assert!(children.is_empty());

        let empty = tempfile::tempdir().unwrap();
        let vault = Vault::open(empty.path()).unwrap();
        match vault.tree().unwrap() {
            Node::Folder { children, .. } => assert!(children.is_empty()),
            _ => panic!("root must be a folder"),
        }
    }

    #[test]
    fn expanded_empty_folder_lists_only_itself() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        fs::create_dir_all(r.join("empty")).unwrap();

        let vault = Vault::open(r).unwrap();
        let tree = vault.tree().unwrap();
        assert_eq!(tree.path(), r);
        // Expanding a folder with no children must not panic and lists just that row.
        let mut expansion = Expansion::new();
        expansion.set_expanded(&r.join("empty"), true);
        let rows = visible_rows(&tree, &expansion);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].node.path(), r.join("empty"));
        assert!(rows[0].expanded);
    }

    #[test]
    fn hidden_entries_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join(".hidden/x.md"), "");
        write(&r.join(".secret.md"), "");
        write(&r.join(".obsidian/config.md"), "");
        write(&r.join("visible.md"), "");

        let vault = Vault::open(r).unwrap();
        assert_eq!(vault.notes().unwrap(), vec![r.join("visible.md")]);
        let Node::Folder { children, .. } = vault.tree().unwrap() else {
            panic!("root must be a folder");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name(), "visible");
    }

    #[test]
    fn gitignored_notes_are_absent_from_the_tree() {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join(".gitignore"), "draft.md\nprivate/\n");
        write(&r.join("draft.md"), "");
        write(&r.join("private/p.md"), "");
        write(&r.join("keep.md"), "");
        write(&r.join("sub/b.md"), "");

        let vault = Vault::open(r).unwrap();
        assert_eq!(
            vault.notes().unwrap(),
            vec![r.join("keep.md"), r.join("sub/b.md")]
        );
        let Node::Folder { children, .. } = vault.tree().unwrap() else {
            panic!("root must be a folder");
        };
        let names: Vec<&str> = children.iter().map(Node::name).collect();
        assert_eq!(names, vec!["sub", "keep"]);
    }

    #[test]
    fn open_rejects_missing_path_and_files() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f.md");
        write(&file, "");
        let err = Vault::open(&file).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(Vault::open(dir.path().join("missing")).is_err());
    }

    #[test]
    fn node_accessors() {
        let node = Node::Note {
            name: "n".into(),
            path: PathBuf::from("/v/n.md"),
        };
        assert_eq!(node.name(), "n");
        assert_eq!(node.path(), Path::new("/v/n.md"));
    }

    /// Vault with `Alpha/a.md`, `Alpha/sub/inner.md`, `zeta/deep.md` and `top.md`.
    fn rows_fixture() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let r = dir.path();
        write(&r.join("Alpha/a.md"), "");
        write(&r.join("Alpha/sub/inner.md"), "");
        write(&r.join("zeta/deep.md"), "");
        write(&r.join("top.md"), "");
        let vault = Vault::open(r).unwrap();
        (dir, vault)
    }

    /// (depth, name, expanded) for each row, so assertions stay short.
    fn summary(rows: &[Row<'_>]) -> Vec<(usize, String, bool)> {
        rows.iter()
            .map(|r| (r.depth, r.node.name().to_owned(), r.expanded))
            .collect()
    }

    #[test]
    fn folders_start_collapsed_and_show_only_top_level() {
        let (_dir, vault) = rows_fixture();
        let tree = vault.tree().unwrap();
        let rows = visible_rows(&tree, &Expansion::new());
        assert_eq!(
            summary(&rows),
            vec![
                (0, "Alpha".into(), false),
                (0, "zeta".into(), false),
                (0, "top".into(), false),
            ]
        );
    }

    #[test]
    fn toggle_shows_and_hides_children_at_the_right_depth() {
        let (dir, vault) = rows_fixture();
        let tree = vault.tree().unwrap();
        let mut exp = Expansion::new();
        let alpha = dir.path().join("Alpha");
        assert!(exp.toggle(&alpha));
        assert!(exp.is_expanded(&alpha));
        let rows = visible_rows(&tree, &exp);
        assert_eq!(
            summary(&rows),
            vec![
                (0, "Alpha".into(), true),
                (1, "sub".into(), false),
                (1, "a".into(), false),
                (0, "zeta".into(), false),
                (0, "top".into(), false),
            ]
        );
        // Nested folder is still collapsed, so its note is not listed.
        assert!(!exp.toggle(&alpha));
        assert_eq!(visible_rows(&tree, &exp).len(), 3);
    }

    #[test]
    fn nested_expansion_lists_children_of_expanded_subfolder() {
        let (dir, vault) = rows_fixture();
        let tree = vault.tree().unwrap();
        let mut exp = Expansion::new();
        exp.set_expanded(&dir.path().join("Alpha"), true);
        exp.set_expanded(&dir.path().join("Alpha/sub"), true);
        let rows = visible_rows(&tree, &exp);
        assert_eq!(
            summary(&rows),
            vec![
                (0, "Alpha".into(), true),
                (1, "sub".into(), true),
                (2, "inner".into(), false),
                (1, "a".into(), false),
                (0, "zeta".into(), false),
                (0, "top".into(), false),
            ]
        );
    }

    #[test]
    fn reveal_expands_every_folder_above_a_note() {
        let (dir, vault) = rows_fixture();
        let tree = vault.tree().unwrap();
        let mut exp = Expansion::new();
        exp.reveal(&vault, &dir.path().join("Alpha/sub/inner.md"));
        assert!(exp.is_expanded(&dir.path().join("Alpha")));
        assert!(exp.is_expanded(&dir.path().join("Alpha/sub")));
        assert!(!exp.is_expanded(&dir.path().join("zeta")));
        assert!(!exp.is_expanded(dir.path()));
        let names: Vec<String> = visible_rows(&tree, &exp)
            .iter()
            .map(|r| r.node.name().to_owned())
            .collect();
        // Inside `sub`, the note `inner` is the only child, so it comes before the
        // note `a` of the parent folder: folders first, then notes, per folder.
        assert_eq!(names, vec!["Alpha", "sub", "inner", "a", "zeta", "top"]);
    }

    #[test]
    fn reveal_ignores_paths_outside_the_vault() {
        let (_dir, vault) = rows_fixture();
        let other = tempfile::tempdir().unwrap();
        let mut exp = Expansion::new();
        exp.reveal(&vault, &other.path().join("x/y.md"));
        assert_eq!(exp, Expansion::new());
    }

    #[test]
    fn notes_are_always_reported_collapsed() {
        let (dir, vault) = rows_fixture();
        let mut exp = Expansion::new();
        // Marking a note's path as expanded must not make its row expanded.
        exp.set_expanded(&dir.path().join("top.md"), true);
        let tree = vault.tree().unwrap();
        let rows = visible_rows(&tree, &exp);
        let top = rows.iter().find(|r| r.node.name() == "top").unwrap();
        assert!(!top.expanded);
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn a_note_as_root_has_no_rows() {
        let node = Node::Note {
            name: "n".into(),
            path: PathBuf::from("/v/n.md"),
        };
        assert!(visible_rows(&node, &Expansion::new()).is_empty());
    }
}
