//! The macOS window buttons (traffic lights), which gpui 0.2.2 cannot hide by itself. Takes the
//! window as its raw handle, so that this layer names no UI framework.

/// Shows or hides the window's three traffic lights. Only macOS has them: elsewhere this does nothing.
#[cfg(not(target_os = "macos"))]
pub fn set_traffic_lights_hidden<W>(_window: &W, _hidden: bool) {}
