//! Applies vault watcher updates in the app (task 9.2).
//!
//! [`WatchFeed`] runs the `workspace` crate's [`VaultWatcher`] on its own thread, so the window
//! thread never waits on file events. The window drains the feed on a timer. The decisions in
//! this file are pure functions, so they are tested without a window or a file system.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use workspace::tree::Vault;
use workspace::watcher::{BufferUpdate, Change, VaultWatcher};

/// How long the watcher thread waits for events before it looks again. The window polls the
/// feed on the same interval, so a change reaches the window within about half a second.
pub const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Changes from a vault watcher, read without blocking the window thread.
pub struct WatchFeed {
    rx: Receiver<Vec<Change>>,
}

impl WatchFeed {
    /// Starts watching `vault`. The watcher thread ends when the feed is dropped.
    pub fn start(vault: &Vault) -> io::Result<Self> {
        let watcher =
            VaultWatcher::start(vault).map_err(|err| io::Error::other(err.to_string()))?;
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("vault-watch".into())
            .spawn(move || loop {
                let changes = watcher.next_changes(POLL_INTERVAL);
                // The receiver is gone once the window has been dropped.
                if !changes.is_empty() && tx.send(changes).is_err() {
                    break;
                }
            })?;
        Ok(Self { rx })
    }

    /// Every batch that arrived since the last call, in order. Never blocks.
    pub fn drain(&self) -> Vec<Change> {
        self.rx.try_iter().flatten().collect()
    }
}

/// Hash of a file's text. Lets the app compare what is on disk with what it last synced,
/// without keeping a second copy of the text.
pub fn text_hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// True when the file on disk holds exactly what the document last wrote, read or acknowledged.
/// An event for such a file is the document's own save (or a repeat of one) and is ignored.
pub fn is_own_write(disk: u64, known: Option<u64>) -> bool {
    known == Some(disk)
}

/// `offset` clipped to `text` and moved back to a character boundary, so that a caret kept
/// across a reload stays inside the new text.
pub fn clip_caret(text: &str, offset: usize) -> usize {
    let mut at = offset.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

/// What the open note does after a planned update has been checked against the disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Settled {
    /// The file changed and the buffer is clean: load the new text.
    Reload,
    /// The file changed and the buffer has unsaved edits: keep them and show a banner.
    Conflict,
    /// The file is gone: keep the buffer, show a banner, and mark it dirty so a save recreates it.
    Deleted,
    /// The file was renamed: the buffer now uses this path.
    Moved(PathBuf),
}

/// Settles one planned update. `on_disk` says whether the file exists now and `dirty` whether
/// the buffer has unsaved edits.
pub fn settle(update: &BufferUpdate, on_disk: bool, dirty: bool) -> Settled {
    match update {
        BufferUpdate::Moved { to, .. } => Settled::Moved(to.clone()),
        _ if !on_disk => Settled::Deleted,
        _ if dirty => Settled::Conflict,
        _ => Settled::Reload,
    }
}

/// A banner over the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Banner {
    /// "File changed on disk", with Reload and Keep mine.
    Conflict,
    /// "File deleted on disk".
    Deleted,
}

/// The banner that a settled update leaves showing. `None` clears any banner. A move leaves
/// the banner as it was, so it has no entry here and the caller skips it.
pub fn banner_after(settled: &Settled) -> Option<Banner> {
    match settled {
        Settled::Conflict => Some(Banner::Conflict),
        Settled::Deleted => Some(Banner::Deleted),
        Settled::Reload | Settled::Moved(_) => None,
    }
}

/// True when a batch changes the shape of the tree: a note or folder appeared, went or moved,
/// or events were lost. A change to the text of a file does not need a rebuild.
pub fn needs_tree_refresh(batch: &[Change]) -> bool {
    batch
        .iter()
        .any(|change| !matches!(change, Change::Modified(_)))
}

/// The batch as the open note at `open` should see it. A file that appeared, or was renamed
/// to `open`, may have new content, so it also counts as modified. Without this, an editor that
/// saves by writing a temporary file and renaming it over the note would never reload the note.
pub fn changes_for_open_note(batch: &[Change], open: &Path) -> Vec<Change> {
    let mut out = batch.to_vec();
    for change in batch {
        match change {
            Change::Created(path) | Change::Renamed { to: path, .. } if path == open => {
                out.push(Change::Modified(path.clone()));
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    #[test]
    fn own_write_is_detected_by_content_hash() {
        let saved = text_hash("# Saved\n");
        // Our own save: the disk holds what we wrote.
        assert!(is_own_write(saved, Some(saved)));
        // An external edit changes the content, so it is not ours.
        assert!(!is_own_write(text_hash("# Saved\nmore\n"), Some(saved)));
        // Nothing known yet (no file): nothing is ours.
        assert!(!is_own_write(saved, None));
        assert_ne!(text_hash("a"), text_hash("b"));
    }

    #[test]
    fn caret_is_clipped_to_the_new_length() {
        assert_eq!(clip_caret("hello", 3), 3);
        assert_eq!(clip_caret("hello", 5), 5);
        assert_eq!(clip_caret("hi", 40), 2);
        assert_eq!(clip_caret("", 7), 0);
    }

    #[test]
    fn caret_moves_back_to_a_char_boundary() {
        // "é" is two bytes (offsets 1..3). Offset 2 is inside it.
        assert_eq!(clip_caret("aé", 2), 1);
        assert_eq!(clip_caret("aé", 3), 3);
        // A four-byte character.
        assert_eq!(clip_caret("🎉x", 2), 0);
    }

    #[test]
    fn clean_buffer_reloads_and_dirty_buffer_conflicts() {
        let update = BufferUpdate::Reload(p("/v/a.md"));
        assert_eq!(settle(&update, true, false), Settled::Reload);
        assert_eq!(settle(&update, true, true), Settled::Conflict);
        let update = BufferUpdate::KeepUnsaved(p("/v/a.md"));
        assert_eq!(settle(&update, true, true), Settled::Conflict);
    }

    #[test]
    fn missing_file_is_deleted_whatever_the_buffer_state() {
        let update = BufferUpdate::Missing(p("/v/a.md"));
        assert_eq!(settle(&update, false, false), Settled::Deleted);
        assert_eq!(settle(&update, false, true), Settled::Deleted);
        // A file that is back on disk after a remove event is a change, not a deletion.
        assert_eq!(settle(&update, true, false), Settled::Reload);
        assert_eq!(settle(&update, true, true), Settled::Conflict);
    }

    #[test]
    fn move_keeps_the_buffer_under_the_new_path() {
        let update = BufferUpdate::Moved {
            from: p("/v/a.md"),
            to: p("/v/b.md"),
        };
        assert_eq!(settle(&update, false, true), Settled::Moved(p("/v/b.md")));
    }

    #[test]
    fn banner_follows_the_settled_update() {
        assert_eq!(banner_after(&Settled::Conflict), Some(Banner::Conflict));
        assert_eq!(banner_after(&Settled::Deleted), Some(Banner::Deleted));
        assert_eq!(banner_after(&Settled::Reload), None);
        assert_eq!(banner_after(&Settled::Moved(p("/v/b.md"))), None);
    }

    #[test]
    fn content_changes_keep_the_tree_and_structural_ones_rebuild_it() {
        assert!(!needs_tree_refresh(&[Change::Modified(p("/v/a.md"))]));
        assert!(needs_tree_refresh(&[Change::Created(p("/v/new.md"))]));
        assert!(needs_tree_refresh(&[Change::Removed(p("/v/a.md"))]));
        assert!(needs_tree_refresh(&[Change::Rescan]));
        assert!(needs_tree_refresh(&[Change::Renamed {
            from: p("/v/a.md"),
            to: p("/v/b.md"),
        }]));
    }

    #[test]
    fn creating_or_renaming_onto_the_open_note_counts_as_a_modification() {
        let open = p("/v/a.md");
        let batch = [Change::Created(open.clone())];
        assert_eq!(
            changes_for_open_note(&batch, &open),
            vec![
                Change::Created(open.clone()),
                Change::Modified(open.clone())
            ]
        );
        let batch = [Change::Renamed {
            from: p("/v/a.md~"),
            to: open.clone(),
        }];
        assert!(changes_for_open_note(&batch, &open).contains(&Change::Modified(open.clone())));
        // Other notes are left alone.
        let batch = [Change::Created(p("/v/other.md"))];
        assert_eq!(changes_for_open_note(&batch, &open), batch.to_vec());
    }
}
