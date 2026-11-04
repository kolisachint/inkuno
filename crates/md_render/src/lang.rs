//! Language detection from code fence info strings (task 5.2, DESIGN §6.6).

/// Languages the highlighter can tokenize. `PlainText` means no highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    PlainText,
    Rust,
    Python,
    JavaScript,
    Json,
    Bash,
    Markdown,
    Toml,
}

/// Map a fence info string (e.g. "rust", "rs", "Rust ignore", "py", "sh") to a [`Language`].
///
/// Only the first word is used, and matching is case-insensitive. Words are split on
/// whitespace, commas and `{`, so rustdoc-style attributes such as `rust,ignore` also
/// resolve. Unknown or empty info strings yield [`Language::PlainText`].
pub fn language_from_info(info: &str) -> Language {
    let word = info
        .trim()
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("");
    match word.to_ascii_lowercase().as_str() {
        "rust" | "rs" => Language::Rust,
        "python" | "py" => Language::Python,
        "javascript" | "js" | "jsx" => Language::JavaScript,
        "json" => Language::Json,
        "bash" | "sh" | "shell" | "zsh" => Language::Bash,
        "markdown" | "md" => Language::Markdown,
        "toml" => Language::Toml,
        _ => Language::PlainText,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_strings_map_to_languages() {
        let cases: &[(&str, Language)] = &[
            ("rust", Language::Rust),
            ("rs", Language::Rust),
            ("Rust", Language::Rust),
            ("Rust ignore", Language::Rust),
            ("rust,ignore", Language::Rust),
            ("python", Language::Python),
            ("py", Language::Python),
            ("PY", Language::Python),
            ("javascript", Language::JavaScript),
            ("js", Language::JavaScript),
            ("jsx", Language::JavaScript),
            ("json", Language::Json),
            ("JSON", Language::Json),
            ("bash", Language::Bash),
            ("sh", Language::Bash),
            ("shell", Language::Bash),
            ("zsh", Language::Bash),
            ("markdown", Language::Markdown),
            ("md", Language::Markdown),
            ("toml", Language::Toml),
            ("cobol", Language::PlainText),
            ("", Language::PlainText),
            ("   ", Language::PlainText),
            ("{.rust}", Language::PlainText),
        ];
        for (info, expected) in cases {
            assert_eq!(language_from_info(info), *expected, "info string {info:?}");
        }
    }

    #[test]
    fn only_first_word_is_used() {
        assert_eq!(
            language_from_info("python title=\"x.py\""),
            Language::Python
        );
        assert_eq!(language_from_info("cobol rust"), Language::PlainText);
    }
}
