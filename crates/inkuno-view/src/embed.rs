//! A page shown inside a window of the app, not in a window of its own. The page is a webview
//! child of the parent's native view. Only macOS has it: on the other systems the webview needs
//! a GTK loop or a message loop that the app does not run, so `child` refuses there and the
//! caller keeps the viewer's own window.

use std::path::Path;

use raw_window_handle::HasWindowHandle;

/// A message from the script of an embedded page (`keys::EMBEDDED`).
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// Escape: the keyboard goes back to the app.
    Focus,
    /// A Cmd chord the page does not need, in gpui keystroke syntax (`cmd-shift-p`).
    Key(String),
}

/// The message of an embedded page's script: `inkuno:focus`, or `inkuno:key:<keystroke>` with a
/// non-empty keystroke. None for any other body (a link the page sends is not one of these).
pub fn message(body: &str) -> Option<Message> {
    let rest = body.strip_prefix("inkuno:")?;
    if rest == "focus" {
        return Some(Message::Focus);
    }
    let key = rest.strip_prefix("key:")?;
    (!key.is_empty()).then(|| Message::Key(key.to_string()))
}

/// The gap kept between an embedded page and the edge it must end above (px).
const CLEAR_GAP: f32 = 8.;

/// The height of an embedded page whose top and bottom edges (window px) are `top` and `bottom`. It
/// ends `CLEAR_GAP` above `clear_above` when that edge is given and within the page, and is zero when
/// no room is left above it.
pub fn page_height(top: f32, bottom: f32, clear_above: Option<f32>) -> f32 {
    let end = clear_above.map_or(bottom, |edge| bottom.min(edge - CLEAR_GAP));
    (end - top).max(0.)
}

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

    /// Gives the keyboard back to the window of the page's parent: after this, the app's shortcuts
    /// reach it again instead of the page.
    pub fn focus_parent(&self) {
        let _ = self.view.focus_parent();
    }
}

/// Embeds the page of `file`, served from `root`, in the window `parent`. The page starts with no
/// place and is visible: call `Child::place` and `Child::show` to set it. Its Escape and app chords
/// are sent to `on_message` (see `keys::EMBEDDED`), which runs on the event loop. Fails when the
/// file is not a page under `root`, or when the webview cannot be built.
#[cfg(target_os = "macos")]
pub fn child(
    parent: &impl HasWindowHandle,
    file: &Path,
    root: &Path,
    on_message: impl Fn(Message) + 'static,
) -> Result<Child, String> {
    let (root, file) = crate::locate(file, Some(root))?;
    let url = inkuno_platform::web::url_for(&root, &file).ok_or_else(|| {
        format!(
            "{} cannot be served from {}",
            file.display(),
            root.display()
        )
    })?;
    let view = crate::page(&url, &root, Some(Box::new(on_message)))
        .with_initialization_script(crate::keys::EMBEDDED)
        .build_as_child(parent)
        .map_err(|error| format!("cannot open the page: {error}"))?;
    Ok(Child { view })
}

/// Embeds a page in a window. Not on this system: always an error.
#[cfg(not(target_os = "macos"))]
pub fn child(
    _parent: &impl HasWindowHandle,
    _file: &Path,
    _root: &Path,
    _on_message: impl Fn(Message) + 'static,
) -> Result<Child, String> {
    Err("not supported".to_string())
}

#[cfg(test)]
mod tests {
    use super::{Message, message, page_height};

    #[test]
    fn the_messages_of_the_embedded_script_are_read() {
        assert_eq!(message("inkuno:focus"), Some(Message::Focus));
        assert_eq!(
            message("inkuno:key:cmd-shift-p"),
            Some(Message::Key("cmd-shift-p".into()))
        );
    }

    #[test]
    fn other_bodies_are_not_messages() {
        for body in [
            "",
            "inkuno:",
            "inkuno:key:",
            "inkuno:focus ",
            "inkuno:Focus",
            "inkuno:keys:cmd-p",
            "key:cmd-p",
            "https://a.org",
            "mailto:a@b.c",
        ] {
            assert_eq!(message(body), None, "{body}");
        }
    }

    #[test]
    fn a_page_ends_above_the_edge_it_must_clear() {
        // No edge: the whole height.
        assert_eq!(page_height(100., 500., None), 400.);
        // An edge inside the page: it ends the gap above the edge.
        assert_eq!(page_height(100., 500., Some(300.)), 192.);
        // An edge below the page (the messages under another pane): the whole height.
        assert_eq!(page_height(100., 500., Some(600.)), 400.);
        // No room above the edge: no height.
        assert_eq!(page_height(100., 500., Some(105.)), 0.);
        assert_eq!(page_height(300., 500., Some(200.)), 0.);
    }
}
