//! The shared `Editor` entity (Phase 3 skeleton).
//!
//! `Editor` owns the md_core [`Buffer`] and all per-editor state. Each plan task owns the
//! state type for its own field and the `impl Editor` blocks in its own file, so no task adds
//! fields here. Wiring is done through the `HOOK` comments below: each task uncomments or
//! replaces its own one line. The skeleton compiles with every HOOK line commented out.
//!
//! Edits must go through `Editor::replace_range` (task 3.2, in `selection.rs`), never through
//! the buffer directly, so that the selection stays valid.

use std::path::{Path, PathBuf};

use gpui::{div, prelude::*, App, Context, FocusHandle, Focusable, IntoElement, Render, Window};
use md_core::buffer::Buffer;

use crate::block_list::BlockListState;
use crate::blocks::code_fence::CodeCache;
use crate::element::Layouts;
use crate::ime::ImeState;
use crate::selection::SelectionState;

/// Text shown when the app starts without a file (used by the app's `Document::sample`).
pub const SAMPLE_TEXT: &str = "# Sample\n\nThis is a sample note.\n";

/// The editor view. It is the root of the editor window and an entity like any other view.
// The state fields are read only once their tasks fill in the bodies.
#[allow(dead_code)]
pub struct Editor {
    /// The source of truth (DESIGN §2). Read with [`Editor::buffer`]. Change only through
    /// `Editor::replace_range` (3.2).
    pub(crate) buffer: Buffer,
    /// Focus for keyboard input. Set in `Editor::new`, used by `track_focus` in render.
    pub(crate) focus_handle: FocusHandle,
    /// Selection and caret state (task 3.2). Owns `goal_column`, which 3.3 uses.
    pub(crate) selection: SelectionState,
    /// Geometry of every visible block in the last frame (each block's `TextElement` writes its
    /// own entry during paint). Read by hit testing (3.2) and by IME bounds (3.4).
    pub(crate) layouts: Layouts,
    /// IME marked-text state (task 3.4).
    pub(crate) ime: ImeState,
    /// Block list state for the virtualized list (task 3.5).
    pub(crate) blocks: BlockListState,
    /// The folder of the note. Relative image paths resolve against it (task 4.3). Set by the app.
    pub(crate) base_dir: Option<PathBuf>,
    /// Highlighted code fence blocks, so that a frame does not highlight them again (task 5.3).
    pub(crate) code_cache: CodeCache,
}

impl Editor {
    /// Create an editor showing `text`, with the caret at offset 0.
    pub fn new(cx: &mut Context<Self>, text: &str) -> Self {
        Self {
            buffer: Buffer::new(text),
            focus_handle: cx.focus_handle(),
            selection: SelectionState::default(),
            layouts: Layouts::default(),
            ime: ImeState::default(),
            blocks: BlockListState::default(),
            base_dir: None,
            code_cache: CodeCache::default(),
        }
    }

    /// Set the folder that relative image paths resolve against: the folder of the note, or
    /// `None` for an unsaved note (task 4.3).
    pub fn set_base_dir(&mut self, dir: Option<PathBuf>, cx: &mut Context<Self>) {
        self.base_dir = dir;
        cx.notify();
    }

    /// The folder that relative image paths resolve against, if one is set.
    pub fn base_dir(&self) -> Option<&Path> {
        self.base_dir.as_deref()
    }

    /// The buffer, for reading (text, length, version). Edits go through `replace_range`.
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// The full source text of the buffer.
    pub fn text(&self) -> String {
        self.buffer.text()
    }
}

impl Focusable for Editor {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Editor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = cx.entity();
        // The background proves in a screenshot that the view is painted, even with no content.
        let root = div()
            .size_full()
            .bg(gpui::white())
            .track_focus(&self.focus_handle)
            .key_context("MdEditor");
        // HOOK 3.3 (keys.rs): attach movement, copy, cut and paste handlers to the root.
        let root = crate::keys::attach(root, &editor);
        // HOOK 3.7 (edit_keys.rs): attach Backspace, Delete, Enter, Tab, undo, redo and select all.
        let root = crate::edit_keys::attach(root, &editor);
        // HOOK 3.5 (block_list.rs): the virtualized block list (replaced the 3.1 whole-buffer TextElement).
        let root = root.child(crate::block_list::render_blocks(&editor, cx));
        let _ = &editor;
        root
    }
}

/// App-wide setup for the editor. Call once at start-up, before opening a window.
pub fn init(cx: &mut App) {
    // HOOK 3.3 (keys.rs): register the default key bindings for the "MdEditor" context.
    crate::keys::bind_keys(cx);
    // HOOK 3.7 (edit_keys.rs): register the editing key bindings for the "MdEditor" context.
    crate::edit_keys::bind_keys(cx);
    let _ = cx;
}
