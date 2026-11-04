use std::ops::Range;

/// A single text replacement in source byte offsets.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edit {
    pub range: Range<usize>,
    pub text: String,
}

impl Edit {
    pub fn insert(at: usize, text: impl Into<String>) -> Self {
        Self {
            range: at..at,
            text: text.into(),
        }
    }

    pub fn delete(range: Range<usize>) -> Self {
        Self {
            range,
            text: String::new(),
        }
    }

    pub fn replace(range: Range<usize>, text: impl Into<String>) -> Self {
        Self {
            range,
            text: text.into(),
        }
    }
}

/// Apply edits (given in non-overlapping order, offsets relative to the ORIGINAL text) to a string.
pub fn apply_edits(text: &str, edits: &[Edit]) -> String {
    let mut sorted: Vec<&Edit> = edits.iter().collect();
    sorted.sort_by_key(|e| (e.range.start, e.range.end));

    let mut out = text.to_string();
    for edit in sorted.iter().rev() {
        out.replace_range(edit.range.clone(), &edit.text);
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockKind {
    Paragraph,
    Heading(u8),
    List,
    BlockQuote,
    Table,
    CodeFence,
    MathBlock,
    ThematicBreak,
    HtmlBlock,
    Blank,
}

/// A top-level block. Block ranges tile the document: no gaps, no overlaps, ends include the trailing newline.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Block {
    pub kind: BlockKind,
    pub range: Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpanKind {
    Strong,
    Emphasis,
    Code,
    Link,
    Image,
    Math,
    Strikethrough,
}

/// An inline span: `range` covers the whole span including markers; `markers` are the ranges hidden when the caret is outside; `content` is the visible text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InlineSpan {
    pub kind: SpanKind,
    pub range: Range<usize>,
    pub markers: Vec<Range<usize>>,
    pub content: Range<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_at_start_middle_and_end() {
        assert_eq!(apply_edits("abc", &[Edit::insert(0, "X")]), "Xabc");
        assert_eq!(apply_edits("abc", &[Edit::insert(1, "X")]), "aXbc");
        assert_eq!(apply_edits("abc", &[Edit::insert(3, "X")]), "abcX");
    }

    #[test]
    fn insert_into_empty_text() {
        assert_eq!(apply_edits("", &[Edit::insert(0, "hi")]), "hi");
    }

    #[test]
    fn delete_range() {
        assert_eq!(apply_edits("hello world", &[Edit::delete(5..11)]), "hello");
        assert_eq!(apply_edits("hello", &[Edit::delete(0..0)]), "hello");
    }

    #[test]
    fn replace_range() {
        assert_eq!(
            apply_edits("hello world", &[Edit::replace(6..11, "there")]),
            "hello there"
        );
    }

    #[test]
    fn no_edits_returns_original() {
        assert_eq!(apply_edits("unchanged", &[]), "unchanged");
    }

    #[test]
    fn multiple_edits_use_original_offsets() {
        // Given out of order on purpose; offsets refer to the original text "one two three".
        let edits = [
            Edit::replace(8..13, "3"),
            Edit::insert(0, "> "),
            Edit::delete(3..4),
        ];
        assert_eq!(apply_edits("one two three", &edits), "> onetwo 3");
    }

    #[test]
    fn adjacent_edits_both_apply() {
        let edits = [Edit::replace(0..1, "A"), Edit::replace(1..2, "B")];
        assert_eq!(apply_edits("xy", &edits), "AB");
    }

    #[test]
    fn multibyte_utf8_replace_and_insert() {
        // "é" is 2 bytes, "日" is 3 bytes, "🙂" is 4 bytes.
        let text = "é日🙂!";
        assert_eq!(text.len(), 2 + 3 + 4 + 1);
        let edits = [
            Edit::replace(2..5, "本"), // replace 日
            Edit::insert(9, "?"),      // insert before "!"
        ];
        assert_eq!(apply_edits(text, &edits), "é本🙂?!");
        assert_eq!(apply_edits(text, &[Edit::delete(5..9)]), "é日!");
    }

    #[test]
    fn edit_constructors() {
        assert_eq!(
            Edit::insert(2, "x"),
            Edit {
                range: 2..2,
                text: "x".into()
            }
        );
        assert_eq!(
            Edit::delete(1..3),
            Edit {
                range: 1..3,
                text: String::new()
            }
        );
        assert_eq!(
            Edit::replace(1..3, "y"),
            Edit {
                range: 1..3,
                text: "y".into()
            }
        );
    }
}
