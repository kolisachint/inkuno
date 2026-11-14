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

/// The words that ask for a review of `note` (a path relative to the agent's working folder,
/// or absolute). The answer is read back by [`findings`].
pub fn review_prompt(note: &str) -> String {
    format!(
        "You are the assistant inside Inkuno, a note app for Markdown.\n\
         \n\
         Read the file `{note}` and review it as a careful editor. Look for statements that are \
         unclear or wrong, gaps, contradictions, and typos that change the meaning. Give at most \
         12 findings, the most useful first. No praise. Write the comments in the language of the \
         note. Do not change any file.\n\
         \n\
         Answer with NOTHING but findings, one after another, each in exactly this form, with no \
         text before, between or after them:\n\
         PASSAGE: <text copied exactly from ONE line of the note, short (3 to 12 words), unique \
         in the note, no Markdown markup added>\n\
         COMMENT: <the remark, one line>\n\
         \n\
         When there is nothing to say, answer with the single line NO FINDINGS."
    )
}

/// The findings in a review answer: (passage, comment) pairs, in order.
///
/// Tolerant: a `PASSAGE:` line opens a finding and the `COMMENT:` line after it closes it. A
/// comment may go on over the following lines until a blank line, a code fence or the next
/// `PASSAGE:`. Labels may come with a list mark, bold or backticks around them. Text before,
/// between and after the findings is ignored, and so is a pair with an empty part.
pub fn findings(answer: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut open: Option<Open> = None;
    for line in answer.lines() {
        let line = line.trim();
        if let Some(value) = label(line, "PASSAGE") {
            finish(&mut open, &mut found);
            open = Some(Open {
                passage: unquote(value),
                comment: None,
            });
        } else if let Some(value) = label(line, "COMMENT") {
            if let Some(open) = open.as_mut().filter(|open| open.comment.is_none()) {
                open.comment = Some(value.to_string());
            }
        } else if line.is_empty() || line.starts_with("```") {
            // The comment ends here; a passage still waiting for its comment stays open.
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

/// A finding read so far: its passage, and its comment once the `COMMENT:` line came.
struct Open {
    passage: String,
    comment: Option<String>,
}

/// Moves the open finding into `found` when both of its parts are there.
fn finish(open: &mut Option<Open>, found: &mut Vec<(String, String)>) {
    if let Some(Open {
        passage,
        comment: Some(comment),
    }) = open.take()
    {
        let comment = comment.trim();
        if !passage.is_empty() && !comment.is_empty() {
            found.push((passage, comment.to_string()));
        }
    }
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

/// What placing the findings in a note gave: the new text, and how many findings were placed and
/// how many were left out.
#[derive(Debug, PartialEq, Eq)]
pub struct Placed {
    pub text: String,
    pub placed: usize,
    pub skipped: usize,
}

/// Wraps each finding's passage in `text`, in order, with `markdown::comment_on`. A finding is
/// left out when its passage is not in the text exactly once, or when it lies inside a comment
/// that is already there (a second finding on the same words would nest in the first).
pub fn place(text: &str, found: &[(String, String)]) -> Placed {
    let mut out = Placed {
        text: text.to_string(),
        placed: 0,
        skipped: 0,
    };
    for (passage, comment) in found {
        let inside = out.text.find(passage.as_str()).is_some_and(|at| {
            markdown::all_comments(&out.text)
                .iter()
                .any(|note| note.at <= at && at < note.end)
        });
        match markdown::comment_on(&out.text, passage, comment) {
            Ok(next) if !inside => {
                out.text = next;
                out.placed += 1;
            }
            _ => out.skipped += 1,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(passage: &str, comment: &str) -> (String, String) {
        (passage.into(), comment.into())
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
        let prompt = review_prompt("Notes/Draft.md");
        assert!(prompt.contains("`Notes/Draft.md`"));
        assert!(prompt.contains("at most 12 findings"));
        assert!(prompt.contains("PASSAGE: <text copied exactly"));
        assert!(prompt.contains("COMMENT: <the remark, one line>"));
        assert!(prompt.contains("NO FINDINGS"));
        assert!(prompt.contains("Do not change any file."));
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
    fn one_pair_of_quotes_or_backticks_leaves_the_passage() {
        let answer = "PASSAGE: \"quoted words here\"\nCOMMENT: a\n\
                      PASSAGE: `code words`\nCOMMENT: b\n\
                      PASSAGE: 'single quoted'\nCOMMENT: c\n\
                      PASSAGE: ``double``\nCOMMENT: d";
        let found = findings(answer);
        assert_eq!(found[0].0, "quoted words here");
        assert_eq!(found[1].0, "code words");
        assert_eq!(found[2].0, "single quoted");
        assert_eq!(found[3].0, "`double`");
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
        let placed = place(text, &found);
        assert_eq!(placed.placed, 2);
        assert_eq!(placed.skipped, 2);
        assert_eq!(
            placed.text,
            "# T\n\n{==the cache is cleared daily==}{>>Daily when?<<}\n\
             {==ships in March==}{>>Which year?<<}\nalpha and alpha\n"
        );
    }

    #[test]
    fn a_passage_inside_a_comment_already_there_is_left_out() {
        let text = "{==alpha==}{>>first<<} beta\n";
        let placed = place(text, &[pair("alpha", "again")]);
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
}
