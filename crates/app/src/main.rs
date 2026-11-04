// S2 hello window. Structure mirrors zed's crates/gpui/examples/hello_world.rs at the
// pinned rev (see Cargo.toml). GPUI is only compiled on macOS; other platforms print a notice.

#[cfg(target_os = "macos")]
use gpui::{
    div, prelude::*, px, rgb, size, App, Bounds, Context, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions,
};
#[cfg(target_os = "macos")]
use gpui_platform::application;

#[cfg(target_os = "macos")]
struct HelloNoteapp {
    text: SharedString,
}

#[cfg(target_os = "macos")]
impl Render for HelloNoteapp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x505050))
            .size(px(500.0))
            .justify_center()
            .items_center()
            .shadow_lg()
            .border_1()
            .border_color(rgb(0x0000ff))
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Hello, {}", self.text))
    }
}

#[cfg(target_os = "macos")]
fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("noteapp".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| HelloNoteapp {
                    text: "noteapp".into(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("noteapp targets macOS; GPUI window disabled on this platform.");
}
