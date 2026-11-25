//! An HTML page of the vault, shown in a tab: a native webview laid over the tab's area. macOS
//! only, where the webview can be a child of the app's window (`inkuno_view::embed`). The shell
//! hides every page before each frame; a page whose tab is drawn is built at its first paint and
//! placed over its area at each paint, so it shows again. Dropping the page closes the webview.

use std::path::{Path, PathBuf};

use gpui::{Bounds, Context, IntoElement, Pixels, Render, Window, canvas, div, prelude::*};
use inkuno_view::embed::{self, Child};

use crate::lang::tr;

/// An HTML page shown in a tab.
pub struct HtmlPage {
    path: PathBuf,
    /// The folder the page is served from: the page reads nothing outside it.
    root: PathBuf,
    /// The webview, built at the first paint of the tab.
    view: Option<Child>,
    /// Set once building the webview failed: the tab then says so, and no build is tried again.
    failed: bool,
    /// Px kept clear at the bottom of the tab, where the status messages are drawn over the note.
    bottom_inset: f32,
}

impl HtmlPage {
    /// The page of `path`, served from `root`. Nothing is built until the tab is drawn.
    pub fn new(path: &Path, root: &Path) -> Self {
        HtmlPage {
            path: path.to_path_buf(),
            root: root.to_path_buf(),
            view: None,
            failed: false,
            bottom_inset: 0.,
        }
    }

    /// Keeps `px` clear at the bottom of the tab: the page is placed above that band at the next paint.
    pub fn set_bottom_inset(&mut self, px: f32) {
        self.bottom_inset = px;
    }

    /// Hides the page. The next paint of its tab shows it again.
    pub fn hide(&self) {
        if let Some(view) = &self.view {
            view.show(false);
        }
    }

    /// Builds the webview if needed, then places it over `bounds` and shows it. Returns true
    /// only when the build has just failed, so that the tab is drawn again to say so.
    fn place(&mut self, bounds: Bounds<Pixels>, window: &Window) -> bool {
        let mut just_failed = false;
        if self.view.is_none() && !self.failed {
            match embed::child(window, &self.path, &self.root) {
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
        let height = f32::from(bounds.size.height) - self.bottom_inset;
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
                            if page.place(bounds, window) {
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
