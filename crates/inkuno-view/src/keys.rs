//! Keys of the viewer, as a script injected into every page: back, forward and reload.
//! A script rather than window events, because the focused webview swallows key presses.

/// Cmd-[ / Alt-Left go back, Cmd-] / Alt-Right go forward, Cmd-R / Ctrl-R reload; both sets
/// work on every system.
pub const SCRIPT: &str = r#"
window.addEventListener('keydown', function (e) {
  var key = e.key;
  if ((e.metaKey && key === '[') || (e.altKey && key === 'ArrowLeft')) {
    e.preventDefault(); history.back();
  } else if ((e.metaKey && key === ']') || (e.altKey && key === 'ArrowRight')) {
    e.preventDefault(); history.forward();
  } else if ((e.metaKey || e.ctrlKey) && (key === 'r' || key === 'R')) {
    e.preventDefault(); location.reload();
  }
}, true);
"#;

/// Links to the outside: a real click on a link to `http:`, `https:` or `mailto:` is not followed in
/// the viewer. Its address goes to the app (`window.ipc`), which opens it in the system's browser or
/// mail client. Only the main frame does this, and only for a click the user made (`isTrusted`);
/// a page that navigates or opens a window by itself gets nothing (the handlers refuse it).
pub const LINKS: &str = r#"
(function () {
  if (window !== window.top) return;
  window.addEventListener('click', function (e) {
    if (!e.isTrusted || e.button !== 0) return;
    var target = e.target instanceof Element ? e.target.closest('a[href]') : null;
    if (!target) return;
    var href;
    try { href = new URL(target.getAttribute('href'), document.baseURI).href; } catch (err) { return; }
    if (/^(https?|mailto):/i.test(href)) {
      e.preventDefault();
      window.ipc.postMessage(href);
    }
  }, true);
})();
"#;

#[cfg(test)]
mod tests {
    use super::{LINKS, SCRIPT};

    #[test]
    fn the_script_binds_the_three_actions() {
        for needle in ["history.back()", "history.forward()", "location.reload()"] {
            assert!(SCRIPT.contains(needle), "{needle}");
        }
    }

    #[test]
    fn the_link_script_sends_only_real_clicks_of_the_main_frame() {
        for needle in [
            "window !== window.top",
            "e.isTrusted",
            "window.ipc.postMessage(href)",
            "preventDefault",
        ] {
            assert!(LINKS.contains(needle), "{needle}");
        }
    }
}
