//! The comments of a note: `passage<!-- comment @author 2026-11-24T10:02Z: text ↳ @author time: reply -->`, read,
//! written, answered and resolved (see `inkuno_logic::markdown`). The legacy form `passage<!-- comment: text -->`
//! reads with no author and no time.

use inkuno_logic::markdown::{self, Stamp};

use crate::text::{byte_offset, offset_utf16};

/// A comment of the note, as its panel lists it (see `inkuno_logic::markdown::Note`).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct CommentInfo {
    /// The commented text; empty for a comment on its own.
    pub noted: String,
    /// What is said, without the name of the row or column it starts with.
    pub said: String,
    /// Where the whole comment starts (its passage, or the comment on its own).
    pub start_utf16: u64,
    /// Where the comment's text starts, after `<!-- comment `.
    pub inside_utf16: u64,
    /// Where the whole comment ends.
    pub end_utf16: u64,
    /// In a table: `Cell C2`, `Row Léa`, `Column Âge`, `Table`; empty elsewhere.
    pub scope: String,
    /// Who wrote the comment; `None` when unknown (`Unknown`, or the legacy form).
    #[uniffi(default = None)]
    pub author: Option<String>,
    /// When it was written, in UTC to the minute (`2026-11-24T10:02Z`); `None` in the legacy form.
    #[uniffi(default = None)]
    pub time: Option<String>,
    /// The replies to the comment, in order.
    #[uniffi(default = [])]
    pub replies: Vec<ReplyInfo>,
}

/// A reply to a comment (see `inkuno_logic::markdown::Reply`).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct ReplyInfo {
    /// Who wrote the reply; `None` when unknown.
    pub author: Option<String>,
    /// When it was written, in UTC to the minute; `None` in the legacy form.
    pub time: Option<String>,
    /// What the reply says.
    pub text: String,
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

fn refused(message: &str) -> CommentError {
    CommentError::Refused {
        message: message.into(),
    }
}

/// The comments of `text`, outside code blocks, in order.
#[uniffi::export]
pub fn all_comments(text: String) -> Vec<CommentInfo> {
    markdown::all_comments(&text)
        .into_iter()
        .map(|note| info(&text, note))
        .collect()
}

/// `text` with a comment written after `passage` (the passage is found once in its line), in the legacy form
/// with no author and no time. See `inkuno_logic::markdown::comment_on`: the comment is spaced so that it
/// cannot end early.
#[uniffi::export]
pub fn comment_on(text: String, passage: String, comment: String) -> Result<String, CommentError> {
    markdown::comment_on(&text, &passage, &comment)
        .map_err(|message| CommentError::Refused { message })
}

/// `text` with a comment by `author`, written at `unix_time`, after `passage` (the passage is found once in
/// its line). See `inkuno_logic::markdown::comment_on_by`: the comment is spaced so that it cannot end early.
/// An empty author reads as `Unknown`.
#[uniffi::export]
pub fn comment_add_by(
    text: String,
    passage: String,
    comment: String,
    author: String,
    unix_time: i64,
) -> Result<String, CommentError> {
    let time = markdown::utc_minute(unix_time);
    markdown::comment_on_by(
        &text,
        &passage,
        &comment,
        Stamp {
            author: &author,
            time: &time,
        },
    )
    .map_err(|message| CommentError::Refused { message })
}

/// `text` with `reply`, by `author` at `unix_time`, added after the replies of the comment that starts at
/// `comment_start_utf16`. The reply is refused when it is empty or no comment starts there.
#[uniffi::export]
pub fn comment_reply(
    text: String,
    comment_start_utf16: u64,
    reply: String,
    author: String,
    unix_time: i64,
) -> Result<String, CommentError> {
    let (start, end) = comment_span(&text, comment_start_utf16)?;
    let markup = markdown::markup_in(&text[start..end]);
    let markup_at = end - markup.len();
    let mut comment =
        markdown::parse_comment(markup).ok_or_else(|| refused("the comment cannot be read"))?;
    let mut flat = markdown::spaced(&reply).trim().to_string();
    if flat.is_empty() {
        return Err(refused("the reply is empty"));
    }
    // In a table cell a `|` of the reply is written `\|`, as the comment itself is.
    let (line_start, line_end) = line_bounds(&text, markup_at);
    if markdown::is_table_line(&text[line_start..line_end]) {
        flat = flat.replace('|', "\\|");
    }
    let time = markdown::utc_minute(unix_time);
    markdown::add_reply(
        &mut comment,
        &flat,
        Stamp {
            author: &author,
            time: &time,
        },
    );
    let mut out = String::with_capacity(text.len() + flat.len() + 64);
    out.push_str(&text[..markup_at]);
    out.push_str(&markdown::format_comment(&comment));
    out.push_str(&text[end..]);
    Ok(out)
}

/// `text` with the comment that starts at `comment_start_utf16` resolved: its markup goes and its passage
/// stays. A comment alone on its line takes the line with it. Undo and the revisions recover it.
#[uniffi::export]
pub fn comment_resolve(text: String, comment_start_utf16: u64) -> Result<String, CommentError> {
    let (_, end) = comment_span(&text, comment_start_utf16)?;
    let (line_start, line_end) = line_bounds(&text, end);
    // Found by its closing mark, not its start: a comment that ends where the next one begins (no passage
    // between them) is then the one resolved.
    let resolved = markdown::resolve(&text[line_start..line_end], end - 1 - line_start)
        .ok_or_else(|| refused("the comment is not on its line"))?;
    if resolved.trim().is_empty() {
        let after = if line_end < text.len() {
            line_end + 1
        } else {
            line_end
        };
        Ok(format!("{}{}", &text[..line_start], &text[after..]))
    } else {
        Ok(format!(
            "{}{}{}",
            &text[..line_start],
            resolved,
            &text[line_end..]
        ))
    }
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

fn info(text: &str, note: markdown::Note<'_>) -> CommentInfo {
    CommentInfo {
        noted: note.noted.to_string(),
        said: note.said.to_string(),
        start_utf16: offset_utf16(text, note.at),
        inside_utf16: offset_utf16(text, note.inside),
        end_utf16: offset_utf16(text, note.end),
        scope: note.scope,
        author: markdown::shown_author(&note.author).map(String::from),
        time: note.time,
        replies: note.replies.into_iter().map(reply_info).collect(),
    }
}

fn reply_info(reply: markdown::Reply) -> ReplyInfo {
    ReplyInfo {
        author: markdown::shown_author(&reply.author).map(String::from),
        time: reply.time,
        text: reply.text,
    }
}

/// The byte range of the comment that starts at UTF-16 offset `at` of `text` (its passage and markup). Refused
/// when no comment outside code starts there.
fn comment_span(text: &str, at: u64) -> Result<(usize, usize), CommentError> {
    let at = byte_offset(text, at);
    markdown::all_comments(text)
        .into_iter()
        .find(|note| note.at == at)
        .map(|note| (note.at, note.end))
        .ok_or_else(|| refused("no comment starts here"))
}

/// The byte range of the line of `text` that holds byte `at`, without its line break.
fn line_bounds(text: &str, at: usize) -> (usize, usize) {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-24T10:02Z and 10:05Z, in Unix seconds.
    const ANA_AT: i64 = 1_795_514_520;
    const BO_AT: i64 = ANA_AT + 180;

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
        // The legacy form has no author, no time and no replies.
        assert_eq!(found[0].author, None);
        assert_eq!(found[0].time, None);
        assert!(found[0].replies.is_empty());
    }

    #[test]
    fn a_comment_lists_its_author_time_and_replies() {
        let text =
            "one<!-- comment @Ana 2026-11-24T10:02Z: vague ↳ @Bo 2026-11-24T10:05Z: précis -->\n";
        let found = all_comments(text.into());
        assert_eq!(found[0].author.as_deref(), Some("Ana"));
        assert_eq!(found[0].time.as_deref(), Some("2026-11-24T10:02Z"));
        assert_eq!(found[0].said, "vague");
        assert_eq!(
            found[0].replies,
            [ReplyInfo {
                author: Some("Bo".into()),
                time: Some("2026-11-24T10:05Z".into()),
                text: "précis".into(),
            }]
        );
    }

    #[test]
    fn an_unknown_author_is_not_shown() {
        let text = "one<!-- comment @Unknown 2026-11-24T10:02Z: vague -->\n";
        assert_eq!(all_comments(text.into())[0].author, None);
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
    fn a_comment_by_an_author_is_stamped_in_utc() {
        let text = comment_add_by(
            "one two\n".into(),
            "two".into(),
            "vague".into(),
            "Ana".into(),
            ANA_AT,
        )
        .unwrap();
        assert_eq!(
            text,
            "one two<!-- comment @Ana 2026-11-24T10:02Z: vague -->\n"
        );
        let found = all_comments(text);
        assert_eq!(found[0].author.as_deref(), Some("Ana"));
        assert_eq!(found[0].time.as_deref(), Some("2026-11-24T10:02Z"));
    }

    #[test]
    fn a_comment_by_an_accented_author_keeps_its_name_and_text() {
        let text = comment_add_by(
            "Crème fraîche\n".into(),
            "fraîche".into(),
            "très bon".into(),
            "Léa Ö".into(),
            ANA_AT,
        )
        .unwrap();
        let found = all_comments(text);
        // The passage runs from the start of the line up to the marker.
        assert_eq!(found[0].noted, "Crème fraîche");
        assert_eq!(found[0].said, "très bon");
        assert_eq!(found[0].author.as_deref(), Some("Léa Ö"));
    }

    #[test]
    fn a_comment_by_an_empty_author_reads_as_unknown() {
        let text = comment_add_by(
            "one\n".into(),
            "one".into(),
            "x".into(),
            "  ".into(),
            ANA_AT,
        )
        .unwrap();
        assert!(text.contains("@Unknown 2026-11-24T10:02Z: x"));
        assert_eq!(all_comments(text)[0].author, None);
    }

    #[test]
    fn a_refused_comment_by_an_author_says_why() {
        let refused = comment_add_by(
            "one\n".into(),
            "one".into(),
            " ".into(),
            "Ana".into(),
            ANA_AT,
        );
        assert_eq!(refused.unwrap_err().to_string(), "the comment is empty");
    }

    #[test]
    fn a_reply_is_added_after_the_comments_replies() {
        let text = "one two<!-- comment @Ana 2026-11-24T10:02Z: vague -->\n".to_string();
        let reply = comment_reply(text, 0, "fine, merci".into(), "Bo".into(), BO_AT).unwrap();
        assert_eq!(
            reply,
            "one two<!-- comment @Ana 2026-11-24T10:02Z: vague ↳ @Bo 2026-11-24T10:05Z: fine, merci -->\n"
        );
        let found = all_comments(reply);
        assert_eq!(found[0].replies.len(), 1);
        assert_eq!(found[0].replies[0].author.as_deref(), Some("Bo"));
        assert_eq!(found[0].replies[0].text, "fine, merci");
    }

    #[test]
    fn a_reply_after_a_non_ascii_line_is_found_by_its_utf16_offset() {
        // "é\n" is two UTF-16 units but three bytes: the comment starts at unit 2, byte 3.
        let text = "é\nété<!-- comment @Ana 2026-11-24T10:02Z: vu -->\n".to_string();
        let reply = comment_reply(text, 2, "oui".into(), "Léa".into(), BO_AT).unwrap();
        let found = all_comments(reply);
        assert_eq!(found[0].noted, "été");
        assert_eq!(found[0].replies[0].author.as_deref(), Some("Léa"));
        assert_eq!(found[0].replies[0].text, "oui");
    }

    #[test]
    fn a_reply_with_a_literal_arrow_is_escaped_and_read_back() {
        let text = "one<!-- comment @Ana 2026-11-24T10:02Z: vague -->\n".to_string();
        let reply = comment_reply(text, 0, "a ↳ b".into(), "Bo".into(), BO_AT).unwrap();
        assert!(reply.contains("\\↳"));
        assert_eq!(all_comments(reply)[0].replies[0].text, "a ↳ b");
    }

    #[test]
    fn a_reply_to_a_legacy_comment_keeps_the_legacy_text() {
        let text = "one<!-- comment: vague -->\n".to_string();
        let reply = comment_reply(text, 0, "fine".into(), "Bo".into(), BO_AT).unwrap();
        let found = all_comments(reply);
        assert_eq!(found[0].said, "vague");
        assert_eq!(found[0].author, None);
        assert_eq!(found[0].replies[0].text, "fine");
    }

    #[test]
    fn a_reply_in_a_table_cell_escapes_its_bars() {
        let text = "| A | B |\n| - | - |\n| x<!-- comment @Ana 2026-11-24T10:02Z: vu --> | y |\n"
            .to_string();
        let start = text.find("x<!--").unwrap() as u64;
        let reply = comment_reply(text, start, "a | b".into(), "Bo".into(), BO_AT).unwrap();
        assert!(reply.contains("a \\| b"));
        assert_eq!(reply.lines().nth(2).unwrap().matches('|').count(), 3 + 1);
    }

    #[test]
    fn an_empty_reply_or_a_missing_comment_is_refused() {
        let text = "one<!-- comment @Ana 2026-11-24T10:02Z: vague -->\n".to_string();
        assert_eq!(
            comment_reply(text.clone(), 0, "  ".into(), "Bo".into(), BO_AT)
                .unwrap_err()
                .to_string(),
            "the reply is empty"
        );
        assert!(comment_reply(text, 1, "x".into(), "Bo".into(), BO_AT).is_err());
        assert!(comment_reply("plain\n".into(), 0, "x".into(), "Bo".into(), BO_AT).is_err());
    }

    #[test]
    fn a_comment_inside_code_cannot_be_answered() {
        let text = "```\ncode<!-- comment @Ana 2026-11-24T10:02Z: x -->\n```\n".to_string();
        let start = offset_utf16(&text, text.find("code<!--").unwrap());
        assert!(comment_reply(text, start, "y".into(), "Bo".into(), BO_AT).is_err());
    }

    #[test]
    fn a_resolved_comment_keeps_its_passage() {
        let text = "Ship it<!-- comment @Ana 2026-11-24T10:02Z: Keep it. -->\nnext\n".to_string();
        assert_eq!(comment_resolve(text, 0).unwrap(), "Ship it\nnext\n");
    }

    #[test]
    fn a_resolved_comment_alone_on_its_line_takes_the_line() {
        let text = "keep\n<!-- comment @Ana 2026-11-24T10:02Z: whole line -->\nend\n".to_string();
        assert_eq!(comment_resolve(text, 5).unwrap(), "keep\nend\n");
    }

    #[test]
    fn a_resolved_comment_after_a_non_ascii_passage() {
        let text = "é\nété<!-- comment @Ana 2026-11-24T10:02Z: vu -->\n".to_string();
        assert_eq!(comment_resolve(text, 2).unwrap(), "é\nété\n");
    }

    #[test]
    fn of_two_adjacent_comments_only_the_one_named_is_resolved() {
        let text = "a<!-- comment: one --><!-- comment: two -->\n".to_string();
        let second = offset_utf16(&text, text.find("<!-- comment: two").unwrap());
        assert_eq!(
            comment_resolve(text, second).unwrap(),
            "a<!-- comment: one -->\n"
        );
    }

    #[test]
    fn resolving_a_missing_comment_is_refused() {
        assert_eq!(
            comment_resolve("plain\n".into(), 0)
                .unwrap_err()
                .to_string(),
            "no comment starts here"
        );
    }

    #[test]
    fn a_comment_on_its_own_line_round_trips() {
        let line = comment_line("reread this".into());
        assert_eq!(line, "<!-- comment: reread this -->");
        assert_eq!(comment_line_text(line).as_deref(), Some("reread this"));
        assert_eq!(comment_line_text("plain".into()), None);
    }
}
