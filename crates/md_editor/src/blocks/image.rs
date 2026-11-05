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
    GlobalElementId, Hsla, ImgResourceLoader, InspectorElementId, IntoElement, LayoutId, Pixels,
    Resource, Style, Window,
};

use crate::blocks::table::{BORDER_COLOR, HEADER_BACKGROUND};
use crate::editor::Editor;
use crate::element::LayoutSnapshot;

/// Text colour of the placeholder's note line.
const MUTED_TEXT: Hsla = Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.45,
    a: 1.0,
};

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

/// Resolve an image source against the note's folder. Pure, unit-tested.
pub fn resolve(src: &str, base_dir: Option<&Path>) -> ImageLocation {
    if src.starts_with("http://") || src.starts_with("https://") {
        return ImageLocation::Remote;
    }
    let path = Path::new(src);
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
        let Some(path) = &self.path else {
            return placeholder(&self.alt, "remote images are not loaded yet");
        };
        if !path.is_file() {
            return placeholder(&self.alt, "image not found");
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
            Some(Err(_)) => placeholder(&self.alt, "image could not be loaded"),
            None => placeholder(&self.alt, "loading image"),
        }
    }
}

/// The placeholder box: the alt text, and a line that says why the picture is not shown.
fn placeholder(alt: &str, note: &str) -> AnyElement {
    let label = if alt.is_empty() { "Image" } else { alt };
    div()
        .w_full()
        .px_3()
        .py_2()
        .border_1()
        .border_color(BORDER_COLOR)
        .bg(HEADER_BACKGROUND)
        .child(div().child(label.to_string()))
        .child(div().text_color(MUTED_TEXT).child(note.to_string()))
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
