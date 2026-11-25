//! Vault: a folder of `.md` files, plus the app's small config.

use std::{
    collections::{HashMap, HashSet},
    env, fs,
    hash::{DefaultHasher, Hash, Hasher},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use super::{revisions, scratch, sidecar};
use inkuno_logic::note::{self, Note, lowered};
use inkuno_logic::{lang::tr, markdown, revision, trash};

/// The user's configuration folder, per platform.
pub fn config_dir() -> PathBuf {
    let var = |key: &str| env::var_os(key).map(PathBuf::from);
    let dir = if cfg!(target_os = "macos") {
        var("HOME").map(|home| home.join("Library/Application Support"))
    } else {
        var("XDG_CONFIG_HOME").or_else(|| var("HOME").map(|home| home.join(".config")))
    };
    dir.unwrap_or_default()
}

fn config_path() -> PathBuf {
    config_dir().join("inkuno/config")
}

/// (vault, open notes from most recent to oldest): one line each.
pub fn load_config() -> (Option<PathBuf>, Vec<PathBuf>) {
    // Older still: the config of the app's first name is read but never rewritten.
    let text = fs::read_to_string(config_path())
        .or_else(|_| fs::read_to_string(config_dir().join("encre/config")))
        .unwrap_or_default();
    let mut lines = text.lines().filter(|l| !l.is_empty()).map(PathBuf::from);
    (lines.next(), lines.collect())
}

pub fn save_config(vault: &Path, recent: &[PathBuf]) {
    let path = config_path();
    let text: String = std::iter::once(vault)
        .chain(recent.iter().map(PathBuf::as_path))
        .map(|p| format!("{}\n", p.display()))
        .collect();
    let result = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|_| fs::write(&path, text));
    if let Err(e) = result {
        eprintln!(
            "inkuno: {} ({}): {e}",
            tr("config not saved", "config non enregistrée"),
            path.display()
        );
    }
}

pub fn load_file(name: &str) -> String {
    // A file left under the first name is read when the current one is absent.
    fs::read_to_string(config_dir().join("inkuno").join(name))
        .or_else(|_| fs::read_to_string(config_dir().join("encre").join(name)))
        .unwrap_or_default()
}

pub fn save_file(name: &str, text: &str) {
    let path = config_dir().join("inkuno").join(name);
    // Mere convenience: if writing fails, the app will reopen with its original settings.
    let _ = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|_| fs::write(path, text));
}

/// Layout of the navigation panel, kept as is on one line.
pub fn load_layout() -> String {
    load_file("layout")
}

pub fn save_layout(text: &str) {
    save_file("layout", text)
}

/// Chosen appearance (theme, fonts, size): one `key=value` line per setting.
pub fn load_settings() -> String {
    load_file("settings")
}

pub fn save_settings(text: &str) {
    save_file("settings", text)
}

/// Agent-editable JSON config: `keybindings.json` (user shortcuts) and `themes.json`
/// (custom themes). They live next to the other config files:
/// `~/.config/inkuno/` on Linux, `~/Library/Application Support/inkuno/` on macOS,
/// `%APPDATA%\inkuno\` on Windows. A present JSON file wins over the legacy `keys`
/// and `settings` files; a faulty entry is ignored on its own (see `ui::keys::read_json`
/// and `ui::theme::parse_themes`).
pub fn keybindings_path() -> PathBuf {
    config_dir().join("inkuno/keybindings.json")
}

pub fn themes_path() -> PathBuf {
    config_dir().join("inkuno/themes.json")
}

/// Contents of `keybindings.json`, empty when the file is absent.
pub fn load_keybindings() -> String {
    // Reads the first name's file when the current one is absent.
    fs::read_to_string(keybindings_path())
        .or_else(|_| fs::read_to_string(config_dir().join("encre/keybindings.json")))
        .unwrap_or_default()
}

pub fn save_keybindings(text: &str) {
    let path = keybindings_path();
    // Mere convenience: if writing fails, the legacy `keys` file stays in force.
    let _ = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|_| fs::write(path, text));
}

/// Contents of `themes.json`, empty when the file is absent.
pub fn load_themes() -> String {
    // Same fallback as the other agent-editable JSON.
    fs::read_to_string(themes_path())
        .or_else(|_| fs::read_to_string(config_dir().join("encre/themes.json")))
        .unwrap_or_default()
}

pub fn save_themes(text: &str) {
    let path = themes_path();
    let _ = path
        .parent()
        .map_or(Ok(()), fs::create_dir_all)
        .and_then(|_| fs::write(path, text));
}

/// Folder offered for a first vault: `Notes` in the user's Documents, or
/// in their home folder if they have no Documents.
pub fn default_vault() -> Option<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from)?;
    let documents = home.join("Documents");
    Some(if documents.is_dir() { documents } else { home }.join("Notes"))
}

/// A path as shown: the home folder shortened to `~` outside Windows.
pub fn shown(path: &Path) -> String {
    let home = env::var_os("HOME").map(PathBuf::from);
    match home
        .as_deref()
        .and_then(|home| path.strip_prefix(home).ok())
    {
        Some(rest) => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

/// Piece of a name for natural sort: a number compares by its value, before text.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Chunk {
    Number(u128),
    Text(String),
}

/// "Natural" sort key, like file managers use: `Note 2` before
/// `Note 10`, ignoring case.
pub fn natural(name: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut rest = name;
    while let Some(first) = rest.chars().next() {
        let digit = first.is_ascii_digit();
        let end = rest
            .find(|c: char| c.is_ascii_digit() != digit)
            .unwrap_or(rest.len());
        let (part, tail) = rest.split_at(end);
        chunks.push(match part.parse() {
            Ok(n) if digit => Chunk::Number(n),
            _ => Chunk::Text(part.to_lowercase()),
        });
        rest = tail;
    }
    chunks
}

pub fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Name of an image in links: its file name, extension included. (Moved here when the
/// graph view was cut; it was `graph::image_name`.)
pub fn image_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// The vault's templates folder: `Templates` or `Modèles`, among its folders.
#[cfg(test)]
pub fn template_dir(root: &Path, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .find(|d| {
            d.parent() == Some(root)
                && d.file_name()
                    .is_some_and(|n| note::is_template_folder(&n.to_string_lossy()))
        })
        .cloned()
}

/// The note is a template: it is in the templates folder, however deep under it. A template
/// is neither in the graph nor in tags and links: its `{{words}}` are not links.
pub fn in_templates(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(note::is_template)
}

/// The file is an image the app can display.
pub fn is_image(path: &Path) -> bool {
    let known = [
        "png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "tif", "tiff", "avif",
    ];
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| known.iter().any(|k| e.eq_ignore_ascii_case(k)))
}

/// Files kept in the vault without being read: they are listed in the tree, found in
/// the palette, targeted by `[[name.pdf]]` and opened in the system's application.
const ATTACHMENTS: [&str; 34] = [
    "pdf", "mp4", "m4v", "mov", "mkv", "webm", "avi", "mp3", "m4a", "wav", "ogg", "opus", "flac",
    "zip", "7z", "tar", "gz", "rar", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods",
    "odp", "epub", "txt", "json", "xml", "html", "rtf", "log",
];

pub fn is_attachment(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ATTACHMENTS.iter().any(|k| e.eq_ignore_ascii_case(k)))
}

/// The file is an image, a table or an attachment: what the tree shows besides notes.
pub fn is_file_shown(path: &Path) -> bool {
    is_image(path) || is_table(path) || is_attachment(path)
}

/// The file is a table (CSV or TSV): listed with the notes, opened as plain text.
pub fn is_table(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| ["csv", "tsv"].iter().any(|k| e.eq_ignore_ascii_case(k)))
}

/// The images among the vault's files (the graph and `![[…]]` links ignore tables).
pub fn pictures(files: &[PathBuf]) -> Vec<PathBuf> {
    files.iter().filter(|p| is_image(p)).cloned().collect()
}

/// True the first time this folder is entered. A symbolic link that points back up to a
/// parent folder (or two links to the same one) must neither loop nor duplicate notes.
fn first_visit(visited: &mut HashSet<PathBuf>, dir: &Path) -> bool {
    visited.insert(fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf()))
}

/// The entry is a folder, link followed. Its type comes with the folder listing: the disk
/// is queried only for a link, not once per file in the vault.
fn is_dir(entry: &fs::DirEntry) -> bool {
    entry
        .file_type()
        .is_ok_and(|kind| kind.is_dir() || (kind.is_symlink() && entry.path().is_dir()))
}

/// The folders of the app's own state: its config folder (layout, keys, and the scratch notes, which
/// live under it) and the scratch folder. A vault can hold them: on macOS the config folder is under
/// the home folder, and the home folder can be a vault. Their files are not the vault's.
fn app_folders() -> [PathBuf; 2] {
    [config_dir().join("inkuno"), scratch::dir()]
}

/// The app's own folders that lie inside `root`, each as given and as the disk spells it (the same
/// two spellings as `history_folders`: the walks compare them with the paths they visit, which
/// start from `root` as given, and the watch with the events, which the system reports canonical).
fn app_folders_in(root: &Path) -> Vec<PathBuf> {
    let real_root = real(root);
    let mut folders: Vec<PathBuf> = Vec::new();
    for app in app_folders() {
        let Ok(rel) = real(&app).strip_prefix(&real_root).map(Path::to_path_buf) else {
            continue;
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        for spelling in [root.join(&rel), real_root.join(&rel)] {
            if !folders.contains(&spelling) {
                folders.push(spelling);
            }
        }
    }
    folders
}

/// The vault's root as given and as the disk spells it, once each when they are the same. Built
/// once, when the watch starts (see `history_folders`): the events carry the canonical spelling.
fn watch_roots(root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![root.to_path_buf()];
    let canonical = real(root);
    if !roots.contains(&canonical) {
        roots.push(canonical);
    }
    roots
}

/// Whether `path` is one of the app's own files, scratch notes included.
pub fn is_app_file(path: &Path) -> bool {
    app_folders().iter().any(|app| path.starts_with(app))
}

/// Whether `path` is in one of the `skip` folders, or is one of them.
fn under(path: &Path, skip: &[PathBuf]) -> bool {
    skip.iter().any(|folder| path.starts_with(folder))
}

/// The path as the disk spells it: canonical when it exists, else the canonical spelling of its
/// nearest existing folder with the rest appended. Used to compare the vault with the app folders.
fn real(path: &Path) -> PathBuf {
    match fs::canonicalize(path) {
        Ok(path) => path,
        Err(_) => match (path.parent(), path.file_name()) {
            (Some(parent), Some(name)) => real(parent).join(name),
            _ => path.to_path_buf(),
        },
    }
}

/// Fingerprint of the vault's state: the paths of its notes, images and folders,
/// and the date of each file. It changes as soon as another program touches it, without
/// reading a single file.
pub fn fingerprint(root: &Path) -> u64 {
    let mut seen = Vec::new();
    let skip = app_folders_in(root);
    let mut visited = HashSet::new();
    first_visit(&mut visited, root);
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') || under(&path, &skip) {
                continue;
            }
            if is_dir(&entry) {
                if first_visit(&mut visited, &path) {
                    seen.push((path.clone(), None));
                    dirs.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "md") {
                seen.push((path, entry.metadata().and_then(|m| m.modified()).ok()));
            } else if is_image(&path) || is_table(&path) {
                // Another program rewriting the open table or diagram must be seen.
                seen.push((path, entry.metadata().and_then(|m| m.modified()).ok()));
            }
        }
    }
    // The order in which a folder is read is not guaranteed.
    seen.sort();
    let mut hasher = DefaultHasher::new();
    seen.hash(&mut hasher);
    hasher.finish()
}

/// A vault change worth rereading the disk for: not a mere read,
/// and not a hidden file or folder (`.trash`, `.git`…). `roots` are the vault's root in each
/// spelling (see `watch_roots`): an event may carry either one.
fn matters(roots: &[PathBuf], event: &Event) -> bool {
    let hidden = |path: &PathBuf| {
        roots.iter().any(|root| {
            path.strip_prefix(root).is_ok_and(|rel| {
                rel.components()
                    .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
            })
        })
    };
    !event.kind.is_access() && (event.paths.is_empty() || !event.paths.iter().all(hidden))
}

/// Whether a system event means the vault must be reread: an error (events may be lost), or a
/// change that matters outside the app's own folders (a scratch note saved is no vault change) and
/// outside the revisions (`history`, see `history_folders`: they are no vault change either).
fn reread_for(
    roots: &[PathBuf],
    skip: &[PathBuf],
    history: &[PathBuf],
    event: &notify::Result<Event>,
) -> bool {
    match event {
        Err(_) => true,
        Ok(event) => {
            let own = !event.paths.is_empty()
                && event
                    .paths
                    .iter()
                    .all(|p| under(p, skip) || under(p, history));
            matters(roots, event) && !own
        }
    }
}

/// The revision folders of the vault (`.inkuno/revisions`) and of the scratch folder (`.revisions`),
/// each as given and as the disk spells it (canonical: on macOS `/var` is `/private/var`, and the
/// system reports events in that spelling even when the vault was opened through a symbolic link).
/// Built once, when the watch starts: the event callback must not touch the disk.
fn history_folders(root: &Path) -> Vec<PathBuf> {
    let mut folders: Vec<PathBuf> = Vec::new();
    for folder in [
        root.join(revision::VAULT_DIR),
        scratch::dir().join(revision::SCRATCH_DIR),
    ] {
        for spelling in [real(&folder), folder] {
            if !folders.contains(&spelling) {
                folders.push(spelling);
            }
        }
    }
    folders
}

/// Whether a system event touches the revisions, in the `history` folders (see `history_folders`):
/// `inkuno rev save` writes them from a terminal, and they are hidden, so `matters` ignores them.
/// Such an event marks the Versions list stale, and nothing else is reread.
/// An error may have lost one, so it counts too.
fn revisions_moved(history: &[PathBuf], event: &notify::Result<Event>) -> bool {
    match event {
        Err(_) => true,
        Ok(event) => event.paths.iter().any(|p| under(p, history)),
    }
}

/// A signal that wakes the task waiting on it, without it having to poll: the system's
/// thread raises it, the task sleeps until then.
#[derive(Default)]
pub struct Signal {
    raised: AtomicBool,
    waker: std::sync::Mutex<Option<std::task::Waker>>,
}

impl Signal {
    pub fn raise(&self) {
        self.raised.store(true, Ordering::Release);
        if let Some(waker) = self.waker.lock().unwrap().take() {
            waker.wake();
        }
    }

    /// True, and the signal drops, if it was raised since the last call.
    pub fn take(&self) -> bool {
        self.raised.swap(false, Ordering::AcqRel)
    }

    /// Waits until the signal is raised (true) or `deadline` passes (false).
    pub async fn wait<D: std::future::Future<Output = ()> + Unpin>(&self, mut deadline: D) -> bool {
        std::future::poll_fn(|cx| {
            if self.take() {
                return std::task::Poll::Ready(true);
            }
            *self.waker.lock().unwrap() = Some(cx.waker().clone());
            // Raised in the meantime, before the waker is in place: it is not missed.
            if self.take() {
                return std::task::Poll::Ready(true);
            }
            std::pin::Pin::new(&mut deadline).poll(cx).map(|()| false)
        })
        .await
    }
}

/// Asks the system to report any change in the vault: `changed` is raised
/// as soon as there is one, and `history` when a revision (`.inkuno/revisions`) changes, which
/// raises no other signal. `None` if the system cannot do it (inotify limit
/// reached, network folder): the caller then rereads the vault at regular intervals.
/// Watching stops when the result is dropped.
pub fn watch(
    root: &Path,
    changed: Arc<Signal>,
    history: Arc<Signal>,
) -> Option<RecommendedWatcher> {
    let roots = watch_roots(root);
    let skip = app_folders_in(root);
    let history_dirs = history_folders(root);
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        if revisions_moved(&history_dirs, &event) {
            history.raise();
        }
        if reread_for(&roots, &skip, &history_dirs, &event) {
            changed.raise();
        }
    })
    .ok()?;
    watcher.watch(root, RecursiveMode::Recursive).ok()?;
    Some(watcher)
}

/// The config files the app reloads while it runs (see `watch_config`), by file name.
pub fn is_live_config_name(name: &str) -> bool {
    matches!(name, "keybindings.json" | "themes.json" | "settings")
}

/// Asks the system to report the changes of the config files (`is_live_config_name`) in the
/// configuration folder: `on_change` runs for each event on one of them, nothing else. The
/// folder is created if missing. `None` if the system cannot watch it. Watching stops when
/// the result is dropped.
pub fn watch_config(on_change: impl Fn() + Send + 'static) -> Option<RecommendedWatcher> {
    let dir = config_dir().join("inkuno");
    fs::create_dir_all(&dir).ok()?;
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
        let Ok(event) = event else { return };
        if event.kind.is_access() {
            return;
        }
        let live = event.paths.iter().any(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_live_config_name)
        });
        if live {
            on_change();
        }
    })
    .ok()?;
    watcher.watch(&dir, RecursiveMode::NonRecursive).ok()?;
    Some(watcher)
}

/// All the vault's notes (recursive, hidden folders ignored), most recent
/// first, all its folders, even empty, and its other files: images and tables.
#[cfg(any(test, feature = "test-support"))]
pub fn scan(root: &Path) -> (Vec<Note>, Vec<PathBuf>, Vec<PathBuf>) {
    rescan(root, &[])
}

/// Like `scan`, without rereading the notes of `known` whose date has not changed.
pub fn rescan(root: &Path, known: &[Note]) -> (Vec<Note>, Vec<PathBuf>, Vec<PathBuf>) {
    let (notes, dirs, images, _) = rescan_full(root, known);
    (notes, dirs, images)
}

/// Folders that could not be listed, and why.
pub type Unreadable = Vec<(PathBuf, io::ErrorKind)>;

/// Like `rescan`, with the folders that could not be listed and why: the root itself when the whole
/// vault cannot be read. A folder inside one of them is not reached, so not reported.
pub fn rescan_full(
    root: &Path,
    known: &[Note],
) -> (Vec<Note>, Vec<PathBuf>, Vec<PathBuf>, Unreadable) {
    let mut unreadable = Vec::new();
    let known: HashMap<&Path, &Note> = known.iter().map(|n| (n.path.as_path(), n)).collect();
    let mut files = Vec::new();
    let mut images = Vec::new();
    let mut found = Vec::new();
    let skip = app_folders_in(root);
    let mut visited = HashSet::new();
    first_visit(&mut visited, root);
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) => {
                unreadable.push((dir, e.kind()));
                continue;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') || under(&path, &skip) {
                continue;
            }
            if is_dir(&entry) {
                if first_visit(&mut visited, &path) {
                    found.push(path.clone());
                    dirs.push(path);
                }
            } else if path.extension().is_some_and(|e| e == "md") {
                files.push((
                    path,
                    entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(UNIX_EPOCH),
                ));
            } else if is_file_shown(&path) {
                images.push(path);
            }
        }
    }
    let note = |(path, mtime): &(PathBuf, SystemTime)| {
        let (tags, links, aliases, body, lower) = match known.get(path.as_path()) {
            Some(note) if note.mtime == *mtime => (
                note.tags.clone(),
                note.links.clone(),
                note.aliases.clone(),
                note.body.clone(),
                note.lower.clone(),
            ),
            _ => {
                let text = fs::read_to_string(path).unwrap_or_default();
                let (tags, links) = markdown::index(&text);
                let body = Arc::from(text);
                let lower = lowered(&body);
                (
                    tags.into(),
                    links.into(),
                    markdown::aliases(&body).into(),
                    body,
                    lower,
                )
            }
        };
        // A template has no tags, links or aliases: what it writes between `{{ }}` or after `#` is not its own.
        let (tags, links, aliases) = if in_templates(root, path) {
            (Vec::new().into(), Vec::new().into(), Vec::new().into())
        } else {
            (tags, links, aliases)
        };
        Note {
            name: stem(path),
            tags,
            links,
            aliases,
            mtime: *mtime,
            path: path.clone(),
            body,
            lower,
        }
    };
    let mut notes: Vec<Note> = files.iter().map(note).collect();
    notes.sort_by_key(|note| std::cmp::Reverse(note.mtime));
    (notes, found, images, unreadable)
}

/// File or folder name stripped of what a file system or
/// a wikilink refuses; `None` if nothing is left.
pub fn clean_name(name: &str) -> Option<String> {
    let name: String = name
        .chars()
        .filter(|c| !c.is_control() && !r#"/\:*?"<>|#^[]"#.contains(*c))
        .take(80)
        .collect();
    // No leading dot (hidden file) nor trailing one (refused by Windows).
    let name = name.trim().trim_matches('.').trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Moves or renames, never overwriting what already has that name.
pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
    if to.exists() && !same_file(from, to) {
        let taken = tr("this name is already taken", "ce nom est déjà pris");
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, taken));
    }
    fs::rename(from, to)
}

/// Whether two paths name one entry of the disk: the same device and inode on Unix, the same
/// canonical path elsewhere. On a case-insensitive disk `Zz.md` and `zz.md` are one entry, so a
/// case-only rename is not a clash.
#[cfg(unix)]
pub fn same_file(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (a.metadata(), b.metadata()) {
        (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
        _ => false,
    }
}

/// Whether two paths name one entry of the disk: the same canonical path.
#[cfg(not(unix))]
pub fn same_file(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Renames a note and returns its path. If its name followed its title, the
/// first line is rewritten so they stay in step. The note's pending changes move with it, and the
/// new title is written as the editor shows the note (`sidecar::edit`), so a change placed under the
/// heading stays placed, as the command line's rename does.
pub fn rename_note(vault: Option<&Path>, path: &Path, name: &str) -> io::Result<PathBuf> {
    fs::read_to_string(path)?;
    let to = path.with_file_name(format!("{name}.md"));
    rename(path, &to)?;
    // The list follows the note; if it cannot, the note goes back to its name.
    if let Err(e) = sidecar::move_along(vault, path, &to) {
        let _ = rename(&to, path);
        return Err(e);
    }
    let old = stem(path);
    let home = revisions::home_of(&to, vault);
    sidecar::edit(home.as_ref(), &to, "", |shown| {
        // The front matter stays as is: the title is the first line after it.
        let (meta, body) = shown.split_at(markdown::front_matter(shown));
        let first = body.lines().find(|l| !l.trim().is_empty())?;
        if old != title_of(shown) {
            return None;
        }
        let hashes = first.trim_start().chars().take_while(|c| *c == '#').count();
        let title = if hashes > 0 {
            format!("{} {name}", "#".repeat(hashes))
        } else {
            name.to_string()
        };
        Some(format!("{meta}{}", body.replacen(first, &title, 1)))
    })?;
    Ok(to)
}

/// Moves the note or folder to the vault's trash: the hidden folder
/// `.trash`, which the index ignores. Nothing is destroyed.
pub fn trash(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let bin = root.join(".trash");
    fs::create_dir_all(&bin)?;
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let free = bin.join(trash::name(&name, |c| bin.join(c).exists()));
    fs::rename(path, &free)?;
    Ok(free)
}

/// Vault file that keeps the icons: one `icon<tab>path` line per item,
/// the path relative to the vault and written with `/`. It travels with the vault.
const ICONS: &str = ".inkuno-icons";

/// Icons chosen for notes, files and folders of the vault, by path.
pub fn load_icons(root: &Path) -> HashMap<PathBuf, String> {
    // An older list under the first name is read when the current one is absent.
    let text = fs::read_to_string(root.join(ICONS))
        .or_else(|_| fs::read_to_string(root.join(".encre-icons")))
        .unwrap_or_default();
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(icon, path)| (root.join(path), icon.to_string()))
        .collect()
}

pub fn save_icons(root: &Path, icons: &HashMap<PathBuf, String>) -> io::Result<()> {
    let line = |(path, icon): (&PathBuf, &String)| {
        let inside = path
            .strip_prefix(root)
            .ok()?
            .to_string_lossy()
            .replace('\\', "/");
        Some(format!("{icon}\t{inside}\n"))
    };
    let mut lines: Vec<String> = icons.iter().filter_map(line).collect();
    lines.sort();
    write(&root.join(ICONS), lines.concat())
}

/// The commands to try, in order, to open a terminal in `dir` under the system
/// `os` (`std::env::consts::OS`): the program and its arguments. `preferred` is the terminal
/// the user names in `$TERMINAL`.
pub fn terminals(os: &str, dir: &str, preferred: Option<&str>) -> Vec<(String, Vec<String>)> {
    let command = |program: &str, args: &[&str]| {
        (
            program.to_string(),
            args.iter().map(|a| a.to_string()).collect(),
        )
    };
    match os {
        "macos" => vec![command("open", &["-a", "Terminal", dir])],
        // Windows Terminal if installed, otherwise the classic console.
        "windows" => vec![
            command("wt", &["-d", dir]),
            command("cmd", &["/c", "start", "cmd"]),
        ],
        // Linux has no single answer. Those that merely notify an already running instance
        // ignore the current folder: it is given to them as an argument.
        _ => {
            let mut all: Vec<_> = preferred.iter().map(|p| command(p, &[])).collect();
            all.extend([
                command("xdg-terminal-exec", &[]),
                command("x-terminal-emulator", &[]),
                command("gnome-terminal", &["--working-directory", dir]),
                command("ptyxis", &["--new-window", "-d", dir]),
                command("kgx", &["--working-directory", dir]),
                command("konsole", &["--workdir", dir]),
                command("kitty", &[]),
                command("alacritty", &[]),
                command("foot", &[]),
                command("wezterm", &["start", "--cwd", dir]),
                command("xterm", &[]),
            ]);
            all
        }
    }
}

/// Opens the system terminal in `dir`: the first of `terminals` that starts. False if
/// none is installed. Tests open nothing: `cfg!(test)` is true only in this crate's own tests, so the
/// app's tests (which enable `test-support`) need the feature too.
pub fn open_terminal(dir: &Path) -> bool {
    let preferred = env::var("TERMINAL").ok().filter(|t| !t.trim().is_empty());
    let found = terminals(
        env::consts::OS,
        &dir.to_string_lossy(),
        preferred.as_deref(),
    );
    !cfg!(any(test, feature = "test-support"))
        && found.into_iter().any(|(program, args)| {
            // No console for the launcher (`cmd /c start`); the terminal opens its own.
            let child = crate::command(&program)
                .args(args)
                .current_dir(dir)
                .stdin(std::process::Stdio::null())
                .spawn();
            // Once the terminal is closed, its process is reaped: no zombie while Inkuno runs.
            child
                .map(|mut child| std::thread::spawn(move || child.wait()))
                .is_ok()
        })
}

/// Backup: the whole vault (trash included) in a `<vault> <date>.tar.gz` archive
/// in folder `dir`, numbered if the name is taken. Blocking: run off the UI thread.
// The archive is written here rather than by the system's `tar`: on Windows, both the one
// Windows ships and Git for Windows' crash (access violation) on a name outside the ANSI
// code page, such as `中文.md`. flate2 compresses; the tar format is a few headers.
pub fn backup(root: &Path, dir: &Path, date: &str) -> io::Result<PathBuf> {
    let Some(name) = root.file_name() else {
        return Err(io::Error::other(tr(
            "this vault has no parent folder",
            "ce coffre n'a pas de dossier parent",
        )));
    };
    // Inside the vault, the archive would contain itself.
    if dir.starts_with(root) {
        return Err(io::Error::other(tr(
            "choose a folder outside the vault",
            "choisir un dossier hors du coffre",
        )));
    }
    let to = free_path(dir, &format!("{} {date}", name.to_string_lossy()), "tar.gz");
    let written = fs::File::create_new(&to).and_then(|file| {
        let mut gz =
            flate2::write::GzEncoder::new(io::BufWriter::new(file), flate2::Compression::default());
        tar_entry(&mut gz, root, &name.to_string_lossy())?;
        // The end of a tar: two empty blocks.
        gz.write_all(&[0; 1024])?;
        gz.finish()?
            .into_inner()
            .map_err(|e| e.into_error())?
            .sync_all()
    });
    if let Err(e) = written {
        // An incomplete archive must not pass for a backup.
        let _ = fs::remove_file(&to);
        return Err(e);
    }
    Ok(to)
}

/// Writes `path` into the tar under `name`, and a folder's contents after it, by name.
fn tar_entry(out: &mut impl Write, path: &Path, name: &str) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_secs());
    #[cfg(unix)]
    let mode = std::os::unix::fs::PermissionsExt::mode(&meta.permissions()) & 0o7777;
    #[cfg(not(unix))]
    let mode = if meta.is_dir() { 0o755 } else { 0o644 };
    if meta.is_symlink() {
        let target = fs::read_link(path)?;
        return tar_header(
            out,
            name,
            b'2',
            mode,
            0,
            mtime,
            &target.to_string_lossy().replace('\\', "/"),
        );
    }
    if meta.is_dir() {
        tar_header(out, &format!("{name}/"), b'5', mode, 0, mtime, "")?;
        let mut entries: Vec<_> = fs::read_dir(path)?.collect::<io::Result<_>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            tar_entry(
                out,
                &entry.path(),
                &format!("{name}/{}", entry.file_name().to_string_lossy()),
            )?;
        }
        return Ok(());
    }
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    tar_header(out, name, b'0', mode, size, mtime, "")?;
    // Exactly `size` bytes, even if the file changes meanwhile: the header has already said how many.
    let copied = io::copy(&mut io::Read::take(&mut file, size), out)?;
    io::copy(&mut io::Read::take(io::repeat(0), size - copied), out)?;
    out.write_all(&vec![0; (512 - size % 512) as usize % 512])
}

/// A ustar header; preceded by a pax one (UTF-8 by definition) when the name, the link
/// or the size does not fit in its field.
fn tar_header(
    out: &mut impl Write,
    name: &str,
    kind: u8,
    mode: u32,
    size: u64,
    mtime: u64,
    link: &str,
) -> io::Result<()> {
    let fits = |s: &str, n: usize| s.is_ascii() && s.len() <= n;
    let mut pax = String::new();
    for (key, value, needed) in [
        ("path", name, !fits(name, 100)),
        ("linkpath", link, !fits(link, 100)),
        ("size", &size.to_string(), size >= 1 << 33),
    ] {
        if needed {
            // Each record starts with its own length, digits included.
            let body = format!(" {key}={value}\n");
            let mut len = body.len() + 1;
            while (len.to_string().len() + body.len()) != len {
                len = len.to_string().len() + body.len();
            }
            pax.push_str(&format!("{len}{body}"));
        }
    }
    if !pax.is_empty() {
        tar_header(
            out,
            "././@PaxHeader",
            b'x',
            0o644,
            pax.len() as u64,
            mtime,
            "",
        )?;
        out.write_all(pax.as_bytes())?;
        out.write_all(&vec![0; (512 - pax.len() % 512) % 512])?;
    }
    let mut block = [0u8; 512];
    let mut put = |at: usize, len: usize, bytes: &[u8]| {
        let n = bytes.len().min(len);
        block[at..at + n].copy_from_slice(&bytes[..n]);
    };
    // Past the field: the pax header above has the whole of it.
    let ascii = |s: &str| {
        s.chars()
            .map(|c| if c.is_ascii() { c } else { '_' })
            .collect::<String>()
    };
    put(0, 100, ascii(name).as_bytes());
    put(100, 8, format!("{:07o}\0", mode).as_bytes());
    put(108, 8, b"0000000\0");
    put(116, 8, b"0000000\0");
    put(
        124,
        12,
        format!("{:011o}\0", size.min((1 << 33) - 1)).as_bytes(),
    );
    put(
        136,
        12,
        format!("{:011o}\0", mtime.min((1 << 33) - 1)).as_bytes(),
    );
    put(148, 8, b"        ");
    put(156, 1, &[kind]);
    put(157, 100, ascii(link).as_bytes());
    put(257, 8, b"ustar\x0000");
    let sum: u32 = block.iter().map(|&b| b as u32).sum();
    block[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    out.write_all(&block)
}

/// Contents of the vault's trash, most recently modified first.
pub fn trashed(root: &Path) -> Vec<PathBuf> {
    let entries = fs::read_dir(root.join(".trash"))
        .into_iter()
        .flatten()
        .flatten();
    let mut found: Vec<(SystemTime, PathBuf)> = entries
        .map(|e| {
            (
                e.metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(UNIX_EPOCH),
                e.path(),
            )
        })
        .collect();
    found.sort_by(|a, b| b.cmp(a));
    found.into_iter().map(|(_, path)| path).collect()
}

/// Takes `path` out of the trash and puts it back at the vault root (the trash does not keep
/// where it came from), under its name, numbered if taken: nothing is overwritten.
pub fn restore(root: &Path, path: &Path) -> io::Result<PathBuf> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    let free = (1..)
        .map(|n| match n {
            1 => root.join(format!("{stem}{extension}")),
            n => root.join(format!("{stem} {n}{extension}")),
        })
        .find(|p| !p.exists())
        .unwrap();
    fs::rename(path, &free)?;
    Ok(free)
}

/// Free path in `dir` for a file `name.extension`: the name is followed
/// by a number if already taken.
pub fn free_path(dir: &Path, name: &str, extension: &str) -> PathBuf {
    let numbered = (1..).map(|n| match n {
        1 => dir.join(format!("{name}.{extension}")),
        n => dir.join(format!("{name} {n}.{extension}")),
    });
    numbered.into_iter().find(|p| !p.exists()).unwrap()
}

/// The line with its comment markup taken out and the commented text kept: `# Plan<!-- comment: x -->`
/// reads `# Plan`, so a commented title does not put the comment into the file name.
fn without_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    for (whole, noted, _) in markdown::comments(line) {
        out.push_str(&line[at..noted.end]);
        at = whole.end;
    }
    out.push_str(&line[at..]);
    out
}

/// File name (without extension) taken from the first non-empty line.
pub fn title_of(content: &str) -> String {
    let content = &content[markdown::front_matter(content)..];
    let first = content.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    clean_name(without_comments(first).trim().trim_start_matches('#'))
        .unwrap_or_else(|| tr("Untitled", "Sans titre").to_string())
}

/// `# …` title at the top of the note, as a file name.
pub fn h1_of(content: &str) -> Option<String> {
    let content = &content[markdown::front_matter(content)..];
    let first = content.lines().find(|l| !l.trim().is_empty())?;
    clean_name(without_comments(first.trim_start()).strip_prefix("# ")?)
}

/// A dropped item, stored in the vault: where it is, whether it is a folder, how many files it holds.
#[derive(Debug, PartialEq)]
#[cfg(test)]
pub struct Imported {
    pub to: PathBuf,
    pub dir: bool,
    pub files: usize,
}

/// A free name in `dir`: `name.ext`, then `name 2.ext`, `name 3.ext`… Nothing is ever overwritten.
#[cfg(test)]
fn free_name(dir: &Path, name: &std::ffi::OsStr, folder: bool) -> PathBuf {
    let as_path = Path::new(name);
    let (stem, ext) = match (folder, as_path.extension()) {
        (false, Some(ext)) => (
            as_path
                .file_stem()
                .unwrap_or(name)
                .to_string_lossy()
                .into_owned(),
            Some(ext.to_string_lossy().into_owned()),
        ),
        _ => (name.to_string_lossy().into_owned(), None),
    };
    (1..)
        .map(|n| {
            let numbered = if n == 1 {
                stem.clone()
            } else {
                format!("{stem} {n}")
            };
            dir.join(match &ext {
                Some(ext) => format!("{numbered}.{ext}"),
                None => numbered,
            })
        })
        .find(|path| !path.exists() && path.symlink_metadata().is_err())
        .unwrap()
}

/// Copies `from` to `to`: a file is first written under a hidden name then renamed, so an interrupted
/// copy never leaves a half file. Returns the number of files copied and of items
/// left out (symbolic links, hidden files, things other than files).
#[cfg(test)]
fn copy_into(from: &Path, to: &Path, skipped: &mut usize) -> io::Result<usize> {
    if from.is_dir() {
        fs::create_dir(to)?;
        let mut entries: Vec<_> = fs::read_dir(from)?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        let mut files = 0;
        for entry in entries {
            let (path, kind) = (entry.path(), entry.file_type()?);
            if entry.file_name().to_string_lossy().starts_with('.')
                || kind.is_symlink()
                || !(kind.is_file() || kind.is_dir())
            {
                *skipped += 1;
                continue;
            }
            files += copy_into(&path, &to.join(entry.file_name()), skipped)?;
        }
        return Ok(files);
    }
    let part = to.with_file_name(format!(
        ".{}.inkuno-part",
        to.file_name().unwrap_or_default().to_string_lossy()
    ));
    let copied = fs::copy(from, &part).and_then(|_| fs::rename(&part, to));
    if copied.is_err() {
        let _ = fs::remove_file(&part);
    }
    copied.map(|()| 1)
}

/// Stores in `dir` files and folders from elsewhere (a drop): each under a free name, a
/// folder with everything it holds except what is hidden or a symbolic link. Returns what was stored and,
/// for each thing left out, why. A folder is never copied into itself.
#[cfg(test)]
pub fn import(sources: &[PathBuf], dir: &Path) -> (Vec<Imported>, Vec<String>) {
    let (mut done, mut problems) = (Vec::new(), Vec::new());
    for from in sources {
        let name = from.file_name().unwrap_or_default();
        let kind = from.symlink_metadata();
        let shown = from.display();
        match kind {
            Err(e) => problems.push(format!("{shown} : {e}")),
            Ok(kind) if kind.is_symlink() || !(kind.is_file() || kind.is_dir()) => {
                problems.push(format!("{shown} : not a file or a folder"))
            }
            Ok(_) if name.is_empty() || name.to_string_lossy().starts_with('.') => {
                problems.push(format!("{shown} : hidden"))
            }
            Ok(kind) if kind.is_dir() && dir.starts_with(from) => {
                problems.push(format!("{shown} : a folder cannot be copied into itself"))
            }
            Ok(kind) => {
                let to = free_name(dir, name, kind.is_dir());
                let mut skipped = 0;
                match copy_into(from, &to, &mut skipped) {
                    Ok(files) => {
                        if skipped > 0 {
                            problems.push(format!(
                                "{shown} : {skipped} hidden or linked items left out"
                            ));
                        }
                        done.push(Imported {
                            to,
                            dir: kind.is_dir(),
                            files,
                        });
                    }
                    Err(e) => {
                        // A half-copied folder goes entirely: nothing is left behind.
                        if kind.is_dir() {
                            let _ = fs::remove_dir_all(&to);
                        }
                        problems.push(format!("{shown} : {e}"));
                    }
                }
            }
        }
    }
    (done, problems)
}

/// Saves a pasted image in `dir`, under a free name, and returns that name.
pub fn save_image(dir: &Path, extension: &str, bytes: &[u8]) -> io::Result<String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let name = (1..)
        .map(|n| match n {
            1 => format!("image-{stamp}.{extension}"),
            n => format!("image-{stamp}-{n}.{extension}"),
        })
        .find(|name| !dir.join(name).exists())
        .unwrap();
    fs::write(dir.join(&name), bytes)?;
    Ok(name)
}

/// Atomic write: a crash never leaves a truncated note.
pub fn write(path: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    let tmp = path.with_file_name(format!(".{}.tmp", stem(path)));
    let mut file = fs::File::create(&tmp)?;
    file.write_all(content.as_ref())?;
    // Without `fsync`, a power cut can leave the rename without the data.
    file.sync_all()?;
    fs::rename(&tmp, path)
}

/// Saves the note and returns its path; a new note is created in
/// `dir`. If `synced` (the file name
/// already followed the title), the file is renamed when the title changes, unless
/// the new name is already taken.
pub fn save(
    dir: &Path,
    current: Option<&Path>,
    synced: bool,
    content: &str,
) -> io::Result<PathBuf> {
    let title = title_of(content);
    let target = match current {
        Some(p) if !synced || stem(p) == title => p.to_path_buf(),
        Some(p) => {
            let renamed = p.with_file_name(format!("{title}.md"));
            if renamed.exists() {
                p.to_path_buf()
            } else {
                renamed
            }
        }
        None => (1..)
            .map(|n| match n {
                1 => dir.join(format!("{title}.md")),
                n => dir.join(format!("{title} {n}.md")),
            })
            .find(|p| !p.exists())
            .unwrap(),
    };
    write(&target, content)?;
    if let Some(old) = current
        && old != target
    {
        fs::remove_file(old)?;
    }
    Ok(target)
}

/// Process-wide lock of the saves that write a note: a save on a background thread and one on the
/// UI thread never read and write the same note at once.
static SAVING: Mutex<()> = Mutex::new(());

/// Held while a note's file and its pending changes are read, checked and written. A poisoned lock
/// is taken anyway: the saves keep no state of their own.
pub fn save_lock() -> MutexGuard<'static, ()> {
    SAVING.lock().unwrap_or_else(|e| e.into_inner())
}

/// What `save_quiet` did.
#[derive(Debug)]
pub enum Quiet {
    /// Written: the text the file now holds, its pending changes split off.
    Saved { content: String },
    /// Not written: the file changed since `base`, so another program wrote it. The caller merges.
    Outside,
}

/// Saves an existing note without merging. `base` is the text last read or written from the file; if
/// the file still holds it, `shown` is written as `save` would write it, else nothing is written
/// (`Quiet::Outside`). It runs under `save_lock`, so it may run off the UI thread.
pub fn save_quiet(
    home: Option<&revisions::Home>,
    path: &Path,
    shown: &str,
    base: &str,
    author: &str,
) -> io::Result<Quiet> {
    let _guard = save_lock();
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        // A file that is gone is written back by the synchronous save, as it always was.
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Quiet::Outside),
        Err(e) => return Err(e),
    };
    if sidecar::display(home, path, &raw) != base {
        return Ok(Quiet::Outside);
    }
    let (content, entries) = sidecar::split(home, path, shown, author);
    write(path, &content)?;
    if let Some(home) = home {
        sidecar::store(home, path, &entries)?;
    }
    Ok(Quiet::Saved { content })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorts_names_naturally() {
        let mut names = vec![
            "Note 10",
            "note 2",
            "Note 1",
            "Notes",
            "2026-10-09",
            "2026-9-30",
            "Note 1b",
        ];
        names.sort_by_cached_key(|n| natural(n));
        assert_eq!(
            names,
            [
                "2026-9-30",
                "2026-10-09",
                "Note 1",
                "Note 1b",
                "note 2",
                "Note 10",
                "Notes"
            ]
        );
    }

    /// Runs a future on the current thread, which sleeps between wakeups.
    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        struct Unpark(std::thread::Thread);
        impl std::task::Wake for Unpark {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Arc::new(Unpark(std::thread::current())).into();
        let mut cx = std::task::Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            if let std::task::Poll::Ready(value) = future.as_mut().poll(&mut cx) {
                return value;
            }
            std::thread::park_timeout(std::time::Duration::from_secs(5));
        }
    }

    #[test]
    fn config_watch_filters_the_live_files() {
        for name in ["keybindings.json", "themes.json", "settings"] {
            assert!(is_live_config_name(name), "{name}");
        }
        for name in [
            "keys",
            "layout",
            "config",
            "crash",
            "themes.json~",
            "settings.tmp",
        ] {
            assert!(!is_live_config_name(name), "{name}");
        }
    }

    #[test]
    fn json_config_lives_next_to_the_other_files() {
        use std::path::Path;
        // Paths only: no file is written here, the real folder stays untouched.
        assert_eq!(
            keybindings_path(),
            config_dir().join("inkuno/keybindings.json")
        );
        assert_eq!(themes_path(), config_dir().join("inkuno/themes.json"));
        assert!(keybindings_path().ends_with(Path::new("inkuno/keybindings.json")));
        assert!(themes_path().ends_with(Path::new("inkuno/themes.json")));
        // Absent files read as empty: the legacy `keys` and `settings` stay in force.
        // (Only asserted when no JSON file exists on this machine.)
        if !keybindings_path().is_file() {
            assert!(load_keybindings().is_empty());
        }
        if !themes_path().is_file() {
            assert!(load_themes().is_empty());
        }
    }

    #[test]
    fn signal_wakes_a_sleeping_task() {
        let signal = Arc::new(Signal::default());
        // Raised before the wait: no sleep. Dropped afterwards.
        signal.raise();
        assert!(block_on(signal.wait(std::future::pending())));
        assert!(!signal.take());
        // Deadline passed, no signal: false.
        assert!(!block_on(signal.wait(std::future::ready(()))));
        // Raised by another thread while the task sleeps.
        let other = signal.clone();
        let thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            other.raise();
        });
        let start = std::time::Instant::now();
        assert!(block_on(signal.wait(std::future::pending())));
        assert!(start.elapsed() < std::time::Duration::from_secs(4));
        thread.join().unwrap();
    }

    #[test]
    fn derives_titles() {
        assert_eq!(title_of("\n# Courses: lundi\nlait"), "Courses lundi");
        assert_eq!(title_of("  [[x]] / y  "), "x  y");
        assert_eq!(title_of("\n\n"), "Untitled");
        assert_eq!(title_of("# .."), "Untitled");
        // A comment on the title is not part of the name: its text stays out, its passage stays in.
        assert_eq!(title_of("# Plan<!-- comment: check -->\nx"), "Plan");
        assert_eq!(title_of("<!-- comment: check -->\nx"), "Untitled");
        assert_eq!(h1_of("# Plan<!-- comment: check -->"), Some("Plan".into()));
        assert_eq!(h1_of("\n# Courses: lundi\n"), Some("Courses lundi".into()));
        assert_eq!(
            (h1_of("## Sous-titre"), h1_of("texte"), h1_of("# ")),
            (None, None, None)
        );
    }

    #[test]
    fn quiet_save_writes_only_while_the_disk_holds_the_base() {
        let root = env::temp_dir().join(format!("inkuno-quiet-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let path = root.join("Plan.md");
        fs::write(&path, "# Plan\n\nun\n").unwrap();
        // The disk still holds the base: the shown text is written.
        match save_quiet(None, &path, "# Plan\n\nun deux\n", "# Plan\n\nun\n", "me").unwrap() {
            Quiet::Saved { content } => assert_eq!(content, "# Plan\n\nun deux\n"),
            other => panic!("expected Saved, got {other:?}"),
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Plan\n\nun deux\n");
        // Another program wrote the file since: nothing is written, the merge is left to the caller.
        fs::write(&path, "# Plan\n\nautre\n").unwrap();
        let quiet = save_quiet(
            None,
            &path,
            "# Plan\n\nun deux trois\n",
            "# Plan\n\nun deux\n",
            "me",
        )
        .unwrap();
        assert!(matches!(quiet, Quiet::Outside));
        assert_eq!(fs::read_to_string(&path).unwrap(), "# Plan\n\nautre\n");
        // A file that is gone is left to the synchronous save, which writes it back.
        fs::remove_file(&path).unwrap();
        let quiet = save_quiet(None, &path, "# Plan\n", "# Plan\n", "me").unwrap();
        assert!(matches!(quiet, Quiet::Outside));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn saves_and_renames() {
        let root = env::temp_dir().join(format!("inkuno-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let a = save(&root, None, true, "# A\n").unwrap();
        assert_eq!(a, root.join("A.md"));
        // New note with the same title: no overwrite.
        assert_eq!(
            save(&root, None, true, "# A\n").unwrap(),
            root.join("A 2.md")
        );
        // The title changes: the file follows.
        let b = save(&root, Some(&a), true, "# B\n").unwrap();
        assert_eq!(b, root.join("B.md"));
        assert!(!a.exists());
        // Name already taken or note not synced: the file is kept.
        assert_eq!(save(&root, Some(&b), true, "# A 2\n").unwrap(), b);
        assert_eq!(save(&root, Some(&b), false, "# Z\n").unwrap(), b);
        assert_eq!(fs::read_to_string(&b).unwrap(), "# Z\n");
        assert_eq!(scan(&root).0.len(), 2);
        // Front matter: the name comes from the title after it, and renaming touches only that title.
        let meta = "---\ntitle: autre\n---\n";
        let c = save(&root, None, true, &format!("{meta}# Fiche\n---\n")).unwrap();
        assert_eq!(
            (c.clone(), h1_of(&format!("{meta}\n# Fiche\n"))),
            (root.join("Fiche.md"), Some("Fiche".into()))
        );
        let c = rename_note(None, &c, "Carte").unwrap();
        assert_eq!(
            fs::read_to_string(&c).unwrap(),
            format!("{meta}# Carte\n---\n")
        );
        fs::remove_file(&c).unwrap();
        // The vault's fingerprint follows what another program changes in it.
        let before = fingerprint(&root);
        assert_eq!(before, fingerprint(&root));
        fs::write(root.join("ailleurs.md"), "x").unwrap();
        assert_ne!(before, fingerprint(&root));
        fs::remove_file(root.join("ailleurs.md")).unwrap();
        // So does a drawing rewritten in place (an agent editing the open diagram), with no other file touched.
        let drawing = root.join("schéma.svg");
        fs::write(&drawing, "<svg/>").unwrap();
        let before = fingerprint(&root);
        let later =
            fs::metadata(&drawing).unwrap().modified().unwrap() + std::time::Duration::from_secs(1);
        fs::File::options()
            .write(true)
            .open(&drawing)
            .unwrap()
            .set_modified(later)
            .unwrap();
        assert_ne!(before, fingerprint(&root));
        fs::remove_file(&drawing).unwrap();

        // Renaming a note in step with its title rewrites that title; otherwise the text stays as is.
        let c = rename_note(None, &root.join("A 2.md"), "C").unwrap();
        assert_eq!(fs::read_to_string(&c).unwrap(), "# A\n");
        fs::write(root.join("D.md"), "\n## D\ntexte D\n").unwrap();
        let e = rename_note(None, &root.join("D.md"), "E").unwrap();
        assert_eq!(fs::read_to_string(&e).unwrap(), "\n## E\ntexte D\n");
        // Never overwrite.
        assert!(rename_note(None, &e, "C").is_err() && e.exists());

        // Trash: nothing is destroyed, and the index does not see it.
        fs::create_dir(root.join("vide")).unwrap();
        trash(&root, &c).unwrap();
        fs::write(&c, "autre").unwrap();
        trash(&root, &c).unwrap();
        assert_eq!(
            fs::read_to_string(root.join(".trash/C.md")).unwrap(),
            "# A\n"
        );
        assert_eq!(
            fs::read_to_string(root.join(".trash/2 C.md")).unwrap(),
            "autre"
        );
        fs::write(root.join("vide/Photo.PNG"), "").unwrap();
        let (notes, dirs, images) = scan(&root);
        assert_eq!((notes.len(), dirs), (2, vec![root.join("vide")]));
        // Each note keeps its text as is, and the same in lowercase for search.
        assert!(
            notes
                .iter()
                .all(|n| !n.body.is_empty() && *n.lower == n.body.to_lowercase())
        );
        assert!(notes.iter().any(|n| n.lower != n.body));
        assert_eq!(images, [root.join("vide/Photo.PNG")]);
        assert_eq!(clean_name(" ../a:b. "), Some("ab".into()));
        assert_eq!(clean_name(" . "), None);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn keeps_icons_with_the_vault() {
        let root = env::temp_dir().join(format!("inkuno-icons-{}", std::process::id()));
        fs::create_dir_all(root.join("Projets")).unwrap();
        assert!(load_icons(&root).is_empty());
        let icons = HashMap::from([
            (root.join("Projets"), "rocket".to_string()),
            (root.join("Projets/Idées.md"), "idea".to_string()),
        ]);
        save_icons(&root, &icons).unwrap();
        assert_eq!(
            fs::read_to_string(root.join(ICONS)).unwrap(),
            "idea\tProjets/Idées.md\nrocket\tProjets\n"
        );
        assert_eq!(load_icons(&root), icons);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn picks_the_terminal_of_the_system() {
        let programs = |os, preferred| {
            terminals(os, "/coffre", preferred)
                .into_iter()
                .map(|(p, _)| p)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            terminals("macos", "/coffre", None),
            [(
                "open".to_string(),
                vec!["-a".to_string(), "Terminal".into(), "/coffre".into()]
            )]
        );
        assert_eq!(programs("windows", Some("kitty")), ["wt", "cmd"]);
        // `$TERMINAL` comes first; without it, the desktop's choice first.
        assert_eq!(
            programs("linux", Some("foot"))[..2],
            ["foot", "xdg-terminal-exec"]
        );
        assert_eq!(programs("linux", None)[0], "xdg-terminal-exec");
        let gnome = terminals("linux", "/coffre", None)
            .into_iter()
            .find(|(p, _)| p == "gnome-terminal")
            .unwrap();
        assert_eq!(gnome.1, ["--working-directory", "/coffre"]);
    }

    #[test]
    fn backs_up_the_vault() {
        let base = env::temp_dir().join(format!("inkuno-backup-{}", std::process::id()));
        let (root, out) = (base.join("Mon coffre"), base.join("sauvegardes"));
        fs::create_dir_all(root.join(".trash")).unwrap();
        fs::create_dir_all(&out).unwrap();
        fs::write(root.join("A.md"), "# A\n").unwrap();
        fs::write(root.join(".trash/Vieux.md"), "jeté").unwrap();
        fs::write(root.join("abz文cd.md"), "# abz文cd\n").unwrap();
        let first = backup(&root, &out, "2026-10-09").unwrap();
        assert_eq!(first, out.join("Mon coffre 2026-10-09.tar.gz"));
        // Same day: a second archive, the first one stays.
        assert_eq!(
            backup(&root, &out, "2026-10-09").unwrap(),
            out.join("Mon coffre 2026-10-09 2.tar.gz")
        );
        let mut listed = Vec::new();
        io::Read::read_to_end(
            &mut flate2::read::GzDecoder::new(fs::File::open(&first).unwrap()),
            &mut listed,
        )
        .unwrap();
        let listed = String::from_utf8_lossy(&listed);
        assert!(
            listed.contains("Mon coffre/A.md")
                && listed.contains("Mon coffre/.trash/Vieux.md")
                && listed.contains("jeté"),
            "{listed}"
        );
        // A name outside the ANSI code page goes in a pax header: Windows' `tar` crashed on it.
        assert!(listed.contains("path=Mon coffre/abz文cd.md\n"), "{listed}");
        // Inside the vault, the archive would contain itself: refused, nothing is written.
        assert!(backup(&root, &root.join(".trash"), "2026-10-09").is_err());
        assert_eq!(fs::read_dir(root.join(".trash")).unwrap().count(), 1);
        fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn a_sidebar_rename_keeps_a_pending_change_under_the_title() {
        // The probe of the final check: a change right under the H1 stays placed after the rename, the
        // list moves with the note, and the title is written as the command line writes it.
        let root =
            std::env::temp_dir().join(format!("inkuno-rename-pending-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let home = revisions::Home::Vault(root.clone());
        let note = root.join("Alpha.md");
        fs::write(&note, "# Alpha\ntext\n").unwrap();
        sidecar::save(
            Some(&home),
            &note,
            "# Alpha\n{++\nnew\n++}\ntext\n",
            "assistant",
        )
        .unwrap();
        let to = rename_note(Some(&root), &note, "Alpha2").unwrap();
        let disk = fs::read_to_string(&to).unwrap();
        assert_eq!(disk, "# Alpha2\ntext\n");
        assert!(sidecar::unplaced(&home, &to, &disk).is_empty());
        assert_eq!(sidecar::read(&home, &to).len(), 1);
        assert!(sidecar::display(Some(&home), &to, &disk).starts_with("# Alpha2\n{++"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn restores_from_the_trash() {
        let root = env::temp_dir().join(format!("inkuno-restore-{}", std::process::id()));
        fs::create_dir_all(root.join("Dossier")).unwrap();
        assert!(trashed(&root).is_empty());
        for (name, text) in [("A.md", "premier"), ("Dossier/b.md", "dedans")] {
            fs::write(root.join(name), text).unwrap();
        }
        trash(&root, &root.join("A.md")).unwrap();
        trash(&root, &root.join("Dossier")).unwrap();
        // The name was taken in the meantime: the restored note gets another, nothing is overwritten.
        fs::write(root.join("A.md"), "second").unwrap();
        let mut names: Vec<String> = trashed(&root).iter().map(|p| stem(p)).collect();
        names.sort();
        assert_eq!(names, ["A", "Dossier"]);
        assert_eq!(
            restore(&root, &root.join(".trash/A.md")).unwrap(),
            root.join("A 2.md")
        );
        assert_eq!(
            restore(&root, &root.join(".trash/Dossier")).unwrap(),
            root.join("Dossier")
        );
        assert_eq!(fs::read_to_string(root.join("A.md")).unwrap(), "second");
        assert_eq!(fs::read_to_string(root.join("A 2.md")).unwrap(), "premier");
        assert_eq!(
            fs::read_to_string(root.join("Dossier/b.md")).unwrap(),
            "dedans"
        );
        assert!(trashed(&root).is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn ignores_reads_and_hidden_files() {
        use notify::{
            EventKind,
            event::{AccessKind, CreateKind},
        };
        let root = Path::new("/v");
        let roots = watch_roots(root);
        let event = |kind, path: &str| Event::new(kind).add_path(root.join(path));
        let created = EventKind::Create(CreateKind::File);
        assert!(matters(&roots, &event(created, "a.md")));
        assert!(matters(&roots, &event(created, "dossier/a.md")));
        assert!(!matters(&roots, &event(created, ".trash/a.md")));

        assert!(!matters(&roots, &event(created, ".git/objects/ab")));
        assert!(!matters(
            &roots,
            &event(EventKind::Access(AccessKind::Read), "a.md")
        ));
        // Outside the vault or without a path: when in doubt, reread.
        assert!(matters(
            &roots,
            &Event::new(created).add_path("/ailleurs/.x".into())
        ));
        assert!(matters(&roots, &Event::new(created)));
    }

    /// A revision event is for the Versions list only: no reread of the vault.
    #[test]
    fn a_revision_written_from_a_terminal_only_marks_the_versions_stale() {
        use notify::{EventKind, event::CreateKind};
        let root = Path::new("/v");
        let roots = watch_roots(root);
        let history = history_folders(root);
        let event =
            |path: &str| Event::new(EventKind::Create(CreateKind::File)).add_path(root.join(path));
        // `inkuno rev save` writes .inkuno/revisions/Plan.md/0002.md: the history signal, and no reread.
        let saved = event(".inkuno/revisions/Plan.md/0002.md");
        assert!(revisions_moved(&history, &Ok(saved.clone())));
        assert!(!matters(&roots, &saved));
        assert!(!reread_for(&watch_roots(root), &[], &history, &Ok(saved)));
        // A note, a hidden folder of another kind, and the vault's own bin: not revisions.
        assert!(!revisions_moved(&history, &Ok(event("Plan.md"))));
        assert!(!revisions_moved(&history, &Ok(event(".trash/a.md"))));
        assert!(!revisions_moved(&history, &Ok(event(".inkuno/trash/a.md"))));
        // An error may have lost a revision: the list is marked stale.
        assert!(revisions_moved(
            &history,
            &Err(notify::Error::generic("lost"))
        ));
    }

    /// The system reports a vault opened through a symbolic link under its canonical spelling
    /// (macOS: `/private/var` for `/var`). Its revisions must be recognised in that spelling too,
    /// and never raise a reread of the vault.
    #[cfg(unix)]
    #[test]
    fn a_revision_reported_under_its_canonical_spelling_is_recognised() {
        use notify::{EventKind, event::CreateKind};
        let base = env::temp_dir().join(format!("inkuno-rev-link-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let vault = base.join("vault");
        fs::create_dir_all(vault.join(revision::VAULT_DIR).join("Plan.md")).unwrap();
        let link = base.join("link");
        std::os::unix::fs::symlink(&vault, &link).unwrap();
        let canonical = fs::canonicalize(&vault).unwrap();
        // Given as the link: the folder in both spellings, once each when they are the same.
        let history = history_folders(&link);
        assert!(history.contains(&link.join(revision::VAULT_DIR)));
        assert!(history.contains(&canonical.join(revision::VAULT_DIR)));
        let created = EventKind::Create(CreateKind::File);
        let revision_event = Event::new(created)
            .add_path(canonical.join(revision::VAULT_DIR).join("Plan.md/0002.md"));
        assert!(revisions_moved(&history, &Ok(revision_event.clone())));
        assert!(!reread_for(
            &watch_roots(&link),
            &[],
            &history,
            &Ok(revision_event)
        ));
        // A note reached under the canonical spelling is still a vault change, and no revision.
        let note = Event::new(created).add_path(canonical.join("Plan.md"));
        assert!(!revisions_moved(&history, &Ok(note.clone())));
        assert!(reread_for(&watch_roots(&link), &[], &history, &Ok(note)));
        let _ = fs::remove_dir_all(&base);
    }

    /// A symbolic link that points back up to the vault, or a second link to an already
    /// walked folder, does not loop and duplicates no note.
    #[cfg(unix)]
    #[test]
    fn scan_survives_symlink_loops() {
        let root = env::temp_dir().join(format!("inkuno-loop-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("a/b")).unwrap();
        fs::write(root.join("a/b/Note.md"), "# Note\n").unwrap();
        std::os::unix::fs::symlink(&root, root.join("a/b/loop")).unwrap();
        std::os::unix::fs::symlink(root.join("a"), root.join("alias")).unwrap();
        let (notes, dirs, _) = scan(&root);
        assert_eq!(
            notes.len(),
            1,
            "{:?}",
            notes.iter().map(|n| &n.path).collect::<Vec<_>>()
        );
        assert!(dirs.len() <= 3, "{dirs:?}");
        // The fingerprint, too, is computed without looping and does not move.
        assert_eq!(fingerprint(&root), fingerprint(&root));
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn finds_the_templates_folder() {
        let root = Path::new("/v");
        let dirs: Vec<PathBuf> = ["/v/Notes", "/v/Modèles", "/v/Notes/Templates"]
            .into_iter()
            .map(PathBuf::from)
            .collect();
        assert_eq!(template_dir(root, &dirs), Some(PathBuf::from("/v/Modèles")));
        // Case and accents do not count; only a folder at the root is the templates folder.
        for name in ["templates", "TEMPLATES", "Modeles", "MODÈLES", "modèles"] {
            assert_eq!(
                template_dir(root, &[root.join(name)]),
                Some(root.join(name)),
                "{name}"
            );
        }
        assert_eq!(
            template_dir(
                root,
                &[
                    PathBuf::from("/v/Notes/Templates"),
                    root.join("Template"),
                    root.join("Modèle")
                ]
            ),
            None
        );
        assert_eq!(template_dir(root, &[]), None);
        // A template is a file under this folder, at any depth; the folder itself, a
        // file of the same name, a note beside it or elsewhere are not.
        assert!(in_templates(root, Path::new("/v/Templates/Réunion.md")));
        assert!(in_templates(root, Path::new("/v/modèles/Projets/Plan.md")));
        assert!(!in_templates(root, Path::new("/v/Templates")));
        assert!(!in_templates(root, Path::new("/v/Notes/Templates/A.md")));
        assert!(!in_templates(root, Path::new("/v/Réunion.md")));
        assert!(!in_templates(root, Path::new("/autre/Templates/A.md")));
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("inkuno-import-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn imports_files_and_folders_without_overwriting() {
        let base = scratch("a");
        let (vault, outside) = (base.join("coffre"), base.join("dehors"));
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(outside.join("Photos/été")).unwrap();
        fs::write(outside.join("Photos/a.png"), "a").unwrap();
        fs::write(outside.join("Photos/été/b.jpg"), "bé").unwrap();
        fs::write(outside.join("Photos/.cache"), "caché").unwrap();
        fs::write(outside.join("rapport.pdf"), "pdf").unwrap();
        fs::write(outside.join("vide.txt"), "").unwrap();
        fs::write(outside.join("sans-extension"), "x").unwrap();
        fs::write(outside.join("a.tar.gz"), "z").unwrap();
        fs::create_dir(outside.join("Dossier vide")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.join("rapport.pdf"), outside.join("Photos/lien.pdf"))
            .unwrap();
        let all: Vec<PathBuf> = [
            "rapport.pdf",
            "Photos",
            "vide.txt",
            "sans-extension",
            "a.tar.gz",
            "Dossier vide",
        ]
        .iter()
        .map(|n| outside.join(n))
        .collect();
        let (done, problems) = import(&all, &vault);
        let names: Vec<_> = done
            .iter()
            .map(|d| d.to.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "rapport.pdf",
                "Photos",
                "vide.txt",
                "sans-extension",
                "a.tar.gz",
                "Dossier vide"
            ]
        );
        assert_eq!(
            done.iter().map(|d| (d.dir, d.files)).collect::<Vec<_>>(),
            [
                (false, 1),
                (true, 2),
                (false, 1),
                (false, 1),
                (false, 1),
                (true, 0)
            ]
        );
        assert_eq!(
            fs::read_to_string(vault.join("Photos/été/b.jpg")).unwrap(),
            "bé"
        );
        // The hidden one and the symbolic link stay out, and we say so; the sources have not moved.
        assert!(!vault.join("Photos/.cache").exists() && !vault.join("Photos/lien.pdf").exists());
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(outside.join("Photos/a.png").is_file());
        // The same again: each name takes a number, files already there keep their content.
        fs::write(vault.join("rapport.pdf"), "le mien").unwrap();
        let (again, _) = import(&all, &vault);
        let names: Vec<_> = again
            .iter()
            .map(|d| d.to.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "rapport 2.pdf",
                "Photos 2",
                "vide 2.txt",
                "sans-extension 2",
                "a.tar 2.gz",
                "Dossier vide 2"
            ]
        );
        assert_eq!(
            fs::read_to_string(vault.join("rapport.pdf")).unwrap(),
            "le mien"
        );
        let (third, _) = import(&[outside.join("rapport.pdf")], &vault);
        assert_eq!(third[0].to, vault.join("rapport 3.pdf"));
        // No half copy is left behind.
        assert!(
            fs::read_dir(&vault)
                .unwrap()
                .flatten()
                .all(|e| !e.file_name().to_string_lossy().ends_with(".inkuno-part"))
        );
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn imports_refuse_what_cannot_be_copied() {
        let base = scratch("b");
        let vault = base.join("coffre");
        fs::create_dir_all(vault.join("Sous")).unwrap();
        fs::write(base.join("f.md"), "# f").unwrap();
        // A folder does not go into itself or into one of its subfolders; a missing source, a hidden name,
        // a missing destination folder: refused with the reason, the rest goes through.
        let (done, problems) = import(
            &[
                base.join("coffre"),
                vault.join("Sous"),
                base.join("absent.png"),
                base.join("f.md"),
            ],
            &vault.join("Sous"),
        );
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].to, vault.join("Sous/f.md"));
        assert_eq!(problems.len(), 3, "{problems:?}");
        assert!(problems[0].contains("itself") && problems[1].contains("itself"));
        let hidden = base.join(".cache");
        fs::write(&hidden, "x").unwrap();
        assert!(import(&[hidden], &vault).0.is_empty());
        let (none, problems) = import(&[base.join("f.md")], &base.join("nulle/part"));
        assert!(none.is_empty() && problems.len() == 1, "{problems:?}");
        assert_eq!(import(&[], &vault), (Vec::new(), Vec::new()));
        // Attachments are recognised by their extension, ignoring case.
        for name in ["a.PDF", "b.Mp4", "c.docx"] {
            assert!(
                is_attachment(Path::new(name)) && is_file_shown(Path::new(name)),
                "{name}"
            );
        }
        for name in ["a.md", "b", "c.exe", ".pdf"] {
            assert!(!is_attachment(Path::new(name)), "{name}");
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn a_vault_holding_the_app_folders_shows_none_of_their_files() {
        use notify::{EventKind, event::CreateKind};
        let root = scratch("holding");
        // The scratch folder lies inside the vault, under a config folder of its own.
        let app = root.join("Application Support/inkuno");
        crate::scratch::isolate(app.join("scratch"));
        fs::create_dir_all(app.join("scratch")).unwrap();
        fs::write(app.join("scratch/2026-11-14-103200.md"), "idée\n").unwrap();
        fs::write(root.join("plan.md"), "# Plan\n").unwrap();
        let (notes, dirs, _) = scan(&root);
        let shown: Vec<PathBuf> = notes.iter().map(|n| n.path.clone()).collect();
        assert_eq!(shown, vec![root.join("plan.md")]);
        // The scratch subtree is not listed (the config folder itself is skipped at run time by
        // `app_folders`, whose `config_dir` the test cannot choose).
        assert!(dirs.iter().all(|d| !d.starts_with(app.join("scratch"))));
        // A scratch note written does not change the vault's fingerprint, nor wake its reread.
        let before = fingerprint(&root);
        fs::write(app.join("scratch/2026-11-14-103900.md"), "deux\n").unwrap();
        assert_eq!(fingerprint(&root), before);
        let skip = app_folders_in(&root);
        let created = EventKind::Create(CreateKind::File);
        let scratch_write = Event::new(created).add_path(app.join("scratch/2026-11-14-103900.md"));
        assert!(!reread_for(
            &watch_roots(&root),
            &skip,
            &[],
            &Ok(scratch_write)
        ));
        let note = Event::new(created).add_path(root.join("plan.md"));
        assert!(reread_for(&watch_roots(&root), &skip, &[], &Ok(note)));
        assert!(reread_for(
            &watch_roots(&root),
            &skip,
            &[],
            &Err(notify::Error::generic("lost"))
        ));
        assert!(is_app_file(&app.join("scratch/2026-11-14-103200.md")));
        assert!(!is_app_file(&root.join("plan.md")));
        let _ = fs::remove_dir_all(root);
    }

    /// A vault opened through a symbolic link: the system reports its events under the canonical
    /// spelling. A change in a hidden folder, or in the app's own folder, is no reread in either
    /// spelling; a note is a reread in both.
    #[cfg(unix)]
    #[test]
    fn a_vault_opened_through_a_link_skips_its_hidden_and_own_folders_in_either_spelling() {
        use notify::{EventKind, event::CreateKind};
        let base = env::temp_dir().join(format!("inkuno-spelling-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let vault = base.join("vault");
        let scratch_dir = vault.join("Application Support/inkuno/scratch");
        fs::create_dir_all(vault.join(".git")).unwrap();
        fs::create_dir_all(&scratch_dir).unwrap();
        crate::scratch::isolate(scratch_dir.clone());
        let link = base.join("link");
        std::os::unix::fs::symlink(&vault, &link).unwrap();
        let canonical = fs::canonicalize(&vault).unwrap();
        let roots = watch_roots(&link);
        let skip = app_folders_in(&link);
        // The app's folder in both spellings: the one the walks use, and the one events carry.
        assert!(skip.contains(&link.join("Application Support/inkuno/scratch")));
        assert!(skip.contains(&canonical.join("Application Support/inkuno/scratch")));
        let created = EventKind::Create(CreateKind::File);
        let at =
            |path: PathBuf| -> notify::Result<Event> { Ok(Event::new(created).add_path(path)) };
        for folder in [&link, &canonical] {
            let hidden = folder.join(".git/objects/ab");
            assert!(!reread_for(&roots, &skip, &[], &at(hidden)));
            let scratch_note =
                folder.join("Application Support/inkuno/scratch/2026-11-16-101000.md");
            assert!(!reread_for(&roots, &skip, &[], &at(scratch_note)));
            let note = folder.join("plan.md");
            assert!(reread_for(&roots, &skip, &[], &at(note)));
        }
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn watcher_reports_a_new_note() {
        let root = env::temp_dir().join(format!("inkuno-watch-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let changed = Arc::new(Signal::default());
        let history = Arc::new(Signal::default());
        let _watcher = watch(&root, changed.clone(), history)
            .expect("the system must be able to watch a folder");
        // No simulated clock here: the system takes a few milliseconds (FSEvents, a bit more).
        fs::write(root.join("neuve.md"), "x").unwrap();
        let start = std::time::Instant::now();
        while !changed.raised.load(Ordering::Acquire)
            && start.elapsed() < std::time::Duration::from_secs(10)
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(changed.take(), "no event in 10 s");
        fs::remove_dir_all(&root).unwrap();
    }

    /// On disk: a revision written in the history folder raises the history signal, not the vault's.
    #[test]
    fn watcher_reports_a_revision_to_the_history_signal_only() {
        let root = env::temp_dir().join(format!("inkuno-watch-rev-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        // The history folder exists before the watch: its creation is not what is tested.
        let folder = root.join(".inkuno/revisions/Plan.md");
        fs::create_dir_all(&folder).unwrap();
        let changed = Arc::new(Signal::default());
        let history = Arc::new(Signal::default());
        let _watcher = watch(&root, changed.clone(), history.clone())
            .expect("the system must be able to watch a folder");
        // FSEvents (macOS) can report the folder's creation after the watch has started: let it
        // come, then drop it, so that only the revision's write is under test.
        std::thread::sleep(std::time::Duration::from_millis(500));
        changed.take();
        history.take();
        // `inkuno rev save` writes a revision file from a terminal.
        fs::write(folder.join("0001.md"), "x").unwrap();
        let start = std::time::Instant::now();
        while !history.raised.load(Ordering::Acquire)
            && start.elapsed() < std::time::Duration::from_secs(10)
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(history.take(), "no event in 10 s");
        // Give the system time to report every event of that write: none reaches the vault's signal.
        std::thread::sleep(std::time::Duration::from_millis(500));
        assert!(!changed.take(), "a revision must not reread the vault");
        fs::remove_dir_all(&root).unwrap();
    }

    /// A folder that cannot be listed is reported, root or inner, with the reason; the rest is read as
    /// usual, and the same folder read again once the permission is back.
    #[cfg(unix)]
    #[test]
    fn reports_unreadable_folders() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("inkuno-unreadable-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let deep = root.join("a/b/c/d");
        fs::create_dir_all(&deep).unwrap();
        fs::create_dir_all(root.join("Fermé/Dedans")).unwrap();
        fs::write(root.join("Note.md"), "# Note").unwrap();
        fs::write(deep.join("Profonde.md"), "x").unwrap();
        fs::write(root.join("Fermé/Cachée.md"), "x").unwrap();
        let mode =
            |path: &Path, m: u32| fs::set_permissions(path, fs::Permissions::from_mode(m)).unwrap();
        // Root (`chmod 0`) can read anything: nothing to see there. The folder inside first: closed, the
        // outer one hides it.
        mode(&root.join("Fermé/Dedans"), 0o000);
        mode(&root.join("Fermé"), 0o000);
        if fs::read_dir(root.join("Fermé")).is_ok() {
            return eprintln!("permissions are not enforced here (root?): skipped");
        }
        // An inner folder at depth 1, one inside it, one at depth 4 (write and cross, no read: 311).
        mode(&deep, 0o311);
        let (notes, dirs, _, unreadable) = rescan_full(&root, &[]);
        let names: Vec<_> = notes.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["Note"]);
        assert!(
            dirs.contains(&root.join("Fermé")) && dirs.contains(&deep),
            "an unreadable folder is still a folder of the vault"
        );
        let mut got: Vec<_> = unreadable
            .iter()
            .map(|(p, kind)| (p.strip_prefix(&root).unwrap().to_path_buf(), *kind))
            .collect();
        got.sort();
        // The folder inside the closed one is not reached: only the closed one is said.
        assert_eq!(
            got,
            [
                (PathBuf::from("Fermé"), io::ErrorKind::PermissionDenied),
                (PathBuf::from("a/b/c/d"), io::ErrorKind::PermissionDenied)
            ]
        );
        // The root: nothing is read, and it says so; given back, everything is there.
        mode(&root, 0o000);
        let (notes, _, _, unreadable) = rescan_full(&root, &[]);
        assert!(
            notes.is_empty() && unreadable == [(root.clone(), io::ErrorKind::PermissionDenied)]
        );
        for dir in [
            &root,
            &root.join("Fermé"),
            &root.join("Fermé/Dedans"),
            &deep,
        ] {
            mode(dir, 0o755);
        }
        let (notes, _, _, unreadable) = rescan_full(&root, &[]);
        assert!(
            unreadable.is_empty() && notes.len() == 3,
            "{:?}",
            notes.iter().map(|n| &n.name).collect::<Vec<_>>()
        );
        // A readable but empty vault is empty, not unreadable.
        let empty = root.join("vide");
        fs::create_dir(&empty).unwrap();
        assert!(rescan_full(&empty, &[]).3.is_empty());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn json_config_lives_beside_the_other_files() {
        // Paths only: no file is written here, the real folder stays untouched.
        assert_eq!(
            keybindings_path(),
            config_dir().join("inkuno/keybindings.json")
        );
        assert_eq!(themes_path(), config_dir().join("inkuno/themes.json"));
    }
}
