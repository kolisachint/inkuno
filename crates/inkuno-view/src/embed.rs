//! A page shown inside a window of the app, not in a window of its own. The page is a webview
//! child of the parent's native view. Only macOS has it: on the other systems the webview needs
//! a GTK loop or a message loop that the app does not run, so `child` refuses there and the
//! caller keeps the viewer's own window.

use std::path::Path;

use raw_window_handle::HasWindowHandle;

/// A page embedded in a window (macOS). Dropping it closes the page.
#[cfg(target_os = "macos")]
pub struct Child {
    view: wry::WebView,
}

/// A page embedded in a window. No system but macOS has one, so this has no value there.
#[cfg(not(target_os = "macos"))]
pub enum Child {}

#[cfg(target_os = "macos")]
impl Child {
    /// Places the page over the rectangle of its parent, in logical points from the top left.
    pub fn place(&self, x: f64, y: f64, width: f64, height: f64) {
        let rect = wry::Rect {
            position: wry::dpi::LogicalPosition::new(x, y).into(),
            size: wry::dpi::LogicalSize::new(width, height).into(),
        };
        let _ = self.view.set_bounds(rect);
    }

    /// Shows or hides the page. A hidden page keeps its state and its scroll.
    pub fn show(&self, visible: bool) {
        let _ = self.view.set_visible(visible);
    }
}

/// Embeds the page of `file`, served from `root`, in the window `parent`. The page starts with no
/// place and is visible: call `Child::place` and `Child::show` to set it. Fails when the file is
/// not a page under `root`, or when the webview cannot be built.
#[cfg(target_os = "macos")]
pub fn child(parent: &impl HasWindowHandle, file: &Path, root: &Path) -> Result<Child, String> {
    let (root, file) = crate::locate(file, Some(root))?;
    let url = inkuno_platform::web::url_for(&root, &file).ok_or_else(|| {
        format!(
            "{} cannot be served from {}",
            file.display(),
            root.display()
        )
    })?;
    let view = crate::page(&url, &root)
        .build_as_child(parent)
        .map_err(|error| format!("cannot open the page: {error}"))?;
    Ok(Child { view })
}

/// Embeds a page in a window. Not on this system: always an error.
#[cfg(not(target_os = "macos"))]
pub fn child(_parent: &impl HasWindowHandle, _file: &Path, _root: &Path) -> Result<Child, String> {
    Err("not supported".to_string())
}
