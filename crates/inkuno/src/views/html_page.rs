//! An HTML page of the vault, shown in a tab: a native webview laid over the tab's area. macOS
//! only, where the webview can be a child of the app's window (`inkuno_view::embed`). The shell
//! hides every page before each frame; a page whose tab is drawn is built at its first paint and
//! placed over its area at each paint, so it shows again. Dropping the page closes the webview.

use std::io::Read;
use std::path::{Path, PathBuf};

use gpui::{
    Bounds, Context, EventEmitter, IntoElement, Pixels, Render, Window, canvas, div, prelude::*,
};
use inkuno_logic::html::html_title;
use inkuno_view::embed::{self, Child, Message};

use crate::lang::tr;

/// How much of the file is read for its title: the head, where `<title>` is.
const TITLE_BYTES: u64 = 16 * 1024;

/// What an HTML page tells the shell (macOS). The shell focuses the page's pane, gives the page back
/// the keyboard and runs the keys (`Shell::html_page_keys`).
#[derive(Clone, Debug)]
pub enum HtmlPageEvent {
    /// A key the page's script handed over: Escape, or a Cmd chord the page does not need.
    Keys(Message),
}

impl EventEmitter<HtmlPageEvent> for HtmlPage {}

/// An HTML page shown in a tab.
pub struct HtmlPage {
    path: PathBuf,
    /// The folder the page is served from: the page reads nothing outside it.
    root: PathBuf,
    /// The webview, built at the first paint of the tab.
    view: Option<Child>,
    /// Set once building the webview failed: the tab then says so, and no build is tried again.
    failed: bool,
    /// The top edge of the status messages in window px, when there are some: the page ends above it.
    clear_above: Option<f32>,
    /// The page's `<title>`, read at its opening: the tab is named by it.
    title: Option<String>,
}

impl HtmlPage {
    /// The page of `path`, served from `root`. Nothing is built until the tab is drawn.
    pub fn new(path: &Path, root: &Path) -> Self {
        HtmlPage {
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            view: None,
            failed: false,
            clear_above: None,
            title: read_title(path),
        }
    }

    /// The page's `<title>`, when it has a non-empty one (see `inkuno_logic::html::html_title`).
    pub fn title(&self) -> Option<String> {
        self.title.clone()
    }

    /// Gives the keyboard back to the app: the webview gives up the first responder, so the app's
    /// shortcuts reach the window again. Nothing when the page is not built.
    pub fn focus_parent(&self) {
        if let Some(view) = &self.view {
            view.focus_parent();
        }
    }

    /// The top edge of the status messages (window px), or None with none: the page is placed above
    /// it at the next paint. A page the messages do not reach keeps its whole height.
    pub fn set_clear_above(&mut self, edge: Option<f32>) {
        self.clear_above = edge;
    }

    /// Hides the page. The next paint of its tab shows it again.
    pub fn hide(&self) {
        if let Some(view) = &self.view {
            view.show(false);
        }
    }

    /// Builds the webview if needed, then places it over `bounds` and shows it. Returns true
    /// only when the build has just failed, so that the tab is drawn again to say so.
    fn place(&mut self, bounds: Bounds<Pixels>, window: &Window, cx: &mut Context<Self>) -> bool {
        let mut just_failed = false;
        if self.view.is_none() && !self.failed {
            // The page's keys come back as messages from the event loop, which must not touch gpui
            // at once: each one is emitted as an event from a task, and the shell handles it.
            let page = cx.weak_entity();
            let async_app = cx.to_async();
            let on_message = move |message: Message| {
                let (page, async_app) = (page.clone(), async_app.clone());
                async_app
                    .spawn(async move |cx| {
                        let _ = page.update(cx, |_, cx| cx.emit(HtmlPageEvent::Keys(message)));
                    })
                    .detach();
            };
            match embed::child(window, &self.path, &self.root, on_message) {
                Ok(view) => self.view = Some(view),
                Err(_) => {
                    self.failed = true;
                    just_failed = true;
                }
            }
        }
        let Some(view) = &self.view else {
            return just_failed;
        };
        let width = f32::from(bounds.size.width);
        let top = f32::from(bounds.origin.y);
        let height = embed::page_height(top, top + f32::from(bounds.size.height), self.clear_above);
        if width <= 0. || height <= 0. {
            view.show(false);
            return just_failed;
        }
        view.place(
            f64::from(f32::from(bounds.origin.x)),
            f64::from(f32::from(bounds.origin.y)),
            f64::from(width),
            f64::from(height),
        );
        view.show(true);
        just_failed
    }
}

impl Render for HtmlPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.failed {
            return div()
                .size_full()
                .p_6()
                .child(tr(
                    "This page cannot be shown here.",
                    "Cette page ne peut pas être affichée ici.",
                ))
                .into_any_element();
        }
        // The canvas has no drawing of its own: its paint places the webview over its bounds.
        let page = cx.entity();
        div()
            .size_full()
            .child(
                canvas(
                    move |bounds, window, cx| {
                        page.update(cx, |page, cx| {
                            if page.place(bounds, window, cx) {
                                cx.notify();
                            }
                        })
                    },
                    |_, _, _, _| {},
                )
                .size_full(),
            )
            .into_any_element()
    }
}

/// The title of the page at `path`, read from the head of the file. None when the file cannot be
/// read or has no title.
fn read_title(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut head = Vec::new();
    Read::take(file, TITLE_BYTES).read_to_end(&mut head).ok()?;
    html_title(&String::from_utf8_lossy(&head))
}
