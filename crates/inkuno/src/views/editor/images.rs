//! Pictures of the note: web pictures fetched in the background, the box drawn in place of a
//! picture that cannot be drawn, and files dropped on the note, copied into its `assets/` folder.

use super::*;
use crate::logic::markdown as md;
use crate::logic::markdown::Slot;
use crate::platform::remote;
use gpui::ExternalPaths;

/// A picture of the web, fetched by the background executor (gpui's asset cache keeps it).
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct Remote(pub(super) String);

impl gpui::Asset for Remote {
    type Source = Self;
    type Output = Result<PathBuf, String>;

    // The future must not borrow `cx`: an `async fn` would.
    #[allow(clippy::manual_async_fn)]
    fn load(source: Self, _: &mut App) -> impl Future<Output = Self::Output> + Send + 'static {
        async move { remote::fetch_image(&source.0) }
    }
}

/// Where a web picture stands.
pub(super) enum Fetch {
    Loading,
    Ready(PathBuf),
    Failed,
}

pub(super) fn fetch(src: &str, window: &mut Window, cx: &mut App) -> Fetch {
    match window.use_asset::<Remote>(&Remote(src.to_string()), cx) {
        None => Fetch::Loading,
        Some(Ok(path)) => Fetch::Ready(path),
        Some(Err(_)) => Fetch::Failed,
    }
}

/// The box drawn in place of a picture that is not drawn: its size, its alternative text, and
/// what it says.
#[derive(Clone, Debug)]
pub(super) struct Frame {
    pub(super) size: Size<Pixels>,
    pub(super) alt: String,
    pub(super) said: &'static str,
}

/// The text of a box: its alternative text, then what it says.
pub(super) fn frame_text(frame: &Frame) -> String {
    match (frame.alt.is_empty(), frame.said.is_empty()) {
        (_, true) => frame.alt.clone(),
        (true, false) => frame.said.to_string(),
        (false, false) => format!("{} · {}", frame.alt, frame.said),
    }
}

/// The alternative text of a picture: its `alt`, else its file name.
pub(super) fn alt_of(picture: &md::Picture) -> String {
    match picture.alt.is_empty() {
        true => picture
            .src
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string(),
        false => picture.alt.clone(),
    }
}

/// Size of a web picture while it loads, in a text column `column` wide.
pub(super) fn loading_size(column: f32) -> (f32, f32) {
    (column.min(480.), 180.)
}

/// Size on screen of a picture of `natural` size, with its `width=` and `height=` attributes if
/// any: never wider than the column, the aspect kept.
pub(super) fn fit(
    natural: (f32, f32),
    attrs: (Option<f32>, Option<f32>),
    column: f32,
) -> (f32, f32) {
    let (nw, nh) = (natural.0.max(1.), natural.1.max(1.));
    // Never NaN or negative, whatever the attributes say.
    let attrs = (
        attrs.0.filter(|v| v.is_finite() && *v > 0.),
        attrs.1.filter(|v| v.is_finite() && *v > 0.),
    );
    let (w, h) = match attrs {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w * nh / nw),
        (None, Some(h)) => (h * nw / nh, h),
        (None, None) => (nw, nh),
    };
    let scale = (column / w.max(1.)).min(1.);
    (w * scale, h * scale)
}

/// The file name a dropped file gets in `assets/`: the same name, or the first free one. A file
/// with the same bytes is reused; nothing is overwritten.
fn save_asset(assets: &Path, name: &str, bytes: &[u8]) -> std::io::Result<String> {
    fs::create_dir_all(assets)?;
    let (name, slot) = md::free_name(name, |candidate| {
        let path = assets.join(candidate);
        if !path.exists() {
            Slot::Free
        } else if fs::read(&path).is_ok_and(|held| held == bytes) {
            Slot::Same
        } else {
            Slot::Other
        }
    });
    if slot == Slot::Free {
        vault::write(&assets.join(&name), bytes)?;
    }
    Ok(name)
}

/// The text that a drop inserts at `at`: one line per file, an image as `![name](assets/…)`, any
/// other file as a link. Each one starts on its own line.
fn drop_text(content: &str, at: usize, files: &[(String, bool)]) -> String {
    let lines: Vec<String> = files
        .iter()
        .map(|(name, image)| {
            let label = name.replace(['[', ']'], "");
            let target = format!("assets/{}", md::percent_encode(name));
            match image {
                true => format!("![{label}]({target})"),
                false => format!("[{label}]({target})"),
            }
        })
        .collect();
    let lead = if at > 0 && !content[..at].ends_with('\n') {
        "\n"
    } else {
        ""
    };
    let tail = if at < content.len() && !content[at..].starts_with('\n') {
        "\n"
    } else {
        ""
    };
    format!("{lead}{}{tail}", lines.join("\n"))
}

/// The largest dropped file that is read into `assets/`.
const MAX_DROP: u64 = 50 * 1024 * 1024;

impl Editor {
    /// Files dropped on the note, at the pointer: see `drop_paths`.
    pub(super) fn drop_files(
        &mut self,
        paths: &ExternalPaths,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let at = self.index_at(window.mouse_position());
        self.drop_paths(paths.paths(), at, cx);
    }

    /// Files dropped on the note, at byte `at`: copied into its `assets/` folder (the scratch
    /// folder's for a scratch note), and linked there, in one undo step.
    // ponytail: a file that cannot be read is skipped without a message.
    pub fn drop_paths(&mut self, paths: &[PathBuf], at: usize, cx: &mut Context<Self>) {
        if self.readonly {
            return self.refuse_pending(cx);
        }
        let Some(dir) = self.dirs.first().cloned() else {
            return;
        };
        let assets = dir.join("assets");
        let mut files = Vec::new();
        let mut too_big = false;
        for path in paths {
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            // ponytail: a file over 50 MB is not read whole on the UI thread; it is skipped and said so.
            if fs::metadata(path).is_ok_and(|m| m.len() > MAX_DROP) {
                too_big = true;
                continue;
            }
            let Ok(bytes) = fs::read(path) else {
                continue;
            };
            if let Ok(saved) = save_asset(&assets, name, &bytes) {
                files.push((saved, md::is_image_file(name)));
            }
        }
        if too_big {
            cx.emit(EditorEvent::Notice(
                tr(
                    "A dropped file is over 50 MB and was not added.",
                    "Un fichier déposé dépasse 50 Mo et n'a pas été ajouté.",
                )
                .into(),
            ));
        }
        if files.is_empty() {
            return;
        }
        let text = drop_text(&self.content, at, &files);
        if self.touches_pending(&(at..at), &text) {
            return self.refuse_pending(cx);
        }
        self.push_undo(true);
        self.edit(at..at, &text, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_the_column_and_the_attributes() {
        assert_eq!(fit((400., 200.), (None, None), 200.), (200., 100.));
        assert_eq!(fit((100., 50.), (None, None), 200.), (100., 50.));
        assert_eq!(fit((100., 50.), (Some(300.), None), 200.), (200., 100.));
        assert_eq!(fit((100., 50.), (Some(50.), Some(20.)), 200.), (50., 20.));
        assert_eq!(fit((100., 50.), (None, Some(100.)), 800.), (200., 100.));
    }

    #[test]
    fn drop_lines_sit_on_their_own_lines() {
        let files = [
            ("a 2.png".to_string(), true),
            ("doc.pdf".to_string(), false),
        ];
        assert_eq!(
            drop_text("x\ny", 2, &files),
            "![a 2.png](assets/a%202.png)\n[doc.pdf](assets/doc.pdf)\n"
        );
        assert_eq!(
            drop_text("xy", 1, &files[..1]),
            "\n![a 2.png](assets/a%202.png)\n"
        );
        assert_eq!(
            drop_text("x\n", 2, &files[..1]),
            "![a 2.png](assets/a%202.png)"
        );
        assert_eq!(
            drop_text("", 0, &files[..1]),
            "![a 2.png](assets/a%202.png)"
        );
    }
}
