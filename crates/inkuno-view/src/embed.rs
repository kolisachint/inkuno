//! A page shown inside a window of the app, not in a window of its own. The page is a webview
//! child of the parent's native view. Only macOS has it: on the other systems the webview needs
//! a GTK loop or a message loop that the app does not run, so `child` refuses there and the
//! caller keeps the viewer's own window.

use std::{
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hasher},
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use raw_window_handle::HasWindowHandle;

/// A message from the script of an embedded page (`keys::embedded_script`).
#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    /// Escape: the keyboard goes back to the app.
    Focus,
    /// A Cmd chord the page does not need, in gpui keystroke syntax (`cmd-shift-p`).
    Key(String),
}

/// The message of an embedded page's script: `inkuno:<token>:focus`, or `inkuno:<token>:key:<keystroke>`
/// with a non-empty keystroke, where `token` is the page's own secret (`new_token`). None for any other
/// body, and for every body when `token` is empty or does not match: a page's scripts cannot send an
/// app message without the token the script of `embed::child` keeps to itself.
pub fn message(body: &str, token: &str) -> Option<Message> {
    if token.is_empty() {
        return None;
    }
    let rest = body.strip_prefix("inkuno:")?;
    let rest = rest.strip_prefix(token)?.strip_prefix(':')?;
    if rest == "focus" {
        return Some(Message::Focus);
    }
    let key = rest.strip_prefix("key:")?;
    (!key.is_empty()).then(|| Message::Key(key.to_string()))
}

/// A random secret of one embedded page: 32 lowercase hex digits, so it is safe inside a script's
/// string literal. Each half is a hash keyed by std's random hasher state, over the time in nanoseconds.
pub fn new_token() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let mut token = String::with_capacity(32);
    for _ in 0..2 {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(nanos);
        token.push_str(&format!("{:016x}", hasher.finish()));
    }
    token
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
/// are sent to `on_message` (see `keys::embedded_script`), which runs on the event loop. Each page gets
/// its own token (`new_token`), which its script sends with every message: a message without it is not
/// read. Fails when the
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
    let token = new_token();
    let on_message: Box<dyn Fn(Message)> = Box::new(on_message);
    let view = crate::page(&url, &root, Some((token.clone(), on_message)))
        .with_initialization_script(crate::keys::embedded_script(&token))
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
    use super::{Message, message, new_token, page_height};

    const TOKEN: &str = "0123abcd";

    #[test]
    fn the_messages_of_the_embedded_script_are_read() {
        assert_eq!(
            message("inkuno:0123abcd:focus", TOKEN),
            Some(Message::Focus)
        );
        assert_eq!(
            message("inkuno:0123abcd:key:cmd-shift-p", TOKEN),
            Some(Message::Key("cmd-shift-p".into()))
        );
    }

    #[test]
    fn a_message_without_the_right_token_is_refused() {
        // Missing, wrong, or the token only as a prefix or a suffix.
        for body in [
            "inkuno:focus",
            "inkuno:key:cmd-q",
            "inkuno:0123abcd",
            "inkuno:0123abcd:",
            "inkuno:0123abce:focus",
            "inkuno:0123abcd0:focus",
            "inkuno:x0123abcd:focus",
            "inkuno:focus:0123abcd",
        ] {
            assert_eq!(message(body, TOKEN), None, "{body}");
        }
        // An empty token matches nothing, not even an empty one in the body.
        assert_eq!(message("inkuno::focus", ""), None);
        assert_eq!(message("inkuno:focus", ""), None);
    }

    #[test]
    fn other_bodies_are_not_messages() {
        for body in [
            "",
            "inkuno:",
            "inkuno:0123abcd:key:",
            "inkuno:0123abcd:focus ",
            "inkuno:0123abcd:Focus",
            "inkuno:0123abcd:keys:cmd-p",
            "key:cmd-p",
            "https://a.org",
            "mailto:a@b.c",
        ] {
            assert_eq!(message(body, TOKEN), None, "{body}");
        }
    }

    #[test]
    fn each_page_gets_a_new_token_of_hex_digits() {
        let (first, second) = (new_token(), new_token());
        assert_eq!(first.len(), 32);
        assert!(
            first
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        assert_ne!(first, second);
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
