//! Tree-sitter code highlighting (task 5.1, DESIGN §6.6 and §6.7).
//!
//! The [`Highlighter`] trait is the interface. It has no GPUI types. The
//! tree-sitter implementation builds one `HighlightConfiguration` per language
//! once, in [`TreeSitterHighlighter::new`].

use std::collections::{BTreeSet, HashMap};
use std::ops::Range;

use tree_sitter::Language as GrammarLanguage;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter as TsHighlighter};

use crate::lang::{language_from_info, Language};

/// Coarse token category used by the renderer to pick a theme colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Variable,
    Constant,
    Operator,
    Punctuation,
    Property,
    Attribute,
    Heading,
    Emphasis,
    Link,
    Other,
}

/// A highlighted byte range of the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub range: Range<usize>,
    pub kind: TokenKind,
}

/// Tokenizes source text for highlighting.
pub trait Highlighter: Send + Sync {
    /// Byte-range tokens for `source` in `lang`. Unhighlighted text produces no token. Tokens are sorted and non-overlapping (innermost capture wins).
    fn highlight(&self, source: &str, lang: Language) -> Vec<Token>;
}

/// Highlighter backed by tree-sitter grammars and their bundled highlight queries.
pub struct TreeSitterHighlighter {
    configs: HashMap<Language, HighlightConfiguration>,
    /// Inline Markdown, reached through injection from the block Markdown grammar.
    markdown_inline: HighlightConfiguration,
    /// Token kind for each highlight index. Every configuration shares this index space.
    kinds: Vec<TokenKind>,
}

impl TreeSitterHighlighter {
    /// Build the configuration for every supported language. This compiles the
    /// bundled queries, so it is expensive and should run once.
    pub fn new() -> Self {
        // The block Markdown injection query marks inline content for injection, but
        // without `injection.include-children` the inline layer is dropped. Set it on the
        // inline pattern (the bundled query lacks it).
        let markdown_injections = tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
            "(#set! injection.language \"markdown_inline\"))",
            "(#set! injection.language \"markdown_inline\") (#set! injection.include-children))",
        );

        let grammars: [(Language, &str, GrammarLanguage, &str, &str); 7] = [
            (
                Language::Rust,
                "rust",
                tree_sitter_rust::LANGUAGE.into(),
                tree_sitter_rust::HIGHLIGHTS_QUERY,
                "",
            ),
            (
                Language::Python,
                "python",
                tree_sitter_python::LANGUAGE.into(),
                tree_sitter_python::HIGHLIGHTS_QUERY,
                "",
            ),
            (
                Language::JavaScript,
                "javascript",
                tree_sitter_javascript::LANGUAGE.into(),
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                "",
            ),
            (
                Language::Json,
                "json",
                tree_sitter_json::LANGUAGE.into(),
                tree_sitter_json::HIGHLIGHTS_QUERY,
                "",
            ),
            (
                Language::Bash,
                "bash",
                tree_sitter_bash::LANGUAGE.into(),
                tree_sitter_bash::HIGHLIGHT_QUERY,
                "",
            ),
            (
                Language::Toml,
                "toml",
                tree_sitter_toml_ng::LANGUAGE.into(),
                tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
                "",
            ),
            (
                Language::Markdown,
                "markdown",
                tree_sitter_md::LANGUAGE.into(),
                tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
                &markdown_injections,
            ),
        ];

        let mut configs: Vec<(Language, HighlightConfiguration)> = grammars
            .into_iter()
            .map(|(lang, name, grammar, highlights, injections)| {
                let config = HighlightConfiguration::new(grammar, name, highlights, injections, "")
                    .unwrap_or_else(|e| panic!("bundled {name} highlight query is invalid: {e}"));
                (lang, config)
            })
            .collect();
        let mut markdown_inline = HighlightConfiguration::new(
            tree_sitter_md::INLINE_LANGUAGE.into(),
            "markdown_inline",
            tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
            tree_sitter_md::INJECTION_QUERY_INLINE,
            "",
        )
        .unwrap_or_else(|e| panic!("bundled markdown_inline highlight query is invalid: {e}"));

        // All configurations must agree on highlight indices, so they share one name list:
        // the union of every capture name used by any query. `@none` is left out, so it
        // never opens a highlight. Otherwise an outer `@none` opened after an inner capture
        // at the same position would pop the wrong one when injected layers close.
        let names: Vec<String> = configs
            .iter()
            .map(|(_, c)| c)
            .chain(std::iter::once(&markdown_inline))
            .flat_map(|c| c.query.capture_names().iter().map(|n| n.to_string()))
            .filter(|n| n != "none")
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let kinds = names.iter().map(|n| kind_for_capture(n)).collect();

        for (_, config) in &mut configs {
            config.configure(names.as_slice());
        }
        markdown_inline.configure(names.as_slice());

        Self {
            configs: configs.into_iter().collect(),
            markdown_inline,
            kinds,
        }
    }

    /// Resolve an injected language name (from a fence info string or the
    /// Markdown grammar) to its configuration.
    fn injection_config(&self, name: &str) -> Option<&HighlightConfiguration> {
        if name == "markdown_inline" {
            return Some(&self.markdown_inline);
        }
        self.configs.get(&language_from_info(name))
    }
}

impl Default for TreeSitterHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter for TreeSitterHighlighter {
    fn highlight(&self, source: &str, lang: Language) -> Vec<Token> {
        let Some(config) = self.configs.get(&lang) else {
            return Vec::new();
        };
        // A fresh tree-sitter highlighter per call keeps `&self` shareable across threads.
        let mut ts = TsHighlighter::new();
        let Ok(events) = ts.highlight(config, source.as_bytes(), None, None, |name| {
            self.injection_config(name)
        }) else {
            return Vec::new();
        };

        let mut tokens: Vec<Token> = Vec::new();
        // Stack of open captures. The innermost capture decides the kind. A `None` entry
        // only keeps Start and End events paired.
        let mut stack: Vec<Option<TokenKind>> = Vec::new();
        for event in events {
            match event {
                Ok(HighlightEvent::HighlightStart(h)) => {
                    stack.push(self.kinds.get(h.0).copied());
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    stack.pop();
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    if start >= end {
                        continue;
                    }
                    if let Some(kind) = stack.last().copied().flatten() {
                        push_token(&mut tokens, start..end, kind);
                    }
                }
                Err(_) => break,
            }
        }
        tokens
    }
}

/// Append a token, merging it into the previous one when both have the same kind and touch.
fn push_token(tokens: &mut Vec<Token>, range: Range<usize>, kind: TokenKind) {
    if let Some(last) = tokens.last_mut() {
        if last.kind == kind && last.range.end == range.start {
            last.range.end = range.end;
            return;
        }
    }
    tokens.push(Token { range, kind });
}

/// Map a tree-sitter capture name to a token kind by its first dotted component.
/// Unrecognized components map to `Other`.
fn kind_for_capture(name: &str) -> TokenKind {
    use TokenKind::*;

    match name.split('.').next().unwrap_or(name) {
        "keyword" => Keyword,
        "string" => String,
        "comment" => Comment,
        "number" => Number,
        "function" => Function,
        "type" | "constructor" => Type,
        "variable" => Variable,
        "constant" | "boolean" => Constant,
        "operator" => Operator,
        "punctuation" => Punctuation,
        "property" => Property,
        "attribute" => Attribute,
        "text" | "markup" => {
            if name.starts_with("text.title") || name.starts_with("markup.heading") {
                Heading
            } else if name.starts_with("text.emphasis")
                || name.starts_with("text.strong")
                || name.starts_with("markup.italic")
                || name.starts_with("markup.bold")
            {
                Emphasis
            } else if name.starts_with("text.uri")
                || name.starts_with("text.reference")
                || name.starts_with("markup.link")
            {
                Link
            } else if name.starts_with("text.literal") || name.starts_with("markup.raw") {
                String
            } else {
                Other
            }
        }
        _ => Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts<'a>(source: &'a str, tokens: &[Token], kind: TokenKind) -> Vec<&'a str> {
        tokens
            .iter()
            .filter(|t| t.kind == kind)
            .map(|t| &source[t.range.clone()])
            .collect()
    }

    fn assert_sorted_non_overlapping(source: &str, tokens: &[Token]) {
        for t in tokens {
            assert!(t.range.start < t.range.end, "empty token {t:?}");
            assert!(t.range.end <= source.len(), "token past end {t:?}");
            assert!(source.is_char_boundary(t.range.start));
            assert!(source.is_char_boundary(t.range.end));
        }
        for pair in tokens.windows(2) {
            assert!(
                pair[0].range.end <= pair[1].range.start,
                "tokens overlap or are unsorted: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn rust_keywords_and_strings() {
        let src = r#"fn main() { let s = "hi"; }"#;
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Rust);
        assert_sorted_non_overlapping(src, &tokens);
        let keywords = texts(src, &tokens, TokenKind::Keyword);
        assert!(keywords.contains(&"fn"), "keywords: {keywords:?}");
        assert!(keywords.contains(&"let"), "keywords: {keywords:?}");
        assert_eq!(texts(src, &tokens, TokenKind::String), vec!["\"hi\""]);
        let fn_token = tokens
            .iter()
            .find(|t| t.kind == TokenKind::Keyword && &src[t.range.clone()] == "fn")
            .expect("fn keyword token");
        assert_eq!(fn_token.range, 0..2);
    }

    #[test]
    fn python_keywords_and_strings() {
        let src = "def f(x):\n    return \"hi\"\n";
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Python);
        assert_sorted_non_overlapping(src, &tokens);
        let keywords = texts(src, &tokens, TokenKind::Keyword);
        assert!(keywords.contains(&"def"), "keywords: {keywords:?}");
        assert!(keywords.contains(&"return"), "keywords: {keywords:?}");
        assert_eq!(texts(src, &tokens, TokenKind::String), vec!["\"hi\""]);
    }

    #[test]
    fn json_numbers_and_constants() {
        let src = r#"{"a": 1, "b": true}"#;
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Json);
        assert_sorted_non_overlapping(src, &tokens);
        assert_eq!(texts(src, &tokens, TokenKind::Number), vec!["1"]);
        assert_eq!(texts(src, &tokens, TokenKind::Constant), vec!["true"]);
    }

    #[test]
    fn bash_tokens_are_sorted() {
        let src = "# greet\necho \"hi\" $HOME\n";
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Bash);
        assert_sorted_non_overlapping(src, &tokens);
        assert_eq!(texts(src, &tokens, TokenKind::Comment), vec!["# greet"]);
        assert!(texts(src, &tokens, TokenKind::String).contains(&"\"hi\""));
    }

    #[test]
    fn toml_tokens_are_sorted() {
        let src = "[pkg]\nname = \"noteapp\"\nversion = 3\n";
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Toml);
        assert_sorted_non_overlapping(src, &tokens);
        assert!(texts(src, &tokens, TokenKind::String).contains(&"\"noteapp\""));
    }

    #[test]
    fn markdown_heading_and_emphasis() {
        let src = "# Title\n\nSome *em* text\n";
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Markdown);
        assert_sorted_non_overlapping(src, &tokens);
        let headings = texts(src, &tokens, TokenKind::Heading);
        assert!(headings.iter().any(|h| h.contains("Title")), "{headings:?}");
        assert!(!texts(src, &tokens, TokenKind::Emphasis).is_empty());
    }

    #[test]
    fn markdown_fence_is_highlighted_by_injection() {
        let src = "```rust\nfn main() {}\n```\n";
        let hl = TreeSitterHighlighter::new();
        let tokens = hl.highlight(src, Language::Markdown);
        assert_sorted_non_overlapping(src, &tokens);
        assert!(texts(src, &tokens, TokenKind::Keyword).contains(&"fn"));
    }

    #[test]
    fn plain_text_yields_no_tokens() {
        let hl = TreeSitterHighlighter::new();
        assert!(hl.highlight("fn main() {}", Language::PlainText).is_empty());
        assert!(hl.highlight("", Language::Rust).is_empty());
    }

    #[test]
    fn every_supported_language_produces_sorted_tokens() {
        let hl = TreeSitterHighlighter::new();
        let samples = [
            (Language::Rust, "fn f() -> u8 { 1 }\n"),
            (Language::Python, "class A:\n    pass\n"),
            (Language::JavaScript, "const x = () => 'a';\n"),
            (Language::Json, "[1, 2, \"x\"]\n"),
            (Language::Bash, "for i in 1 2; do echo $i; done\n"),
            (Language::Toml, "a = [1, 2]\n"),
            (Language::Markdown, "- item **bold**\n"),
        ];
        for (lang, src) in samples {
            let tokens = hl.highlight(src, lang);
            assert!(!tokens.is_empty(), "{lang:?} produced no tokens");
            assert_sorted_non_overlapping(src, &tokens);
        }
    }

    #[test]
    fn capture_names_map_to_kinds() {
        assert_eq!(kind_for_capture("keyword"), TokenKind::Keyword);
        assert_eq!(kind_for_capture("function.method"), TokenKind::Function);
        assert_eq!(kind_for_capture("string.special"), TokenKind::String);
        assert_eq!(kind_for_capture("type.builtin"), TokenKind::Type);
        assert_eq!(
            kind_for_capture("punctuation.bracket"),
            TokenKind::Punctuation
        );
        assert_eq!(kind_for_capture("text.title"), TokenKind::Heading);
        assert_eq!(kind_for_capture("markup.heading.1"), TokenKind::Heading);
        assert_eq!(kind_for_capture("text.uri"), TokenKind::Link);
        assert_eq!(kind_for_capture("text.emphasis"), TokenKind::Emphasis);
        assert_eq!(kind_for_capture("label"), TokenKind::Other);
    }

    #[test]
    fn highlighter_is_object_safe_and_thread_shareable() {
        let hl: Box<dyn Highlighter> = Box::new(TreeSitterHighlighter::new());
        fn assert_send_sync<T: Send + Sync>(_: &T) {}
        assert_send_sync(&hl);
        assert!(!hl.highlight("let x = 1;", Language::Rust).is_empty());
    }
}
