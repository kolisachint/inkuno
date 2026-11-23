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

#[cfg(test)]
mod tests {
    use super::SCRIPT;

    #[test]
    fn the_script_binds_the_three_actions() {
        for needle in ["history.back()", "history.forward()", "location.reload()"] {
            assert!(SCRIPT.contains(needle), "{needle}");
        }
    }
}
