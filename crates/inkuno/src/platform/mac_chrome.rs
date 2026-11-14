//! The macOS window buttons (traffic lights), which gpui 0.2.2 cannot hide by itself. Takes the
//! window as its raw handle, so that this layer names no UI framework.

/// Shows or hides the window's three traffic lights: close, minimize and zoom. Only macOS has them;
/// elsewhere this does nothing.
///
/// The window's `NSView` is the content view gpui created for it (`MacWindow::window_handle`,
/// gpui 0.2.2 `platform/mac/window.rs:1549-1556`); its `window` is the `NSWindow`, whose
/// `standardWindowButton:` gives each button (`NSWindowCloseButton` 0, `NSWindowMiniaturizeButton` 1,
/// `NSWindowZoomButton` 2; cocoa 0.26.0 `appkit.rs:389-397`). Nothing else of the window is touched.
/// AppKit must be called on the main thread: the shell calls this from its render, which runs there.
// objc 0.2's `msg_send!` and `sel!` expand a `cfg(feature = "cargo-clippy")` this crate does not
// declare: the lint is about their expansion, not about this code.
#[allow(unexpected_cfgs)]
#[cfg(target_os = "macos")]
pub fn set_traffic_lights_hidden<W: raw_window_handle::HasWindowHandle>(window: &W, hidden: bool) {
    use objc::runtime::{BOOL, NO, Object, YES};
    use objc::{msg_send, sel, sel_impl};
    use raw_window_handle::RawWindowHandle;

    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };
    let hidden: BOOL = if hidden { YES } else { NO };
    // SAFETY: `ns_view` is the live content view of this window (see above), so `window` is its
    // NSWindow. Each object is null-checked before it is messaged: on Apple runtimes a message to nil
    // is harmless (objc 0.2.7 `message/apple/mod.rs:21-29` does not check), but the checks keep the
    // intent plain. The button indexes are NSUInteger (`usize` here), as `standardWindowButton:` takes.
    unsafe {
        let view = handle.ns_view.as_ptr() as *mut Object;
        if view.is_null() {
            return;
        }
        let ns_window: *mut Object = msg_send![view, window];
        if ns_window.is_null() {
            return;
        }
        for kind in 0..3usize {
            let button: *mut Object = msg_send![ns_window, standardWindowButton: kind];
            if !button.is_null() {
                let _: () = msg_send![button, setHidden: hidden];
            }
        }
    }
}

/// Shows or hides the window's traffic lights. Only macOS has them: elsewhere this does nothing.
#[cfg(not(target_os = "macos"))]
pub fn set_traffic_lights_hidden<W>(_window: &W, _hidden: bool) {}
