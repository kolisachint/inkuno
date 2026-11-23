//! Pictures in a line: Markdown `![alt](src)`, `![[src]]`, and HTML `<img src="…">`; file names
//! of the `assets/` folder (encoding, collisions, image kinds).

use std::ops::Range;

/// A picture a line points to.
#[derive(Clone, Debug, PartialEq)]
pub struct Picture {
    /// Its markup in the line.
    pub range: Range<usize>,
    /// Where it comes from: a path (decoded) or an http(s) URL (as written).
    pub src: String,
    /// The alternative text, shown when the picture cannot be drawn.
    pub alt: String,
    /// `width=` and `height=` of an HTML image, in pixels.
    pub width: Option<f32>,
    pub height: Option<f32>,
}

/// The first picture of the line, Markdown or HTML, whichever comes first.
pub fn picture(line: &str) -> Option<Picture> {
    match (markdown_picture(line), html_picture(line)) {
        (Some(a), Some(b)) => Some(if a.range.start <= b.range.start { a } else { b }),
        (a, b) => a.or(b),
    }
}

fn markdown_picture(line: &str) -> Option<Picture> {
    let start = line.find("![")?;
    let rest = &line[start + 2..];
    if let Some(inner) = rest.strip_prefix('[') {
        let end = inner.find("]]")?;
        let raw = inner[..end].split('|').next()?.trim();
        return (!raw.is_empty()).then(|| Picture {
            range: start..start + end + 5,
            src: source(raw),
            alt: String::new(),
            width: None,
            height: None,
        });
    }
    let open = rest.find("](")?;
    let end = open + rest[open..].find(')')?;
    let raw = rest[open + 2..end].split(" \"").next()?.trim();
    (!raw.is_empty()).then(|| Picture {
        range: start..start + end + 3,
        src: source(raw),
        alt: rest[..open].to_string(),
        width: None,
        height: None,
    })
}

fn html_picture(line: &str) -> Option<Picture> {
    let lower = line.to_ascii_lowercase();
    let mut from = 0;
    let start = loop {
        let i = from + lower[from..].find("<img")?;
        let after = lower.as_bytes().get(i + 4).copied();
        if matches!(after, Some(b' ' | b'\t' | b'/' | b'>')) {
            break i;
        }
        from = i + 4;
    };
    let end = start + line[start..].find('>')? + 1;
    let attrs = attributes(&line[start..end]);
    let value = |name: &str| {
        attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    };
    // Only a finite size above zero, and at most 10000 px: "inf", "NaN", "-50" and "0" mean no size.
    let size = |name: &str| {
        value(name)
            .and_then(|v| v.trim().trim_end_matches("px").trim().parse::<f32>().ok())
            .filter(|v| v.is_finite() && *v > 0.)
            .map(|v| v.min(10_000.))
    };
    let raw = value("src")?.trim();
    (!raw.is_empty()).then(|| Picture {
        range: start..end,
        src: source(raw),
        alt: value("alt").unwrap_or_default().to_string(),
        width: size("width"),
        height: size("height"),
    })
}

/// The attributes of an HTML tag, names in lower case: `name="v"`, `name='v'`, `name=v`, `name`.
fn attributes(tag: &str) -> Vec<(String, String)> {
    let b = tag.as_bytes();
    let mut out = Vec::new();
    let mut i = 4.min(b.len());
    loop {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            i += 1;
        }
        if i >= b.len() || b[i] == b'>' {
            return out;
        }
        let name_start = i;
        while i < b.len() && !matches!(b[i], b'=' | b'>' | b'/') && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == name_start {
            // A stray `=`: skipped.
            i += 1;
            continue;
        }
        let name = tag[name_start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= b.len() || b[i] != b'=' {
            out.push((name, String::new()));
            continue;
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let value = match b.get(i) {
            Some(&q) if q == b'"' || q == b'\'' => {
                let close = tag[i + 1..].find(q as char).map_or(b.len(), |n| i + 1 + n);
                let value = &tag[i + 1..close];
                i = close + 1;
                value
            }
            _ => {
                let from = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                    i += 1;
                }
                &tag[from..i]
            }
        };
        out.push((name, value.to_string()));
    }
}

/// A source as the line writes it: an URL stays as it is, a path is decoded (`%20` is a space).
fn source(raw: &str) -> String {
    if is_remote(raw) {
        raw.to_string()
    } else {
        percent_decode(raw)
    }
}

/// Whether the source is an http or https URL.
pub fn is_remote(src: &str) -> bool {
    let lower = src.get(..8).unwrap_or(src).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Decodes `%XX` escapes; a `%` not followed by two hex digits stays.
pub fn percent_decode(text: &str) -> String {
    let b = text.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match (
            b[i],
            b.get(i + 1).and_then(|&c| hex(c)),
            b.get(i + 2).and_then(|&c| hex(c)),
        ) {
            (b'%', Some(hi), Some(lo)) => {
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            _ => {
                out.push(b[i]);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Encodes a file name for a Markdown link: spaces and the characters that end or break the
/// link are `%XX`; the rest, accents included, is written as it is.
pub fn percent_encode(name: &str) -> String {
    let mut out = Vec::with_capacity(name.len());
    for c in name.bytes() {
        match c {
            b' '
            | b'('
            | b')'
            | b'['
            | b']'
            | b'<'
            | b'>'
            | b'%'
            | b'#'
            | b'?'
            | b'"'
            | b'\\'
            | 0..=0x1f => out.extend(format!("%{c:02X}").bytes()),
            _ => out.push(c),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Whether the file name is an image this app draws (the extension, any case).
pub fn is_image_file(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| {
        [
            "png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tif", "tiff", "avif", "svg",
        ]
        .iter()
        .any(|known| ext.eq_ignore_ascii_case(known))
    })
}

/// What a candidate name holds: nothing, the same bytes as the file to copy, or other bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Free,
    Same,
    Other,
}

/// The first name of `name`, `name 2`, `name 3`… (before the extension) that is free or holds
/// the same bytes; never overwrites another file. Returns the name and what it holds.
pub fn free_name(name: &str, slot: impl Fn(&str) -> Slot) -> (String, Slot) {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => name.split_at(i),
        _ => (name, ""),
    };
    (1..)
        .find_map(|n| {
            let candidate = match n {
                1 => name.to_string(),
                n => format!("{stem} {n}{ext}"),
            };
            match slot(&candidate) {
                Slot::Other => None,
                held => Some((candidate, held)),
            }
        })
        .expect("an infinite range finds a free name")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_and_markdown_pictures() {
        let p = picture("a ![alt text](assets/my%20pic.png) b").unwrap();
        assert_eq!(
            (p.range, p.src.as_str(), p.alt.as_str()),
            (2..34, "assets/my pic.png", "alt text")
        );
        let p = picture("![[carte.png|200]]").unwrap();
        assert_eq!(p.src, "carte.png");
        let p = picture(
            r#"<p><img src="https://x.test/a.png?x=1" alt='Logo' width="120px" height=40></p>"#,
        )
        .unwrap();
        assert_eq!(p.src, "https://x.test/a.png?x=1");
        assert_eq!(
            (p.alt.as_str(), p.width, p.height),
            ("Logo", Some(120.), Some(40.))
        );
        assert_eq!(picture("<IMG SRC=x.svg />").unwrap().src, "x.svg");
        assert!(picture("<img alt=\"no src\">").is_none());
        assert!(picture("![]()").is_none());
        assert!(picture("<imgx src=a>").is_none());
        // The first of the two wins.
        assert_eq!(picture("<img src=b.png> ![](a.png)").unwrap().src, "b.png");
        assert_eq!(picture("![](a.png) <img src=b.png>").unwrap().src, "a.png");
    }

    #[test]
    fn encoding_round_trips() {
        assert_eq!(
            percent_encode("my (1) pic #2.png"),
            "my%20%281%29%20pic%20%232.png"
        );
        assert_eq!(percent_encode("carré.png"), "carré.png");
        assert_eq!(
            percent_decode(&percent_encode("50% (é).png")),
            "50% (é).png"
        );
        assert_eq!(percent_decode("100%zz%2"), "100%zz%2");
        assert!(is_remote("HTTPS://github.com/x.png"));
        assert!(!is_remote("https"));
        assert!(!is_remote("ftp://x"));
    }

    #[test]
    fn image_kinds_ignore_case() {
        assert!(is_image_file("Photo.JPG"));
        assert!(is_image_file("plan.svg"));
        assert!(!is_image_file("notes.md"));
        assert!(!is_image_file("png"));
    }

    #[test]
    fn names_never_overwrite() {
        let taken = |n: &str| match n {
            "a.png" | "a 2.png" => Slot::Other,
            "a 3.png" => Slot::Same,
            _ => Slot::Free,
        };
        assert_eq!(free_name("a.png", taken), ("a 3.png".into(), Slot::Same));
        assert_eq!(
            free_name("b.png", |_| Slot::Free),
            ("b.png".into(), Slot::Free)
        );
        assert_eq!(
            free_name("noext", |n| if n == "noext" {
                Slot::Other
            } else {
                Slot::Free
            })
            .0,
            "noext 2"
        );
    }

    #[test]
    fn nonsense_sizes_are_no_size() {
        for bad in ["inf", "NaN", "-50", "0", "abc"] {
            let p = picture(&format!(
                r#"<img src="a.png" width="{bad}" height="{bad}">"#
            ))
            .unwrap();
            assert_eq!((p.width, p.height), (None, None), "{bad}");
        }
        let p = picture(r#"<img src="a.png" width="1e9" height="12">"#).unwrap();
        assert_eq!((p.width, p.height), (Some(10_000.), Some(12.)));
    }
}
