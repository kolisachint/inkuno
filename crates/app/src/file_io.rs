//! File open and save for the app window (plan task 3.6). Owner: task 3.6.
//!
//! The window's root view is a [`Document`]: the editor plus file state (path and the saved
//! buffer version, used for the dirty marker).

#![allow(dead_code, unused_variables)]

use std::io;
use std::path::{Path, PathBuf};

use gpui::{actions, div, prelude::*, App, Context, Entity, IntoElement, Render, Window};
use md_editor::editor::{Editor, SAMPLE_TEXT};

actions!(md_editor_file_io, [Save]);

/// Read a file as text, exactly as stored. 3.6 tests that open then save without edits gives
/// a byte-identical file.
pub fn read_file(path: &Path) -> io::Result<String> {
    todo!("3.6")
}

/// Write `text` to `path` as UTF-8, with no added or removed bytes.
pub fn write_file(path: &Path, text: &str) -> io::Result<()> {
    todo!("3.6")
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
        let editor = cx.new(|cx| Editor::new(cx, SAMPLE_TEXT));
        let saved_version = editor.read(cx).buffer().version();
        Self {
            editor,
            path: None,
            saved_version,
        }
    }

    /// True when the buffer has changed since the last open or save (3.6).
    pub fn is_dirty(&self, cx: &App) -> bool {
        todo!("3.6")
    }

    /// Write the buffer to `path`, or to `self.path` when `path` is `None`, then reset the
    /// saved version (3.6).
    pub fn save(&mut self, path: Option<PathBuf>, cx: &mut Context<Self>) -> io::Result<()> {
        todo!("3.6")
    }
}

impl Render for Document {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 3.6 adds the dirty marker and the Save action handler here.
        div().size_full().child(self.editor.clone())
    }
}

/// Register the Save action and its key binding (Cmd+S). Called from `main` (3.6 fills it in).
pub fn init(cx: &mut App) {}

/// The document shown at start-up. 3.6 replaces the sample with the file named on the command
/// line, when one is given.
pub fn open_initial(cx: &mut Context<Document>) -> Document {
    Document::sample(cx)
}
