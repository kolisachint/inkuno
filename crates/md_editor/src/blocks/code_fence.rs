//! Code fence blocks (plan task 5.3, DESIGN §4.2).
//!
//! A fence is drawn as monospace text on a light background, with its code highlighted by
//! `md_render`. The fence lines (opening and closing, with the info string) are hidden while the
//! caret is outside the block, and shown in a muted colour while it is inside.
//!
//! The parsing, colour and segment functions are pure, so they are unit-tested on every
//! platform. `element.rs` turns them into GPUI runs and paints the background. Math and Mermaid
//! fences are not handled here: they stay plain code (on hold, see AGENTS.md).

use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::OnceLock;

use md_render::highlight::{Highlighter, Token, TokenKind, TreeSitterHighlighter};
use md_render::lang::{language_from_info, Language};

use crate::element::line_ranges;

/// Padding inside a code block, in pixels: left and right, and top and bottom.
pub const CODE_PAD_X_PX: f32 = 12.0;
pub const CODE_PAD_Y_PX: f32 = 8.0;
/// Corner radius of the code block background, in pixels.
pub const CODE_BLOCK_RADIUS_PX: f32 = 6.0;
/// Highlighted blocks kept in the cache. The cache is cleared when it is full.
const CACHE_LIMIT: usize = 256;

/// A parsed code fence. Every range is an absolute byte offset in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fence {
    /// The opening fence line, without its newline. It holds the info string.
    pub open_line: Range<usize>,
    /// The closing fence line, or `None` when the block runs to its end without one.
    pub close_line: Option<Range<usize>>,
    /// The info string after the opening fence, trimmed (e.g. `rust {.class}`).
    pub info: String,
    /// The code lines between the fences, without the newline before the closing fence.
    pub code: Range<usize>,
}

impl Fence {
    /// The language of the code, from the info string. Unknown languages are plain text.
    pub fn language(&self) -> Language {
        language_from_info(&self.info)
    }
}

/// Parse a code fence block. `block_text` is the block without its trailing newline and `base`
/// is its offset in the document. Returns `None` when the first line is not an opening fence.
///
/// An opening fence is at most 3 spaces of indent, then 3 or more backticks or tildes. A
/// backtick fence's info string may not contain a backtick. A closing fence uses the same
/// character, at least as many, at most 3 spaces of indent, and nothing else on its line. A
/// block with no closing fence runs to its end.
pub fn parse_fence(block_text: &str, base: usize) -> Option<Fence> {
    let lines = line_ranges(block_text);
    let first = lines.first()?.clone();
    let (marker, count, info) = opening(&block_text[first.clone()])?;
    let close_index = lines
        .iter()
        .skip(1)
        .position(|line| is_closing(&block_text[line.clone()], marker, count))
        .map(|index| index + 1);
    let code_start = (first.end + 1).min(block_text.len());
    let code_end = match close_index {
        // The newline before the closing fence ends the code. With no code lines it is empty.
        Some(index) => lines[index].start.saturating_sub(1).max(code_start),
        None => block_text.len(),
    };
    Some(Fence {
        open_line: base + first.start..base + first.end,
        close_line: close_index.map(|index| base + lines[index].start..base + lines[index].end),
        info: info.trim().to_string(),
        code: base + code_start..base + code_end.max(code_start),
    })
}

/// The fence lines of a code fence block, absolute: the opening line and, if present, the
/// closing one. Empty when the block is not a fence. `block_hidden_ranges` hides them while the
/// caret is outside the block.
pub fn fence_lines(block_text: &str, base: usize) -> Vec<Range<usize>> {
    match parse_fence(block_text, base) {
        Some(fence) => std::iter::once(fence.open_line)
            .chain(fence.close_line)
            .collect(),
        None => Vec::new(),
    }
}

/// The opening fence of `line`: its character, the number of characters and the info string.
fn opening(line: &str) -> Option<(char, usize, &str)> {
    let rest = &line[indent(line)?..];
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    // The marker is ASCII, so its character count is also its byte length.
    let count = rest.chars().take_while(|&c| c == marker).count();
    if count < 3 {
        return None;
    }
    let info = rest[count..].trim();
    if marker == '`' && info.contains('`') {
        return None;
    }
    Some((marker, count, info))
}

/// Whether `line` closes a fence opened with `count` `marker` characters.
fn is_closing(line: &str, marker: char, count: usize) -> bool {
    let Some(start) = indent(line) else {
        return false;
    };
    let rest = &line[start..];
    let run = rest.chars().take_while(|&c| c == marker).count();
    run >= count && rest[run..].trim().is_empty()
}

/// The indent of `line` in bytes, or `None` when it is 4 spaces or more (not a fence line).
fn indent(line: &str) -> Option<usize> {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    (spaces <= 3).then_some(spaces)
}

/// The colour runs of one source line of a code block. `line` is an absolute range and
/// `tokens` are relative to `code_base`, the absolute start of the highlighted code. The
/// result gives each run's length and token kind (`None` for unhighlighted text), and covers
/// `line` with no gaps. Adjacent runs of one kind are merged.
pub fn line_segments(
    line: Range<usize>,
    code_base: usize,
    tokens: &[Token],
) -> Vec<(usize, Option<TokenKind>)> {
    let mut out: Vec<(usize, Option<TokenKind>)> = Vec::new();
    let mut pos = line.start;
    while pos < line.end {
        // The first token that ends after `pos`. It covers `pos` when it starts at or before it.
        let index = tokens.partition_point(|token| code_base + token.range.end <= pos);
        let next = tokens.get(index);
        let kind = next
            .filter(|token| code_base + token.range.start <= pos)
            .map(|token| token.kind);
        let boundary = match (next, kind) {
            (Some(token), Some(_)) => code_base + token.range.end,
            (Some(token), None) => code_base + token.range.start,
            (None, _) => line.end,
        };
        let stop = boundary.min(line.end);
        let len = stop - pos;
        match out.last_mut() {
            Some((last_len, last_kind)) if *last_kind == kind => *last_len += len,
            _ => out.push((len, kind)),
        }
        pos = stop;
    }
    out
}

/// The shared tree-sitter highlighter. Building it compiles the bundled queries, so it is built
/// once, on first use.
fn highlighter() -> &'static TreeSitterHighlighter {
    static HIGHLIGHTER: OnceLock<TreeSitterHighlighter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(TreeSitterHighlighter::new)
}

/// Highlight results of code blocks, kept by the editor so that a frame does not highlight
/// again. Keys are the language and the code text. The cache is cleared when it is full.
#[derive(Default)]
pub struct CodeCache {
    entries: HashMap<(Language, String), Rc<[Token]>>,
}

impl CodeCache {
    /// The tokens of `code` in `lang`, relative to the start of `code`.
    pub fn tokens(&mut self, code: &str, lang: Language) -> Rc<[Token]> {
        if lang == Language::PlainText || code.is_empty() {
            return Rc::from(Vec::new());
        }
        let key = (lang, code.to_string());
        if let Some(tokens) = self.entries.get(&key) {
            return Rc::clone(tokens);
        }
        if self.entries.len() >= CACHE_LIMIT {
            self.entries.clear();
        }
        let tokens: Rc<[Token]> = highlighter().highlight(code, lang).into();
        self.entries.insert(key, Rc::clone(&tokens));
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inline_render::block_hidden_ranges;
    use crate::theme::{code_token_color, CodeColors};
    use md_core::BlockKind;

    #[test]
    fn backtick_fence_with_info_string() {
        let text = "```rust\nfn f() {}\n```";
        let fence = parse_fence(text, 10).expect("fence");
        assert_eq!(fence.open_line, 10..17);
        assert_eq!(fence.close_line, Some(28..31));
        assert_eq!(fence.info, "rust");
        assert_eq!(fence.code, 18..27);
        assert_eq!(fence.language(), Language::Rust);
    }

    #[test]
    fn tilde_fence_unclosed_runs_to_the_end() {
        let fence = parse_fence("~~~~\na\nb", 0).expect("fence");
        assert_eq!(fence.open_line, 0..4);
        assert_eq!(fence.close_line, None);
        assert_eq!(fence.info, "");
        assert_eq!(fence.code, 5..8, "both code lines");
        assert_eq!(fence.language(), Language::PlainText);
    }

    #[test]
    fn info_string_with_attributes_names_the_language() {
        let text = "``` rust {.class}\nx\n```";
        let fence = parse_fence(text, 0).expect("fence");
        assert_eq!(fence.info, "rust {.class}");
        assert_eq!(fence.language(), Language::Rust);
    }

    #[test]
    fn indented_fences() {
        let text = "  ```py\nx = 1\n   ```";
        let fence = parse_fence(text, 0).expect("fence");
        assert_eq!(fence.open_line, 0..7);
        assert_eq!(fence.close_line, Some(14..20));
        assert_eq!(fence.language(), Language::Python);
        assert!(
            parse_fence("    ```\nx\n```", 0).is_none(),
            "4 spaces is code"
        );
    }

    #[test]
    fn closing_fence_must_match_and_be_long_enough() {
        // A shorter run, or other text on the line, does not close a four-backtick fence.
        let fence = parse_fence("````\nx\n```\n", 0).expect("fence");
        assert_eq!(fence.close_line, None);
        let fence = parse_fence("```\nx\n``` y", 0).expect("fence");
        assert_eq!(fence.close_line, None);
        // A tilde fence is not closed by backticks.
        let fence = parse_fence("~~~\nx\n```", 0).expect("fence");
        assert_eq!(fence.close_line, None);
    }

    #[test]
    fn empty_fence_has_an_empty_code_range() {
        let fence = parse_fence("```\n```", 0).expect("fence");
        assert_eq!(fence.code, 4..4);
        assert_eq!(fence.close_line, Some(4..7));
    }

    #[test]
    fn non_fences_are_rejected() {
        assert!(parse_fence("``not a fence", 0).is_none());
        assert!(parse_fence("``", 0).is_none());
        assert!(
            parse_fence("```a`b", 0).is_none(),
            "backtick in a backtick info string"
        );
        assert!(parse_fence("text\n```", 0).is_none());
    }

    #[test]
    fn fence_lines_are_the_opening_and_closing_lines() {
        assert_eq!(fence_lines("```rust\nx\n```", 5), vec![5..12, 15..18]);
        assert_eq!(
            fence_lines("```\nx", 0),
            vec![0..3],
            "unclosed: opening only"
        );
        assert!(fence_lines("plain", 0).is_empty());
    }

    #[test]
    fn fence_lines_hidden_outside_and_none_inside() {
        let text = "```rust\nfn f() {}\n```";
        let outside = block_hidden_ranges(text, 0, &BlockKind::CodeFence, None);
        assert_eq!(outside, vec![0..7, 18..21], "both fence lines");
        let inside = block_hidden_ranges(text, 0, &BlockKind::CodeFence, Some(10));
        assert!(inside.is_empty(), "caret inside shows the fence lines");
    }

    #[test]
    fn token_colours_are_distinct_for_the_main_kinds() {
        let colors = CodeColors::light();
        let token = |kind| code_token_color(kind, &colors);
        assert_eq!(token(TokenKind::Keyword), colors.keyword);
        assert_eq!(token(TokenKind::String), colors.string);
        assert_eq!(token(TokenKind::Comment), colors.comment);
        assert_ne!(token(TokenKind::Keyword), token(TokenKind::String));
        assert_ne!(token(TokenKind::Function), token(TokenKind::Type));
        assert_eq!(token(TokenKind::Number), token(TokenKind::Constant));
    }

    #[test]
    fn line_segments_cover_the_line() {
        // Code "fn x" at 100: "fn" is a keyword (100..102), "x" a function (103..104).
        let tokens = [
            Token {
                range: 0..2,
                kind: TokenKind::Keyword,
            },
            Token {
                range: 3..4,
                kind: TokenKind::Function,
            },
        ];
        assert_eq!(
            line_segments(100..104, 100, &tokens),
            vec![
                (2, Some(TokenKind::Keyword)),
                (1, None),
                (1, Some(TokenKind::Function)),
            ]
        );
    }

    #[test]
    fn line_segments_start_inside_a_token_and_merge() {
        let tokens = [
            Token {
                range: 0..2,
                kind: TokenKind::Keyword,
            },
            Token {
                range: 3..6,
                kind: TokenKind::String,
            },
        ];
        // The line 101..108 starts inside the keyword and ends past the string.
        let segments = line_segments(101..108, 100, &tokens);
        assert_eq!(
            segments,
            vec![
                (1, Some(TokenKind::Keyword)),
                (1, None),
                (3, Some(TokenKind::String)),
                (2, None),
            ]
        );
        assert_eq!(segments.iter().map(|(len, _)| len).sum::<usize>(), 7);
        assert!(line_segments(5..5, 0, &tokens).is_empty());
        assert_eq!(line_segments(0..2, 0, &[]), vec![(2, None)]);
    }

    #[test]
    fn cache_highlights_once_and_skips_plain_text() {
        let mut cache = CodeCache::default();
        let first = cache.tokens("def f():\n    pass\n", Language::Python);
        assert!(first.iter().any(|t| t.kind == TokenKind::Keyword));
        let second = cache.tokens("def f():\n    pass\n", Language::Python);
        assert!(
            Rc::ptr_eq(&first, &second),
            "the second call reuses the result"
        );
        assert!(cache.tokens("def f():", Language::PlainText).is_empty());
        assert!(cache.tokens("", Language::Python).is_empty());
    }
}
