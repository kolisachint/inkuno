//! File watcher for the vault (task 9.2).
//!
//! `notify` reports raw file system events. This module turns them into
//! vault-level [`Change`]s, coalesces bursts, and plans how open buffers
//! should react. The classification and planning are pure functions and are
//! tested without a file system. [`VaultWatcher`] feeds real events into them.
//!
//! The tree is always rebuilt from disk with [`Vault::tree`], so a change
//! only needs to tell the caller that something under the vault moved.
//!
//! Paths that a `.gitignore` excludes are not reported, as they are hidden from
//! the tree and from search. A change to a `.gitignore` reloads the rules and
//! reports a [`Change::Rescan`].

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use ignore::{Match, WalkBuilder};
use notify::event::{EventKind, ModifyKind, RenameMode};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::tree::{is_note_path, Vault};

/// Quiet period that ends a batch of events.
const QUIET: Duration = Duration::from_millis(100);
/// Longest time one batch may keep collecting events.
const MAX_BATCH: Duration = Duration::from_millis(500);

/// A change inside the vault, in terms of vault paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A note file or a folder appeared.
    Created(PathBuf),
    /// A note file's contents or metadata changed on disk.
    Modified(PathBuf),
    /// A visible path was removed. It may have been a note, a folder or another file.
    Removed(PathBuf),
    /// A visible path was renamed or moved within the vault.
    Renamed { from: PathBuf, to: PathBuf },
    /// Events were lost or the watch failed. The caller should rebuild the tree
    /// and treat every open note as possibly changed.
    Rescan,
}

impl Change {
    /// True when this change names `path` as its own (or, for a rename, either side).
    fn touches(&self, path: &Path) -> bool {
        match self {
            Change::Created(p) | Change::Modified(p) | Change::Removed(p) => p == path,
            Change::Renamed { from, to } => from == path || to == path,
            Change::Rescan => false,
        }
    }
}

/// What a path is on disk, as far as classification needs to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Entry {
    Missing,
    File,
    Dir,
    /// Symbolic links and other entries that the tree never shows.
    Other,
}

/// Where event paths live. `watched` is the root as `notify` reports it
/// (canonical). `reported` is the vault root that callers use.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scope<'a> {
    pub watched: &'a Path,
    pub reported: &'a Path,
}

/// The `.gitignore` rules of a vault: one matcher per `.gitignore` file. As in git,
/// a file's rules apply only to paths below the folder that holds it.
#[derive(Debug, Default)]
pub(crate) struct Rules {
    /// Folder holding a `.gitignore`, and the rules read from it. Deepest folders come first.
    files: Vec<(PathBuf, Gitignore)>,
}

impl Rules {
    /// Reads every `.gitignore` below `root`. Hidden folders are skipped, as the
    /// note walker skips them, so the rules match what the tree and search see.
    pub(crate) fn load(root: &Path) -> Self {
        let walker = WalkBuilder::new(root)
            .hidden(false)
            .ignore(false)
            .git_ignore(false)
            .git_global(false)
            .git_exclude(false)
            .parents(false)
            .require_git(false)
            .filter_entry(|entry| {
                let hidden_folder = entry.depth() > 0
                    && entry.file_type().is_some_and(|t| t.is_dir())
                    && entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.starts_with('.'));
                !hidden_folder
            })
            .build();
        let mut files: Vec<(PathBuf, Gitignore)> = Vec::new();
        for entry in walker.flatten() {
            let is_file = entry.file_type().is_some_and(|t| t.is_file());
            if !is_file || entry.file_name().to_str() != Some(".gitignore") {
                continue;
            }
            let file = entry.into_path();
            let Some(dir) = file.parent().map(Path::to_path_buf) else {
                continue;
            };
            let mut builder = GitignoreBuilder::new(&dir);
            // A line that does not parse is skipped; the other lines still apply.
            let _ = builder.add(&file);
            if let Ok(gitignore) = builder.build() {
                files.push((dir, gitignore));
            }
        }
        files.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.components().count()));
        Self { files }
    }

    /// True when a `.gitignore` excludes `path`, or one of its parent folders.
    /// `entry` is what the path is on disk. A removed path has no entry kind, so
    /// both readings are tried.
    fn excludes(&self, path: &Path, entry: Entry) -> bool {
        match entry {
            Entry::Dir => self.matches(path, true),
            Entry::File => self.matches(path, false),
            Entry::Missing | Entry::Other => self.matches(path, false) || self.matches(path, true),
        }
    }

    fn matches(&self, path: &Path, is_dir: bool) -> bool {
        for (dir, gitignore) in &self.files {
            let Ok(rel) = path.strip_prefix(dir) else {
                continue;
            };
            if rel.as_os_str().is_empty() {
                continue;
            }
            match gitignore.matched_path_or_any_parents(rel, is_dir) {
                Match::Ignore(_) => return true,
                Match::Whitelist(_) => return false,
                Match::None => {}
            }
        }
        false
    }
}

/// Maps an event path to the vault path the caller sees. Returns `None` for the
/// root itself, paths outside the root, paths with a hidden or non-UTF-8 component,
/// and paths that a `.gitignore` excludes.
fn visible_path(
    scope: Scope<'_>,
    rules: &Rules,
    probe: &dyn Fn(&Path) -> Entry,
    path: &Path,
) -> Option<PathBuf> {
    let rel = path.strip_prefix(scope.watched).ok()?;
    if rel.as_os_str().is_empty() {
        return None;
    }
    for component in rel.components() {
        match component.as_os_str().to_str() {
            Some(name) if !name.starts_with('.') => {}
            _ => return None,
        }
    }
    if rules.excludes(path, probe(path)) {
        return None;
    }
    Some(scope.reported.join(rel))
}

/// True when a create, remove or modify event names a `.gitignore` inside the vault.
fn changes_gitignore(scope: Scope<'_>, event: &Event) -> bool {
    let content_event = matches!(
        event.kind,
        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_)
    );
    content_event
        && event.paths.iter().any(|path| {
            path.strip_prefix(scope.watched).is_ok()
                && path.file_name().and_then(|name| name.to_str()) == Some(".gitignore")
        })
}

/// True when a path appeared (was created or renamed in) and the tree should show it.
fn appeared(probe: &dyn Fn(&Path) -> Entry, path: &Path) -> bool {
    match probe(path) {
        Entry::Dir => true,
        Entry::File | Entry::Missing => is_note_path(path),
        Entry::Other => false,
    }
}

/// Turns one raw event into vault changes. `probe` reports what a path is on disk
/// now. It is used only where the event kind alone is ambiguous. A change to a
/// `.gitignore` reloads `rules` from disk and returns a rescan.
pub(crate) fn classify(
    scope: Scope<'_>,
    rules: &mut Rules,
    event: &Event,
    probe: &dyn Fn(&Path) -> Entry,
) -> Vec<Change> {
    if event.need_rescan() {
        return vec![Change::Rescan];
    }
    if changes_gitignore(scope, event) {
        *rules = Rules::load(scope.watched);
        return vec![Change::Rescan];
    }
    classify_paths(scope, rules, event, probe)
}

fn classify_paths(
    scope: Scope<'_>,
    rules: &Rules,
    event: &Event,
    probe: &dyn Fn(&Path) -> Entry,
) -> Vec<Change> {
    let mut out = Vec::new();
    match &event.kind {
        EventKind::Create(_) => {
            for path in &event.paths {
                if let (Some(v), true) = (
                    visible_path(scope, rules, probe, path),
                    appeared(probe, path),
                ) {
                    out.push(Change::Created(v));
                }
            }
        }
        EventKind::Remove(_) => {
            for path in &event.paths {
                if let Some(v) = visible_path(scope, rules, probe, path) {
                    out.push(Change::Removed(v));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) if event.paths.len() == 2 => {
            out.extend(classify_rename(
                scope,
                rules,
                &event.paths[0],
                &event.paths[1],
                probe,
            ));
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            for path in &event.paths {
                if let Some(v) = visible_path(scope, rules, probe, path) {
                    out.push(Change::Removed(v));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            for path in &event.paths {
                if let (Some(v), true) = (
                    visible_path(scope, rules, probe, path),
                    appeared(probe, path),
                ) {
                    out.push(Change::Created(v));
                }
            }
        }
        EventKind::Modify(ModifyKind::Name(_)) => {
            // Ambiguous rename events: the path either exists now or it does not.
            for path in &event.paths {
                let Some(v) = visible_path(scope, rules, probe, path) else {
                    continue;
                };
                if probe(path) == Entry::Missing {
                    out.push(Change::Removed(v));
                } else if appeared(probe, path) {
                    out.push(Change::Created(v));
                }
            }
        }
        EventKind::Modify(
            ModifyKind::Data(_) | ModifyKind::Metadata(_) | ModifyKind::Any | ModifyKind::Other,
        ) => {
            for path in &event.paths {
                let is_file = matches!(probe(path), Entry::File | Entry::Missing);
                if is_file && is_note_path(path) {
                    if let Some(v) = visible_path(scope, rules, probe, path) {
                        out.push(Change::Modified(v));
                    }
                }
            }
        }
        EventKind::Access(_) | EventKind::Any | EventKind::Other => {}
    }
    out
}

fn classify_rename(
    scope: Scope<'_>,
    rules: &Rules,
    from: &Path,
    to: &Path,
    probe: &dyn Fn(&Path) -> Entry,
) -> Vec<Change> {
    match (
        visible_path(scope, rules, probe, from),
        visible_path(scope, rules, probe, to),
    ) {
        (Some(f), Some(t)) => {
            if appeared(probe, to) {
                vec![Change::Renamed { from: f, to: t }]
            } else {
                // A note became a non-note file, so from the tree's view it is gone.
                vec![Change::Removed(f)]
            }
        }
        (Some(f), None) => vec![Change::Removed(f)],
        (None, Some(t)) if appeared(probe, to) => vec![Change::Created(t)],
        _ => Vec::new(),
    }
}

/// Merges a batch of changes without losing their order. A rescan replaces
/// everything. Exact repeats in a row are dropped, and a `Modified` is dropped
/// when the latest change to that path already created or modified it.
pub(crate) fn coalesce(batch: Vec<Change>) -> Vec<Change> {
    if batch.contains(&Change::Rescan) {
        return vec![Change::Rescan];
    }
    let mut out: Vec<Change> = Vec::with_capacity(batch.len());
    for change in batch {
        if let Change::Modified(path) = &change {
            let redundant = out
                .iter()
                .rev()
                .find(|c| c.touches(path))
                .is_some_and(|c| matches!(c, Change::Created(_) | Change::Modified(_)));
            if redundant {
                continue;
            }
        }
        if out.last() == Some(&change) {
            continue;
        }
        out.push(change);
    }
    out
}

/// A note open in the app, and whether it has unsaved edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenBuffer {
    pub path: PathBuf,
    pub dirty: bool,
}

/// What the app should do with an open buffer after a change on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferUpdate {
    /// The file changed and the buffer has no unsaved edits: reload it from disk.
    Reload(PathBuf),
    /// The file changed but the buffer has unsaved edits: keep the buffer as it is.
    KeepUnsaved(PathBuf),
    /// The file was renamed: the buffer should now use `to`.
    Moved { from: PathBuf, to: PathBuf },
    /// The file was removed from disk. The buffer is kept, and the app decides what to do.
    Missing(PathBuf),
}

/// Plans buffer updates for a batch of changes, in the order the changes arrived.
/// A clean buffer is never reloaded over unsaved edits.
pub fn plan_buffer_updates(changes: &[Change], open: &[OpenBuffer]) -> Vec<BufferUpdate> {
    let changed_update = |buf: &OpenBuffer| {
        if buf.dirty {
            BufferUpdate::KeepUnsaved(buf.path.clone())
        } else {
            BufferUpdate::Reload(buf.path.clone())
        }
    };
    let mut out = Vec::new();
    for change in changes {
        match change {
            Change::Created(_) => {}
            Change::Modified(path) => {
                out.extend(
                    open.iter()
                        .filter(|buf| buf.path == *path)
                        .map(changed_update),
                );
            }
            Change::Removed(path) => {
                out.extend(
                    open.iter()
                        .filter(|buf| buf.path.starts_with(path))
                        .map(|buf| BufferUpdate::Missing(buf.path.clone())),
                );
            }
            Change::Renamed { from, to } => {
                for buf in open {
                    if let Some(new_path) = remap(&buf.path, from, to) {
                        out.push(BufferUpdate::Moved {
                            from: buf.path.clone(),
                            to: new_path,
                        });
                    }
                }
            }
            Change::Rescan => out.extend(open.iter().map(changed_update)),
        }
    }
    out
}

/// Maps `path` from under `from` to under `to`. Returns `None` if it is not under `from`.
fn remap(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
    let rest = path.strip_prefix(from).ok()?;
    if rest.as_os_str().is_empty() {
        Some(to.to_path_buf())
    } else {
        Some(to.join(rest))
    }
}

fn probe_fs(path: &Path) -> Entry {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Entry::Dir,
        Ok(meta) if meta.is_file() => Entry::File,
        Ok(_) => Entry::Other,
        Err(_) => Entry::Missing,
    }
}

/// Watches a vault recursively and reports changes in batches.
///
/// Dropping the watcher stops the OS watch.
pub struct VaultWatcher {
    // Kept alive for the OS watch. Dropping it stops the events.
    _watcher: RecommendedWatcher,
    rx: Receiver<Vec<Change>>,
}

impl VaultWatcher {
    /// Starts watching the vault root and everything below it.
    pub fn start(vault: &Vault) -> notify::Result<Self> {
        let reported = vault.root().to_path_buf();
        let watched = fs::canonicalize(&reported).map_err(notify::Error::io)?;
        let (tx, rx) = mpsc::channel();
        let (cb_watched, cb_reported) = (watched.clone(), reported.clone());
        let mut rules = Rules::load(&watched);
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            let changes = match res {
                Ok(event) => {
                    let scope = Scope {
                        watched: &cb_watched,
                        reported: &cb_reported,
                    };
                    classify(scope, &mut rules, &event, &probe_fs)
                }
                Err(_) => vec![Change::Rescan],
            };
            if !changes.is_empty() {
                // The receiver is gone only when the watcher was dropped.
                let _ = tx.send(changes);
            }
        })?;
        watcher.watch(&watched, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            rx,
        })
    }

    /// Waits up to `timeout` for changes. Events that follow within a short window
    /// are gathered into the same batch. Returns an empty list when nothing happens.
    pub fn next_changes(&self, timeout: Duration) -> Vec<Change> {
        let mut all = match self.rx.recv_timeout(timeout) {
            Ok(batch) => batch,
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                return Vec::new();
            }
        };
        let started = Instant::now();
        while let Some(left) = MAX_BATCH.checked_sub(started.elapsed()) {
            match self.rx.recv_timeout(QUIET.min(left)) {
                Ok(batch) => all.extend(batch),
                Err(_) => break,
            }
        }
        coalesce(all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, MetadataKind, RemoveKind};
    use std::collections::HashMap;

    const WATCHED: &str = "/v";

    fn scope() -> Scope<'static> {
        Scope {
            watched: Path::new(WATCHED),
            reported: Path::new(WATCHED),
        }
    }

    /// A fake file system: paths not listed are missing.
    fn fake(entries: &[(&str, Entry)]) -> impl Fn(&Path) -> Entry {
        let map: HashMap<PathBuf, Entry> = entries
            .iter()
            .map(|(p, e)| (PathBuf::from(p), *e))
            .collect();
        move |p: &Path| map.get(p).copied().unwrap_or(Entry::Missing)
    }

    fn ev(kind: EventKind, paths: &[&str]) -> Event {
        let mut event = Event::new(kind);
        for p in paths {
            event = event.add_path(PathBuf::from(p));
        }
        event
    }

    fn run(kind: EventKind, paths: &[&str], entries: &[(&str, Entry)]) -> Vec<Change> {
        run_with(&mut Rules::default(), kind, paths, entries)
    }

    fn run_with(
        rules: &mut Rules,
        kind: EventKind,
        paths: &[&str],
        entries: &[(&str, Entry)],
    ) -> Vec<Change> {
        let probe = fake(entries);
        classify(scope(), rules, &ev(kind, paths), &probe)
    }

    impl Rules {
        /// Rules built from `(folder, lines of its .gitignore)` pairs, without a disk read.
        fn from_lines(files: &[(&str, &[&str])]) -> Self {
            let mut out: Vec<(PathBuf, Gitignore)> = files
                .iter()
                .map(|(dir, lines)| {
                    let mut builder = GitignoreBuilder::new(dir);
                    for line in lines.iter() {
                        builder.add_line(None, line).unwrap();
                    }
                    (PathBuf::from(dir), builder.build().unwrap())
                })
                .collect();
            out.sort_by_key(|(dir, _)| std::cmp::Reverse(dir.components().count()));
            Self { files: out }
        }
    }

    fn created(p: &str) -> Change {
        Change::Created(PathBuf::from(p))
    }
    fn modified(p: &str) -> Change {
        Change::Modified(PathBuf::from(p))
    }
    fn removed(p: &str) -> Change {
        Change::Removed(PathBuf::from(p))
    }

    #[test]
    fn created_note_and_folder_are_reported() {
        let kind = EventKind::Create(CreateKind::File);
        assert_eq!(
            run(kind, &["/v/a.md"], &[("/v/a.md", Entry::File)]),
            vec![created("/v/a.md")]
        );
        let kind = EventKind::Create(CreateKind::Folder);
        assert_eq!(
            run(kind, &["/v/dir"], &[("/v/dir", Entry::Dir)]),
            vec![created("/v/dir")]
        );
    }

    #[test]
    fn created_non_notes_and_hidden_paths_are_ignored() {
        let kind = EventKind::Create(CreateKind::File);
        assert!(run(kind, &["/v/image.png"], &[("/v/image.png", Entry::File)]).is_empty());
        let kind = EventKind::Create(CreateKind::File);
        assert!(run(kind, &["/v/.hidden.md"], &[("/v/.hidden.md", Entry::File)]).is_empty());
        let kind = EventKind::Create(CreateKind::Folder);
        assert!(run(kind, &["/v/.git"], &[("/v/.git", Entry::Dir)]).is_empty());
        let kind = EventKind::Create(CreateKind::File);
        assert!(run(kind, &["/v/sub/.x/n.md"], &[]).is_empty());
    }

    #[test]
    fn paths_outside_the_root_and_the_root_itself_are_ignored() {
        let kind = EventKind::Create(CreateKind::File);
        assert!(run(
            kind,
            &["/elsewhere/n.md"],
            &[("/elsewhere/n.md", Entry::File)]
        )
        .is_empty());
        let kind = EventKind::Modify(ModifyKind::Data(DataChange::Any));
        assert!(run(kind, &["/v"], &[("/v", Entry::Dir)]).is_empty());
    }

    #[test]
    fn created_note_in_a_hidden_folder_is_not_reported_even_when_nested() {
        let kind = EventKind::Create(CreateKind::File);
        let changes = run(
            kind,
            &["/v/.obsidian/n.md"],
            &[("/v/.obsidian/n.md", Entry::File)],
        );
        assert!(changes.is_empty());
    }

    #[test]
    fn data_and_metadata_changes_to_notes_are_modified() {
        let kind = EventKind::Modify(ModifyKind::Data(DataChange::Content));
        assert_eq!(
            run(kind, &["/v/a.md"], &[("/v/a.md", Entry::File)]),
            vec![modified("/v/a.md")]
        );
        let kind = EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any));
        assert_eq!(
            run(kind, &["/v/a.md"], &[("/v/a.md", Entry::File)]),
            vec![modified("/v/a.md")]
        );
    }

    #[test]
    fn modifications_to_non_notes_folders_and_access_events_are_ignored() {
        let kind = EventKind::Modify(ModifyKind::Data(DataChange::Content));
        assert!(run(kind, &["/v/a.png"], &[("/v/a.png", Entry::File)]).is_empty());
        let kind = EventKind::Modify(ModifyKind::Metadata(MetadataKind::Any));
        assert!(run(kind, &["/v/dir.md"], &[("/v/dir.md", Entry::Dir)]).is_empty());
        let kind = EventKind::Access(notify::event::AccessKind::Any);
        assert!(run(kind, &["/v/a.md"], &[("/v/a.md", Entry::File)]).is_empty());
        assert!(run(EventKind::Other, &["/v/a.md"], &[]).is_empty());
    }

    #[test]
    fn removed_paths_are_reported_but_hidden_ones_are_not() {
        let kind = EventKind::Remove(RemoveKind::File);
        assert_eq!(run(kind, &["/v/a.md"], &[]), vec![removed("/v/a.md")]);
        let kind = EventKind::Remove(RemoveKind::Folder);
        assert_eq!(run(kind, &["/v/dir"], &[]), vec![removed("/v/dir")]);
        let kind = EventKind::Remove(RemoveKind::File);
        assert!(run(kind, &["/v/.a.md"], &[]).is_empty());
    }

    #[test]
    fn rename_from_and_to_map_to_removed_and_created() {
        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::From));
        assert_eq!(run(kind, &["/v/a.md"], &[]), vec![removed("/v/a.md")]);
        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::To));
        assert_eq!(
            run(kind, &["/v/b.md"], &[("/v/b.md", Entry::File)]),
            vec![created("/v/b.md")]
        );
    }

    #[test]
    fn rename_both_maps_to_renamed_for_notes_and_folders() {
        let kind = || EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        let entries = [("/v/b.md", Entry::File)];
        assert_eq!(
            run(kind(), &["/v/a.md", "/v/b.md"], &entries),
            vec![Change::Renamed {
                from: PathBuf::from("/v/a.md"),
                to: PathBuf::from("/v/b.md"),
            }]
        );
        let entries = [("/v/new", Entry::Dir)];
        assert_eq!(
            run(kind(), &["/v/old", "/v/new"], &entries),
            vec![Change::Renamed {
                from: PathBuf::from("/v/old"),
                to: PathBuf::from("/v/new"),
            }]
        );
    }

    #[test]
    fn rename_to_a_non_note_counts_as_removal() {
        let kind = EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        let entries = [("/v/a.txt", Entry::File)];
        assert_eq!(
            run(kind, &["/v/a.md", "/v/a.txt"], &entries),
            vec![removed("/v/a.md")]
        );
    }

    #[test]
    fn rename_across_the_vault_boundary_maps_to_one_sided_changes() {
        let kind = || EventKind::Modify(ModifyKind::Name(RenameMode::Both));
        let entries = [("/v/in.md", Entry::File)];
        assert_eq!(
            run(kind(), &["/elsewhere/in.md", "/v/in.md"], &entries),
            vec![created("/v/in.md")]
        );
        assert_eq!(
            run(kind(), &["/v/out.md", "/elsewhere/out.md"], &[]),
            vec![removed("/v/out.md")]
        );
        assert!(run(kind(), &["/x/a.md", "/y/b.md"], &[]).is_empty());
    }

    #[test]
    fn ambiguous_rename_events_use_the_disk_state() {
        let kind = || EventKind::Modify(ModifyKind::Name(RenameMode::Any));
        assert_eq!(
            run(kind(), &["/v/gone.md"], &[]),
            vec![removed("/v/gone.md")]
        );
        assert_eq!(
            run(kind(), &["/v/here.md"], &[("/v/here.md", Entry::File)]),
            vec![created("/v/here.md")]
        );
        assert!(run(kind(), &["/v/here.png"], &[("/v/here.png", Entry::File)]).is_empty());
    }

    #[test]
    fn rescan_flag_yields_only_a_rescan() {
        let event = ev(EventKind::Other, &["/v/a.md"]).set_flag(notify::event::Flag::Rescan);
        let changes = classify(scope(), &mut Rules::default(), &event, &fake(&[]));
        assert_eq!(changes, vec![Change::Rescan]);
    }

    #[test]
    fn reported_root_replaces_the_watched_root_in_output() {
        let scope = Scope {
            watched: Path::new("/canon/vault"),
            reported: Path::new("/link/vault"),
        };
        let event = ev(
            EventKind::Create(CreateKind::File),
            &["/canon/vault/sub/n.md"],
        );
        let probe = fake(&[("/canon/vault/sub/n.md", Entry::File)]);
        assert_eq!(
            classify(scope, &mut Rules::default(), &event, &probe),
            vec![Change::Created(PathBuf::from("/link/vault/sub/n.md"))]
        );
    }

    #[test]
    fn coalesce_drops_repeats_and_redundant_modifications() {
        let batch = vec![
            created("/v/a.md"),
            modified("/v/a.md"),
            modified("/v/a.md"),
            modified("/v/b.md"),
            modified("/v/b.md"),
        ];
        assert_eq!(
            coalesce(batch),
            vec![created("/v/a.md"), modified("/v/b.md")]
        );
    }

    #[test]
    fn coalesce_keeps_order_of_create_remove_create() {
        let batch = vec![created("/v/a.md"), removed("/v/a.md"), created("/v/a.md")];
        assert_eq!(coalesce(batch.clone()), batch);
    }

    #[test]
    fn coalesce_keeps_modification_after_remove() {
        let batch = vec![removed("/v/a.md"), modified("/v/a.md")];
        assert_eq!(coalesce(batch.clone()), batch);
    }

    #[test]
    fn coalesce_collapses_to_rescan() {
        let batch = vec![created("/v/a.md"), Change::Rescan, modified("/v/b.md")];
        assert_eq!(coalesce(batch), vec![Change::Rescan]);
    }

    fn buf(path: &str, dirty: bool) -> OpenBuffer {
        OpenBuffer {
            path: PathBuf::from(path),
            dirty,
        }
    }

    #[test]
    fn modified_clean_buffer_reloads_and_dirty_buffer_is_kept() {
        let open = [
            buf("/v/a.md", false),
            buf("/v/b.md", true),
            buf("/v/c.md", false),
        ];
        let plan = plan_buffer_updates(&[modified("/v/a.md"), modified("/v/b.md")], &open);
        assert_eq!(
            plan,
            vec![
                BufferUpdate::Reload(PathBuf::from("/v/a.md")),
                BufferUpdate::KeepUnsaved(PathBuf::from("/v/b.md")),
            ]
        );
    }

    #[test]
    fn created_changes_do_not_touch_open_buffers() {
        let open = [buf("/v/a.md", false)];
        assert!(plan_buffer_updates(&[created("/v/a.md")], &open).is_empty());
    }

    #[test]
    fn removed_folder_marks_buffers_below_it_missing() {
        let open = [
            buf("/v/dir/a.md", false),
            buf("/v/dir/sub/b.md", true),
            buf("/v/dirx/c.md", false),
        ];
        assert_eq!(
            plan_buffer_updates(&[removed("/v/dir")], &open),
            vec![
                BufferUpdate::Missing(PathBuf::from("/v/dir/a.md")),
                BufferUpdate::Missing(PathBuf::from("/v/dir/sub/b.md")),
            ]
        );
    }

    #[test]
    fn renamed_folder_moves_buffers_below_it() {
        let open = [buf("/v/old/a.md", false), buf("/v/other.md", false)];
        let change = Change::Renamed {
            from: PathBuf::from("/v/old"),
            to: PathBuf::from("/v/new"),
        };
        assert_eq!(
            plan_buffer_updates(&[change], &open),
            vec![BufferUpdate::Moved {
                from: PathBuf::from("/v/old/a.md"),
                to: PathBuf::from("/v/new/a.md"),
            }]
        );
    }

    #[test]
    fn rescan_reloads_clean_buffers_and_keeps_dirty_ones() {
        let open = [buf("/v/a.md", false), buf("/v/b.md", true)];
        assert_eq!(
            plan_buffer_updates(&[Change::Rescan], &open),
            vec![
                BufferUpdate::Reload(PathBuf::from("/v/a.md")),
                BufferUpdate::KeepUnsaved(PathBuf::from("/v/b.md")),
            ]
        );
    }

    #[test]
    fn gitignored_notes_and_folders_are_not_reported() {
        let mut rules = Rules::from_lines(&[("/v", &["*.draft.md", "private/"])]);
        let create = || EventKind::Create(CreateKind::File);
        let modify = || EventKind::Modify(ModifyKind::Data(DataChange::Content));
        let file = Entry::File;

        assert!(run_with(
            &mut rules,
            create(),
            &["/v/a.draft.md"],
            &[("/v/a.draft.md", file)]
        )
        .is_empty());
        assert!(run_with(
            &mut rules,
            modify(),
            &["/v/a.draft.md"],
            &[("/v/a.draft.md", file)]
        )
        .is_empty());
        assert!(run_with(
            &mut rules,
            create(),
            &["/v/private/p.md"],
            &[("/v/private/p.md", file)]
        )
        .is_empty());
        assert!(run_with(
            &mut rules,
            EventKind::Create(CreateKind::Folder),
            &["/v/private"],
            &[("/v/private", Entry::Dir)]
        )
        .is_empty());
        assert!(run_with(
            &mut rules,
            EventKind::Remove(RemoveKind::File),
            &["/v/private/p.md"],
            &[]
        )
        .is_empty());
        assert!(run_with(
            &mut rules,
            EventKind::Remove(RemoveKind::Folder),
            &["/v/private"],
            &[]
        )
        .is_empty());
        assert!(run_with(
            &mut rules,
            EventKind::Remove(RemoveKind::File),
            &["/v/a.draft.md"],
            &[]
        )
        .is_empty());

        // A visible note is still reported.
        assert_eq!(
            run_with(&mut rules, create(), &["/v/a.md"], &[("/v/a.md", file)]),
            vec![created("/v/a.md")]
        );
        // Renaming a note to an ignored name removes it from the tree.
        assert_eq!(
            run_with(
                &mut rules,
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                &["/v/a.md", "/v/a.draft.md"],
                &[("/v/a.draft.md", file)]
            ),
            vec![removed("/v/a.md")]
        );
    }

    #[test]
    fn nested_gitignore_applies_only_below_its_folder() {
        let mut rules = Rules::from_lines(&[("/v/sub", &["secret.md"])]);
        let create = || EventKind::Create(CreateKind::File);

        assert!(run_with(
            &mut rules,
            create(),
            &["/v/sub/secret.md"],
            &[("/v/sub/secret.md", Entry::File)]
        )
        .is_empty());
        assert_eq!(
            run_with(
                &mut rules,
                create(),
                &["/v/other/secret.md"],
                &[("/v/other/secret.md", Entry::File)]
            ),
            vec![created("/v/other/secret.md")]
        );
        assert_eq!(
            run_with(
                &mut rules,
                create(),
                &["/v/sub/b.md"],
                &[("/v/sub/b.md", Entry::File)]
            ),
            vec![created("/v/sub/b.md")]
        );
    }

    #[test]
    fn changing_a_gitignore_reloads_the_rules_and_rescans() {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        let scope = Scope {
            watched: &root,
            reported: &root,
        };
        let probe = |_: &Path| Entry::File;
        let event = |kind: EventKind, path: PathBuf| Event::new(kind).add_path(path);
        let modify = || EventKind::Modify(ModifyKind::Data(DataChange::Content));
        let create = || EventKind::Create(CreateKind::File);
        let gitignore = root.join(".gitignore");
        let draft = root.join("a.draft.md");
        let note = root.join("a.md");
        let mut rules = Rules::default();

        // Writing the file rescans, and its patterns apply from then on.
        fs::write(&gitignore, "*.draft.md\n").unwrap();
        let changes = classify(
            scope,
            &mut rules,
            &event(modify(), gitignore.clone()),
            &probe,
        );
        assert_eq!(changes, vec![Change::Rescan]);
        assert!(classify(scope, &mut rules, &event(create(), draft.clone()), &probe).is_empty());
        assert_eq!(
            classify(scope, &mut rules, &event(create(), note.clone()), &probe),
            vec![Change::Created(note.clone())]
        );

        // A nested .gitignore applies below its own folder.
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::write(root.join("sub/.gitignore"), "x.md\n").unwrap();
        let changes = classify(
            scope,
            &mut rules,
            &event(create(), root.join("sub/.gitignore")),
            &probe,
        );
        assert_eq!(changes, vec![Change::Rescan]);
        assert!(classify(
            scope,
            &mut rules,
            &event(create(), root.join("sub/x.md")),
            &probe
        )
        .is_empty());

        // Removing the root .gitignore rescans and stops excluding its matches.
        fs::remove_file(&gitignore).unwrap();
        let changes = classify(
            scope,
            &mut rules,
            &event(EventKind::Remove(RemoveKind::File), gitignore.clone()),
            &probe,
        );
        assert_eq!(changes, vec![Change::Rescan]);
        assert_eq!(
            classify(scope, &mut rules, &event(create(), draft.clone()), &probe),
            vec![Change::Created(draft)]
        );
    }

    // Real file system tests. Each waits for events with a generous timeout.

    const TIMEOUT: Duration = Duration::from_secs(15);

    /// Collects changes into `seen` until `done` holds, or fails after `TIMEOUT`.
    fn wait_until(
        watcher: &VaultWatcher,
        seen: &mut Vec<Change>,
        done: impl Fn(&[Change]) -> bool,
    ) {
        let deadline = Instant::now() + TIMEOUT;
        while !done(seen) {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(
                !left.is_zero(),
                "timed out waiting for events; seen: {seen:?}"
            );
            seen.extend(watcher.next_changes(left.min(Duration::from_millis(200))));
        }
    }

    fn tree_paths(vault: &Vault) -> Vec<PathBuf> {
        fn walk(node: &crate::tree::Node, out: &mut Vec<PathBuf>) {
            out.push(node.path().to_path_buf());
            if let crate::tree::Node::Folder { children, .. } = node {
                for child in children {
                    walk(child, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&vault.tree().unwrap(), &mut out);
        out
    }

    fn setup() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        let vault = Vault::open(dir.path()).unwrap();
        (dir, vault)
    }

    #[test]
    fn created_file_on_disk_is_reported_and_appears_in_tree() {
        let (dir, vault) = setup();
        let watcher = VaultWatcher::start(&vault).unwrap();
        let path = dir.path().join("new.md");
        fs::write(&path, "hello").unwrap();

        let mut seen = Vec::new();
        wait_until(&watcher, &mut seen, |s| {
            s.contains(&created(path.to_str().unwrap()))
        });
        assert!(tree_paths(&vault).contains(&path));
    }

    #[test]
    fn external_edit_reloads_clean_buffer_and_keeps_dirty_one() {
        let (dir, vault) = setup();
        let clean = dir.path().join("clean.md");
        let dirty = dir.path().join("dirty.md");
        fs::write(&clean, "old").unwrap();
        fs::write(&dirty, "old").unwrap();
        let watcher = VaultWatcher::start(&vault).unwrap();

        fs::write(&clean, "new from outside").unwrap();
        fs::write(&dirty, "new from outside").unwrap();

        let mut seen = Vec::new();
        wait_until(&watcher, &mut seen, |s| {
            s.contains(&modified(clean.to_str().unwrap()))
                && s.contains(&modified(dirty.to_str().unwrap()))
        });
        let open = [
            OpenBuffer {
                path: clean.clone(),
                dirty: false,
            },
            OpenBuffer {
                path: dirty.clone(),
                dirty: true,
            },
        ];
        assert_eq!(
            plan_buffer_updates(&seen, &open),
            vec![
                BufferUpdate::Reload(clean.clone()),
                BufferUpdate::KeepUnsaved(dirty.clone()),
            ]
        );
    }

    #[test]
    fn removed_file_is_reported_and_marks_buffer_missing() {
        let (dir, vault) = setup();
        let note = dir.path().join("gone.md");
        fs::write(&note, "x").unwrap();
        let watcher = VaultWatcher::start(&vault).unwrap();

        fs::remove_file(&note).unwrap();

        let mut seen = Vec::new();
        wait_until(&watcher, &mut seen, |s| {
            s.contains(&removed(note.to_str().unwrap()))
        });
        let open = [OpenBuffer {
            path: note.clone(),
            dirty: false,
        }];
        assert_eq!(
            plan_buffer_updates(&seen, &open),
            vec![BufferUpdate::Missing(note.clone())]
        );
    }

    #[test]
    fn renamed_file_is_reported_as_rename_or_remove_and_create() {
        let (dir, vault) = setup();
        let from = dir.path().join("before.md");
        let to = dir.path().join("after.md");
        fs::write(&from, "x").unwrap();
        let watcher = VaultWatcher::start(&vault).unwrap();

        fs::rename(&from, &to).unwrap();

        let (f, t) = (from.clone(), to.clone());
        let mut seen = Vec::new();
        wait_until(&watcher, &mut seen, move |s| {
            let renamed = s.contains(&Change::Renamed {
                from: f.clone(),
                to: t.clone(),
            });
            let split =
                s.contains(&Change::Removed(f.clone())) && s.contains(&Change::Created(t.clone()));
            renamed || split
        });
        assert!(tree_paths(&vault).contains(&to));
        assert!(!tree_paths(&vault).contains(&from));
    }

    #[test]
    fn gitignored_file_on_disk_is_not_reported() {
        let (dir, vault) = setup();
        fs::write(dir.path().join(".gitignore"), "*.draft.md\n").unwrap();
        let watcher = VaultWatcher::start(&vault).unwrap();
        let draft = dir.path().join("x.draft.md");
        let visible = dir.path().join("x.md");
        fs::write(&draft, "hidden").unwrap();
        fs::write(&visible, "shown").unwrap();

        let mut seen = Vec::new();
        wait_until(&watcher, &mut seen, |s| {
            s.iter().any(|c| c.touches(&visible))
        });
        seen.extend(watcher.next_changes(Duration::from_millis(300)));
        assert!(
            !seen.iter().any(|c| c.touches(&draft)),
            "gitignored file was reported: {seen:?}"
        );
    }
}
