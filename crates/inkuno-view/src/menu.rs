//! The menu bar of the viewer on macOS. A WKWebView takes its Cmd-C, Cmd-V, Cmd-X, Cmd-A, Cmd-Z and
//! Cmd-W from the standard items of the application's menu: without a menu bar those keys do
//! nothing, and Cmd-Q does not quit. Only compiled on macOS; the other systems have no such bar.

use muda::{Menu, PredefinedMenuItem, Submenu};

/// Builds the menu bar and installs it in the application. Call it after the event loop exists
/// (the application object is created with it). Keep the returned menu alive while the window is
/// open: the bar goes with it.
pub fn install() -> muda::Result<Menu> {
    let menu = Menu::new();
    let app = Submenu::with_items("Inkuno", true, &[&PredefinedMenuItem::quit(None)])?;
    let edit = Submenu::with_items(
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(None),
            &PredefinedMenuItem::redo(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::cut(None),
            &PredefinedMenuItem::copy(None),
            &PredefinedMenuItem::paste(None),
            &PredefinedMenuItem::select_all(None),
        ],
    )?;
    let window = Submenu::with_items("Window", true, &[&PredefinedMenuItem::close_window(None)])?;
    menu.append_items(&[&app, &edit, &window])?;
    menu.init_for_nsapp();
    Ok(menu)
}
