//! The views: each is a gpui entity (or the drawing of one) that talks to the shell through its
//! events. The editor, the side panel, the palette, the grid, the canvas, the viewer
//! and the kanban board. (The graph view was cut: plain Markdown editing only.)

pub mod canvas;
pub mod editor;
pub mod field;
/// The graph view is cut, but `keymap.rs` (a shared registry owned by the integrator) still
/// binds `graph::Cycle`: this stub keeps that path resolving until it is deregistered.
pub mod graph {
    gpui::actions!(graph, [Cycle]);
}
pub mod kanban;
pub mod nav;
pub mod palette;
pub mod sheet;
pub mod viewer;
