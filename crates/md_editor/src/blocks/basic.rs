//! Basic block styles and inline runs (plan task 4.1). Owner: task 4.1.
//!
//! Pure functions, no GPUI types, so they are unit-tested on every platform. `element.rs`
//! converts a [`BlockStyle`] and the [`RunStyle`]s of a line into GPUI `TextRun`s and paints
//! the [`Decoration`]. Markers (`**`, `#`, `>`) stay visible in this task; hiding them is 4.4.

use std::ops::Range;

use md_core::{BlockKind, InlineSpan, SpanKind};

/// Left indent of a blockquote, in pixels.
pub const QUOTE_INDENT_PX: f32 = 16.0;

/// How one block is drawn. Offsets and text are unchanged; only the look differs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockStyle {
    /// Multiplier for the font size and the line height.
    pub font_scale: f32,
    /// Whole block in bold (headings).
    pub bold: bool,
    /// Colour of the text that is not a link.
    pub color: TextColor,
    /// Left indent of every line, in pixels.
    pub indent_px: f32,
    /// Extra painting around the text.
    pub decoration: Decoration,
}

/// Text colour role. The element maps each role to a concrete colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextColor {
    Normal,
    Muted,
    Link,
    Marker,
}

/// Painting next to the text of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decoration {
    None,
    /// A vertical bar on the left edge of the block (blockquote).
    QuoteBar,
    /// A horizontal line across the block (thematic break).
    Rule,
}

/// The style of a paragraph, and the base of every other style.
pub const PARAGRAPH: BlockStyle = BlockStyle {
    font_scale: 1.0,
    bold: false,
    color: TextColor::Normal,
    indent_px: 0.0,
    decoration: Decoration::None,
};

/// Style of a top-level block of `kind`.
pub fn block_style(kind: &BlockKind) -> BlockStyle {
    match kind {
        BlockKind::Heading(level) => BlockStyle {
            font_scale: heading_scale(*level),
            bold: true,
            ..PARAGRAPH
        },
        BlockKind::BlockQuote => BlockStyle {
            color: TextColor::Muted,
            indent_px: QUOTE_INDENT_PX,
            decoration: Decoration::QuoteBar,
            ..PARAGRAPH
        },
        BlockKind::ThematicBreak => BlockStyle {
            color: TextColor::Marker,
            decoration: Decoration::Rule,
            ..PARAGRAPH
        },
        _ => PARAGRAPH,
    }
}

/// Whether the inline spans of a block of `kind` are drawn with their styles. Code fences,
/// math, HTML and tables are not Markdown text, so their contents stay plain.
pub fn shows_inline_styles(kind: &BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Paragraph | BlockKind::Heading(_) | BlockKind::BlockQuote | BlockKind::List
    )
}

fn heading_scale(level: u8) -> f32 {
    match level {
        1 => 2.0,
        2 => 1.6,
        3 => 1.35,
        4 => 1.2,
        5 => 1.1,
        _ => 1.0,
    }
}

/// Styling of one contiguous part of a line. `range` is relative to the start of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStyle {
    pub range: Range<usize>,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
    pub link: bool,
}

/// Split `line` (whose first byte is at absolute offset `base`) into contiguous runs that cover
/// it completely, with no gaps or overlaps. Styles come from `spans` (absolute offsets, as
/// `md_core::inline::inline_spans` returns them). Nested spans combine. Adjacent parts with the
/// same style are merged. An empty line gives no runs.
///
/// Strong, emphasis, code and strikethrough style their whole span, markers included. A link
/// styles only its text, so the URL part is not underlined. Images and math are not styled.
pub fn line_runs(line: &str, base: usize, spans: &[InlineSpan]) -> Vec<RunStyle> {
    let len = line.len();
    if len == 0 {
        return Vec::new();
    }
    let end = base + len;

    // Cut points inside the line: its ends and every span boundary that falls within it.
    let mut cuts = vec![0, len];
    for span in spans {
        for abs in [
            span.range.start,
            span.range.end,
            span.content.start,
            span.content.end,
        ] {
            if (base..=end).contains(&abs) {
                cuts.push(abs - base);
            }
        }
    }
    cuts.sort_unstable();
    cuts.dedup();

    let mut runs: Vec<RunStyle> = Vec::with_capacity(cuts.len());
    for pair in cuts.windows(2) {
        let (start, stop) = (base + pair[0], base + pair[1]);
        let mut run = RunStyle {
            range: pair[0]..pair[1],
            bold: false,
            italic: false,
            code: false,
            strike: false,
            link: false,
        };
        for span in spans {
            let covered = match span.kind {
                SpanKind::Link => &span.content,
                _ => &span.range,
            };
            if covered.start > start || stop > covered.end {
                continue;
            }
            match span.kind {
                SpanKind::Strong => run.bold = true,
                SpanKind::Emphasis => run.italic = true,
                SpanKind::Code => run.code = true,
                SpanKind::Strikethrough => run.strike = true,
                SpanKind::Link => run.link = true,
                SpanKind::Image | SpanKind::Math => {}
            }
        }
        match runs.last_mut() {
            Some(last) if same_style(last, &run) => last.range.end = run.range.end,
            _ => runs.push(run),
        }
    }
    runs
}

fn same_style(a: &RunStyle, b: &RunStyle) -> bool {
    (a.bold, a.italic, a.code, a.strike, a.link) == (b.bold, b.italic, b.code, b.strike, b.link)
}

#[cfg(test)]
mod tests {
    use super::*;
    use md_core::inline::inline_spans;

    /// Runs of `text` at base `base`, with the spans of `text`.
    fn runs_of(text: &str, base: usize) -> Vec<RunStyle> {
        line_runs(text, base, &inline_spans(text, base))
    }

    /// Assert that the runs tile `0..len` exactly.
    fn assert_covers(runs: &[RunStyle], len: usize) {
        let mut next = 0;
        for run in runs {
            assert_eq!(run.range.start, next, "gap or overlap in {runs:?}");
            assert!(run.range.start < run.range.end, "empty run in {runs:?}");
            next = run.range.end;
        }
        assert_eq!(next, len, "runs do not reach the end of the line");
    }

    #[test]
    fn heading_scales_strictly_decrease_from_h1_to_h6() {
        let scales: Vec<f32> = (1..=6)
            .map(|level| block_style(&BlockKind::Heading(level)).font_scale)
            .collect();
        assert_eq!(scales, vec![2.0, 1.6, 1.35, 1.2, 1.1, 1.0]);
        assert!(scales.windows(2).all(|pair| pair[0] > pair[1]));
        for level in 1..=6 {
            assert!(block_style(&BlockKind::Heading(level)).bold);
        }
    }

    #[test]
    fn paragraph_and_other_blocks_use_the_base_style() {
        assert_eq!(block_style(&BlockKind::Paragraph), PARAGRAPH);
        assert_eq!(block_style(&BlockKind::List), PARAGRAPH);
        assert_eq!(block_style(&BlockKind::CodeFence), PARAGRAPH);
    }

    #[test]
    fn blockquote_and_rule_decorations() {
        let quote = block_style(&BlockKind::BlockQuote);
        assert_eq!(quote.decoration, Decoration::QuoteBar);
        assert_eq!(quote.color, TextColor::Muted);
        assert!(quote.indent_px > 0.0);
        let rule = block_style(&BlockKind::ThematicBreak);
        assert_eq!(rule.decoration, Decoration::Rule);
        assert_eq!(rule.color, TextColor::Marker);
    }

    #[test]
    fn inline_styles_apply_to_text_blocks_only() {
        assert!(shows_inline_styles(&BlockKind::Paragraph));
        assert!(shows_inline_styles(&BlockKind::Heading(3)));
        assert!(!shows_inline_styles(&BlockKind::CodeFence));
        assert!(!shows_inline_styles(&BlockKind::MathBlock));
        assert!(!shows_inline_styles(&BlockKind::Table));
    }

    #[test]
    fn plain_text_is_one_unstyled_run() {
        let runs = runs_of("plain text", 0);
        assert_eq!(
            runs,
            vec![RunStyle {
                range: 0..10,
                bold: false,
                italic: false,
                code: false,
                strike: false,
                link: false,
            }]
        );
    }

    #[test]
    fn empty_line_has_no_runs() {
        assert!(runs_of("", 5).is_empty());
    }

    #[test]
    fn bold_is_split_out_of_plain_text() {
        let text = "a **bold** b";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
        let bold: Vec<&RunStyle> = runs.iter().filter(|run| run.bold).collect();
        assert_eq!(bold.len(), 1);
        assert_eq!(bold[0].range, 2..10, "the strong span with its markers");
        assert!(runs.iter().all(|run| !run.italic && !run.code));
    }

    #[test]
    fn emphasis_is_italic() {
        let text = "*it*";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
        assert!(runs.iter().all(|run| run.italic && !run.bold));
    }

    #[test]
    fn inline_code_is_code() {
        let text = "x `code` y";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
        let code: Vec<&RunStyle> = runs.iter().filter(|run| run.code).collect();
        assert_eq!(code.len(), 1);
        assert_eq!(code[0].range, 2..8);
    }

    #[test]
    fn nested_bold_and_italic_combine() {
        let text = "***both***";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
        assert!(runs.iter().any(|run| run.bold && run.italic));
    }

    #[test]
    fn link_styles_its_text_not_its_url() {
        let text = "[docs](https://example.com)";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
        let link: Vec<&RunStyle> = runs.iter().filter(|run| run.link).collect();
        assert_eq!(link.len(), 1);
        assert_eq!(link[0].range, 1..5, "the link text only");
    }

    #[test]
    fn strikethrough_is_struck() {
        let text = "~~gone~~";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
        assert!(runs.iter().all(|run| run.strike));
    }

    #[test]
    fn runs_use_absolute_spans_with_a_base_offset() {
        // The line "x **b**" starts at byte 40 of the document.
        let line = "x **b**";
        let spans = inline_spans(line, 40);
        let runs = line_runs(line, 40, &spans);
        assert_covers(&runs, line.len());
        assert!(runs.iter().any(|run| run.bold && run.range == (2..7)));
    }

    #[test]
    fn multibyte_text_is_covered_by_byte_ranges() {
        let text = "日本 **語**";
        let runs = runs_of(text, 0);
        assert_covers(&runs, text.len());
    }

    #[test]
    fn spans_outside_the_line_do_not_style_it() {
        // A span that ends before this line's text does not style it.
        let spans = vec![InlineSpan {
            kind: SpanKind::Strong,
            range: 0..5,
            markers: vec![0..2, 3..5],
            content: 2..3,
        }];
        let runs = line_runs("abcdefgh", 10, &spans);
        assert_covers(&runs, 8);
        assert!(runs.iter().all(|run| !run.bold));
    }
}
