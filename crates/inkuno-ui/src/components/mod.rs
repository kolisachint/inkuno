//! Reusable UI pieces, one file per piece. Each file names the class of the v2 prototype
//! (`docs/design/prototypes/v2.html`) it mirrors, and takes its sizes, radii and colours from `Theme`.
//! Change a piece here and every window that draws it changes with it.

pub mod button;
pub mod card;
pub mod icon_button;
pub mod kbd;
pub mod list_row;
pub mod menu;
pub mod nav_row;
pub mod popover;
pub mod scroll;
pub mod segmented;
pub mod switch;
