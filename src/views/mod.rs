//! The views: each is a gpui entity (or the drawing of one) that talks to the shell through its
//! events. The editor, the side panel, the palette and the viewer.
//! (The graph view, the kanban board, the canvas and the sheet were cut: plain Markdown
//! editing only.)

pub mod editor;
pub mod field;
pub mod nav;
pub mod palette;
pub mod viewer;
