//! Source Code Mode (plan task 8.1, DESIGN §5).
//!
//! With Source Mode on, the buffer is drawn as plain source: no marker is hidden, no block is
//! drawn as a table, an image or a list, and every line has the body size in a monospace face.
//! The text is coloured with the markdown grammar (`md_render::highlight`), cached per block
//! text like code fences (`CodeCache`).
//!
//! The rules here are pure functions, so they are unit-tested without GPUI. `TextElement`
//! (`element.rs`), `item_element` (`blocks/mod.rs`) and caret movement (`keys.rs`) apply them.

use std::ops::Range;

use md_core::BlockKind;
use md_render::highlight::TokenKind;

use crate::blocks::basic::{BlockStyle, Decoration, TextColor};
use crate::blocks::code_fence;
use crate::inline_render::{block_hidden_ranges, document_hidden_ranges};

/// The style of a block in Source Mode: the body size, no bold, no indent and no decoration.
pub fn source_style() -> BlockStyle {
    BlockStyle {
        font_scale: 1.0,
        bold: false,
        color: TextColor::Normal,
        indent_px: 0.0,
        decoration: Decoration::None,
    }
}

/// Hidden byte ranges of one block (see [`block_hidden_ranges`]). Empty in Source Mode, so every
/// marker is drawn.
pub fn block_hidden_in_mode(
    source: bool,
    block_text: &str,
    base: usize,
    kind: &BlockKind,
    caret: Option<usize>,
) -> Vec<Range<usize>> {
    if source {
        Vec::new()
    } else {
        block_hidden_ranges(block_text, base, kind, caret)
    }
}

/// Hidden byte ranges of the whole document (see [`document_hidden_ranges`]). Empty in Source
/// Mode. Caret movement uses it, so in Source Mode the caret steps through every marker.
pub fn document_hidden_in_mode(
    source: bool,
    text: &str,
    caret: Option<usize>,
) -> Vec<Range<usize>> {
    if source {
        Vec::new()
    } else {
        document_hidden_ranges(text, caret)
    }
}

/// Number of hidden ranges in the document for the mode. It is the counter behind the 8.1
/// acceptance check: zero in Source Mode.
pub fn hidden_range_count(source: bool, text: &str, caret: Option<usize>) -> usize {
    document_hidden_in_mode(source, text, caret).len()
}

/// The colour of a token in Source Mode. Markers (`#`, `*`, `-`, brackets of links) are muted, so
/// the text they mark stands out. Other kinds use the code palette.
pub fn token_color(kind: TokenKind) -> u32 {
    match kind {
        TokenKind::Punctuation => 0x8c959f,
        TokenKind::Emphasis => 0x6639ba,
        other => code_fence::token_color(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::code_fence::CodeCache;
    use md_render::highlight::Token;
    use md_render::lang::Language;

    /// A document with one of each marker type: headings, emphasis, strong, code spans, links,
    /// images, quotes, task and plain lists, a rule, a fence and a table.
    const EVERY_MARKER: &str = "# Heading\n\nSome *em* and **strong** and `code` and \
        [link](https://example.com) and ![alt](img.png).\n\n> quoted\n\n- item\n- [x] done\n\n\
        ---\n\n```rust\nlet x = 1;\n```\n\n| a | b |\n| - | - |\n| 1 | 2 |\n";

    #[test]
    fn source_mode_hides_no_marker() {
        // Normal mode hides markers for these carets, so the zero below is not trivially true.
        for caret in [None, Some(0), Some(EVERY_MARKER.len())] {
            assert!(
                hidden_range_count(false, EVERY_MARKER, caret) > 0,
                "normal mode hides markers for caret {caret:?}"
            );
            assert_eq!(
                hidden_range_count(true, EVERY_MARKER, caret),
                0,
                "source mode hides nothing for caret {caret:?}"
            );
        }
    }

    #[test]
    fn source_mode_block_ranges_are_empty() {
        let text = "## Title *x*";
        let kind = BlockKind::Heading(2);
        assert!(block_hidden_in_mode(true, text, 0, &kind, None).is_empty());
        assert!(!block_hidden_in_mode(false, text, 0, &kind, None).is_empty());
    }

    #[test]
    fn markdown_highlight_colours_heading_and_emphasis() {
        let src = "# Title\n\nSome *em* text\n";
        let tokens: Vec<Token> = CodeCache::default()
            .tokens(src, Language::Markdown)
            .to_vec();
        let text_of = |kind: TokenKind| -> Vec<&str> {
            tokens
                .iter()
                .filter(|t| t.kind == kind)
                .map(|t| &src[t.range.clone()])
                .collect()
        };
        assert!(
            text_of(TokenKind::Heading)
                .iter()
                .any(|t| t.contains("Title")),
            "heading token: {:?}",
            text_of(TokenKind::Heading)
        );
        assert!(
            text_of(TokenKind::Emphasis)
                .iter()
                .any(|t| t.contains("em")),
            "emphasis token: {:?}",
            text_of(TokenKind::Emphasis)
        );
    }

    #[test]
    fn source_colours_mute_markers_and_keep_code_palette() {
        assert_eq!(token_color(TokenKind::Punctuation), 0x8c959f);
        assert_eq!(
            token_color(TokenKind::Heading),
            code_fence::token_color(TokenKind::Heading)
        );
        assert_eq!(
            token_color(TokenKind::Keyword),
            code_fence::token_color(TokenKind::Keyword)
        );
    }
}
