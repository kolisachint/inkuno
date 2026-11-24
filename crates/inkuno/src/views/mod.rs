//! The views: each is a gpui entity (or the drawing of one) that talks to the shell through its
//! events. The editor, the side panel, the palette and the viewer.

pub mod editor;
pub mod field;
#[cfg(all(target_os = "macos", feature = "viewer"))]
pub mod html_page;
pub mod nav;
pub mod palette;
pub mod viewer;
