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

/// Keys of an embedded page (`embed::child`): Escape and the app's chords go to the app, not the page.
/// A click in the page makes its webview the first responder, which swallows every key; the script
/// hands those keys to the app (`window.ipc`) and stops them here. Escape sends `inkuno:focus`; a Cmd
/// chord sends `inkuno:key:<keystroke>` in gpui syntax (`cmd-shift-p`). The key is read from the
/// physical key (`e.code`), so Option and Shift do not change it and the arrows and Enter are keys.
/// The page keeps its copy, paste, cut, select-all and undo chords (Cmd-C, V, X, A without Shift, Cmd-Z
/// with or without Shift), and the chords of `SCRIPT` (Cmd-[, Cmd-], Cmd-R). A forwarded key does not
/// reach the page's own handlers. Only the main frame runs the script: keys inside an iframe are not
/// forwarded. Not in the viewer window.
#[cfg(target_os = "macos")]
pub const EMBEDDED: &str = r#"
(function () {
  var NAMES = {
    Backslash: '\\', BracketLeft: '[', BracketRight: ']', Minus: '-', Equal: '=', Comma: ',',
    Period: '.', Slash: '/', Semicolon: ';', Quote: "'", Backquote: '`', Enter: 'enter', Tab: 'tab',
    Backspace: 'backspace', ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down'
  };
  function keyOf(code) {
    var letter = /^Key([A-Z])$/.exec(code);
    if (letter) return letter[1].toLowerCase();
    var digit = /^Digit([0-9])$/.exec(code);
    if (digit) return digit[1];
    return Object.prototype.hasOwnProperty.call(NAMES, code) ? NAMES[code] : null;
  }
  window.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') {
      e.preventDefault(); e.stopImmediatePropagation();
      window.ipc.postMessage('inkuno:focus');
      return;
    }
    if (!e.metaKey) return;
    var key = keyOf(e.code);
    if (key === null) return;
    var kept = key === 'z' || key === 'r' ||
      (!e.shiftKey && ['c', 'v', 'x', 'a', '[', ']'].indexOf(key) !== -1);
    if (kept) return;
    var mods = [];
    if (e.ctrlKey) mods.push('ctrl');
    if (e.altKey) mods.push('alt');
    if (e.shiftKey) mods.push('shift');
    mods.push('cmd');
    e.preventDefault(); e.stopImmediatePropagation();
    window.ipc.postMessage('inkuno:key:' + mods.concat([key]).join('-'));
  }, true);
})();
"#;

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use super::EMBEDDED;
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

    #[cfg(target_os = "macos")]
    #[test]
    fn the_embedded_script_hands_escape_and_app_chords_to_the_app() {
        for needle in [
            "'inkuno:focus'",
            "'inkuno:key:'",
            "e.metaKey",
            "keyOf(e.code)",
            "e.stopImmediatePropagation()",
            "Backslash: '\\\\'",
            "window.ipc.postMessage",
            "}, true);",
        ] {
            assert!(EMBEDDED.contains(needle), "{needle}");
        }
    }
}
