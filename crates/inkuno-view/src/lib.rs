//! Read-only viewer of local HTML prototypes: a native window (tao + wry) that shows a file
//! and follows the links between the pages under one root. No browser, no server: the files
//! come through the custom protocol `inkuno://workspace/` (`inkuno_platform::web`). No gpui.

pub mod embed;
mod keys;
#[cfg(target_os = "macos")]
mod menu;
mod protocol;
mod watch;

use std::{
    cell::Cell,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use inkuno_platform::web;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{NewWindowResponse, PageLoadEvent, WebViewBuilder};

#[cfg(target_os = "linux")]
use {tao::platform::unix::WindowExtUnix, wry::WebViewBuilderExtUnix};

/// What the helper threads and handlers tell the event loop.
enum UserEvent {
    /// The files changed: reload the page.
    Reload,
    /// A page finished loading: its title for the window.
    Title(String),
}

/// The policy of every navigation and new-window request: the workspace scheme, `about:blank` and
/// the frames of a page (`about:srcdoc`) stay; everything else is refused. Links to the outside
/// never load here: a click on one is sent to the app by the page's script (`keys::LINKS`), and
/// any other attempt, from a script or a redirect, opens nothing.
fn allow(url: &str) -> bool {
    let own = url
        .get(..format!("{}://", web::SCHEME).len())
        .is_some_and(|head| head.eq_ignore_ascii_case(&format!("{}://", web::SCHEME)));
    own || url == "about:blank" || url == "about:srcdoc"
}

/// Whether a link may be opened now: at most one per second, so a page cannot open a flood of them.
fn may_open(last: Option<Instant>, now: Instant) -> bool {
    last.is_none_or(|last| now.duration_since(last) >= Duration::from_secs(1))
}

/// The root and the canonical file to show. The root defaults to the file's folder; the file
/// must be a file under it.
fn locate(file: &Path, root: Option<&Path>) -> Result<(PathBuf, PathBuf), String> {
    let file = file
        .canonicalize()
        .map_err(|error| format!("{}: {error}", file.display()))?;
    if !file.is_file() {
        return Err(format!("{} is not a file", file.display()));
    }
    let root = match root {
        Some(root) => root
            .canonicalize()
            .map_err(|error| format!("{}: {error}", root.display()))?,
        None => file
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| format!("{} has no folder", file.display()))?,
    };
    if !file.starts_with(&root) {
        return Err(format!(
            "{} is not inside {}",
            file.display(),
            root.display()
        ));
    }
    Ok((root, file))
}

/// The window title of a page URL: its file name.
fn title_of(root: &Path, url: &str) -> Option<String> {
    let path = web::resolve(root, url).ok()?;
    let name = path.file_name()?.to_string_lossy();
    Some(format!("{name} — Inkuno"))
}

/// The webview of the page at `url`, served from `root`: the scripts, the custom protocol, the
/// navigation policy and the link handler. The window and an embedded page share it.
fn page<'a>(url: &str, root: &Path) -> WebViewBuilder<'a> {
    let serve_root = root.to_path_buf();
    let last_open = Cell::new(None::<Instant>);
    WebViewBuilder::new()
        .with_url(url)
        .with_initialization_script(keys::SCRIPT)
        .with_initialization_script(keys::LINKS)
        .with_custom_protocol(web::SCHEME.to_string(), move |_id, request| {
            protocol::respond(&serve_root, &request.uri().to_string())
        })
        .with_ipc_handler(move |request| {
            // The page's script sends the address of a link the user clicked.
            let link = request.body();
            let now = Instant::now();
            if web::is_external(link) && may_open(last_open.get(), now) {
                last_open.set(Some(now));
                let _ = open::that_detached(link);
            }
        })
        .with_navigation_handler(|url| allow(&url))
        .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
}

/// Opens a window on `file` and follows its links under `root` (default: the file's folder).
/// Returns an error when the file or root is unusable or the window cannot be built. On
/// success it runs the event loop until the window closes; on some platforms that never
/// returns, the process exits instead.
pub fn run(file: &Path, root: Option<&Path>) -> Result<(), String> {
    let (root, file) = locate(file, root)?;
    let url = web::url_for(&root, &file).ok_or_else(|| {
        format!(
            "{} cannot be served from {}",
            file.display(),
            root.display()
        )
    })?;

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    // The menu bar gives the webview its copy, paste and close keys; without it they do nothing.
    #[cfg(target_os = "macos")]
    let menu = match menu::install() {
        Ok(menu) => Some(menu),
        Err(error) => {
            eprintln!("inkuno view: no menu bar: {error}");
            None
        }
    };
    let window = WindowBuilder::new()
        .with_title(title_of(&root, &url).unwrap_or_else(|| "Inkuno".into()))
        .with_inner_size(tao::dpi::LogicalSize::new(1100.0, 800.0))
        .build(&event_loop)
        .map_err(|error| format!("cannot open the window: {error}"))?;

    let title_root = root.clone();
    let title_proxy = event_loop.create_proxy();
    let builder = page(&url, &root).with_on_page_load_handler(move |event, url| {
        // A page that did not load (a refused or missing link) has no file name: the plain title.
        if matches!(event, PageLoadEvent::Finished) {
            let title = title_of(&title_root, &url).unwrap_or_else(|| "Inkuno".into());
            let _ = title_proxy.send_event(UserEvent::Title(title));
        }
    });

    #[cfg(target_os = "linux")]
    let webview = {
        let vbox = window
            .default_vbox()
            .ok_or("the window has no GTK container")?;
        builder.build_gtk(vbox)
    };
    #[cfg(not(target_os = "linux"))]
    let webview = builder.build(&window);
    let webview = webview.map_err(|error| format!("cannot open the viewer: {error}"))?;

    let reload_proxy = event_loop.create_proxy();
    // Without a watcher the page still shows; it just does not reload when the files change.
    let watcher = match watch::start(&root, move || {
        let _ = reload_proxy.send_event(UserEvent::Reload);
    }) {
        Ok(watcher) => Some(watcher),
        Err(error) => {
            eprintln!("inkuno view: live reload is off: {error}");
            None
        }
    };

    event_loop.run(move |event, _, control_flow| {
        // The watcher and the webview live as long as the loop.
        let _keep = &watcher;
        #[cfg(target_os = "macos")]
        let _menu = &menu;
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::UserEvent(UserEvent::Reload) => {
                let _ = webview.evaluate_script("location.reload()");
            }
            Event::UserEvent(UserEvent::Title(title)) => window.set_title(&title),
            _ => {}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn the_policy_keeps_the_workspace_and_refuses_the_rest() {
        for url in [
            "inkuno://workspace/a.html",
            "INKUNO://workspace/",
            "about:blank",
            "about:srcdoc",
        ] {
            assert!(allow(url), "{url}");
        }
        // Links to the outside are refused too: a click sends them to the app, never a load here.
        for url in [
            "https://a.org",
            "HTTP://a.org",
            "mailto:a@b.c",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "data:text/html,x",
            "",
            "ink",
        ] {
            assert!(!allow(url), "{url}");
        }
    }

    #[test]
    fn a_link_is_opened_at_most_once_a_second() {
        let start = Instant::now();
        assert!(may_open(None, start));
        assert!(!may_open(Some(start), start + Duration::from_millis(999)));
        assert!(may_open(Some(start), start + Duration::from_secs(1)));
    }

    #[test]
    fn locate_defaults_the_root_and_checks_containment() {
        let dir = std::env::temp_dir().join(format!("inkuno-view-locate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/a.html"), "x").unwrap();
        fs::write(dir.join("b.html"), "x").unwrap();
        let (root, file) = locate(&dir.join("sub/a.html"), None).unwrap();
        assert_eq!(root, dir.join("sub").canonicalize().unwrap());
        assert_eq!(file, dir.join("sub/a.html").canonicalize().unwrap());
        assert!(locate(&dir.join("sub/a.html"), Some(&dir)).is_ok());
        assert!(locate(&dir.join("b.html"), Some(&dir.join("sub"))).is_err());
        assert!(locate(&dir.join("sub"), None).is_err());
        assert!(locate(&dir.join("missing.html"), None).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_title_is_the_file_name() {
        let dir = std::env::temp_dir().join(format!("inkuno-view-title-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("p.html"), "x").unwrap();
        assert_eq!(
            title_of(&dir, "inkuno://workspace/p.html#x").as_deref(),
            Some("p.html — Inkuno")
        );
        assert_eq!(title_of(&dir, "https://a.org"), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
