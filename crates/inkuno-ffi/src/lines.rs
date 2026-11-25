//! What a line is and how it is styled: the editor's layout and paint read these records.

use inkuno_logic::markdown::{self as md, Kind};

use crate::text::{offset_utf16, utf16_len};

/// The kind of a line (see `inkuno_logic::markdown::Kind`).
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Para,
    Heading {
        level: u8,
    },
    Bullet,
    Ordered,
    Task {
        checked: bool,
    },
    Quote,
    Fence,
    Code,
    Rule,
    Table,
    /// A line of the front matter (`---` block at the top of the note).
    Meta,
}

impl From<Kind> for LineKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Para => Self::Para,
            Kind::Heading(level) => Self::Heading { level },
            Kind::Bullet => Self::Bullet,
            Kind::Ordered => Self::Ordered,
            Kind::Task(checked) => Self::Task { checked },
            Kind::Quote => Self::Quote,
            Kind::Fence => Self::Fence,
            Kind::Code => Self::Code,
            Kind::Rule => Self::Rule,
            Kind::Table => Self::Table,
            Kind::Meta => Self::Meta,
        }
    }
}

/// One line: its kind and the length of its marker (`# `, `- `, `1. `, `> `…), in UTF-16.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct LineInfo {
    pub kind: LineKind,
    pub marker_len_utf16: u64,
}

/// One line of a note. Lines are the parts of the text between `\n`; `start_utf16` is where the
/// line starts, `len_utf16` its length without the line break.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct LineLayout {
    pub start_utf16: u64,
    pub len_utf16: u64,
    pub kind: LineKind,
    /// 1 to 6 for a heading, 0 otherwise.
    pub heading_level: u8,
    pub marker_len_utf16: u64,
    /// The line is in a fenced code block, its opening and closing fences included. A fence that
    /// is never closed opens no block.
    pub in_code_block: bool,
}

/// The lines of a note, in order.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct DocumentLayout {
    pub lines: Vec<LineLayout>,
}

/// One style bit per run (see `style_bits`), over a range of a line, in UTF-16.
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleRun {
    pub start_utf16: u64,
    pub len_utf16: u64,
    /// The bits of `style_bits()` that hold on the range (never 0: plain text has no run).
    pub bits: u32,
}

/// The bit of each style, as `StyleRun.bits` carries it. A line's styles come from
/// `line_style_runs`, a code line's from `code_style_runs`.
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq, Eq)]
pub struct StyleBits {
    pub bold: u32,
    pub italic: u32,
    pub code: u32,
    pub strike: u32,
    pub dim: u32,
    pub link: u32,
    pub tag: u32,
    pub mark: u32,
    pub highlight: u32,
    pub underline: u32,
    /// Text that steps back but is still read (front matter, link addresses, comments).
    pub quiet: u32,
    pub keyword: u32,
    pub string: u32,
    pub number: u32,
    pub comment: u32,
    pub type_name: u32,
    pub function: u32,
}

/// The bit of each style.
#[uniffi::export]
pub fn style_bits() -> StyleBits {
    StyleBits {
        bold: md::BOLD,
        italic: md::ITALIC,
        code: md::CODE,
        strike: md::STRIKE,
        dim: md::DIM,
        link: md::LINK,
        tag: md::TAG,
        mark: md::MARK,
        highlight: md::HIGHLIGHT,
        underline: md::UNDER,
        quiet: md::QUIET,
        keyword: md::KEYWORD,
        string: md::STRING,
        number: md::NUMBER,
        comment: md::COMMENT,
        type_name: md::TYPE,
        function: md::FUNCTION,
    }
}

/// A link or tag of a line, in UTF-16 over the whole markup (`[text](path)` included).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct LinkInfo {
    pub start_utf16: u64,
    pub len_utf16: u64,
    pub kind: LinkKind,
    /// The wikilink's name, the URL, the note's path, the heading, the tag or the footnote label.
    pub target: String,
    /// The `#anchor` of a note link, if any.
    pub anchor: Option<String>,
}

#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    Wiki,
    Url,
    Note,
    Heading,
    Tag,
    Footnote,
}

/// A heading of the note (front matter excluded).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct HeadingInfo {
    pub level: u8,
    pub title: String,
    /// 0-based index of the heading's line.
    pub line: u64,
    pub start_utf16: u64,
}

/// The tags and the wikilinks of a note, lowercase, each once.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct NoteIndex {
    pub tags: Vec<String>,
    pub links: Vec<String>,
}

/// Kind and marker length of a line. `in_code`: the line is inside a ``` block (the opener is
/// not, the closer is); the block's own fence is not known to this call.
#[uniffi::export]
pub fn classify_line(line: String, in_code: bool) -> LineInfo {
    let (kind, marker) = md::classify(&line, in_code);
    LineInfo {
        kind: kind.into(),
        marker_len_utf16: offset_utf16(&line, marker),
    }
}

/// Every line of `text` with its kind, marker, and whether it is code. Fenced blocks are read
/// as `inkuno_logic::markdown::blocks` reads them; one pass, linear in the size of the text.
#[uniffi::export]
pub fn document_layout(text: String) -> DocumentLayout {
    let blocks = md::blocks(&text);
    let meta = md::front_matter(&text);
    let mut lines = Vec::new();
    let (mut at, mut start_utf16) = (0usize, 0u64);
    for line in text.split('\n') {
        let open = md::open_at(&blocks, at);
        let (kind, marker) = if at < meta {
            (Kind::Meta, 0)
        } else {
            md::classify_in(line, open)
        };
        let heading_level = match kind {
            Kind::Heading(level) => level,
            _ => 0,
        };
        let len_utf16 = utf16_len(line);
        lines.push(LineLayout {
            start_utf16,
            len_utf16,
            kind: kind.into(),
            heading_level,
            marker_len_utf16: offset_utf16(line, marker),
            in_code_block: open.is_some() || kind == Kind::Fence,
        });
        at += line.len() + 1;
        start_utf16 += len_utf16 + 1;
    }
    DocumentLayout { lines }
}

/// Inline styles of a line, as runs. Each run is a stretch of characters that carry the same
/// bits; plain text is left out. Positions are UTF-16 in the line.
#[uniffi::export]
pub fn line_style_runs(line: String) -> Vec<StyleRun> {
    let (_, marker) = md::classify(&line, false);
    style_runs(&line, &md::flags(&line, marker))
}

/// Code colouring of a line of a code block in language `lang` (its info string, such as `rust`).
/// Same runs as `line_style_runs`.
#[uniffi::export]
pub fn code_style_runs(line: String, lang: String) -> Vec<StyleRun> {
    let mut flags = vec![0u32; line.len()];
    md::code(&line, &lang, &mut flags);
    style_runs(&line, &flags)
}

/// Runs over the characters of `line`, from one flag per byte (a character takes the flag of its
/// first byte).
fn style_runs(line: &str, flags: &[u32]) -> Vec<StyleRun> {
    let mut runs: Vec<StyleRun> = Vec::new();
    let mut units = 0u64;
    for (i, c) in line.char_indices() {
        let width = c.len_utf16() as u64;
        let bits = flags[i];
        if bits != 0 {
            match runs.last_mut() {
                Some(run) if run.bits == bits && run.start_utf16 + run.len_utf16 == units => {
                    run.len_utf16 += width;
                }
                _ => runs.push(StyleRun {
                    start_utf16: units,
                    len_utf16: width,
                    bits,
                }),
            }
        }
        units += width;
    }
    runs
}

/// The links and tags of a line, in order.
#[uniffi::export]
pub fn line_links(line: String) -> Vec<LinkInfo> {
    md::links(&line)
        .into_iter()
        .map(|(range, link)| {
            let (kind, target, anchor) = match link {
                md::Link::Wiki(target) => (LinkKind::Wiki, target, None),
                md::Link::Url(target) => (LinkKind::Url, target, None),
                md::Link::Note { path, anchor } => (LinkKind::Note, path, anchor),
                md::Link::Heading(target) => (LinkKind::Heading, target, None),
                md::Link::Tag(target) => (LinkKind::Tag, target, None),
                md::Link::Footnote(label) => (LinkKind::Footnote, label, None),
            };
            LinkInfo {
                start_utf16: offset_utf16(&line, range.start),
                len_utf16: utf16_len(&line[range.start..range.end]),
                kind,
                target,
                anchor,
            }
        })
        .collect()
}

/// The callout a line opens: the `[!TYPE]` marker of a `> [!NOTE] Title` quote.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct CalloutInfo {
    /// Where the `[!TYPE]` marker starts, in UTF-16.
    pub start_utf16: u64,
    /// The length of the marker in UTF-16, brackets included.
    pub len_utf16: u64,
    /// The TYPE of the marker, lowercased: `note`, `warning`, and so on.
    pub kind: String,
}

/// The callout a line opens, if it is a `> [!TYPE]` quote (see `inkuno_logic::markdown::callout`).
/// `None` for a quote without a marker, or for an empty `[!]` marker.
#[uniffi::export]
pub fn line_callout(line: String) -> Option<CalloutInfo> {
    let range = md::callout(&line)?;
    let marker = &line[range.clone()];
    // The marker is `[!` + TYPE + `]`: the TYPE sits between the first two and the last byte.
    let kind = marker[2..marker.len() - 1].to_lowercase();
    if kind.is_empty() {
        return None;
    }
    Some(CalloutInfo {
        start_utf16: offset_utf16(&line, range.start),
        len_utf16: utf16_len(marker),
        kind,
    })
}

/// The headings of `text`, in order (see `inkuno_logic::markdown::headings`).
#[uniffi::export]
pub fn headings(text: String) -> Vec<HeadingInfo> {
    // The headings come in order: the line count and the UTF-16 offset are carried forward, not
    // recounted from the start of the text for each one.
    let (mut at, mut units, mut line) = (0usize, 0u64, 0u64);
    md::headings(&text)
        .into_iter()
        .map(|(level, title, start)| {
            let between = &text[at..start];
            line += between.matches('\n').count() as u64;
            units += utf16_len(between);
            at = start;
            HeadingInfo {
                level,
                title: title.to_string(),
                line,
                start_utf16: units,
            }
        })
        .collect()
}

/// The tags and wikilinks of a whole note (see `inkuno_logic::markdown::index`).
#[uniffi::export]
pub fn note_index(text: String) -> NoteIndex {
    let (tags, links) = md::index(&text);
    NoteIndex { tags, links }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<LineKind> {
        document_layout(text.into())
            .lines
            .into_iter()
            .map(|l| l.kind)
            .collect()
    }

    #[test]
    fn classifies_one_line() {
        assert_eq!(
            classify_line("## Title".into(), false),
            LineInfo {
                kind: LineKind::Heading { level: 2 },
                marker_len_utf16: 3
            }
        );
        assert_eq!(
            classify_line("- [x] done".into(), false),
            LineInfo {
                kind: LineKind::Task { checked: true },
                marker_len_utf16: 6
            }
        );
        assert_eq!(classify_line("1. item".into(), false).marker_len_utf16, 3);
        // The marker counts UTF-16 units: an emoji in front of the marker is not in it, but a
        // quote's content is measured after its `> `.
        assert_eq!(
            classify_line("> 😀 quoted".into(), false).marker_len_utf16,
            2
        );
        assert_eq!(classify_line("---".into(), false).kind, LineKind::Rule);
        assert_eq!(classify_line("```".into(), false).kind, LineKind::Fence);
    }

    #[test]
    fn a_line_in_code_is_code_and_its_closing_fence_is_a_fence() {
        assert_eq!(
            classify_line("let x = 1;".into(), true).kind,
            LineKind::Code
        );
        assert_eq!(classify_line("```".into(), true).kind, LineKind::Fence);
        assert_eq!(
            classify_line("# not a heading".into(), true).kind,
            LineKind::Code
        );
    }

    #[test]
    fn layout_gives_utf16_starts_and_lengths_per_line() {
        // "é😀" is 3 units; the line break is 1 more.
        let layout = document_layout("é😀\n# T".into());
        assert_eq!(layout.lines.len(), 2);
        assert_eq!(layout.lines[0].start_utf16, 0);
        assert_eq!(layout.lines[0].len_utf16, 3);
        assert_eq!(layout.lines[1].start_utf16, 4);
        assert_eq!(layout.lines[1].len_utf16, 3);
        assert_eq!(layout.lines[1].heading_level, 1);
        assert_eq!(layout.lines[1].marker_len_utf16, 2);
        assert_eq!(layout.lines[0].heading_level, 0);
    }

    #[test]
    fn layout_marks_fenced_code_and_its_fences() {
        let text = "intro\n```rust\nfn main() {}\n# not a heading\n```\nafter";
        let layout = document_layout(text.into()).lines;
        let kinds: Vec<LineKind> = layout.iter().map(|l| l.kind).collect();
        assert_eq!(
            kinds,
            [
                LineKind::Para,
                LineKind::Fence,
                LineKind::Code,
                LineKind::Code,
                LineKind::Fence,
                LineKind::Para
            ]
        );
        let in_code: Vec<bool> = layout.iter().map(|l| l.in_code_block).collect();
        assert_eq!(in_code, [false, true, true, true, true, false]);
        // Inside the block, `# …` is not a heading: its heading level is 0.
        assert_eq!(layout[3].heading_level, 0);
    }

    #[test]
    fn an_unclosed_fence_opens_no_block() {
        // A lone ``` is a fence line, but the text after it is ordinary Markdown.
        let layout = document_layout("```\n# still a heading".into()).lines;
        assert_eq!(layout[0].kind, LineKind::Fence);
        assert_eq!(layout[1].kind, LineKind::Heading { level: 1 });
        assert!(!layout[1].in_code_block);
    }

    #[test]
    fn front_matter_lines_are_meta() {
        let text = "---\ntitle: Hi\n---\n# Hi";
        assert_eq!(
            kinds(text),
            [
                LineKind::Meta,
                LineKind::Meta,
                LineKind::Meta,
                LineKind::Heading { level: 1 }
            ]
        );
        assert_eq!(document_layout(text.into()).lines[3].heading_level, 1);
    }

    #[test]
    fn layout_of_an_empty_text_has_one_empty_line() {
        let layout = document_layout(String::new()).lines;
        assert_eq!(layout.len(), 1);
        assert_eq!(layout[0].len_utf16, 0);
        assert_eq!(layout[0].kind, LineKind::Para);
    }

    #[test]
    fn style_runs_use_utf16_and_skip_plain_text() {
        // "é 😀 **bold** x": the bold word is at units 7..11 (é, space, 😀 = 2 units, space, `**`).
        let runs = line_style_runs("é 😀 **bold** x".into());
        assert!(runs.contains(&StyleRun {
            start_utf16: 7,
            len_utf16: 4,
            bits: md::BOLD
        }));
        // The signs around it are dimmed, so the plain text before and after has no run.
        assert!(
            runs.iter()
                .all(|r| r.start_utf16 >= 5 && r.start_utf16 + r.len_utf16 <= 13)
        );
    }

    #[test]
    fn style_runs_merge_characters_with_the_same_bits() {
        // `bc` is one run, not one per character.
        let runs = line_style_runs("a **bc** d".into());
        let bold: Vec<&StyleRun> = runs.iter().filter(|r| r.bits == md::BOLD).collect();
        assert_eq!(bold.len(), 1);
        assert_eq!((bold[0].start_utf16, bold[0].len_utf16), (4, 2));
        assert!(runs.iter().all(|r| r.bits != 0 && r.len_utf16 > 0));
    }

    #[test]
    fn code_runs_colour_keywords_and_strings() {
        let runs = code_style_runs("let s = \"é\";".into(), "rust".into());
        let keyword = runs.iter().find(|r| r.bits & md::KEYWORD != 0).unwrap();
        assert_eq!((keyword.start_utf16, keyword.len_utf16), (0, 3));
        let string = runs.iter().find(|r| r.bits & md::STRING != 0).unwrap();
        assert_eq!(string.len_utf16, 3); // "é" with its quotes
    }

    #[test]
    fn style_bits_match_the_logic_constants() {
        let bits = style_bits();
        assert_eq!(bits.bold, md::BOLD);
        assert_eq!(bits.underline, md::UNDER);
        assert_eq!(bits.function, md::FUNCTION);
    }

    #[test]
    fn links_are_reported_in_utf16() {
        let line = "é [[Plan|alias]] #tag [site](https://x.y)".to_string();
        let links = line_links(line);
        assert_eq!(links[0].kind, LinkKind::Wiki);
        assert_eq!(links[0].target, "Plan");
        assert_eq!(links[0].start_utf16, 2);
        assert_eq!(links[0].len_utf16, 14);
        assert_eq!(links[1].kind, LinkKind::Tag);
        assert_eq!(links[1].target, "tag");
        assert_eq!(links[2].kind, LinkKind::Url);
        assert_eq!(links[2].target, "https://x.y");
        assert_eq!(links[2].anchor, None);
    }

    #[test]
    fn a_callout_is_its_marker_lowercased() {
        assert_eq!(
            line_callout("> [!NOTE] Title".into()),
            Some(CalloutInfo {
                start_utf16: 2,
                len_utf16: 7,
                kind: "note".into(),
            })
        );
        assert_eq!(line_callout("> note".into()), None);
        assert_eq!(line_callout("> [!] empty".into()), None);
    }

    #[test]
    fn a_callout_marker_is_in_utf16_after_non_ascii() {
        // The emoji is two UTF-16 units and four bytes; the marker starts at unit 5, byte 7.
        assert_eq!(
            line_callout("😀 > [!TIP] x".into()),
            Some(CalloutInfo {
                start_utf16: 5,
                len_utf16: 6,
                kind: "tip".into(),
            })
        );
    }

    #[test]
    fn a_note_link_carries_its_anchor() {
        let links = line_links("[text](other.md#intro)".into());
        assert_eq!(links[0].kind, LinkKind::Note);
        assert_eq!(links[0].target, "other.md");
        assert_eq!(links[0].anchor.as_deref(), Some("intro"));
    }

    #[test]
    fn headings_carry_line_index_and_utf16_start() {
        let text = "---\ntitle: x\n---\n# Intro\n\né😀\n## Next";
        let found = headings(text.into());
        assert_eq!(
            found,
            [
                HeadingInfo {
                    level: 1,
                    title: "Intro".into(),
                    line: 3,
                    start_utf16: 17,
                },
                HeadingInfo {
                    level: 2,
                    title: "Next".into(),
                    line: 6,
                    start_utf16: 30,
                },
            ]
        );
    }

    #[test]
    fn headings_of_a_code_block_are_not_headings() {
        let found = headings("```\n# not one\n```\n# one".into());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "one");
        assert_eq!(found[0].line, 3);
    }

    #[test]
    fn note_index_gathers_tags_and_wikilinks() {
        let index = note_index("#Work and [[Plan]] and [[plan]] #home".into());
        assert_eq!(index.tags, ["work", "home"].map(String::from));
        assert_eq!(index.links, ["plan"].map(String::from));
    }
}
