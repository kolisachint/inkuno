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

/// The token placeholder of `EMBEDDED`, replaced by `embedded_script`.
#[cfg(target_os = "macos")]
const TOKEN_PLACEHOLDER: &str = "__INKUNO_TOKEN__";

/// Keys of an embedded page (`embed::child`): Escape and the app's chords go to the app, not the page.
/// A click in the page makes its webview the first responder, which swallows every key; the script
/// hands those keys to the app (`window.ipc`) and stops them here. Escape sends `inkuno:<token>:focus`;
/// a Cmd chord sends `inkuno:<token>:key:<keystroke>` in gpui syntax (`cmd-shift-p`). The token is the
/// page's secret (`embed::new_token`), written into this script only: the page's own scripts cannot
/// read it, and a message without it is refused by the app.
///
/// The key is the typed character (`e.key`, lower-cased) when it is one printable ASCII character, so
/// a non-QWERTY layout sends the letter it types. Other keys (the arrows, Enter, Tab, Backspace, and
/// digits or punctuation that are not a single character) come from the physical key (`e.code`).
/// The page keeps its copy, paste, cut, select-all and undo chords (Cmd-C, V, X, A without Shift,
/// Cmd-Z with or without Shift), and the chords of `SCRIPT` (Cmd-[, Cmd-], Cmd-R), judged on that
/// same key. A forwarded key does not reach the page's own handlers. Only the main frame runs the
/// script: keys inside an iframe are not forwarded. Not in the viewer window.
#[cfg(target_os = "macos")]
const EMBEDDED: &str = r#"
(function () {
  var TOKEN = '__INKUNO_TOKEN__';
  var ipc = window.ipc;
  var NAMES = {
    Backslash: '\\', BracketLeft: '[', BracketRight: ']', Minus: '-', Equal: '=', Comma: ',',
    Period: '.', Slash: '/', Semicolon: ';', Quote: "'", Backquote: '`', Enter: 'enter', Tab: 'tab',
    Backspace: 'backspace', ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down'
  };
  function keyOf(e) {
    if (typeof e.key === 'string' && /^[!-~]$/.test(e.key)) return e.key.toLowerCase();
    var letter = /^Key([A-Z])$/.exec(e.code);
    if (letter) return letter[1].toLowerCase();
    var digit = /^Digit([0-9])$/.exec(e.code);
    if (digit) return digit[1];
    return Object.prototype.hasOwnProperty.call(NAMES, e.code) ? NAMES[e.code] : null;
  }
  function send(message) {
    ipc.postMessage('inkuno:' + TOKEN + ':' + message);
  }
  window.addEventListener('keydown', function (e) {
    if (!e.isTrusted) return;
    if (e.key === 'Escape') {
      e.preventDefault(); e.stopImmediatePropagation();
      send('focus');
      return;
    }
    if (!e.metaKey) return;
    var key = keyOf(e);
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
    send('key:' + mods.concat([key]).join('-'));
  }, true);
})();
"#;

/// The script of one embedded page: `EMBEDDED` with the page's `token` (`embed::new_token`). The token
/// is written into a string literal, so it must be plain letters and digits: `embed::new_token` makes one.
#[cfg(target_os = "macos")]
pub fn embedded_script(token: &str) -> String {
    EMBEDDED.replace(TOKEN_PLACEHOLDER, token)
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "macos")]
    use super::{EMBEDDED, embedded_script};
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
            "send('focus')",
            "send('key:'",
            "e.metaKey",
            "keyOf(e)",
            "e.key.toLowerCase()",
            "e.stopImmediatePropagation()",
            "Backslash: '\\\\'",
            "e.isTrusted",
            "}, true);",
        ] {
            assert!(EMBEDDED.contains(needle), "{needle}");
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn the_embedded_script_carries_its_token_and_posts_only_with_it() {
        let script = embedded_script("0123abcd");
        assert!(script.contains("var TOKEN = '0123abcd';"));
        assert!(!script.contains("__INKUNO_TOKEN__"));
        // Every post goes through `send`, which prefixes the token: no token-less post is left.
        assert_eq!(script.matches("postMessage(").count(), 1);
        assert!(script.contains("ipc.postMessage('inkuno:' + TOKEN + ':' + message)"));
        assert!(!script.contains("'inkuno:focus'"));
        assert!(!script.contains("'inkuno:key:'"));
    }
}
