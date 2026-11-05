//! Folder tree model (task 9.1).
//!
//! A vault is a folder of plain `.md` files. The tree shows folders and notes.
//! A note's `name` is its file name without the `.md` extension.
//! Hidden entries (names starting with `.`) and symbolic links are skipped.

use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    pub fn tree(&self) -> io::Result<Node> {
        let children = scan_dir(&self.root)?;
        Ok(Node::Folder {
            name: folder_name(&self.root),
            path: self.root.clone(),
            children,
        })
    }

    /// Every `.md` file below the root, sorted by path.
    pub fn notes(&self) -> io::Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        collect_notes(&self.root, &mut out)?;
        out.sort();
        Ok(out)
    }
}

/// True when the path has a `.md` extension (case-insensitive).
pub(crate) fn is_note_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Visible entries of a directory. Entries with non-UTF-8 or hidden names are skipped.
fn visible_entries(dir: &Path) -> io::Result<Vec<(String, PathBuf, fs::FileType)>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        out.push((name.to_owned(), entry.path(), entry.file_type()?));
    }
    Ok(out)
}

/// Children of a folder: subfolders that contain notes (folders first), then notes.
/// Both groups are sorted case-insensitively by name.
fn scan_dir(dir: &Path) -> io::Result<Vec<Node>> {
    let mut folders = Vec::new();
    let mut notes = Vec::new();
    for (name, path, file_type) in visible_entries(dir)? {
        if file_type.is_dir() {
            let children = scan_dir(&path)?;
            if !children.is_empty() {
                folders.push(Node::Folder {
                    name,
                    path,
                    children,
                });
            }
        } else if file_type.is_file() && is_note_path(&path) {
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            notes.push(Node::Note { name: stem, path });
        }
    }
    folders.sort_by_cached_key(sort_key);
    notes.sort_by_cached_key(sort_key);
    folders.extend(notes);
    Ok(folders)
}

fn sort_key(node: &Node) -> (String, String) {
    (node.name().to_lowercase(), node.name().to_owned())
}

fn collect_notes(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for (_, path, file_type) in visible_entries(dir)? {
        if file_type.is_dir() {
            collect_notes(&path, out)?;
        } else if file_type.is_file() && is_note_path(&path) {
            out.push(path);
        }
    }
    Ok(())
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
    fn empty_folders_are_omitted_but_root_is_kept() {
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
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name(), "full");

        let empty = tempfile::tempdir().unwrap();
        let vault = Vault::open(empty.path()).unwrap();
        match vault.tree().unwrap() {
            Node::Folder { children, .. } => assert!(children.is_empty()),
            _ => panic!("root must be a folder"),
        }
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
