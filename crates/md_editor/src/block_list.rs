//! Virtualized block list (plan task 3.5). Owner: task 3.5.
//!
//! The list is a GPUI `ListState` (`list()` element). Each item draws one top-level md_core
//! block with `TextElement`, so only visible blocks are laid out (DESIGN §3.1).

#![allow(dead_code, unused_variables)]

use std::cell::Cell;
use std::rc::Rc;

use gpui::{px, AnyElement, Context, Entity, ListAlignment, ListState};
use md_core::Block;

use crate::editor::Editor;

/// Block list state held by `Editor` (field `blocks`). Owned by 3.5.
pub struct BlockListState {
    /// The GPUI list state. Its item count must match the number of blocks.
    pub list: ListState,
    /// Number of block items laid out since creation. The acceptance counter for 3.5, shared
    /// with the list's render closure.
    pub layouts: Rc<Cell<usize>>,
}

impl Default for BlockListState {
    fn default() -> Self {
        Self {
            list: ListState::new(0, ListAlignment::Top, px(200.)),
            layouts: Rc::new(Cell::new(0)),
        }
    }
}

/// Top-level md_core blocks for `text`. Pure function, unit-tested in 3.5. 3.5 may cache the
/// result, for example with `md_core::block::BlockMap`.
pub fn blocks_for(text: &str) -> Vec<Block> {
    todo!("3.5")
}

/// Build the list element for the editor content.
///
/// Called from `Editor::render` (HOOK 3.5). It replaces the HOOK 3.1 line, which draws the
/// whole buffer as one `TextElement`.
pub fn render_blocks(editor: &Entity<Editor>, cx: &mut Context<Editor>) -> AnyElement {
    todo!("3.5")
}
