//! The vault a command works on: the index the app would have, read once from disk, and the rule
//! that nothing is reached outside that folder.

use std::{
    fs, io,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};

use crate::{
    logic::note::Note,
    logic::{rank, wiki},
    platform::vault,
};

use super::outcome::disk;

/// One vault, read from disk when it is opened: a command runs once, then it is done.
pub struct Vault {
    pub root: PathBuf,
    pub notes: Vec<Note>,
    pub dirs: Vec<PathBuf>,
    pub diagrams: Vec<PathBuf>,
    pub pictures: Vec<PathBuf>,
    pub tables: Vec<PathBuf>,
    pub attachments: Vec<PathBuf>,
    pub unreadable: Vec<(PathBuf, io::ErrorKind)>,
}

impl Vault {
    /// Opens `root` and reads it. The error is a sentence an agent can act on.
    pub fn open(root: PathBuf) -> Result<Self, String> {
        if !root.exists() {
            return Err("not found".into());
        }
        if !root.is_dir() {
            return Err("not a folder".into());
        }
        let mut vault = Vault {
            root,
            notes: Vec::new(),
            dirs: Vec::new(),
            diagrams: Vec::new(),
            pictures: Vec::new(),
            tables: Vec::new(),
            attachments: Vec::new(),
            unreadable: Vec::new(),
        };
        vault.rescan();
        if vault.unreadable.iter().any(|(path, _)| path == &vault.root) {
            return Err(disk_kind(vault.unreadable[0].1));
        }
        Ok(vault)
    }

    pub fn rescan(&mut self) {
        let (notes, dirs, files, unreadable) = vault::rescan_full(&self.root, &self.notes);
        let mut diagrams = Vec::new();
        let mut pictures = Vec::new();
        let mut tables = Vec::new();
        let mut attachments = Vec::new();
        for path in files {
            if vault::is_table(&path) {
                tables.push(path);
            } else if vault::is_attachment(&path) {
                attachments.push(path);
            } else if is_diagram(&path) {
                diagrams.push(path);
            } else {
                pictures.push(path);
            }
        }
        self.notes = notes;
        self.dirs = dirs;
        self.diagrams = diagrams;
        self.pictures = pictures;
        self.tables = tables;
        self.attachments = attachments;
        self.unreadable = unreadable;
    }

    /// `note` as a `[[link]]` would write it (a name, then an alias), or a path inside the vault.
    /// A template answers no link.
    pub fn find_note(&self, note: &str) -> Result<&Note, String> {
        if looks_like_path(note) {
            let path = self.inside(note, false)?;
            // `ideas/Plan` names `ideas/Plan.md`: the extension may be left out, as a link leaves it out.
            let as_written = self.notes.iter().find(|n| n.path == path);
            return as_written
                .or_else(|| {
                    let with_md = with_md(&path);
                    self.notes.iter().find(|n| n.path == with_md)
                })
                .ok_or_else(|| format!("no note at `{note}`"));
        }
        let name = note.split(['#', '|']).next().unwrap_or(note).trim();
        if name.is_empty() {
            return Err("the note's name is empty".into());
        }
        wiki::resolve(&self.notes, name, |n| {
            vault::in_templates(&self.root, &n.path)
        })
        .ok_or_else(|| self.missing(name))
    }

    fn missing(&self, name: &str) -> String {
        match suggest(&self.notes, name) {
            Some(other) => format!("no note named `{name}`; did you mean `{other}`?"),
            None => format!("no note named `{name}`"),
        }
    }

    /// A path that stays in the vault. `trash` allows the `.trash` folder and nothing else hidden.
    /// The answer is under `root` as given, like the index: canonical paths (`/private/var` on
    /// macOS, `\\?\C:` on Windows) serve only to check where a link leads.
    pub fn inside(&self, raw: &str, trash: bool) -> Result<PathBuf, String> {
        if raw.contains('\0') || raw.split(['/', '\\']).any(|part| part == "..") {
            return Err("a path cannot leave the vault".into());
        }
        let given = Path::new(raw);
        if given.is_absolute() && !given.starts_with(&self.root) {
            return Err("a path cannot leave the vault".into());
        }
        let joined = if given.is_absolute() {
            given.to_path_buf()
        } else {
            self.root.join(given)
        };
        let root = self.root.canonicalize().map_err(disk)?;
        let rel = joined
            .strip_prefix(&self.root)
            .or_else(|_| joined.strip_prefix(&root))
            .map_err(|_| "a path cannot leave the vault".to_string())?;
        let mut cur = root.clone();
        for (depth, component) in rel.components().enumerate() {
            let Component::Normal(name) = component else {
                return Err("a path cannot leave the vault".into());
            };
            let name = name.to_string_lossy();
            if hidden(&name, depth, trash) {
                return Err("hidden folders are not part of the vault".into());
            }
            cur.push(name.as_ref());
            if cur
                .symlink_metadata()
                .is_ok_and(|meta| meta.file_type().is_symlink())
            {
                let target = cur.canonicalize().map_err(disk)?;
                if !target.starts_with(&root) {
                    return Err("a path cannot leave the vault".into());
                }
                cur = target;
            }
        }
        // A link may lead into a hidden folder of the vault (`linkh -> .inkuno`): the rule holds
        // for the place the path really reaches, under the resolved root.
        let reached = cur
            .strip_prefix(&root)
            .map_err(|_| "a path cannot leave the vault".to_string())?;
        for (depth, component) in reached.components().enumerate() {
            if let Component::Normal(name) = component
                && hidden(&name.to_string_lossy(), depth, trash)
            {
                return Err("hidden folders are not part of the vault".into());
            }
        }
        Ok(self.root.join(rel))
    }

    /// A file of the vault named `name` (a note, a diagram, a picture), or a relative path.
    pub fn find_file(&self, name: &str) -> Result<PathBuf, String> {
        if looks_like_path(name) {
            let path = self.inside(name, false)?;
            if path.is_file() {
                return Ok(path);
            }
            // A note named without its extension, as in `find_note`.
            return match self.find_note(name) {
                Ok(note) => Ok(note.path.clone()),
                Err(_) => Err(format!("no file at `{name}`")),
            };
        }
        if let Ok(note) = self.find_note(name) {
            return Ok(note.path.clone());
        }
        let want = name.to_lowercase();
        self.files()
            .into_iter()
            .find(|path| {
                path.file_name()
                    .is_some_and(|n| n.to_string_lossy().to_lowercase() == want)
            })
            .ok_or_else(|| format!("no file named `{name}`"))
    }

    pub fn files(&self) -> Vec<PathBuf> {
        self.diagrams
            .iter()
            .chain(&self.pictures)
            .chain(&self.tables)
            .chain(&self.attachments)
            .cloned()
            .collect()
    }

    pub fn is_template(&self, note: &Note) -> bool {
        vault::in_templates(&self.root, &note.path)
    }
}

/// A name of the vault that is hidden: a dot-name, except `.trash` at the root when `trash` allows it.
fn hidden(name: &str, depth: usize, trash: bool) -> bool {
    name.starts_with('.') && !(trash && depth == 0 && name == ".trash")
}

fn looks_like_path(note: &str) -> bool {
    note.contains('/')
        || note.contains('\\')
        || note.to_lowercase().ends_with(".md")
        || note.to_lowercase().ends_with(".svg")
}

/// `path` with `.md` added to its name: the note a path without the extension names.
fn with_md(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".md");
    PathBuf::from(name)
}

/// An SVG file of the vault. Inkuno no longer edits diagrams: they open
/// in the zoomable viewer, as any SVG from elsewhere does.
fn is_diagram(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
}

/// The note whose name is closest to `query`, when one clearly contains the other.
pub fn suggest(notes: &[Note], query: &str) -> Option<String> {
    let q = query.to_lowercase();
    let mut best: Option<(i32, &str)> = None;
    for note in notes {
        let name = note.name.to_lowercase();
        if name == q {
            continue;
        }
        let score = if name.contains(&q)
            || (!q.is_empty() && q.contains(&name) && name.chars().count() >= 3)
        {
            2000 - (name.len() as i32 - q.len() as i32).abs()
        } else {
            rank::fuzzy(&q, &name)
                .filter(|score| *score > 0)
                .unwrap_or(i32::MIN)
        };
        if score == i32::MIN {
            continue;
        }
        if best.is_none_or(|(so_far, _)| score > so_far) {
            best = Some((score, note.name.as_str()));
        }
    }
    best.filter(|(score, _)| *score > 0)
        .map(|(_, name)| name.to_string())
}

/// Path of `path` relative to the vault, with `/` even on Windows.
pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn folder(root: &Path, path: &Path) -> String {
    path.parent()
        .and_then(|dir| dir.strip_prefix(root).ok())
        .map(|dir| dir.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

pub fn unix(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Hash of the file as it is on disk. `edit_note` refuses a token that is no longer this.
pub fn token(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn read_text(path: &Path) -> Result<(String, String), String> {
    let text = fs::read_to_string(path).map_err(disk)?;
    let token = token(text.as_bytes());
    Ok((text, token))
}

/// A file of this name, ignoring case, already in `dir`.
pub fn name_taken(dir: &Path, name: &str, extension: &str) -> Option<PathBuf> {
    let want = name.to_lowercase();
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case(extension))
                && vault::stem(path).to_lowercase() == want
        })
}

fn disk_kind(kind: io::ErrorKind) -> String {
    disk(io::Error::new(kind, ""))
}

/// Rewrites `[[old]]` into `[[new]]` in every note that links to it, unless another note
/// is still called `old`. Returns how many notes changed.
pub fn relink(vault: &mut Vault, old: &str, new: &str) -> usize {
    vault.rescan();
    let Some(targets) = wiki::relink_set(&vault.notes, old, new) else {
        return 0;
    };
    let paths: Vec<PathBuf> = targets.iter().map(|n| n.path.clone()).collect();
    let mut changed = 0;
    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let Some(fresh) = crate::logic::markdown::relink(&text, old, new) else {
            continue;
        };
        if vault::write(&path, &fresh).is_ok() {
            changed += 1;
        }
    }
    vault.rescan();
    changed
}
