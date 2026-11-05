//! Read-only image block (plan task 4.3). Owner: task 4.3.
//!
//! A paragraph whose whole text is one image, `![alt](src)`, is drawn as the picture when the
//! caret is outside it (`blocks::item_element`). Relative paths resolve against the note's folder
//! (`Editor::base_dir`). A missing file, or a remote address (not loaded yet), shows a placeholder
//! with the alt text. [`parse_image`] and [`resolve`] are pure and unit-tested on every platform.

use std::ops::Range;
use std::panic::Location;
use std::path::{Path, PathBuf};

use gpui::{
    div, img, prelude::*, px, relative, AnyElement, App, Bounds, ElementId, Entity,
    GlobalElementId, ImgResourceLoader, InspectorElementId, IntoElement, LayoutId, Pixels,
    Resource, Style, Window,
};

use crate::editor::Editor;
use crate::element::LayoutSnapshot;
use crate::theme::Theme;

/// An image that is the whole text of a paragraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageRef {
    pub alt: String,
    pub src: String,
}

/// Parse `text` (one paragraph block) as a single image `![alt](src)`. An optional title
/// (`"..."` or `'...'`) after the source is allowed, and `<...>` may wrap a source with spaces.
/// Returns `None` when anything else is in the text, including a second image or a link around
/// the image. Pure, unit-tested.
pub fn parse_image(text: &str) -> Option<ImageRef> {
    let line = text.trim();
    if line.contains('\n') {
        return None;
    }
    let inner = line.strip_prefix("![")?.strip_suffix(')')?;
    let close = inner.find("](")?;
    let alt = &inner[..close];
    if alt.contains(']') {
        return None;
    }
    let destination = inner[close + 2..].trim();
    let (src, rest) = match destination.strip_prefix('<') {
        Some(wrapped) => {
            let end = wrapped.find('>')?;
            (&wrapped[..end], &wrapped[end + 1..])
        }
        None => match destination.find(char::is_whitespace) {
            Some(index) => (&destination[..index], &destination[index..]),
            None => (destination, ""),
        },
    };
    let rest = rest.trim();
    if !(rest.is_empty() || rest.starts_with('"') || rest.starts_with('\'')) {
        return None;
    }
    // A second image or a nested link in the source means the text is not one image.
    if src.is_empty() || src.contains("](") || src.contains("![") {
        return None;
    }
    Some(ImageRef {
        alt: alt.to_string(),
        src: src.to_string(),
    })
}

/// Where an image comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageLocation {
    /// An http or https address. It is not loaded yet, so it shows a placeholder.
    Remote,
    /// A file. A relative source is joined to the note's folder when there is one.
    Local(PathBuf),
}

/// Percent-decode an image source (`%20` to a space, `%25` to `%`, ...). Upper- and
/// lowercase hex digits both decode, and multibyte UTF-8 sequences decode when the bytes
/// form valid UTF-8. A `%` that is not followed by two hex digits (`%2G`, a trailing `%`)
/// is left as-is, as is a decoded result that is not valid UTF-8 (e.g. `%FF`). `+` is kept:
/// it only means a space in form encoding, not in a path. Pure, unit-tested.
pub fn percent_decode(src: &str) -> String {
    if !src.contains('%') {
        return src.to_string();
    }
    let bytes = src.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let (Some(&hi), Some(&lo)) = (bytes.get(i + 1), bytes.get(i + 2)) {
                if let (Some(h), Some(l)) = (hex_val(hi), hex_val(lo)) {
                    out.push(h << 4 | l);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| src.to_string())
}

/// The value of one ASCII hex digit, or `None`.
fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Resolve an image source against the note's folder. Pure, unit-tested.
pub fn resolve(src: &str, base_dir: Option<&Path>) -> ImageLocation {
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageLocation::Remote;
    }
    // Only local paths are decoded: a remote address is never loaded, so its escapes are
    // left untouched. `<...>` destinations arrive here with the brackets already stripped
    // by `parse_image`, so they decode the same way.
    let decoded = percent_decode(src);
    let path = Path::new(&decoded);
    match base_dir {
        Some(dir) if path.is_relative() => ImageLocation::Local(dir.join(path)),
        _ => ImageLocation::Local(path.to_path_buf()),
    }
}

/// The element for an image paragraph at `source`. `base_dir` is the folder of the note.
pub fn image_element(
    editor: Entity<Editor>,
    source: Range<usize>,
    image: ImageRef,
    base_dir: Option<&Path>,
) -> AnyElement {
    let path = match resolve(&image.src, base_dir) {
        ImageLocation::Remote => None,
        ImageLocation::Local(path) => Some(path),
    };
    ImageBlock {
        editor,
        source,
        alt: image.alt,
        path,
    }
    .into_any_element()
}

/// A box with the picture, or a placeholder. Its layout has no text lines, so the block is
/// recorded in `Editor::layouts` with none, and a click on it places the caret at its start.
pub struct ImageBlock {
    editor: Entity<Editor>,
    /// Byte range of the paragraph in the buffer.
    source: Range<usize>,
    alt: String,
    /// The local file to show, or `None` for a remote address.
    path: Option<PathBuf>,
}

impl ImageBlock {
    /// The picture, or a placeholder when the file is missing, remote or still loading.
    fn content(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let theme = crate::theme::current(cx);
        let Some(path) = &self.path else {
            return placeholder(&self.alt, "remote images are not loaded yet", &theme);
        };
        if !path.is_file() {
            return placeholder(&self.alt, "image not found", &theme);
        }
        let resource = Resource::from(path.clone());
        match window.use_asset::<ImgResourceLoader>(&resource, cx) {
            Some(Ok(data)) => {
                // The natural size in logical pixels caps the width. The box keeps the picture's
                // aspect ratio and is never wider than the editor.
                let device = data.size(0);
                let scale = window.scale_factor();
                let natural_width = device.width.0 as f32 / scale;
                let ratio = device.width.0 as f32 / device.height.0.max(1) as f32;
                div()
                    .w_full()
                    .max_w(px(natural_width))
                    .aspect_ratio(ratio)
                    .child(img(path.as_path()).w_full().h_full())
                    .into_any_element()
            }
            Some(Err(_)) => placeholder(&self.alt, "image could not be loaded", &theme),
            None => placeholder(&self.alt, "loading image", &theme),
        }
    }
}

/// The placeholder box: the alt text, and a line that says why the picture is not shown.
fn placeholder(alt: &str, note: &str, theme: &Theme) -> AnyElement {
    let label = if alt.is_empty() { "Image" } else { alt };
    div()
        .w_full()
        .px_3()
        .py_2()
        .border_1()
        .border_color(theme.table_border)
        .bg(theme.table_header_bg)
        .child(div().child(label.to_string()))
        .child(div().text_color(theme.text_muted).child(note.to_string()))
        .into_any_element()
}

impl IntoElement for ImageBlock {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for ImageBlock {
    /// The picture or placeholder, laid out inside this block.
    type RequestLayoutState = AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut child = self.content(window, cx);
        let child_id = child.request_layout(window, cx);
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        (window.request_layout(style, [child_id], cx), child)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        request_layout.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        request_layout.paint(window, cx);
        let source = self.source.clone();
        let line_height = window.line_height();
        self.editor.update(cx, |editor, _cx| {
            editor.layouts.insert(
                source,
                LayoutSnapshot {
                    lines: Vec::new(),
                    bounds,
                    line_height,
                    revealed: false,
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(alt: &str, src: &str) -> Option<ImageRef> {
        Some(ImageRef {
            alt: alt.to_string(),
            src: src.to_string(),
        })
    }

    #[test]
    fn a_paragraph_that_is_only_an_image_is_detected() {
        assert_eq!(parse_image("![a](b.png)"), image("a", "b.png"));
        assert_eq!(parse_image("![a](b.png)\n"), image("a", "b.png"));
        assert_eq!(parse_image("  ![](img/x.jpg)  "), image("", "img/x.jpg"));
        assert_eq!(
            parse_image("![a](b.png \"Title\")"),
            image("a", "b.png"),
            "a title is allowed"
        );
        assert_eq!(
            parse_image("![a](<my file.png>)"),
            image("a", "my file.png")
        );
    }

    #[test]
    fn text_around_or_beside_an_image_is_not_an_image() {
        assert_eq!(parse_image("![a](b.png) and text"), None);
        assert_eq!(parse_image("text ![a](b.png)"), None);
        assert_eq!(parse_image("![a](b.png)![c](d.png)"), None, "two images");
        assert_eq!(parse_image("![a](b.png) ![c](d.png)"), None, "two images");
        assert_eq!(
            parse_image("[![a](b.png)](https://x.example)"),
            None,
            "a link"
        );
        assert_eq!(parse_image("![a](b.png)\nmore text"), None, "two lines");
        assert_eq!(parse_image("plain paragraph"), None);
        assert_eq!(parse_image("![a]()"), None, "no source");
    }

    #[test]
    fn percent_decode_handles_spaces_escapes_and_case() {
        assert_eq!(percent_decode("my%20pic.png"), "my pic.png");
        assert_eq!(percent_decode("a%2fb.png"), "a/b.png");
        assert_eq!(percent_decode("a%2Fb.png"), "a/b.png");
        assert_eq!(percent_decode("100%25.png"), "100%.png");
        assert_eq!(percent_decode("plain.png"), "plain.png");
        // `+` is not a space in a path.
        assert_eq!(percent_decode("a+b.png"), "a+b.png");
    }

    #[test]
    fn percent_decode_leaves_invalid_escapes_as_is() {
        assert_eq!(percent_decode("a%2G.png"), "a%2G.png");
        assert_eq!(percent_decode("a%.png"), "a%.png");
        assert_eq!(percent_decode("a%2.png"), "a%2.png");
        assert_eq!(percent_decode("100%.png"), "100%.png");
        assert_eq!(percent_decode("%",), "%");
    }

    #[test]
    fn percent_decode_handles_multibyte_utf8() {
        // "café.png": é is U+00E9 (C3 A9 in UTF-8).
        assert_eq!(percent_decode("caf%C3%A9.png"), "caf\u{e9}.png");
        assert_eq!(percent_decode("caf%c3%a9.png"), "caf\u{e9}.png");
        // A decoded result that is not valid UTF-8 is left as-is.
        assert_eq!(percent_decode("a%FF.png"), "a%FF.png");
    }

    #[test]
    fn resolve_decodes_local_paths_but_leaves_remote_addresses_untouched() {
        let dir = Path::new("/vault");
        assert_eq!(
            resolve("my%20pic.png", Some(dir)),
            ImageLocation::Local(PathBuf::from("/vault/my pic.png"))
        );
        assert_eq!(
            resolve("100%25.png", Some(dir)),
            ImageLocation::Local(PathBuf::from("/vault/100%.png"))
        );
        assert_eq!(
            resolve("/abs/my%20pic.png", Some(dir)),
            ImageLocation::Local(PathBuf::from("/abs/my pic.png"))
        );
        assert_eq!(
            resolve("img/a%2G.png", Some(dir)),
            ImageLocation::Local(PathBuf::from("/vault/img/a%2G.png"))
        );
        assert_eq!(
            resolve("https://example.com/a%20b.png", Some(dir)),
            ImageLocation::Remote
        );
        assert_eq!(
            resolve("http://example.com/a%20b.png", None),
            ImageLocation::Remote
        );
    }

    #[test]
    fn resolve_decodes_angle_bracket_destinations() {
        // `<...>` destinations from pasted links (9.5) arrive with brackets stripped.
        let parsed = parse_image("![a](<my%20pic.png>)").expect("angle brackets parse");
        assert_eq!(parsed.src, "my%20pic.png");
        assert_eq!(
            resolve(&parsed.src, Some(Path::new("/vault"))),
            ImageLocation::Local(PathBuf::from("/vault/my pic.png"))
        );
    }

    #[test]
    fn relative_paths_resolve_against_the_note_folder() {
        let dir = Path::new("/notes/project");
        assert_eq!(
            resolve("img/sample.png", Some(dir)),
            ImageLocation::Local(PathBuf::from("/notes/project/img/sample.png"))
        );
        assert_eq!(
            resolve("/abs/x.png", Some(dir)),
            ImageLocation::Local(PathBuf::from("/abs/x.png")),
            "absolute paths are kept"
        );
        assert_eq!(
            resolve("x.png", None),
            ImageLocation::Local(PathBuf::from("x.png")),
            "without a folder the path is kept"
        );
        assert_eq!(
            resolve("https://example.com/a.png", Some(dir)),
            ImageLocation::Remote
        );
        assert_eq!(
            resolve("http://example.com/a.png", None),
            ImageLocation::Remote
        );
    }
}
