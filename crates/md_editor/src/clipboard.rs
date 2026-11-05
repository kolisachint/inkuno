//! Clipboard copy, cut and paste (plan task 3.3). Owner: task 3.3.
//!
//! Copy puts the Markdown source of the selection on the clipboard, unchanged. Paste inserts
//! plain text through `Editor::replace_selection` (3.2).
//!
//! Image paste (9.5, DESIGN §7): when the clipboard holds an image and the note has a folder
//! (`Editor::base_dir`), the bytes are written to `<folder>/assets/pasted-YYYYMMDD-HHMMSS.<ext>`
//! and `![](assets/<file>)` is inserted at the caret. An untitled note gets no file and no link.
//!
//! Key bindings for these actions are in `keys.rs` (`bind_keys`); `keys::attach` dispatches
//! them to the methods below.

use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{actions, ClipboardEntry, ClipboardItem, Context, Image};

use crate::editor::Editor;

actions!(md_editor_clipboard, [Copy, Cut, Paste]);

/// The folder next to the note that holds pasted images (DESIGN §7).
pub const ASSETS_DIR: &str = "assets";

/// The clipboard text for `range` of `text`: the Markdown source bytes, unchanged.
/// Returns `None` for an empty range. Pure function, unit-tested in 3.3.
pub fn copy_text(text: &str, range: Range<usize>) -> Option<String> {
    if range.is_empty() {
        return None;
    }
    text.get(range).map(str::to_string)
}

/// Normalise pasted text: convert `\r\n` and `\r` to `\n`. Pure function, unit-tested in 3.3.
pub fn normalize_paste(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// The file stem for a pasted image: `pasted-YYYYMMDD-HHMMSS` for a Unix time in seconds,
/// in UTC. Pure function, unit-tested in 9.5.
pub fn pasted_stem(unix_secs: u64) -> String {
    let days = (unix_secs / 86_400) as i64;
    let secs = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "pasted-{year:04}{month:02}{day:02}-{:02}{:02}{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Days since 1970-01-01 to (year, month, day) in the proleptic Gregorian calendar.
/// Howard Hinnant's `civil_from_days` algorithm (public domain).
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// Writes `bytes` to `dir/<stem>.<ext>`, or `<stem>-2.<ext>`, `<stem>-3.<ext>`, ... when that
/// name is taken. Creates `dir` if it is missing. The file is created with `create_new`, so an
/// existing file is never overwritten. Returns the path written.
pub fn write_unique(dir: &Path, stem: &str, ext: &str, bytes: &[u8]) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let mut n: u32 = 1;
    loop {
        let name = if n == 1 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}-{n}.{ext}")
        };
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                if let Err(err) = file.write_all(bytes) {
                    _ = fs::remove_file(&path);
                    return Err(err);
                }
                return Ok(path);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => n += 1,
            Err(err) => return Err(err),
        }
    }
}

/// The Markdown image link for `rel`, a relative path with forward slashes, e.g.
/// `assets/pasted-20260101-120000.png`. A path with spaces or parentheses is wrapped in
/// `<...>`, and `<`, `>` and `\` inside it are backslash-escaped. Pure, unit-tested in 9.5.
pub fn image_markdown(rel: &str) -> String {
    let needs_brackets = rel
        .chars()
        .any(|c| c.is_whitespace() || c == '(' || c == ')' || c == '<' || c == '>');
    if !needs_brackets {
        return format!("![]({rel})");
    }
    let mut dest = String::with_capacity(rel.len() + 2);
    for c in rel.chars() {
        match c {
            '<' | '>' | '\\' => {
                dest.push('\\');
                dest.push(c);
            }
            '\n' => dest.push_str("%0A"),
            '\r' => dest.push_str("%0D"),
            c => dest.push(c),
        }
    }
    format!("![](<{dest}>)")
}

impl Editor {
    /// Copy the selected source text to the system clipboard. Does nothing for a caret.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let range = self.selection.selection.range();
        let text = self.text();
        if let Some(copied) = copy_text(&text, range) {
            cx.write_to_clipboard(ClipboardItem::new_string(copied));
        }
    }

    /// Copy the selected source text, then delete it through `replace_range`.
    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        let range = self.selection.selection.range();
        if range.is_empty() {
            return;
        }
        self.copy_selection(cx);
        self.replace_range(range, "", cx);
    }

    /// Insert the clipboard's plain text at the selection through `replace_selection`.
    /// Does nothing when the clipboard has no text.
    pub fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        let Some(item) = cx.read_from_clipboard() else {
            return;
        };
        let image = item.entries().iter().find_map(|entry| match entry {
            ClipboardEntry::Image(image) => Some(image),
            _ => None,
        });
        if let Some(image) = image {
            self.paste_image(image, cx);
            return;
        }
        let Some(clipboard) = item.text() else {
            return;
        };
        let text = normalize_paste(&clipboard);
        if text.is_empty() {
            return;
        }
        self.replace_selection(&text, cx);
    }

    /// Save a pasted image into the note's `assets/` folder and insert its link at the caret.
    /// Does nothing (beyond a stderr note) when the note has no folder yet, or when the write
    /// fails. The link is one `replace_range` call, so it is one undo step.
    fn paste_image(&mut self, image: &Image, cx: &mut Context<Self>) {
        let Some(folder) = self.base_dir().map(Path::to_path_buf) else {
            eprintln!(
                "noteapp: pasted image not saved: the note has no folder (save the note first)"
            );
            return;
        };
        let unix_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let stem = pasted_stem(unix_secs);
        let extension = image.format.extension();
        match write_unique(&folder.join(ASSETS_DIR), &stem, extension, &image.bytes) {
            Ok(path) => {
                let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                    return;
                };
                let rel = format!("{ASSETS_DIR}/{file_name}");
                self.replace_selection(&image_markdown(&rel), cx);
            }
            Err(err) => eprintln!("noteapp: pasted image not saved: {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_text_returns_the_source_bytes() {
        let text = "Some **bold** text";
        assert_eq!(copy_text(text, 5..13).as_deref(), Some("**bold**"));
        assert_eq!(copy_text(text, 0..text.len()).as_deref(), Some(text));
    }

    #[test]
    fn copy_text_keeps_multibyte_text_and_rejects_an_empty_range() {
        let text = "日本語 and emoji 😀";
        assert_eq!(copy_text(text, 0..9).as_deref(), Some("日本語"));
        assert_eq!(copy_text(text, 3..3), None);
        assert_eq!(copy_text("", 0..0), None);
        // A range that splits a character is not valid text, so nothing is copied.
        assert_eq!(copy_text(text, 1..4), None);
    }

    #[test]
    fn normalize_paste_converts_crlf_and_cr_to_lf() {
        assert_eq!(normalize_paste("a\r\nb\rc\nd"), "a\nb\nc\nd");
        assert_eq!(normalize_paste("\r\n\r\n"), "\n\n");
        assert_eq!(normalize_paste("plain"), "plain");
    }

    #[test]
    fn copy_then_paste_round_trips_plain_text() {
        // Copy the selection, then normalize it as paste does. Plain text comes back unchanged.
        let text = "one\r\ntwo words";
        let copied = copy_text(text, 0..text.len()).unwrap();
        assert_eq!(normalize_paste(&copied), "one\ntwo words");
        let fragment = copy_text("abc def", 4..7).unwrap();
        assert_eq!(normalize_paste(&fragment), "def");
    }

    #[test]
    fn pasted_stem_formats_utc_time() {
        assert_eq!(pasted_stem(0), "pasted-19700101-000000");
        // 2000-02-29 23:59:59 UTC (a leap day).
        assert_eq!(pasted_stem(951_868_799), "pasted-20000229-235959");
        // 2026-11-05 15:14:56 UTC.
        assert_eq!(pasted_stem(1_793_891_696), "pasted-20261105-151456");
    }

    #[test]
    fn image_markdown_links_plain_paths_without_brackets() {
        assert_eq!(
            image_markdown("assets/pasted-20260101-120000.png"),
            "![](assets/pasted-20260101-120000.png)"
        );
    }

    #[test]
    fn image_markdown_wraps_paths_with_spaces_in_angle_brackets() {
        assert_eq!(
            image_markdown("assets/my pic (1).png"),
            "![](<assets/my pic (1).png>)"
        );
        assert_eq!(image_markdown("a<b>.png"), "![](<a\\<b\\>.png>)");
        assert_eq!(image_markdown("a\nb.png"), "![](<a%0Ab.png>)");
    }

    #[test]
    fn image_markdown_round_trips_through_the_image_parser() {
        use crate::blocks::image::parse_image;
        for rel in ["assets/pasted-1.png", "assets/my pic (1).png"] {
            let link = image_markdown(rel);
            let parsed = parse_image(&link).expect("the link parses as one image");
            assert_eq!(parsed.alt, "");
            assert_eq!(parsed.src, rel);
        }
    }

    #[test]
    fn write_unique_adds_a_suffix_on_collision() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join(ASSETS_DIR);
        let first = write_unique(&folder, "pasted-x", "png", b"one").unwrap();
        let second = write_unique(&folder, "pasted-x", "png", b"two").unwrap();
        let third = write_unique(&folder, "pasted-x", "png", b"three").unwrap();
        assert_eq!(first.file_name().unwrap(), "pasted-x.png");
        assert_eq!(second.file_name().unwrap(), "pasted-x-2.png");
        assert_eq!(third.file_name().unwrap(), "pasted-x-3.png");
        // Nothing is overwritten.
        assert_eq!(fs::read(&first).unwrap(), b"one");
        assert_eq!(fs::read(&second).unwrap(), b"two");
        assert_eq!(fs::read(&third).unwrap(), b"three");
    }

    #[test]
    fn write_unique_creates_a_missing_folder_and_uses_the_extension() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("notes").join(ASSETS_DIR);
        assert!(!folder.exists());
        let path = write_unique(&folder, "pasted-y", "jpg", &[1, 2, 3]).unwrap();
        assert_eq!(path.parent(), Some(folder.as_path()));
        assert_eq!(path.extension().unwrap(), "jpg");
        assert_eq!(fs::read(&path).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn image_formats_map_to_file_extensions() {
        use gpui::ImageFormat;
        assert_eq!(ImageFormat::Png.extension(), "png");
        assert_eq!(ImageFormat::Jpeg.extension(), "jpg");
        assert_eq!(ImageFormat::Gif.extension(), "gif");
        assert_eq!(ImageFormat::Webp.extension(), "webp");
    }
}
