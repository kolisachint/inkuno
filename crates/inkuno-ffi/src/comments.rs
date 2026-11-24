//! The comments of a note: `passage<!-- comment: text -->`, read, written, and listed for the
//! Comments panel (see `inkuno_logic::markdown`).

use inkuno_logic::markdown;

use crate::text::offset_utf16;

/// A comment of the note, as its panel lists it (see `inkuno_logic::markdown::Note`).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct CommentInfo {
    /// The commented text; empty for a comment on its own.
    pub noted: String,
    /// What is said, without the name of the row or column it starts with.
    pub said: String,
    /// Where the whole comment starts (its passage, or the comment on its own).
    pub start_utf16: u64,
    /// Where the comment's text starts, after `<!-- comment: `.
    pub inside_utf16: u64,
    /// Where the whole comment ends.
    pub end_utf16: u64,
    /// In a table: `Cell C2`, `Row Léa`, `Column Âge`, `Table`; empty elsewhere.
    pub scope: String,
}

/// Why a comment cannot be written (see `inkuno_logic::markdown::comment_on`).
#[derive(uniffi::Error, Debug, Clone, PartialEq, Eq)]
pub enum CommentError {
    Refused { message: String },
}

impl std::fmt::Display for CommentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Refused { message } => f.write_str(message),
        }
    }
}

/// The comments of `text`, outside code blocks, in order.
#[uniffi::export]
pub fn all_comments(text: String) -> Vec<CommentInfo> {
    markdown::all_comments(&text)
        .into_iter()
        .map(|note| CommentInfo {
            noted: note.noted.to_string(),
            said: note.said.to_string(),
            start_utf16: offset_utf16(&text, note.at),
            inside_utf16: offset_utf16(&text, note.inside),
            end_utf16: offset_utf16(&text, note.end),
            scope: note.scope,
        })
        .collect()
}

/// `text` with a comment written after `passage` (the passage is found once in its line). See
/// `inkuno_logic::markdown::comment_on`: the comment is spaced so that it cannot end early.
#[uniffi::export]
pub fn comment_on(text: String, passage: String, comment: String) -> Result<String, CommentError> {
    markdown::comment_on(&text, &passage, &comment)
        .map_err(|message| CommentError::Refused { message })
}

/// A comment on its own line, `<!-- comment: text -->` (see `inkuno_logic::markdown::comment_line`).
#[uniffi::export]
pub fn comment_line(text: String) -> String {
    markdown::comment_line(&text)
}

/// The text of a line that is a comment on its own, if it is one.
#[uniffi::export]
pub fn comment_line_text(line: String) -> Option<String> {
    markdown::comment_line_text(&line).map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_are_listed_in_utf16_with_their_passage() {
        let text = "é lait<!-- comment: check --> fin";
        let found = all_comments(text.into());
        assert_eq!(found.len(), 1);
        // The passage starts at the line's start: "é lait" is 6 units, the marker 13 more.
        assert_eq!(found[0].noted, "é lait");
        assert_eq!(found[0].said, "check");
        assert_eq!(found[0].start_utf16, 0);
        assert_eq!(found[0].inside_utf16, 20);
        assert_eq!(found[0].end_utf16, 29);
        assert_eq!(found[0].scope, "");
    }

    #[test]
    fn a_comment_is_written_after_its_passage() {
        assert_eq!(
            comment_on("one two\n".into(), "two".into(), "vague".into()).unwrap(),
            "one two<!-- comment: vague -->\n"
        );
    }

    #[test]
    fn a_refused_comment_says_why() {
        let refused = comment_on("one\n".into(), "one".into(), "  ".into()).unwrap_err();
        assert_eq!(refused.to_string(), "the comment is empty");
        assert!(comment_on("one\n".into(), "absent".into(), "x".into()).is_err());
    }

    #[test]
    fn a_comment_on_its_own_line_round_trips() {
        let line = comment_line("reread this".into());
        assert_eq!(line, "<!-- comment: reread this -->");
        assert_eq!(comment_line_text(line).as_deref(), Some("reread this"));
        assert_eq!(comment_line_text("plain".into()), None);
    }
}
