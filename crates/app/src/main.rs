// App host. Opens the editor window on macOS and Linux (GPUI is only compiled there).
// Structure follows zed's crates/gpui/examples/hello_world.rs at the pinned rev (see Cargo.toml).

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod file_io;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod file_ops_ui;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod search_panel;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod text_input;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod watch;

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod workspace_view;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use gpui::{
    px, size, App, AppContext, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions,
};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use gpui_platform::application;

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn main() {
    application().run(|cx: &mut App| {
        md_editor::editor::init(cx);
        file_io::init(cx);
        file_ops_ui::init(cx);
        // Ctrl+Shift+F on Linux, Cmd+Shift+F on macOS (ToggleSearch, task 9.4).
        #[cfg(target_os = "macos")]
        let search_key = "cmd-shift-f";
        #[cfg(not(target_os = "macos"))]
        let search_key = "ctrl-shift-f";
        cx.bind_keys([KeyBinding::new(
            search_key,
            search_panel::ToggleSearch,
            None,
        )]);
        let bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("noteapp".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(workspace_view::WorkspaceView::new),
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn main() {
    println!("noteapp targets macOS and Linux; no window on this platform.");
}
