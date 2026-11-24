//! The assistant's words: the prompts sent to the agent, the findings read back from its
//! answer, and the placing of those findings in a note. Pure text: the process that carries
//! them lives in `platform/assistant.rs`.

use std::ops::Range;

use crate::markdown;

/// The words sent with a question about the open note.
pub fn chat_prompt(note: Option<&str>, question: &str) -> String {
    let opening = "You are the assistant inside Inkuno, a note app for Markdown.";
    let file = match note {
        Some(note) => format!(
            " The note the person is looking at is the file `{note}`: read it before you answer."
        ),
        None => String::new(),
    };
    format!("{opening}{file} Do not change any file. Answer briefly, in plain text.\n\n{question}")
}

/// The name the assistant writes its comments and replies with. The listing of [`open_comments`]
/// leaves out a comment whose last message is by it, so the agent does not answer itself.
pub const AUTHOR: &str = "assistant";

/// A comment of the note that a review may answer: where its passage starts, its markup (to find it
/// again), and what it says, as the prompt lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Open {
    /// The byte where the comment's passage starts in the note as it was read.
    pub at: usize,
    /// The whole of the comment in that note: passage and markup, `text[at..end]`.
    pub whole: String,
    /// The commented text; empty for a comment on its own.
    pub passage: String,
    /// The comment as its markup says it: its author, its text and its replies.
    pub comment: markdown::Comment,
}

/// The comments of `text` a review may answer: those whose last message (the last reply, else the
/// comment) is not by [`AUTHOR`], in the note's order. The prompt numbers them from 1, and a reply
/// names its comment by that number.
pub fn open_comments(text: &str) -> Vec<Open> {
    markdown::all_comments(text)
        .into_iter()
        .filter_map(|note| {
            let whole = &text[note.at..note.end];
            let comment = markdown::parse_comment(markdown::markup_in(whole))?;
            let last = match comment.replies.last() {
                Some(reply) => reply.author.as_deref(),
                None => comment.author.as_deref(),
            };
            if last.is_some_and(|by| by.eq_ignore_ascii_case(AUTHOR)) {
                return None;
            }
            Some(Open {
                at: note.at,
                whole: whole.to_string(),
                passage: note.noted.to_string(),
                comment,
            })
        })
        .collect()
}

/// One line of the prompt's list of open comments: `C1 on "passage" — @name: text ↳ @other: reply`.
fn listing_line(number: usize, open: &Open) -> String {
    let said = |author: Option<&str>, text: &str| {
        let owned = author.map(str::to_string);
        let name = markdown::shown_author(&owned).unwrap_or("Unknown");
        format!("@{name}: {text}")
    };
    let subject = match open.passage.is_empty() {
        true => "alone on its line".to_string(),
        false => format!("on \"{}\"", open.passage),
    };
    let mut line = format!(
        "C{number} {subject} — {}",
        said(open.comment.author.as_deref(), &open.comment.text)
    );
    for reply in &open.comment.replies {
        line.push_str(" ↳ ");
        line.push_str(&said(reply.author.as_deref(), &reply.text));
    }
    line
}

/// The words that ask for a review of `note` (a path relative to the agent's working folder,
/// or absolute), with the comments of the note it may answer (`open`, see [`open_comments`]). The
/// answer is read back by [`findings`].
pub fn review_prompt(note: &str, open: &[Open]) -> String {
    let (listed, reply) = match open.is_empty() {
        true => (String::new(), String::new()),
        false => {
            let lines: Vec<String> = open
                .iter()
                .enumerate()
                .map(|(i, open)| listing_line(i + 1, open))
                .collect();
            (
                format!(
                    "The note already has these comments, numbered. You may answer a question or \
                     a remark in one of them, once, with a reply:\n{}\n\n",
                    lines.join("\n")
                ),
                "To answer a comment, give the two lines `REPLY: C<number>` and `COMMENT: <the \
                 reply, one line>`, only where the reply adds something.\n"
                    .to_string(),
            )
        }
    };
    format!(
        "You are the assistant inside Inkuno, a note app for Markdown.\n\
         \n\
         Read the file `{note}` and review it as a careful editor. Look for statements that are \
         unclear or wrong, gaps, contradictions, and typos that change the meaning. Give at most \
         12 findings, the most useful first. No praise. Write the comments in the language of the \
         note. Do not change any file.\n\
         \n\
         {listed}\
         Answer with NOTHING but findings, one after another, each in exactly this form, with no \
         text before, between or after them:\n\
         PASSAGE: <text copied exactly from ONE line of the note, short (3 to 12 words), unique \
         in the note, no Markdown markup added>\n\
         COMMENT: <the remark, one line>\n\
         {reply}\
         \n\
         When there is nothing to say, answer with the single line NO FINDINGS."
    )
}

/// One thing a review answer asks for: a new comment on a passage of the note, or a reply to a
/// comment the prompt listed (`id` is its number, from 1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    New { passage: String, comment: String },
    Reply { id: usize, text: String },
}

/// The findings in a review answer, in order.
///
/// Tolerant: a `PASSAGE:` line opens a new comment and a `REPLY:` line a reply; the `COMMENT:` line
/// after either closes it. A reply's number is read as `C3`, `c3` or `3`. A comment may go on over
/// the following lines until a blank line, a code fence or the next label. Labels may come with a
/// list mark, bold or backticks around them. Text before, between and after the findings is
/// ignored, and so is a finding with an empty part.
pub fn findings(answer: &str) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut open: Option<Partial> = None;
    for line in answer.lines() {
        let line = line.trim();
        if let Some(value) = label(line, "PASSAGE") {
            finish(&mut open, &mut found);
            open = Some(Partial {
                head: Head::Passage(value.to_string()),
                comment: None,
            });
        } else if let Some(value) = label(line, "REPLY") {
            finish(&mut open, &mut found);
            open = comment_number(value).map(|id| Partial {
                head: Head::Reply(id),
                comment: None,
            });
        } else if let Some(value) = label(line, "COMMENT") {
            if let Some(open) = open.as_mut().filter(|open| open.comment.is_none()) {
                open.comment = Some(value.to_string());
            }
        } else if line.is_empty() || markdown::is_fence(line) {
            // The comment ends here; a finding still waiting for its comment stays open.
            if open.as_ref().is_some_and(|open| open.comment.is_some()) {
                finish(&mut open, &mut found);
            }
        } else if let Some(comment) = open.as_mut().and_then(|open| open.comment.as_mut()) {
            comment.push(' ');
            comment.push_str(line);
        }
    }
    finish(&mut open, &mut found);
    found
}

/// What a finding read so far asks for: a passage, or the number of the comment it replies to.
enum Head {
    Passage(String),
    Reply(usize),
}

/// A finding read so far: its head, and its comment once the `COMMENT:` line came.
struct Partial {
    head: Head,
    comment: Option<String>,
}

/// Moves the open finding into `found` when both of its parts are there and it is not empty.
fn finish(open: &mut Option<Partial>, found: &mut Vec<Finding>) {
    let Some(Partial {
        head,
        comment: Some(comment),
    }) = open.take()
    else {
        return;
    };
    let comment = comment.trim();
    if comment.is_empty() {
        return;
    }
    match head {
        Head::Passage(passage) if !passage.is_empty() => found.push(Finding::New {
            passage,
            comment: comment.to_string(),
        }),
        Head::Reply(id) => found.push(Finding::Reply {
            id,
            text: comment.to_string(),
        }),
        Head::Passage(_) => {}
    }
}

/// The number a reply names: `C3`, `c3` or `3`, with any bold or backticks around it. `None` when
/// there is no number.
fn comment_number(value: &str) -> Option<usize> {
    let value = value
        .trim()
        .trim_matches(['*', '`', ' ', '\t'])
        .trim_start_matches(['C', 'c']);
    let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok().filter(|&n| n > 0)
}

/// The value after `name:` when `line` starts with that label, whatever is around the label
/// (list marks, bold, backticks), in any letter case.
fn label<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let rest = line.trim_start_matches([' ', '\t', '-', '*', '`']);
    let head = rest.get(..name.len())?;
    if !head.eq_ignore_ascii_case(name) {
        return None;
    }
    // Bold or backticks may close the label before the colon: `**PASSAGE:**`, `` `COMMENT:` ``.
    let rest = rest[name.len()..].trim_start_matches(['*', '`']);
    let value = rest.strip_prefix(':')?.trim_start_matches('*').trim();
    // A backtick left over from the label (`` `PASSAGE:` text ``) is not a quote.
    let value = match value.strip_prefix('`') {
        Some(stripped) if !(value.len() > 1 && value.ends_with('`')) => stripped,
        _ => value,
    };
    Some(value.trim())
}

/// A passage without the one pair of quotes or backticks around it.
fn unquote(value: &str) -> String {
    let value = value.trim();
    for quote in ['`', '"', '\''] {
        if let Some(inner) = value
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_suffix(quote))
        {
            return inner.trim().to_string();
        }
    }
    value.to_string()
}

/// What placing the findings in a note gave: the new text, how many new comments and how many replies
/// were placed, and how many findings were left out.
#[derive(Debug, PartialEq, Eq)]
pub struct Placed {
    pub text: String,
    pub placed: usize,
    pub replied: usize,
    pub skipped: usize,
}

/// Writes each finding into `text`: the replies first, then the new comments in order.
///
/// A reply goes to the comment `listed` numbers (see [`open_comments`] on the text it was read from).
/// It is written only where that comment still starts at the same byte with the same markup; a reply
/// to a number that is not listed, or to a comment that has changed, is left out. The replies go
/// from the last comment to the first, so that the comments before each one keep their places.
///
/// A new comment wraps its passage with `markdown::comment_on_by`. The passage is tried as written
/// first, then without the one pair of quotes or backticks around it (a model may add them). It is
/// left out when neither form is in the text exactly once, or when it lies inside a comment that is
/// already there (a second finding on the same words would nest in the first).
pub fn place(text: &str, found: &[Finding], listed: &[Open], by: markdown::Stamp) -> Placed {
    let mut out = Placed {
        text: text.to_string(),
        placed: 0,
        replied: 0,
        skipped: 0,
    };
    // The replies of each listed comment, in their order; a number that is not listed is skipped.
    let mut targets: Vec<(usize, Vec<&str>)> = Vec::new();
    for finding in found {
        let Finding::Reply { id, text } = finding else {
            continue;
        };
        match id.checked_sub(1).filter(|&i| i < listed.len()) {
            Some(i) => match targets.iter_mut().find(|(j, _)| *j == i) {
                Some((_, texts)) => texts.push(text),
                None => targets.push((i, vec![text])),
            },
            None => out.skipped += 1,
        }
    }
    targets.sort_by_key(|(i, _)| std::cmp::Reverse(listed[*i].at));
    for (i, texts) in targets {
        let open = &listed[i];
        let at = markdown::all_comments(&out.text)
            .iter()
            .find(|note| note.at == open.at && out.text[note.at..note.end] == open.whole)
            .map(|note| note.at);
        for text in texts {
            match at.and_then(|at| markdown::reply_on(&out.text, at, text, by)) {
                Some(next) => {
                    out.text = next;
                    out.replied += 1;
                }
                None => out.skipped += 1,
            }
        }
    }
    for finding in found {
        let Finding::New { passage, comment } = finding else {
            continue;
        };
        let unquoted = unquote(passage);
        let placed = [passage.as_str(), unquoted.as_str()]
            .into_iter()
            .find_map(|form| {
                let inside = out.text.find(form).is_some_and(|at| {
                    markdown::all_comments(&out.text)
                        .iter()
                        .any(|note| note.at <= at && at < note.end)
                });
                match markdown::comment_on_by(&out.text, form, comment, by) {
                    Ok(next) if !inside => Some(next),
                    _ => None,
                }
            });
        match placed {
            Some(next) => {
                out.text = next;
                out.placed += 1;
            }
            None => out.skipped += 1,
        }
    }
    out
}

/// The one splice that turns `old` into `new`: the range of `old` that differs, and its
/// replacement. `None` when they are the same. Both ends fall on character boundaries.
pub fn edit_between(old: &str, new: &str) -> Option<(Range<usize>, String)> {
    if old == new {
        return None;
    }
    let (a, b) = (old.as_bytes(), new.as_bytes());
    let mut start = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    while !old.is_char_boundary(start) || !new.is_char_boundary(start) {
        start -= 1;
    }
    let room = a.len().min(b.len()) - start;
    let mut tail = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take(room)
        .take_while(|(x, y)| x == y)
        .count();
    while !old.is_char_boundary(old.len() - tail) || !new.is_char_boundary(new.len() - tail) {
        tail -= 1;
    }
    Some((
        start..old.len() - tail,
        new[start..new.len() - tail].to_string(),
    ))
}

/// The instruction of "Improve" when the person typed none.
pub const IMPROVE_INSTRUCTION: &str = "Improve the writing: clearer and tighter, same meaning, same language, same Markdown structure.";

/// The words that ask for an improvement of `note` (a path relative to the agent's working folder,
/// or absolute): of `passage` when there is one, else of the whole note. The answer is read back
/// by [`proposal`].
pub fn improve_prompt(note: &str, passage: Option<&str>, instruction: &str) -> String {
    let (target, given) = match passage {
        Some(passage) => {
            let body = if passage.ends_with('\n') {
                passage.to_string()
            } else {
                format!("{passage}\n")
            };
            (
                "the passage",
                format!(
                    "The passage, copied exactly, between two lines:\nBEGIN PASSAGE\n{body}END PASSAGE\n\n"
                ),
            )
        }
        None => ("the whole note", String::new()),
    };
    format!(
        "You are the assistant inside Inkuno, a note app for Markdown.\n\
         \n\
         Read the file `{note}` first, for its context. Rewrite {target} as this instruction says: \
         {instruction}\n\
         \n\
         Write in the language of the note.\n\
         \n\
         {given}\
         Answer with NOTHING but the new text of {target}, placed between a line `BEGIN TEXT` and a \
         line `END TEXT`:\n\
         BEGIN TEXT\n\
         <the full new text>\n\
         END TEXT\n\
         \n\
         The new text replaces {target} in full, so keep every line that stays. No code fence around \
         it, no comment, no explanation, nothing before or after those two lines. Do not change any \
         file. When nothing should change, answer with the single line `NO CHANGE`."
    )
}

/// Whether `line` is the label `mark`, tolerant: surrounding whitespace, bold or backticks around
/// it, a trailing colon, and any letter case.
fn is_mark(line: &str, mark: &str) -> bool {
    let text = line.trim().trim_matches(['*', '`', ' ', '\t']);
    let text = text
        .strip_suffix(':')
        .unwrap_or(text)
        .trim_end_matches(['*', '`', ' ', '\t']);
    text.eq_ignore_ascii_case(mark)
}

/// The new text in a rewrite's answer: the lines between the first line `BEGIN TEXT` and the last
/// line `END TEXT` (labels tolerant, see [`is_mark`]). One code fence wrapped round the block is
/// dropped. `None` when there is no such block, when the block is empty, or when the answer is
/// `NO CHANGE`.
/// Whether the first line of `text` is a code fence line: three backticks or more once its indent
/// is taken off. The original passage (or note) is checked with this, see [`proposal`].
pub fn starts_with_fence(text: &str) -> bool {
    text.lines().next().is_some_and(markdown::is_fence)
}

/// The block between the first `BEGIN TEXT` and the last `END TEXT` line of `answer`, or `None`
/// when there is no such block, when it is empty, or when it is `NO CHANGE`. A block wrapped in one
/// code fence drops that fence, unless `original_fenced` says the text it replaces starts with a
/// fence line (see [`starts_with_fence`]): a passage that is itself a fenced block keeps its fences.
pub fn proposal(answer: &str, original_fenced: bool) -> Option<String> {
    if is_mark(answer, "NO CHANGE") {
        return None;
    }
    let lines: Vec<&str> = answer.lines().collect();
    let begin = lines.iter().position(|line| is_mark(line, "BEGIN TEXT"))?;
    let end = lines.iter().rposition(|line| is_mark(line, "END TEXT"))?;
    if end <= begin {
        return None;
    }
    let mut inner = &lines[begin + 1..end];
    // Wrapped: the first line opens a fence that the last line closes (same character, as long).
    let wrapped = inner.len() >= 2
        && markdown::fence_of(inner[0])
            .is_some_and(|(fence, _)| markdown::closes(inner[inner.len() - 1], fence));
    if wrapped && !original_fenced {
        inner = &inner[1..inner.len() - 1];
    }
    let text = inner.join("\n");
    let trimmed = text.trim();
    if trimmed.is_empty() || is_mark(trimmed, "NO CHANGE") {
        return None;
    }
    Some(text)
}

/// The whole text of a note once the rewrite `text` (the block of [`proposal`]) is in it. With
/// `range`, the passage of `base` there is replaced; without, the whole note is. The block's
/// leading and trailing line breaks go, and its line breaks take the kind of `base` (CRLF when
/// `base` has CRLF). A passage that ended with a line break keeps one, and so does a note that ended
/// with one.
pub fn rewritten(base: &str, range: Option<Range<usize>>, text: &str) -> String {
    let eol = if base.contains("\r\n") { "\r\n" } else { "\n" };
    let body = text
        .trim_matches(['\r', '\n'])
        .replace("\r\n", "\n")
        .replace('\n', eol);
    match range {
        None => {
            let mut out = body;
            if base.ends_with('\n') {
                out.push_str(eol);
            }
            out
        }
        Some(range) => {
            let breaks = base[range.clone()].ends_with('\n');
            let mut out = String::with_capacity(base.len() + body.len());
            out.push_str(&base[..range.start]);
            out.push_str(&body);
            if breaks {
                out.push_str(eol);
            }
            out.push_str(&base[range.end..]);
            out
        }
    }
}

#[cfg(test)]
mod tests {
    /// `place`, with a fixed author and time for the comments it writes, and no listed comment.
    fn place_at(text: &str, found: &[Finding]) -> super::Placed {
        super::place(text, found, &[], stamp("Ana"))
    }

    /// `place` with the comments `listed` (as the note `text` lists them), written by the assistant.
    fn place_listed(text: &str, found: &[Finding], listed: &[Open]) -> super::Placed {
        super::place(text, found, listed, stamp(super::AUTHOR))
    }

    /// A stamp by `author` at a fixed minute.
    fn stamp(author: &str) -> crate::markdown::Stamp<'_> {
        crate::markdown::Stamp {
            author,
            time: "2026-11-24T10:02Z",
        }
    }

    use super::*;

    /// A new comment on `passage`, as the agent's answer gives it.
    fn pair(passage: &str, comment: &str) -> Finding {
        Finding::New {
            passage: passage.into(),
            comment: comment.into(),
        }
    }

    /// A reply to the comment numbered `id`.
    fn reply(id: usize, text: &str) -> Finding {
        Finding::Reply {
            id,
            text: text.into(),
        }
    }

    #[test]
    fn chat_prompt_names_the_note_and_keeps_the_question_after_a_blank_line() {
        let prompt = chat_prompt(Some("Projects/Plan.md"), "What is missing?");
        assert!(
            prompt.starts_with("You are the assistant inside Inkuno, a note app for Markdown.")
        );
        assert!(prompt.contains("the file `Projects/Plan.md`: read it before you answer."));
        assert!(prompt.contains("Do not change any file."));
        assert!(prompt.ends_with(".\n\nWhat is missing?"));
    }

    #[test]
    fn chat_prompt_without_a_note_has_no_file_sentence() {
        let prompt = chat_prompt(None, "Hi");
        assert_eq!(
            prompt,
            "You are the assistant inside Inkuno, a note app for Markdown. Do not change any file. \
             Answer briefly, in plain text.\n\nHi"
        );
        assert!(!prompt.contains("looking at"), "no note, no file sentence");
    }

    #[test]
    fn review_prompt_names_the_note_and_the_exact_format() {
        let prompt = review_prompt("Notes/Draft.md", &[]);
        assert!(prompt.contains("`Notes/Draft.md`"));
        assert!(prompt.contains("at most 12 findings"));
        assert!(prompt.contains("PASSAGE: <text copied exactly"));
        assert!(prompt.contains("COMMENT: <the remark, one line>"));
        assert!(prompt.contains("NO FINDINGS"));
        assert!(prompt.contains("Do not change any file."));
        // No comment to answer: no list and no reply form.
        assert!(!prompt.contains("REPLY"));
        assert!(!prompt.contains("already has these comments"));
    }

    #[test]
    fn review_prompt_lists_the_open_comments_numbered_with_the_reply_form() {
        let text = "keep<!-- comment @Ana 2026-11-24T10:05Z: is this final? -->\n\
                    alpha<!-- comment: legacy -->\n";
        let prompt = review_prompt("Plan.md", &open_comments(text));
        assert!(
            prompt.contains("C1 on \"keep\" — @Ana: is this final?\n"),
            "{prompt}"
        );
        assert!(
            prompt.contains("C2 on \"alpha\" — @Unknown: legacy\n"),
            "{prompt}"
        );
        assert!(prompt.contains("`REPLY: C<number>`"));
        assert!(prompt.contains("`COMMENT: <the reply, one line>`"));
    }

    #[test]
    fn a_comment_alone_on_its_line_and_its_replies_are_listed_in_order() {
        let text = "<!-- comment @Ana 2026-11-24T10:05Z: a note to self ↳ @Bo 2026-11-24T10:06Z: me too -->\n";
        let listing = review_prompt("Plan.md", &open_comments(text));
        assert!(
            listing.contains("C1 alone on its line — @Ana: a note to self ↳ @Bo: me too\n"),
            "{listing}"
        );
    }

    #[test]
    fn open_comments_leave_out_the_ones_the_assistant_answered_last() {
        let text = "a<!-- comment @Ana 2026-11-24T10:05Z: one -->\n\
                    b<!-- comment @Ana 2026-11-24T10:05Z: two ↳ @assistant 2026-11-24T10:06Z: three -->\n\
                    c<!-- comment @assistant 2026-11-24T10:07Z: four -->\n\
                    d<!-- comment @Ana 2026-11-24T10:08Z: five -->\n";
        let open = open_comments(text);
        let passages: Vec<&str> = open.iter().map(|o| o.passage.as_str()).collect();
        assert_eq!(passages, ["a", "d"], "the agent does not answer itself");
        assert_eq!(
            open[0].whole,
            "a<!-- comment @Ana 2026-11-24T10:05Z: one -->"
        );
    }

    #[test]
    fn the_numbers_of_replies_are_read_in_tolerant_forms() {
        let answer = "REPLY: C2\nCOMMENT: yes\n\
                      reply: c3\ncomment: no\n\
                      - **REPLY:** `4`\n  **COMMENT:** maybe\n\
                      REPLY: 0\nCOMMENT: never\n\
                      REPLY: nothing\nCOMMENT: never either";
        assert_eq!(
            findings(answer),
            vec![reply(2, "yes"), reply(3, "no"), reply(4, "maybe"),]
        );
    }

    #[test]
    fn a_reply_and_a_new_finding_mix_in_one_answer() {
        let answer = "REPLY: C1\nCOMMENT: agreed\nPASSAGE: ships in March\nCOMMENT: Which year?\n\
                      REPLY: C1\nCOMMENT: and a second thought";
        assert_eq!(
            findings(answer),
            vec![
                reply(1, "agreed"),
                pair("ships in March", "Which year?"),
                reply(1, "and a second thought"),
            ]
        );
    }

    #[test]
    fn a_plain_answer_gives_its_pairs_in_order() {
        let answer = "PASSAGE: the cache is cleared daily\nCOMMENT: Daily when? Say the hour.\n\
                      PASSAGE: ships in March\nCOMMENT: Which year?";
        assert_eq!(
            findings(answer),
            vec![
                pair("the cache is cleared daily", "Daily when? Say the hour."),
                pair("ships in March", "Which year?"),
            ]
        );
    }

    #[test]
    fn no_findings_gives_nothing() {
        assert!(findings("NO FINDINGS").is_empty());
        assert!(findings("").is_empty());
    }

    #[test]
    fn labels_are_found_in_any_case_and_with_marks_around_them() {
        let answer = "- **PASSAGE:** first line\n  **COMMENT:** a typo\n\
                      * `Passage:` second line\n`comment:` another";
        assert_eq!(
            findings(answer),
            vec![pair("first line", "a typo"), pair("second line", "another")]
        );
    }

    #[test]
    fn a_comment_continues_on_the_following_lines_until_a_blank_line() {
        let answer = "PASSAGE: Mars is cold\nCOMMENT: This is wrong\nthe mean is 210 K,\nso say so.\n\n\
                      Thanks.\nPASSAGE: Earth\nCOMMENT: Fine";
        assert_eq!(
            findings(answer),
            vec![
                pair(
                    "Mars is cold",
                    "This is wrong the mean is 210 K, so say so."
                ),
                pair("Earth", "Fine"),
            ]
        );
    }

    #[test]
    fn a_comment_also_ends_at_the_next_passage() {
        let answer = "PASSAGE: one\nCOMMENT: first\nPASSAGE: two\nCOMMENT: second";
        assert_eq!(
            findings(answer),
            vec![pair("one", "first"), pair("two", "second")]
        );
    }

    #[test]
    fn findings_hand_over_the_passage_as_written() {
        // The quotes are not stripped here: `place` decides whether they belong to the note.
        let answer = "PASSAGE: \"quoted words here\"\nCOMMENT: a\n\
                      PASSAGE: `code words`\nCOMMENT: b\n\
                      PASSAGE: 'single quoted'\nCOMMENT: c\n\
                      PASSAGE: ``double``\nCOMMENT: d";
        assert_eq!(
            findings(answer),
            vec![
                pair("\"quoted words here\"", "a"),
                pair("`code words`", "b"),
                pair("'single quoted'", "c"),
                pair("``double``", "d"),
            ]
        );
    }

    #[test]
    fn a_passage_in_quotes_is_placed_as_written_or_without_them() {
        let quoted = [pair("\"quoted words\"", "as written")];
        // The note has the quotes: the passage as written is placed.
        let as_written = place_at("say \"quoted words\" now\n", &quoted);
        assert_eq!(as_written.placed, 1);
        assert_eq!(
            as_written.text,
            "say \"quoted words\"<!-- comment @Ana 2026-11-24T10:02Z: as written --> now\n"
        );
        // The note has none: the passage without its quotes is placed.
        let bare = place_at("say quoted words now\n", &quoted);
        assert_eq!(bare.placed, 1);
        assert_eq!(
            bare.text,
            "say quoted words<!-- comment @Ana 2026-11-24T10:02Z: as written --> now\n"
        );
        // Both are in the note: the one as written wins.
        let both = place_at("\"quoted words\" and quoted words\n", &quoted);
        assert_eq!(
            both.text,
            "\"quoted words\"<!-- comment @Ana 2026-11-24T10:02Z: as written --> and quoted words\n"
        );
        // Backticks follow the same rule.
        let code = place_at("run `ls` now\n", &[pair("`ls`", "a command")]);
        assert_eq!(
            code.text,
            "run `ls`<!-- comment @Ana 2026-11-24T10:02Z: a command --> now\n"
        );
        let plain = place_at("run ls now\n", &[pair("`ls`", "a command")]);
        assert_eq!(
            plain.text,
            "run ls<!-- comment @Ana 2026-11-24T10:02Z: a command --> now\n"
        );
        // Neither form is in the note: the finding is left out.
        let missing = place_at("nothing here\n", &quoted);
        assert_eq!((missing.placed, missing.skipped), (0, 1));
    }

    #[test]
    fn pairs_with_an_empty_part_are_dropped() {
        let answer = "PASSAGE:\nCOMMENT: no passage\n\
                      PASSAGE: no comment\n\
                      PASSAGE: kept\nCOMMENT:\n\
                      PASSAGE: kept too\nCOMMENT: yes";
        assert_eq!(findings(answer), vec![pair("kept too", "yes")]);
    }

    #[test]
    fn text_before_between_and_after_is_ignored() {
        let answer = "Here are my findings.\n\nSome preamble\nPASSAGE: one\nCOMMENT: first\n\n\
                      An aside between them.\n\nPASSAGE: two\nCOMMENT: second\n\nThat is all.";
        assert_eq!(
            findings(answer),
            vec![pair("one", "first"), pair("two", "second")]
        );
    }

    #[test]
    fn a_comment_without_its_passage_is_ignored() {
        assert!(findings("COMMENT: alone\nthe rest").is_empty());
    }

    #[test]
    fn a_comment_ends_at_a_tilde_fence_too() {
        // A tilde wrapper is a fence: its closing line is not part of the comment.
        let answer = "~~~text\nPASSAGE: the bridge opens in May\nCOMMENT: Which year?\n~~~";
        assert_eq!(
            findings(answer),
            vec![pair("the bridge opens in May", "Which year?")]
        );
    }

    #[test]
    fn an_answer_wrapped_in_a_code_fence_is_read() {
        let answer = "```text\nPASSAGE: the bridge opens in May\nCOMMENT: Which year?\n```";
        assert_eq!(
            findings(answer),
            vec![pair("the bridge opens in May", "Which year?")]
        );
        let fenced = "```\nPASSAGE: a\nCOMMENT: b\n```\n";
        assert_eq!(findings(fenced), vec![pair("a", "b")]);
    }

    #[test]
    fn findings_are_placed_in_order_and_the_others_are_counted() {
        let text = "# T\n\nthe cache is cleared daily\nships in March\nalpha and alpha\n";
        let found = [
            pair("the cache is cleared daily", "Daily when?"),
            pair("alpha", "twice in the note"),
            pair("nowhere", "not here"),
            pair("ships in March", "Which year?"),
        ];
        let placed = place_at(text, &found);
        assert_eq!(placed.placed, 2);
        assert_eq!((placed.replied, placed.skipped), (0, 2));
        assert_eq!(
            placed.text,
            "# T\n\nthe cache is cleared daily<!-- comment @Ana 2026-11-24T10:02Z: Daily when? -->\n\
             ships in March<!-- comment @Ana 2026-11-24T10:02Z: Which year? -->\nalpha and alpha\n"
        );
    }

    #[test]
    fn a_reply_is_written_on_the_line_of_its_listed_comment() {
        let text = "keep<!-- comment @Ana 2026-11-24T10:05Z: is this final? -->\n\
                    alpha<!-- comment: legacy -->\n\
                    end of the note\n";
        let listed = open_comments(text);
        let found = [
            reply(1, "not yet"),
            reply(2, "say so"),
            pair("end of the note", "a new remark"),
        ];
        let placed = place_listed(text, &found, &listed);
        assert_eq!((placed.placed, placed.replied, placed.skipped), (1, 2, 0));
        assert_eq!(
            placed.text,
            "keep<!-- comment @Ana 2026-11-24T10:05Z: is this final? ↳ @assistant 2026-11-24T10:02Z: not yet -->\n\
             alpha<!-- comment: legacy ↳ @assistant 2026-11-24T10:02Z: say so -->\n\
             end of the note<!-- comment @assistant 2026-11-24T10:02Z: a new remark -->\n"
        );
    }

    #[test]
    fn several_replies_to_one_comment_go_after_each_other_in_order() {
        let text = "alpha<!-- comment @Ana 2026-11-24T10:05Z: first -->\n";
        let listed = open_comments(text);
        let placed = place_listed(text, &[reply(1, "one"), reply(1, "two")], &listed);
        assert_eq!(placed.replied, 2);
        assert_eq!(
            placed.text,
            "alpha<!-- comment @Ana 2026-11-24T10:05Z: first ↳ @assistant 2026-11-24T10:02Z: one \
             ↳ @assistant 2026-11-24T10:02Z: two -->\n"
        );
    }

    #[test]
    fn a_reply_to_a_number_not_listed_or_to_a_changed_comment_is_skipped() {
        let text = "alpha<!-- comment @Ana 2026-11-24T10:05Z: first -->\n";
        let listed = open_comments(text);
        // No comment 2 in the listing: skipped, and the other reply still lands.
        let placed = place_listed(text, &[reply(2, "no"), reply(1, "yes")], &listed);
        assert_eq!((placed.replied, placed.skipped), (1, 1));
        // The comment was edited since it was listed: the same number finds no match.
        let edited = "alpha<!-- comment @Ana 2026-11-24T10:05Z: other -->\n";
        let placed = place_listed(edited, &[reply(1, "late")], &listed);
        assert_eq!((placed.replied, placed.skipped), (0, 1));
        assert_eq!(placed.text, edited);
    }

    #[test]
    fn a_passage_inside_a_comment_already_there_is_left_out() {
        let text = "alpha<!-- comment: first --> beta\n";
        let placed = place_at(text, &[pair("alpha", "again")]);
        assert_eq!(placed.placed, 0);
        assert_eq!(placed.skipped, 1);
        assert_eq!(placed.text, text);
    }

    #[test]
    fn the_edit_between_two_texts_is_the_one_changed_span() {
        assert_eq!(edit_between("abc", "abc"), None);
        assert_eq!(
            edit_between("abcdef", "abXYZdef"),
            Some((2..3, "XYZ".into()))
        );
        assert_eq!(edit_between("ab", "abcd"), Some((2..2, "cd".into())));
        assert_eq!(edit_between("abcd", "ab"), Some((2..4, "".into())));
        // Multi-byte characters: the edit never splits one.
        let (range, text) = edit_between("café!", "cafè!").unwrap();
        let old = "café!";
        assert_eq!(&old[range.clone()], "é");
        assert_eq!(text, "è");
        assert!(old.is_char_boundary(range.start) && old.is_char_boundary(range.end));
    }

    #[test]
    fn the_improve_prompt_names_the_file_the_instruction_and_the_passage() {
        let prompt = improve_prompt("Notes/Draft.md", Some("un peu de texte"), "Make it formal.");
        assert!(prompt.contains("`Notes/Draft.md`"));
        assert!(prompt.contains("Make it formal."));
        assert!(prompt.contains("BEGIN PASSAGE\nun peu de texte\nEND PASSAGE"));
        assert!(prompt.contains("the passage"));
        assert!(prompt.contains("BEGIN TEXT") && prompt.contains("END TEXT"));
        assert!(prompt.contains("NO CHANGE"));
        assert!(prompt.contains("No code fence"));
        assert!(prompt.contains("Do not change any file."));
    }

    #[test]
    fn the_improve_prompt_of_a_whole_note_has_no_passage_block() {
        let prompt = improve_prompt("Plan.md", None, IMPROVE_INSTRUCTION);
        assert!(prompt.contains("the whole note"));
        assert!(!prompt.contains("BEGIN PASSAGE"));
        assert!(prompt.contains(IMPROVE_INSTRUCTION));
    }

    #[test]
    fn the_improve_prompt_asks_for_the_language_of_the_note() {
        let prompt = improve_prompt("Plan.md", Some("un peu de texte"), IMPROVE_INSTRUCTION);
        assert!(
            prompt.contains("Write in the language of the note."),
            "{prompt}"
        );
        let whole = improve_prompt("Plan.md", None, "Make it formal");
        assert!(whole.contains("Write in the language of the note."));
    }

    #[test]
    fn a_passage_ending_with_a_break_is_not_given_a_second_one() {
        let prompt = improve_prompt("a.md", Some("one\n"), "x");
        assert!(prompt.contains("BEGIN PASSAGE\none\nEND PASSAGE"));
        assert!(!prompt.contains("one\n\nEND PASSAGE"));
    }

    #[test]
    fn the_block_between_the_text_marks_is_the_proposal() {
        assert_eq!(
            proposal("BEGIN TEXT\nUn peu plus clair.\nEND TEXT\n", false),
            Some("Un peu plus clair.".into())
        );
        // Lines inside the block are kept, blank ones too; words around it are ignored.
        assert_eq!(
            proposal(
                "Here it is.\nBEGIN TEXT\nfirst\n\nsecond\nEND TEXT\nThat is all.",
                false
            ),
            Some("first\n\nsecond".into())
        );
    }

    #[test]
    fn the_text_labels_are_tolerant() {
        assert_eq!(
            proposal("**BEGIN TEXT:**\nx\n**END TEXT**", false),
            Some("x".into())
        );
        assert_eq!(
            proposal("`begin text`\nx\n`End Text:`", false),
            Some("x".into())
        );
        assert_eq!(
            proposal("- BEGIN TEXT\nx\n- END TEXT", false),
            None,
            "a list mark is not a label"
        );
        assert_eq!(
            proposal("  BEGIN TEXT  \nx\n  END TEXT  ", false),
            Some("x".into())
        );
    }

    #[test]
    fn a_block_wrapped_in_one_code_fence_drops_the_fence() {
        let answer = "BEGIN TEXT\n```markdown\n# Title\n\nBody.\n```\nEND TEXT";
        assert_eq!(proposal(answer, false), Some("# Title\n\nBody.".into()));
        // A fence inside the text, not around it, is text.
        let inside = "BEGIN TEXT\nSee:\n```\ncode\n```\nEND TEXT";
        assert_eq!(proposal(inside, false), Some("See:\n```\ncode\n```".into()));
    }

    #[test]
    fn a_passage_that_is_a_fenced_block_comes_back_with_its_fences() {
        // The original starts with a fence line: the block is the new passage, fences and all.
        let original = "```md\nold\n```";
        assert!(starts_with_fence(original));
        let answer = "BEGIN TEXT\n```md\nnew\n```\nEND TEXT";
        assert_eq!(
            proposal(answer, true),
            Some("```md\nnew\n```".into()),
            "the fence is kept"
        );
        // The same answer for an unfenced original is wrapped text: the fence goes.
        assert_eq!(proposal(answer, false), Some("new".into()));
    }

    #[test]
    fn an_unfenced_passage_returned_inside_a_wrapper_fence_has_it_stripped() {
        let original = "un peu de texte";
        assert!(!starts_with_fence(original));
        let answer = "BEGIN TEXT\n```\nUn peu plus clair.\n```\nEND TEXT";
        assert_eq!(proposal(answer, false), Some("Un peu plus clair.".into()));
        // The fence check reads the first line only, after its indent.
        assert!(starts_with_fence("  ```rust\nfn main() {}\n```"));
        assert!(!starts_with_fence("text\n```\ncode\n```"));
        assert!(!starts_with_fence(""));
    }

    #[test]
    fn a_wrapper_fence_is_dropped_only_by_its_own_closing_fence() {
        // A tilde line inside a backtick wrapper is text, and the wrapper closes on its own fence.
        let mixed = "BEGIN TEXT\n```md\n~~~\nSee.\n~~~\n```\nEND TEXT";
        assert_eq!(
            proposal(mixed, false),
            Some("~~~\nSee.\n~~~".into()),
            "the backtick wrapper goes, the tildes stay"
        );
        // A tilde wrapper, closed by a backtick line, is not a wrapper: nothing is dropped.
        let open = "BEGIN TEXT\n~~~\nSee.\n```\nEND TEXT";
        assert_eq!(proposal(open, false), Some("~~~\nSee.\n```".into()));
        let tilde = "BEGIN TEXT\n~~~\nSee.\n~~~\nEND TEXT";
        assert_eq!(proposal(tilde, false), Some("See.".into()));
    }

    #[test]
    fn no_change_gives_no_proposal() {
        assert_eq!(proposal("NO CHANGE", false), None);
        assert_eq!(proposal("  no change\n", false), None);
        assert_eq!(proposal("BEGIN TEXT\nNO CHANGE\nEND TEXT", false), None);
    }

    #[test]
    fn an_answer_without_a_block_or_with_an_empty_one_gives_no_proposal() {
        assert_eq!(proposal("Sure, here is a better version: ...", false), None);
        assert_eq!(proposal("BEGIN TEXT\nx\n", false), None, "no end mark");
        assert_eq!(
            proposal("END TEXT\nBEGIN TEXT\nx", false),
            None,
            "end before begin"
        );
        assert_eq!(
            proposal("BEGIN TEXT\n\n   \nEND TEXT", false),
            None,
            "an empty block"
        );
        assert_eq!(proposal("", false), None);
    }

    #[test]
    fn a_passage_is_replaced_in_place_mid_line() {
        let base = "# Plan\n\na un peu de texte b\n";
        let range = base.find("un peu").unwrap()..base.find(" b\n").unwrap();
        assert_eq!(
            rewritten(base, Some(range), "Un peu plus clair."),
            "# Plan\n\na Un peu plus clair. b\n"
        );
    }

    #[test]
    fn a_selection_that_ends_with_a_break_keeps_its_break() {
        let base = "# Plan\n\nun peu de texte\nla suite\n";
        let range = 8.."# Plan\n\nun peu de texte\n".len();
        assert_eq!(
            rewritten(base, Some(range), "Un peu plus clair.\n\n"),
            "# Plan\n\nUn peu plus clair.\nla suite\n"
        );
    }

    #[test]
    fn a_selection_without_a_break_at_its_end_adds_none() {
        let base = "# Plan\n\nun peu de texte\nla suite\n";
        let range = 8.."# Plan\n\nun peu de texte".len();
        assert_eq!(
            rewritten(base, Some(range), "Un peu plus clair."),
            "# Plan\n\nUn peu plus clair.\nla suite\n"
        );
    }

    #[test]
    fn the_whole_note_keeps_a_final_break_only_when_the_base_had_one() {
        assert_eq!(rewritten("a\nb\n", None, "A\nB\n"), "A\nB\n");
        assert_eq!(rewritten("a\nb", None, "A\nB\n"), "A\nB");
        assert_eq!(rewritten("", None, "New\n"), "New");
    }

    #[test]
    fn crlf_breaks_take_the_kind_of_the_base() {
        let base = "a\r\nb\r\n";
        assert_eq!(rewritten(base, None, "A\nB"), "A\r\nB\r\n");
        assert_eq!(
            rewritten(base, Some(0.."a\r\n".len()), "x\ny\n"),
            "x\r\ny\r\nb\r\n"
        );
        // A text without CRLF gets LF, whatever the block holds.
        assert_eq!(rewritten("a\nb\n", None, "x\r\ny"), "x\ny\n");
    }
}
