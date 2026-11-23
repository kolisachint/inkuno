//! Live reload: a watcher on the root, its events coalesced until 150 ms of quiet.

use std::{
    path::Path,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// How long the root stays quiet before the page reloads.
pub const QUIET: Duration = Duration::from_millis(150);

/// Calls `fire` once per burst of messages: after `quiet` without a new one. Returns when the
/// sender is gone (a burst still pending then fires first).
pub fn debounce<T>(rx: &Receiver<T>, quiet: Duration, mut fire: impl FnMut()) {
    loop {
        if rx.recv().is_err() {
            return;
        }
        loop {
            match rx.recv_timeout(quiet) {
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => {
                    fire();
                    break;
                }
                Err(RecvTimeoutError::Disconnected) => {
                    fire();
                    return;
                }
            }
        }
    }
}

/// Whether an event path concerns the served tree (`root` is canonical).
pub fn inside(root: &Path, path: &Path) -> bool {
    path.starts_with(root)
}

/// Whether a path lies in a folder or is named with a dot-component under `root` (`root` is
/// canonical): `.git`, editor swap folders and the like are no page.
pub fn hidden(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|relative| {
        relative
            .components()
            .any(|part| part.as_os_str().to_string_lossy().starts_with('.'))
    })
}

/// Whether a file system event means the served files may have changed: a create, a change or a
/// removal of a path under `root` that is not hidden. A read is no change: inotify reports every
/// read as `Access`, and the viewer's own reads of the pages would reload them forever.
pub fn is_change(root: &Path, event: &notify::Event) -> bool {
    !event.kind.is_access()
        && event
            .paths
            .iter()
            .any(|path| inside(root, path) && !hidden(root, path))
}

/// Watches `root` recursively and calls `on_change` after each burst of changes. The watcher
/// must be kept alive by the caller.
pub fn start(
    root: &Path,
    on_change: impl FnMut() + Send + 'static,
) -> Result<RecommendedWatcher, String> {
    let (tx, rx) = mpsc::channel::<()>();
    let base = root.to_path_buf();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if let Ok(event) = event
            && is_change(&base, &event)
        {
            let _ = tx.send(());
        }
    })
    .map_err(|error| format!("cannot watch {}: {error}", root.display()))?;
    watcher
        .watch(root, RecursiveMode::Recursive)
        .map_err(|error| format!("cannot watch {}: {error}", root.display()))?;
    thread::spawn(move || debounce(&rx, QUIET, on_change));
    Ok(watcher)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn a_burst_fires_once_and_a_later_one_again() {
        let (tx, rx) = mpsc::channel();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        let worker = thread::spawn(move || {
            debounce(&rx, Duration::from_millis(40), || {
                seen.fetch_add(1, Ordering::SeqCst);
            })
        });
        for _ in 0..5 {
            tx.send(()).unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        thread::sleep(Duration::from_millis(200));
        assert_eq!(count.load(Ordering::SeqCst), 1);
        tx.send(()).unwrap();
        thread::sleep(Duration::from_millis(200));
        assert_eq!(count.load(Ordering::SeqCst), 2);
        drop(tx);
        worker.join().unwrap();
        assert_eq!(count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_pending_burst_fires_when_the_sender_goes() {
        let (tx, rx) = mpsc::channel();
        tx.send(()).unwrap();
        drop(tx);
        let mut fired = 0;
        debounce(&rx, Duration::from_secs(5), || fired += 1);
        assert_eq!(fired, 1);
    }

    #[test]
    fn only_paths_under_the_root_count() {
        let root = Path::new("/site");
        assert!(inside(root, Path::new("/site/a/b.html")));
        assert!(!inside(root, Path::new("/site2/b.html")));
        assert!(!inside(root, Path::new("/other/b.html")));
    }

    #[test]
    fn reads_and_hidden_paths_are_no_change() {
        use notify::{
            Event, EventKind,
            event::{AccessKind, AccessMode, CreateKind, DataChange, ModifyKind},
        };
        let root = Path::new("/site");
        let read = Event::new(EventKind::Access(AccessKind::Open(AccessMode::Read)))
            .add_path(PathBuf::from("/site/a.css"));
        assert!(!is_change(root, &read));
        let write = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)))
            .add_path(PathBuf::from("/site/a.css"));
        assert!(is_change(root, &write));
        let created = Event::new(EventKind::Create(CreateKind::File))
            .add_path(PathBuf::from("/site/new.html"));
        assert!(is_change(root, &created));
        let hidden_write = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)))
            .add_path(PathBuf::from("/site/.git/index"));
        assert!(!is_change(root, &hidden_write));
        let outside = Event::new(EventKind::Modify(ModifyKind::Data(DataChange::Content)))
            .add_path(PathBuf::from("/other/a.css"));
        assert!(!is_change(root, &outside));
    }
}
