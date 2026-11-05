//! File open and save for the app window (plan task 3.6). Owner: task 3.6.
//!
//! A [`Document`] is the editor plus file state (path and the saved buffer version, used for
//! the dirty marker). The window's root view is the workspace view (task 9.1), which shows a
//! [`Document`] beside the vault sidebar.

use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{
    actions, div, prelude::*, px, rgb, App, Context, Entity, Focusable, IntoElement, KeyBinding,
    Render, Window,
};
use md_editor::editor::{Editor, SAMPLE_TEXT};

actions!(md_editor_file_io, [Save]);

/// Read a file as text, exactly as stored. Bytes are not translated, so line endings, a byte
/// order mark and every UTF-8 sequence survive. Fails with `InvalidData` when the file is not
/// valid UTF-8.
pub fn read_file(path: &Path) -> io::Result<String> {
    fs::read_to_string(path)
}

/// Write `text` to `path` as UTF-8, with no added or removed bytes.
///
/// The text goes to a temporary file in the same directory, which then replaces `path` by a
/// rename, so a crash mid-write leaves the old file intact. An existing file keeps its
/// permissions. A symlink is followed, so the link itself is kept.
pub fn write_file(path: &Path, text: &str) -> io::Result<()> {
    let target = match fs::canonicalize(path) {
        Ok(resolved) => resolved,
        Err(err) if err.kind() == io::ErrorKind::NotFound => path.to_path_buf(),
        Err(err) => return Err(err),
    };
    let name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
    let dir = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let (temp, file) = create_temp_file(dir, name)?;
    let result = finish_write(file, &temp, &target, text);
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Create a new file next to `name` in `dir` with a name that does not exist yet.
fn create_temp_file(dir: &Path, name: &OsStr) -> io::Result<(PathBuf, File)> {
    loop {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut temp_name = OsString::from(".");
        temp_name.push(name);
        temp_name.push(format!(".{}.{n}.tmp", std::process::id()));
        let temp = dir.join(temp_name);
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => return Ok((temp, file)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }
}

fn finish_write(mut file: File, temp: &Path, target: &Path, text: &str) -> io::Result<()> {
    file.write_all(text.as_bytes())?;
    if let Ok(metadata) = fs::metadata(target) {
        fs::set_permissions(temp, metadata.permissions())?;
    }
    file.sync_all()?;
    drop(file);
    fs::rename(temp, target)
}

/// The folder that relative image paths of a note resolve against: the note's own folder, or
/// `None` for an unsaved note (task 4.3).
fn note_folder(path: Option<&Path>) -> Option<PathBuf> {
    path.and_then(Path::parent)
        .filter(|dir| !dir.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

/// True when the buffer version differs from the version saved on disk (3.6).
fn version_is_dirty(current: u64, saved: u64) -> bool {
    current != saved
}

/// Window title for a document: the file name, with a bullet while there are unsaved edits.
fn window_title(name: &str, dirty: bool) -> String {
    let marker = if dirty { "\u{2022} " } else { "" };
    format!("{marker}{name} - noteapp")
}

/// The window's root view: the editor plus file state.
pub struct Document {
    pub editor: Entity<Editor>,
    /// The file this document was opened from or saved to. `None` for the sample document.
    pub path: Option<PathBuf>,
    /// The buffer version at the last open or save. The document is dirty when the buffer's
    /// version differs from this value.
    pub saved_version: u64,
}

impl Document {
    /// A document holding [`SAMPLE_TEXT`] with no path.
    pub fn sample(cx: &mut Context<Self>) -> Self {
        Self::with_text(cx, SAMPLE_TEXT, None)
    }

    /// A document holding `text`, clean (not dirty), with `path` as its file.
    pub fn with_text(cx: &mut Context<Self>, text: &str, path: Option<PathBuf>) -> Self {
        let editor = cx.new(|cx| Editor::new(cx, text));
        let saved_version = editor.read(cx).buffer().version();
        let base_dir = note_folder(path.as_deref());
        editor.update(cx, |editor, cx| editor.set_base_dir(base_dir, cx));
        Self {
            editor,
            path,
            saved_version,
        }
    }

    /// True when the buffer has changed since the last open or save (3.6).
    pub fn is_dirty(&self, cx: &App) -> bool {
        version_is_dirty(self.editor.read(cx).buffer().version(), self.saved_version)
    }

    /// The file name shown in the title, or "Untitled" when there is no path.
    fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Write the buffer to `path`, or to `self.path` when `path` is `None`, then reset the
    /// saved version (3.6).
    pub fn save(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) -> io::Result<()> {
        let Some(target) = path.or_else(|| self.path.clone()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "document has no path to save to",
            ));
        };
        let (text, version) = {
            let editor = self.editor.read(cx);
            (editor.text(), editor.buffer().version())
        };
        write_file(&target, &text)?;
        let base_dir = note_folder(Some(&target));
        self.editor
            .update(cx, |editor, cx| editor.set_base_dir(base_dir, cx));
        self.path = Some(target);
        self.saved_version = version;
        cx.notify();
        Ok(())
    }
}

impl Render for Document {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dirty = self.is_dirty(cx);
        // Give the editor keyboard focus when nothing has it yet, so Cmd+S and typing reach it.
        let focus = self.editor.read(cx).focus_handle(cx);
        if window.focused(cx).is_none() {
            window.focus(&focus, cx);
        }
        window.set_window_title(&window_title(&self.file_name(), dirty));

        let mut root = div()
            .size_full()
            .relative()
            .on_action(cx.listener(|this, _: &Save, _window, cx| {
                if let Err(err) = this.save(None, cx) {
                    eprintln!("noteapp: save failed: {err}");
                }
            }))
            .child(self.editor.clone());
        if dirty {
            root = root.child(
                div()
                    .absolute()
                    .top(px(8.))
                    .right(px(8.))
                    .size(px(8.))
                    .rounded_full()
                    .bg(rgb(0xd97706)),
            );
        }
        root
    }
}

/// Register the Save action and its key binding (Cmd+S). Called from `main`.
pub fn init(cx: &mut App) {
    // Cmd+S on macOS, Ctrl+S on Linux (`cmd-` is Super+S there).
    #[cfg(target_os = "macos")]
    let save_key = "cmd-s";
    #[cfg(not(target_os = "macos"))]
    let save_key = "ctrl-s";
    cx.bind_keys([KeyBinding::new(save_key, Save, None)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A directory under the system temp dir, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let dir = std::env::temp_dir().join(format!(
                "noteapp-3.6-{tag}-{}-{nanos}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const SAMPLES: &[&[u8]] = &[
        b"# Title\n\nBody with a trailing newline.\n",
        b"no trailing newline",
        b"",
        b"windows line endings\r\nsecond line\r\n",
        "h\u{e9}llo \u{65e5}\u{672c}\u{8a9e} \u{1f389}\n".as_bytes(),
        "\u{feff}byte order mark\n".as_bytes(),
    ];

    #[test]
    fn open_then_save_without_edits_is_byte_identical() {
        let dir = TempDir::new("identical");
        for (i, bytes) in SAMPLES.iter().enumerate() {
            let path = dir.path(&format!("sample{i}.md"));
            fs::write(&path, bytes).unwrap();
            let text = read_file(&path).unwrap();
            write_file(&path, &text).unwrap();
            assert_eq!(fs::read(&path).unwrap(), *bytes, "sample {i} changed");
        }
    }

    #[test]
    fn write_then_read_returns_the_same_text() {
        let dir = TempDir::new("roundtrip");
        let path = dir.path("note.md");
        for bytes in SAMPLES {
            let text = std::str::from_utf8(bytes).unwrap();
            write_file(&path, text).unwrap();
            assert_eq!(read_file(&path).unwrap(), text);
        }
    }

    #[test]
    fn write_replaces_the_whole_previous_content() {
        let dir = TempDir::new("replace");
        let path = dir.path("note.md");
        write_file(&path, "a much longer first version\r\n").unwrap();
        write_file(&path, "short").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"short");
    }

    #[test]
    fn write_leaves_no_temporary_file_behind() {
        let dir = TempDir::new("no-temp");
        let path = dir.path("note.md");
        write_file(&path, "text").unwrap();
        let entries: Vec<_> = fs::read_dir(&dir.0)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(entries, vec![OsString::from("note.md")]);
    }

    #[test]
    fn write_fails_without_touching_the_file_when_the_directory_is_missing() {
        let dir = TempDir::new("missing-dir");
        let path = dir.path("no-such-dir").join("note.md");
        let err = write_file(&path, "text").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(!path.exists());
    }

    #[test]
    fn read_rejects_invalid_utf8() {
        let dir = TempDir::new("invalid-utf8");
        let path = dir.path("bad.md");
        fs::write(&path, [0x66, 0xff, 0xfe]).unwrap();
        assert_eq!(
            read_file(&path).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_keeps_existing_permissions_and_follows_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let dir = TempDir::new("unix");
        let real = dir.path("real.md");
        let link = dir.path("link.md");
        fs::write(&real, "old").unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&real, &link).unwrap();

        write_file(&link, "new").unwrap();

        assert_eq!(fs::read(&real).unwrap(), b"new");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        let mode = fs::metadata(&real).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
    }

    #[test]
    fn dirty_follows_the_buffer_version() {
        assert!(!version_is_dirty(0, 0));
        assert!(version_is_dirty(1, 0));
        // Saving records the current version, so the document is clean again.
        assert!(!version_is_dirty(5, 5));
        // Any later edit, including one that undoes an earlier one, makes it dirty again.
        assert!(version_is_dirty(6, 5));
    }

    #[test]
    fn window_title_shows_the_dirty_marker() {
        assert_eq!(window_title("note.md", false), "note.md - noteapp");
        assert_eq!(window_title("note.md", true), "\u{2022} note.md - noteapp");
    }
}
