//! Comments of a note: `passage<!-- comment @name 2026-11-24T10:02Z: text ↳ @name 2026-11-24T10:05Z: reply -->`
//! (the new form, with its author and time) and `passage<!-- comment: text -->` (the legacy form, read as
//! author and time unknown). Pure functions: the time is given by the caller.

use std::ops::Range;

use super::*;

/// Opening of a comment in the note: `passage<!-- comment: text -->`. GitHub hides HTML comments,
/// so the note reads cleanly there. The passage is the text from the start of the line (or of
/// the cell of a table, or the end of the previous comment) up to the marker, trimmed.
pub const COMMENT_OPEN: &str = "<!-- comment:";
const HTML_CLOSE: &str = "-->";
const HTML_OPEN: &str = "<!--";

/// Start of every comment, in either form: `<!-- comment:` (legacy) or `<!-- comment @` (with an
/// author and a time). Both are read; the legacy form has no author and no time.
const HEAD: &str = "<!-- comment";
/// Separator of the replies of a comment: ` ↳ `. A literal `↳` in a text is written `\↳`.
const SEP: &str = " ↳ ";

/// Who wrote a comment or a reply, and when: the author's name and the UTC minute, `2026-11-24T10:02Z`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp<'a> {
    pub author: &'a str,
    pub time: &'a str,
}

/// A comment as its markup says it. A legacy comment (`<!-- comment: text -->`) has no author and
/// no time. Its replies follow the text, in order.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Comment {
    pub author: Option<String>,
    pub time: Option<String>,
    pub text: String,
    pub replies: Vec<Reply>,
}

/// A reply to a comment: an author, a time and a text, as the comment has them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Reply {
    pub author: Option<String>,
    pub time: Option<String>,
    pub text: String,
}

/// The UTC minute of a Unix time, as the markup writes it: `2026-11-24T10:02Z`.
pub fn utc_minute(unix: i64) -> String {
    let (year, month, day, hour, minute) = crate::revision::utc_parts(unix);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}Z")
}

/// Whether `b` is a time as `utc_minute` writes it.
fn is_stamp(b: &[u8]) -> bool {
    b.len() == 17
        && b.iter().enumerate().all(|(i, &c)| match i {
            4 | 7 => c == b'-',
            10 => c == b'T',
            13 => c == b':',
            16 => c == b'Z',
            _ => c.is_ascii_digit(),
        })
}

/// Whether `s` starts with the opening of a comment, in either form: `<!-- comment:` (legacy), or
/// `<!-- comment @name <stamp>:` whose head is complete before the comment's close.
fn opens_here(s: &str) -> bool {
    let Some(rest) = s.strip_prefix(HEAD) else {
        return false;
    };
    if rest.starts_with(':') {
        return true;
    }
    let Some(head_at) = rest.strip_prefix(' ') else {
        return false;
    };
    let close = head_at.find(HTML_CLOSE).unwrap_or(head_at.len());
    head(&head_at[..close]).is_some()
}

/// Whether `text` holds the opening of a comment, in either form.
pub fn has_comment(text: &str) -> bool {
    text.match_indices(HEAD)
        .any(|(at, _)| opens_here(&text[at..]))
}

/// The byte where the next comment opening of `line`, from `from`, starts.
fn next_open(line: &str, from: usize) -> Option<usize> {
    let mut at = from;
    while let Some(found) = line[at..].find(HEAD).map(|i| i + at) {
        if opens_here(&line[found..]) {
            return Some(found);
        }
        at = found + HEAD.len();
    }
    None
}

/// A head at the start of `s`, `@author time: `: the author's range (without the `@`), the time's
/// range, and where the text starts.
fn head(s: &str) -> Option<(Range<usize>, Range<usize>, usize)> {
    let b = s.as_bytes();
    if b.first() != Some(&b'@') {
        return None;
    }
    for i in 2..b.len() {
        if b[i] == b' '
            && b.get(i + 1..i + 18).is_some_and(is_stamp)
            && b.get(i + 18) == Some(&b':')
        {
            let text = i + 19 + usize::from(b.get(i + 19) == Some(&b' '));
            return Some((1..i, i + 1..i + 18, text));
        }
    }
    None
}

/// The parts of a comment, given the bytes between `<!-- comment` and `-->`: the comment first, then
/// its replies. Each part has its author and time (when it has them) and the range of its text, one
/// space after the markup's colon or separator left out.
struct Part {
    author: Option<Range<usize>>,
    time: Option<Range<usize>>,
    text: Range<usize>,
}

/// Whether the comment of `inner` (the bytes after `<!-- comment`) is in the legacy form.
fn is_legacy(inner: &str) -> bool {
    inner.starts_with(':')
}

fn parts(inner: &str) -> Option<Vec<Part>> {
    let b = inner.as_bytes();
    let (first_author, first_time, start) = match b.first()? {
        b':' => (None, None, 1 + usize::from(b.get(1) == Some(&b' '))),
        b' ' if b.get(1) == Some(&b'@') => {
            let (a, t, text) = head(&inner[1..])?;
            (
                Some(a.start + 1..a.end + 1),
                Some(t.start + 1..t.end + 1),
                text + 1,
            )
        }
        _ => return None,
    };
    let legacy = is_legacy(inner);
    let mut end = inner.len();
    if end > start && b[end - 1] == b' ' {
        end -= 1;
    }
    let mut bounds = Vec::new();
    let mut from = start;
    while let Some(found) = inner[from..end].find(SEP).map(|i| i + from) {
        bounds.push(found);
        from = found + SEP.len();
    }
    // A legacy comment is one text, arrows and all: a separator starts a reply only where the
    // segment after it has a head (`@name stamp:`). The new form splits at every separator.
    let mut starts = Vec::new();
    for (k, &sep) in bounds.iter().enumerate() {
        let seg = sep + SEP.len();
        let stop = bounds.get(k + 1).copied().unwrap_or(end);
        if !legacy || head(&inner[seg..stop]).is_some() {
            starts.push(sep);
        }
    }
    let mut out = vec![Part {
        author: first_author,
        time: first_time,
        text: start..starts.first().copied().unwrap_or(end),
    }];
    for (k, &sep) in starts.iter().enumerate() {
        let seg = sep + SEP.len();
        let stop = starts.get(k + 1).copied().unwrap_or(end);
        out.push(match head(&inner[seg..stop]) {
            Some((a, t, text)) => Part {
                author: Some(seg + a.start..seg + a.end),
                time: Some(seg + t.start..seg + t.end),
                text: seg + text..stop,
            },
            None => Part {
                author: None,
                time: None,
                text: seg..stop,
            },
        });
    }
    Some(out)
}

/// The text as the markup holds it in the new form: a `-->` or a `<!--` is spaced out (`spaced`),
/// and a literal `↳` is written `\↳`.
fn escape(text: &str) -> String {
    spaced(text).replace('↳', "\\↳")
}

/// The text of a new-form part as it is read: `\↳` becomes `↳`.
fn unescape(raw: &str) -> String {
    raw.replace("\\↳", "↳")
}

/// A text as the markup holds it: `escape` in the new form; in the legacy form, which has no
/// escapes, only `spaced`.
fn written(text: &str, new: bool) -> String {
    if new { escape(text) } else { spaced(text) }
}

/// A name as the markup holds it: one line, no `↳`, no `-->`, single spaces; `Unknown` when empty.
fn clean_name(name: &str) -> String {
    let flat = spaced(name).replace('↳', "");
    let words = flat.split_whitespace().collect::<Vec<_>>().join(" ");
    if words.is_empty() {
        "Unknown".into()
    } else {
        words
    }
}

/// A comment by `by`, with `text`, and no reply yet.
pub fn new_comment(text: &str, by: Stamp) -> Comment {
    Comment {
        author: Some(clean_name(by.author)),
        time: Some(by.time.to_string()),
        text: text.to_string(),
        replies: Vec::new(),
    }
}

/// Adds a reply by `by` to `comment`, after the ones it has.
pub fn add_reply(comment: &mut Comment, text: &str, by: Stamp) {
    comment.replies.push(Reply {
        author: Some(clean_name(by.author)),
        time: Some(by.time.to_string()),
        text: text.to_string(),
    });
}

/// The comment of `markup`, a whole `<!-- comment … -->` (spaces around it allowed); `None` when it
/// is not one. Legacy and new forms both read.
pub fn parse_comment(markup: &str) -> Option<Comment> {
    let t = markup.trim();
    if !opens_here(t) {
        return None;
    }
    let inner = t[HEAD.len()..].strip_suffix(HTML_CLOSE)?;
    let parts = parts(inner)?;
    let name = |r: &Option<Range<usize>>| r.as_ref().map(|r| inner[r.clone()].to_string());
    let first = &parts[0];
    let root = &inner[first.text.clone()];
    Some(Comment {
        author: name(&first.author),
        time: name(&first.time),
        text: if is_legacy(inner) {
            root.to_string()
        } else {
            unescape(root)
        },
        replies: parts[1..]
            .iter()
            .map(|p| Reply {
                author: name(&p.author),
                time: name(&p.time),
                text: unescape(&inner[p.text.clone()]),
            })
            .collect(),
    })
}

/// `comment` as its one-line markup: `<!-- comment @author time: text ↳ @author time: reply -->`,
/// or the legacy `<!-- comment: text -->` when it has no author and no time.
pub fn format_comment(comment: &Comment) -> String {
    let mut out = String::from(HEAD);
    let stamped = comment.author.is_some() && comment.time.is_some();
    if let (Some(author), Some(time)) = (&comment.author, &comment.time) {
        out.push_str(&format!(" @{} {time}", clean_name(author)));
    }
    out.push_str(": ");
    out.push_str(&written(&comment.text, stamped));
    for reply in &comment.replies {
        out.push_str(SEP);
        if let (Some(author), Some(time)) = (&reply.author, &reply.time) {
            out.push_str(&format!("@{} {time}: ", clean_name(author)));
        }
        out.push_str(&escape(&reply.text));
    }
    out.push(' ');
    out.push_str(HTML_CLOSE);
    out
}

/// `line` with the comment that holds byte `at` resolved: its markup goes and its passage stays. A
/// comment alone on its line leaves nothing. `None` when no comment of the line holds `at`.
pub fn resolve(line: &str, at: usize) -> Option<String> {
    let (whole, noted, _) = comments(line)
        .into_iter()
        .find(|(whole, ..)| (whole.start..=whole.end).contains(&at))?;
    Some(format!(
        "{}{}{}",
        &line[..whole.start],
        &line[noted],
        &line[whole.end..]
    ))
}

/// `text` with a reply by `by` added to the comment that starts at byte `at` (as `Note::at` gives it): the
/// reply is written after the replies it has, on the comment's line, so the comment stays on one line.
/// In a table a `|` of the reply is written `\|`. `None` when no comment starts at `at` on its line, or
/// when its markup does not read.
pub fn reply_on(text: &str, at: usize, reply: &str, by: Stamp) -> Option<String> {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let line_end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    let line = &text[line_start..line_end];
    let col = at - line_start;
    let (whole, ..) = comments(line)
        .into_iter()
        .find(|(whole, ..)| whole.start == col)?;
    let markup = markup_in(&line[whole.clone()]);
    let mut comment = parse_comment(markup)?;
    let kept = &line[whole.start..whole.end - markup.len()];
    let typed = match is_table_line(line) {
        true => reply.replace('|', r"\|"),
        false => reply.to_string(),
    };
    add_reply(&mut comment, typed.trim(), by);
    let marked = format!("{kept}{}", format_comment(&comment));
    Some(format!(
        "{}{marked}{}",
        &text[..line_start + whole.start],
        &text[line_start + whole.end..]
    ))
}

/// `line`, a comment alone on it (indent aside), with its text replaced by `text` (written as
/// `format_comment` writes it): its author, its time and its replies are kept. `None` when `line` is
/// not such a comment.
pub fn retext(line: &str, text: &str) -> Option<String> {
    let t = line.trim();
    if !opens_here(t) {
        return None;
    }
    let inner = t[HEAD.len()..].strip_suffix(HTML_CLOSE)?;
    let first = parts(inner)?.swap_remove(0).text;
    Some(format!(
        "{HEAD}{}{}{}{HTML_CLOSE}",
        &inner[..first.start],
        written(text, !is_legacy(inner)),
        &inner[first.end..]
    ))
}

/// The markup of the comment in `whole`, a passage and its comment as `comments` gives them: from the
/// opening of the comment (the first one `next_open` finds, so a text holding `<!-- comment` cannot
/// move it) to the end. `whole` itself when it holds no opening.
pub fn markup_in(whole: &str) -> &str {
    next_open(whole, 0).map_or(whole, |i| &whole[i..])
}

/// Where the text of `comment` starts in `format_comment(comment)`: past the opening, the author and
/// the time when it has them, and the colon.
pub fn said_offset(comment: &Comment) -> usize {
    let stamp = match (&comment.author, &comment.time) {
        (Some(author), Some(time)) => format!(" @{} {time}", clean_name(author)).len(),
        _ => 0,
    };
    HEAD.len() + stamp + ": ".len()
}

/// Refusal of a comment text that would break the marks: a comment cannot hold `-->` (every one is
/// spaced out, `-- >`) nor `<!--` (spaced, `<! --`), so that neither can close it or open another
/// comment. The line breaks become spaces.
pub fn spaced(text: &str) -> String {
    let mut flat = text.replace("\r\n", " ").replace(['\n', '\r'], " ");
    while flat.contains(HTML_CLOSE) || flat.contains(HTML_OPEN) {
        flat = flat.replace(HTML_CLOSE, "-- >").replace(HTML_OPEN, "<! --");
    }
    flat
}

/// Whether `author` is shown as `Unknown`: no author, an empty one, or the placeholder `unknown`
/// (any case) that `revision::default_author` gives when the environment names nobody. Otherwise the
/// name, trimmed.
pub fn shown_author(author: &Option<String>) -> Option<&str> {
    author
        .as_deref()
        .map(str::trim)
        .filter(|a| !a.is_empty() && !a.eq_ignore_ascii_case("unknown"))
}

/// `passage<!-- comment: text -->`. In a table cell a `|` of the comment is written `\|`. The
/// syntax is read line by line, so the passage is one line, and the comment's line breaks become
/// spaces; an empty comment is refused. A `-->` in the comment is spaced out, so that it cannot
/// close the comment early, and a passage holding comment marks is refused. The comment is
/// written after `passage`; it is anchored to the text from the start of its line (or cell, or
/// the previous comment) up to the marker, so `passage` is the part of the line it covers.
/// `passage<!-- comment: text -->`, the legacy form: no author and no time. `comment_on_by` writes
/// the new form.
pub fn comment_on(text: &str, passage: &str, comment: &str) -> Result<String, String> {
    comment_with(text, passage, comment, None)
}

/// `passage<!-- comment @author time: text -->`: as `comment_on`, the comment by `by`.
pub fn comment_on_by(
    text: &str,
    passage: &str,
    comment: &str,
    by: Stamp,
) -> Result<String, String> {
    comment_with(text, passage, comment, Some(by))
}

fn comment_with(
    text: &str,
    passage: &str,
    comment: &str,
    by: Option<Stamp>,
) -> Result<String, String> {
    if passage.is_empty() {
        return Err("the passage is empty".into());
    }
    if passage.contains(['\n', '\r']) {
        return Err("the passage must be on one line".into());
    }
    if ["{==", "==}", "{>>", "<<}", HTML_CLOSE]
        .iter()
        .any(|mark| passage.contains(mark))
        || has_comment(passage)
    {
        return Err("the passage holds comment marks".into());
    }
    let mut flat = spaced(comment).trim().to_string();
    if flat.is_empty() {
        return Err("the comment is empty".into());
    }
    let mut at = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find(passage) {
        at.push(from + found);
        from += found + passage.len().max(1);
        if at.len() > 1 {
            break;
        }
    }
    let [start] = at.as_slice() else {
        return Err(if at.is_empty() {
            format!("`{passage}` was not found")
        } else {
            format!("`{passage}` occurs more than once")
        });
    };
    let line_start = text[..*start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line_end = text[*start..]
        .find('\n')
        .map(|i| *start + i)
        .unwrap_or(text.len());
    let in_cell = *start + passage.len() <= line_end && is_table_line(&text[line_start..line_end]);
    if in_cell {
        flat = flat.replace('|', "\\|");
    }
    let end = *start + passage.len();
    let mut out = String::with_capacity(text.len() + flat.len() + 20);
    out.push_str(&text[..end]);
    match by {
        Some(by) => out.push_str(&format_comment(&new_comment(&flat, by))),
        None => out.push_str(&comment_line(&flat)),
    }
    out.push_str(&text[end..]);
    Ok(out)
}

/// Comments written in the earlier CriticMarkup form, `{==passage==}{>>comment<<}` (or on their own,
/// `{>>comment<<}`): read, and rewritten to the new form by `migrate_comments`. Same ranges as
/// `comments`: the whole, the commented text and the comment. A comment on its own has an empty
/// commented text, at its start.
fn critic_comments(line: &str) -> Vec<(Range<usize>, Range<usize>, Range<usize>)> {
    let (mut found, mut from) = (Vec::new(), 0);
    while let Some(open) = line[from..].find("{==").map(|i| i + from) {
        let Some(mid) = line[open + 3..].find("==}{>>").map(|i| i + open + 3) else {
            break;
        };
        let Some(end) = line[mid + 6..].find("<<}").map(|i| i + mid + 6) else {
            break;
        };
        found.push((open..end + 3, open + 3..mid, mid + 6..end));
        from = end + 3;
    }
    let full = found.len();
    from = 0;
    while let Some(open) = line[from..].find("{>>").map(|i| i + from) {
        let Some(end) = line[open + 3..].find("<<}").map(|i| i + open + 3) else {
            break;
        };
        if !found[..full]
            .iter()
            .any(|(whole, ..)| whole.contains(&open))
        {
            found.push((open..end + 3, open..open, open + 3..end));
        }
        from = end + 3;
    }
    // A comment whose markers are in inline code is code: its passage may hold code, its markers not.
    let spans = code_spans(line);
    let in_code = |at: usize| spans.iter().any(|s| s.contains(&at));
    found.retain(|(whole, noted, said)| {
        !(in_code(whole.start) || in_code(noted.end) || in_code(said.end))
    });
    found
}

/// Comments of the line: the range of the whole (the passage and its marker), that of the
/// commented text and that of the comment. A comment on its own, `<!-- comment: text -->` with
/// nothing before it, has an empty commented text, at its start. A CriticMarkup comment is read
/// too (`critic_comments`). Inline code is not a comment, and neither is what is inside an HTML
/// comment of another text on this line. Indented and fenced code, and HTML comments over several
/// lines, need the note: `all_comments` reads them.
pub fn comments(line: &str) -> Vec<(Range<usize>, Range<usize>, Range<usize>)> {
    comments_in(line, &plain_html_ranges(line, &[]))
}

/// The HTML comments of `text`, outside code, that are not in the new form: a `<!-- … -->` of any
/// other text. What is inside one is not a comment.
fn plain_html_ranges(text: &str, code: &[Range<usize>]) -> Vec<Range<usize>> {
    html_comment_ranges(text, code)
        .into_iter()
        .filter(|r| !opens_here(&text[r.start..]))
        .collect()
}

/// `comments` of a line, given the ranges of the line (in its own bytes) that hide what is in them:
/// a comment whose markers are in one of them is not read.
fn comments_in(
    line: &str,
    hidden: &[Range<usize>],
) -> Vec<(Range<usize>, Range<usize>, Range<usize>)> {
    let inside = |at: usize| hidden.iter().any(|r| r.contains(&at));
    let mut found = critic_comments(line);
    found.retain(|(whole, noted, said)| {
        ![whole.start, noted.end, said.end]
            .iter()
            .any(|&p| inside(p))
    });
    // The block marker of the line (`- `, `## `, `> `, `- [ ] `) is not part of a passage.
    let marker = match quote(line) {
        Some(q) => q.at + classify_in(&line[q.at..], None).1,
        None => classify(line, false).1,
    };
    let spans = code_spans(line);
    let mut from = 0;
    while let Some(open) = next_open(line, from) {
        if inside(open) || spans.iter().any(|s| s.contains(&open)) {
            from = open + HEAD.len();
            continue;
        }
        let said_at = open + HEAD.len();
        let Some(close) = line[said_at..].find(HTML_CLOSE).map(|i| i + said_at) else {
            break;
        };
        let end = close + HTML_CLOSE.len();
        // The passage starts after the previous comment, after the last bar of a table line.
        let mut start = found
            .iter()
            .filter(|(whole, ..)| whole.end <= open)
            .map(|(whole, ..)| whole.end)
            .max()
            .unwrap_or(0)
            .max(from)
            .max(marker.min(open));
        if is_table_line(line) {
            let bytes = line.as_bytes();
            if let Some(bar) = (start..open)
                .rev()
                .find(|&i| bytes[i] == b'|' && (i == 0 || bytes[i - 1] != b'\\'))
            {
                start = bar + 1;
            }
        }
        let body = &line[said_at..close];
        let inner = parts(body).map_or(said_range(body), |p| p[0].text.clone());
        let said = said_at + inner.start..said_at + inner.end;
        let passage = &line[start..open];
        let lead = passage.len() - passage.trim_start().len();
        let noted = start + lead..start + passage.trim_end().len();
        if noted.is_empty() {
            found.push((open..end, open..open, said));
        } else {
            found.push((noted.start..end, noted, said));
        }
        from = end;
    }
    found.sort_by_key(|(whole, ..)| whole.start);
    found
}

/// The text of a comment line on its own, `<!-- comment: text -->` (indent allowed), if the line is
/// one. The single space on each side of the text is the markup's, the rest is the text as written.
pub fn comment_line_text(line: &str) -> Option<&str> {
    let t = line.trim();
    if !opens_here(t) {
        return None;
    }
    let inner = t[HEAD.len()..].strip_suffix(HTML_CLOSE)?;
    Some(&inner[parts(inner)?.swap_remove(0).text])
}

/// Where the text is inside `<!-- comment: … -->` (the bytes between the marks): without the one
/// space each side, which belongs to the markup.
fn said_range(inner: &str) -> Range<usize> {
    let lo = usize::from(inner.starts_with(' '));
    let hi = inner.len() - usize::from(inner.len() > lo && inner.ends_with(' '));
    lo..hi
}

/// The text of a comment line on its own, as the writer gave it: `\↳` reads as `↳` in the new form,
/// and the legacy text is kept as it is. `None` when `line` is not such a line.
pub fn comment_line_plain(line: &str) -> Option<String> {
    let t = line.trim();
    if !opens_here(t) {
        return None;
    }
    let inner = t[HEAD.len()..].strip_suffix(HTML_CLOSE)?;
    let root = &inner[parts(inner)?.swap_remove(0).text];
    Some(if is_legacy(inner) {
        root.to_string()
    } else {
        unescape(root)
    })
}

/// A comment on its own, `<!-- comment: text -->`, the text spaced as `comment_on` spaces it. The
/// text is written as it is given: a trailing space stays inside the marks.
pub fn comment_line(text: &str) -> String {
    format!("{COMMENT_OPEN} {} {HTML_CLOSE}", spaced(text))
}

/// Whether a comment's whole (as `comments` gives it) is in the new form: it ends with its marker.
pub fn is_html_comment(whole: &str) -> bool {
    whole.ends_with(HTML_CLOSE)
}

/// Byte ranges of `text` that are code, where text is not comment markup: the fenced blocks, and the
/// indented code lines. An indented line is code after a blank line (or a heading, a rule, or other
/// code), outside a list item: a list's indented lines are its paragraphs. Sorted, not overlapping.
fn code_ranges(text: &str) -> Vec<Range<usize>> {
    let blocks = blocks(text);
    let mut out: Vec<Range<usize>> = blocks.iter().map(|b| b.open..b.end).collect();
    let (mut in_list, mut gap, mut at) = (false, true, 0);
    for line in text.split('\n') {
        let end = at + line.len();
        let indent = indent_columns(line);
        let (kind, _) = classify_in(line, None);
        if is_fence(line) || open_at(&blocks, at).is_some() || line.trim().is_empty() {
            gap = true;
        } else if indent >= 4 && !in_list && gap {
            out.push(at..end);
        } else {
            if indent < 4 {
                match kind {
                    Kind::Bullet | Kind::Ordered | Kind::Task(_) => in_list = true,
                    _ if indent == 0 && gap => in_list = false,
                    _ => {}
                }
            }
            gap = matches!(kind, Kind::Heading(_) | Kind::Rule);
        }
        at = end + 1;
    }
    out.sort_by_key(|r| r.start);
    out
}

/// Whether `at` is inside one of `ranges`, which are sorted and do not overlap.
fn in_ranges(ranges: &[Range<usize>], at: usize) -> bool {
    let i = ranges.partition_point(|r| r.end <= at);
    ranges.get(i).is_some_and(|r| r.start <= at)
}

/// Byte ranges of the HTML comments of `text` that are not code, from `<!--` to the `-->` that ends
/// them, the end of the text if none does. A comment may run over several lines. Its text is not
/// migrated (see `migration_edits`).
fn html_comment_ranges(text: &str, code: &[Range<usize>]) -> Vec<Range<usize>> {
    const OPEN: &str = "<!--";
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find(OPEN).map(|i| i + from) {
        let line_start = text[..open].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[open..].find('\n').map_or(text.len(), |i| open + i);
        let at = open - line_start;
        let in_span = code_spans(&text[line_start..line_end])
            .iter()
            .any(|s| s.contains(&at));
        if in_ranges(code, open) || in_span {
            from = open + OPEN.len();
            continue;
        }
        let end = text[open + OPEN.len()..]
            .find(HTML_CLOSE)
            .map_or(text.len(), |i| open + OPEN.len() + i + HTML_CLOSE.len());
        out.push(open..end);
        from = end;
    }
    out
}

/// Replacements that write the comments of `text` in the earlier CriticMarkup form in the new one,
/// as `(range, text)` pairs in order: each replaces the bytes `range` of `text`
/// (`apply_edits` applies them). A comment whose markers are in code (inline code, fenced or
/// indented code) is text, and so is one that overlaps an HTML comment: both are left as they are.
pub fn migration_edits(text: &str) -> Vec<(Range<usize>, String)> {
    if !text.contains("{==") && !text.contains("{>>") {
        return Vec::new();
    }
    let code = code_ranges(text);
    let html = html_comment_ranges(text, &code);
    let mut edits = Vec::new();
    let mut at = 0;
    for piece in text.split_inclusive('\n') {
        let body = piece.strip_suffix('\n').unwrap_or(piece);
        let line_start = at;
        at += piece.len();
        if !body.contains("{>>") || in_ranges(&code, line_start) {
            continue;
        }
        for (whole, noted, said) in critic_comments(body) {
            let span = line_start + whole.start..line_start + whole.end;
            if html
                .iter()
                .any(|r| r.start < span.end && span.start < r.end)
            {
                continue;
            }
            let mut new = body[noted].to_string();
            new.push_str(&comment_line(&body[said]));
            edits.push((span, new));
        }
    }
    edits
}

/// `text` with `edits` (from `migration_edits`, in order) applied.
pub fn apply_edits(text: &str, edits: &[(Range<usize>, String)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for (range, new) in edits {
        out.push_str(&text[last..range.start]);
        out.push_str(new);
        last = range.end;
    }
    out.push_str(&text[last..]);
    out
}

/// The note with every comment in the earlier CriticMarkup form written in the new one, outside
/// code and HTML comments: `{==passage==}{>>text<<}` becomes `passage<!-- comment: text -->`, and a
/// comment on its own `{>>text<<}` becomes `<!-- comment: text -->`. The texts are kept as they are,
/// except that a `-->` is spaced out. The passage is anchored to its line (see `comment_on`), so a
/// passage in the middle of a line covers the text from the start of the line up to its marker.
/// A note with no such comment is returned as it is.
pub fn migrate_comments(text: &str) -> String {
    apply_edits(text, &migration_edits(text))
}

/// A comment of the note, as its panel lists it.
#[derive(Debug, PartialEq)]
pub struct Note<'a> {
    /// The commented text; empty for a comment on its own.
    pub noted: &'a str,
    /// What is said, without the name of the row or column it starts with, if any.
    pub said: &'a str,
    /// The byte where the whole starts, and the one where the comment starts.
    pub at: usize,
    pub inside: usize,
    /// The byte where the whole ends.
    pub end: usize,
    /// What it is on, in a table: `Cell C2`, `Header B`, `Row Léa`, `Column Âge`, `Table`; empty elsewhere.
    pub scope: String,
    /// The author and the time of the comment, and its replies; none of them in the legacy form.
    pub author: Option<String>,
    pub time: Option<String>,
    pub replies: Vec<Reply>,
}

/// Name of column `c` as a spreadsheet writes it: `A`, `B`… `Z`, `AA`.
pub fn column_letters(c: usize) -> String {
    let (mut n, mut name) = (c + 1, String::new());
    while n > 0 {
        name.insert(0, (b'A' + ((n - 1) % 26) as u8) as char);
        n = (n - 1) / 26;
    }
    name
}

/// A comment line under a table starts with what it is about (see `scopes`): its name as the
/// panel says it, and what follows.
pub fn split_scope(said: &str) -> Option<(String, &str)> {
    if let Some(rest) = said.strip_prefix("Table: ") {
        return Some(("Table".into(), rest));
    }
    let (kind, tail) = said
        .strip_prefix("Row ")
        .map(|t| ("Row", t))
        .or_else(|| said.strip_prefix("Column ").map(|t| ("Column", t)))?;
    let digits = |t: &str| t.bytes().take_while(u8::is_ascii_digit).count();
    let Some(quoted) = tail.strip_prefix('"') else {
        let n = digits(tail);
        return tail[n..]
            .strip_prefix(": ")
            .filter(|_| n > 0)
            .map(|rest| (format!("{kind} {}", &tail[..n]), rest));
    };
    let plain = quoted
        .find("\": ")
        .map(|i| (i, quoted[i + 3..].len(), String::new()));
    let numbered = quoted.match_indices("\" (").find_map(|(i, _)| {
        let n = digits(&quoted[i + 3..]);
        let rest = quoted[i + 3 + n..].strip_prefix("): ")?;
        (n > 0).then(|| (i, rest.len(), format!(" ({})", &quoted[i + 3..i + 3 + n])))
    });
    let (i, left, number) = [plain, numbered]
        .into_iter()
        .flatten()
        .min_by_key(|(i, ..)| *i)?;
    Some((
        format!("{kind} {}{number}", &quoted[..i]),
        &quoted[quoted.len() - left..],
    ))
}

/// All the note's comments, outside code blocks.
pub fn all_comments(text: &str) -> Vec<Note<'_>> {
    // Almost no note has any: a single search says so, without reading the lines.
    if !text.contains("{==") && !text.contains("{>>") && !text.contains(HEAD) {
        return Vec::new();
    }
    let mut found = Vec::new();
    // Line of the table we are in (the header is 0, the dashes 1).
    let mut row: Option<usize> = None;
    // Where the next prose line starts: after a fence or a code line the table is over.
    let mut next = 0;
    // Indented code, and what is in an HTML comment of another text (over lines), is not a comment:
    // the fenced blocks are not prose lines at all.
    let code = code_ranges(text);
    let plain = plain_html_ranges(text, &code);
    for (at, line) in prose_lines(text) {
        if at != next {
            row = None;
        }
        next = at + line.len() + 1;
        if in_ranges(&code, at) {
            row = None;
            continue;
        }
        let end = at + line.len();
        let hidden: Vec<Range<usize>> = plain
            .iter()
            .filter(|r| r.start <= end && r.end > at)
            .map(|r| r.start.max(at) - at..r.end.min(end) - at)
            .collect();
        row = line
            .trim_start()
            .starts_with('|')
            .then(|| row.map_or(0, |r| r + 1));
        let cells = row.map(|_| cells(line));
        for (whole, noted, said) in comments_in(line, &hidden) {
            let markup = markup_in(&line[whole.clone()]);
            let (author, time, replies) = parse_comment(markup)
                .map(|c| (c.author, c.time, c.replies))
                .unwrap_or_default();
            let mut note = Note {
                noted: &line[noted.clone()],
                said: &line[said.clone()],
                at: at + whole.start,
                inside: at + said.start,
                end: at + whole.end,
                scope: String::new(),
                author,
                time,
                replies,
            };
            if let Some(cells) = &cells {
                let c = cells
                    .iter()
                    .rposition(|cell| cell.start <= whole.start)
                    .unwrap_or(0);
                note.scope = match row {
                    Some(0) => format!("Header {}", column_letters(c)),
                    Some(r) => format!("Cell {}{}", column_letters(c), r.saturating_sub(1)),
                    None => String::new(),
                };
            } else if noted.is_empty()
                && let Some((scope, rest)) = split_scope(note.said)
            {
                note.scope = scope;
                note.said = rest;
            }
            found.push(note);
        }
    }
    found
}

#[cfg(test)]
mod form_tests {
    use super::*;

    const ANA: Stamp = Stamp {
        author: "Ana",
        time: "2026-11-24T10:02Z",
    };
    const BO: Stamp = Stamp {
        author: "Bo",
        time: "2026-11-24T10:05Z",
    };

    #[test]
    fn a_new_form_comment_is_parsed_with_its_author_time_and_replies() {
        let markup = "<!-- comment @Ana 2026-11-24T10:02Z: text ↳ @Bo 2026-11-24T10:05Z: reply -->";
        let c = parse_comment(markup).unwrap();
        assert_eq!(c.author.as_deref(), Some("Ana"));
        assert_eq!(c.time.as_deref(), Some("2026-11-24T10:02Z"));
        assert_eq!(c.text, "text");
        assert_eq!(
            c.replies,
            [Reply {
                author: Some("Bo".into()),
                time: Some("2026-11-24T10:05Z".into()),
                text: "reply".into(),
            }]
        );
    }

    #[test]
    fn a_legacy_comment_is_read_with_no_author_and_no_time() {
        let c = parse_comment("<!-- comment: the old text -->").unwrap();
        assert_eq!(c.author, None);
        assert_eq!(c.time, None);
        assert_eq!(c.text, "the old text");
        assert!(c.replies.is_empty());
        assert_eq!(parse_comment("<!-- commentary -->"), None);
        assert_eq!(parse_comment("<!-- draft -->"), None);
    }

    #[test]
    fn a_legacy_comment_takes_replies_without_being_rewritten_on_read() {
        let legacy = "Text<!-- comment: old ↳ @Bo 2026-11-24T10:05Z: reply -->\n";
        let c = parse_comment(&legacy[4..legacy.len() - 1]).unwrap();
        assert_eq!((c.author.as_deref(), c.time.as_deref()), (None, None));
        assert_eq!(c.text, "old");
        assert_eq!(c.replies.len(), 1);
        // Reading never rewrites the note: the legacy comment is still there as it was.
        let listed = all_comments(legacy);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].author, None);
        assert_eq!(listed[0].said, "old");
        assert_eq!(migrate_comments(legacy), legacy);
    }

    #[test]
    fn a_new_comment_is_written_in_the_new_form_and_parsed_back() {
        let mut c = new_comment("first", ANA);
        assert_eq!(
            format_comment(&c),
            "<!-- comment @Ana 2026-11-24T10:02Z: first -->"
        );
        add_reply(&mut c, "second", BO);
        let markup = format_comment(&c);
        assert_eq!(
            markup,
            "<!-- comment @Ana 2026-11-24T10:02Z: first ↳ @Bo 2026-11-24T10:05Z: second -->"
        );
        assert_eq!(parse_comment(&markup), Some(c));
    }

    #[test]
    fn a_literal_arrow_is_escaped_and_comes_back() {
        let c = new_comment("a ↳ b \\↳ c", ANA);
        let markup = format_comment(&c);
        assert!(
            markup.contains("a \\↳ b \\\\↳ c") || markup.contains("a \\↳ b"),
            "{markup}"
        );
        assert_eq!(parse_comment(&markup).unwrap().text, "a ↳ b \\↳ c");
        let mut with_reply = c;
        add_reply(&mut with_reply, "x ↳ y", BO);
        let back = parse_comment(&format_comment(&with_reply)).unwrap();
        assert_eq!(back.replies[0].text, "x ↳ y");
    }

    #[test]
    fn a_comment_and_its_replies_stay_on_one_line() {
        let mut c = new_comment("one\ntwo\r\nthree", ANA);
        add_reply(&mut c, "a\nb", BO);
        let markup = format_comment(&c);
        assert!(!markup.contains(['\n', '\r']), "{markup:?}");
        let written = comment_on_by("alpha beta\n", "alpha", "x\r\ny", ANA).unwrap();
        assert_eq!(written.lines().count(), 1, "{written:?}");
        assert!(!written.contains("-->\n"));
    }

    #[test]
    fn a_comment_on_a_passage_is_written_with_its_author_and_time() {
        let out = comment_on_by("alpha beta\n", "alpha", "check this", ANA).unwrap();
        assert_eq!(
            out,
            "alpha<!-- comment @Ana 2026-11-24T10:02Z: check this --> beta\n"
        );
        let listed = all_comments(&out);
        assert_eq!(listed[0].noted, "alpha");
        assert_eq!(listed[0].said, "check this");
        assert_eq!(listed[0].author.as_deref(), Some("Ana"));
        assert_eq!(listed[0].time.as_deref(), Some("2026-11-24T10:02Z"));
    }

    #[test]
    fn resolving_a_comment_keeps_the_passage() {
        let line = "alpha<!-- comment @Ana 2026-11-24T10:02Z: check --> beta";
        assert_eq!(resolve(line, 2).as_deref(), Some("alpha beta"));
        let alone = "<!-- comment @Ana 2026-11-24T10:02Z: check -->";
        assert_eq!(resolve(alone, 0).as_deref(), Some(""));
        assert_eq!(resolve("plain", 2), None);
    }

    #[test]
    fn retext_keeps_the_author_and_the_replies() {
        let line = "<!-- comment @Ana 2026-11-24T10:02Z: old ↳ @Bo 2026-11-24T10:05Z: r -->";
        assert_eq!(
            retext(line, "new").as_deref(),
            Some("<!-- comment @Ana 2026-11-24T10:02Z: new ↳ @Bo 2026-11-24T10:05Z: r -->")
        );
        let legacy = "<!-- comment: old -->";
        assert_eq!(
            retext(legacy, "new").as_deref(),
            Some("<!-- comment: new -->")
        );
    }

    #[test]
    fn utc_minutes_are_written_to_the_minute() {
        assert_eq!(utc_minute(0), "1970-01-01T00:00Z");
        assert_eq!(utc_minute(1_000_000_000), "2001-09-09T01:46Z");
        assert_eq!(utc_minute(1_000_000_059), "2001-09-09T01:47Z");
    }

    #[test]
    fn the_text_of_a_comment_starts_where_said_offset_says() {
        let stamped = new_comment("x", ANA);
        assert_eq!(&format_comment(&stamped)[said_offset(&stamped)..], "x -->");
        let legacy = Comment {
            text: "x".into(),
            ..Comment::default()
        };
        assert_eq!(&format_comment(&legacy)[said_offset(&legacy)..], "x -->");
    }

    #[test]
    fn the_markup_of_a_passage_is_the_comment_after_it() {
        assert_eq!(
            markup_in("passage<!-- comment: x -->"),
            "<!-- comment: x -->"
        );
        assert_eq!(markup_in("plain text"), "plain text");
    }

    #[test]
    fn retext_writes_an_arrow_escaped_so_that_it_is_not_a_reply() {
        let line = "<!-- comment @Ana 2026-11-24T10:02Z: old -->";
        let out = retext(line, "a ↳ b").unwrap();
        let back = parse_comment(&out).unwrap();
        assert_eq!(back.text, "a ↳ b");
        assert!(back.replies.is_empty());
        assert_eq!(back.author.as_deref(), Some("Ana"));
    }

    #[test]
    fn a_plain_comment_with_a_mention_is_not_an_inkuno_comment() {
        let line = "a<!-- comment @todo fix -->";
        assert!(!has_comment(line));
        assert_eq!(parse_comment("<!-- comment @todo fix -->"), None);
        assert!(comments(line).is_empty());
        assert!(all_comments(line).is_empty());
        assert!(has_comment("a<!-- comment: x -->"));
        assert!(has_comment("a<!-- comment @Ana 2026-11-24T10:02Z: x -->"));
        assert!(!has_comment("<!-- commentary -->"));
    }

    #[test]
    fn a_legacy_text_keeps_its_arrows_and_backslashes() {
        let c = parse_comment("<!-- comment: a ↳ b \\↳ c -->").unwrap();
        assert_eq!(c.text, "a ↳ b \\↳ c");
        assert!(c.replies.is_empty());
    }

    #[test]
    fn a_legacy_comment_splits_only_before_a_reply_head() {
        let c =
            parse_comment("<!-- comment: old ↳ plain ↳ @Bo 2026-11-24T10:05Z: r\\↳x -->").unwrap();
        assert_eq!(c.text, "old ↳ plain");
        assert_eq!(
            c.replies,
            [Reply {
                author: Some("Bo".into()),
                time: Some("2026-11-24T10:05Z".into()),
                text: "r↳x".into(),
            }]
        );
    }

    #[test]
    fn a_new_form_comment_unescapes_its_text_and_its_replies() {
        let markup =
            "<!-- comment @Ana 2026-11-24T10:02Z: a \\↳ b ↳ @Bo 2026-11-24T10:05Z: c \\↳ d -->";
        let c = parse_comment(markup).unwrap();
        assert_eq!(c.text, "a ↳ b");
        assert_eq!(c.replies[0].text, "c ↳ d");
    }

    #[test]
    fn a_comment_text_cannot_open_another_comment() {
        let c = new_comment("see <!-- comment @Bo 2026-11-24T10:05Z: x --> here", ANA);
        let markup = format_comment(&c);
        assert_eq!(markup.matches("<!--").count(), 1, "{markup}");
        let note = format!("alpha{markup} beta");
        let listed = all_comments(&note);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].author.as_deref(), Some("Ana"));
        assert_eq!(
            parse_comment(&markup).unwrap().text,
            "see <! -- comment @Bo 2026-11-24T10:05Z: x -- > here"
        );
        // A reply is spaced the same way, and the text that has no mark round-trips as it is.
        let mut with_reply = new_comment("plain", ANA);
        add_reply(&mut with_reply, "<!-- comment: x", BO);
        let markup = format_comment(&with_reply);
        assert_eq!(all_comments(&format!("a{markup}")).len(), 1);
        assert_eq!(
            parse_comment(&markup).unwrap().replies[0].text,
            "<! -- comment: x"
        );
        let exact = new_comment("a ↳ b, c", ANA);
        assert_eq!(parse_comment(&format_comment(&exact)), Some(exact));
    }

    #[test]
    fn the_markup_of_a_comment_is_the_one_its_opening_starts() {
        assert_eq!(
            markup_in("passage<!-- comment @Ana 2026-11-24T10:02Z: x <!-- comment: y -->"),
            "<!-- comment @Ana 2026-11-24T10:02Z: x <!-- comment: y -->"
        );
        // A legacy text holding an opening: the comment is still the one that starts the markup.
        let listed =
            all_comments("alpha<!-- comment @Ana 2026-11-24T10:02Z: a <!-- comment: b --> beta");
        assert_eq!(listed[0].author.as_deref(), Some("Ana"));
        assert_eq!(listed[0].said, "a <!-- comment: b");
    }

    #[test]
    fn a_retext_of_a_legacy_comment_keeps_its_arrows_raw() {
        let legacy = "<!-- comment: old -->";
        let out = retext(legacy, "a ↳ b").unwrap();
        assert_eq!(out, "<!-- comment: a ↳ b -->");
        assert_eq!(comment_line_plain(&out).as_deref(), Some("a ↳ b"));
    }

    #[test]
    fn a_passage_holding_an_opening_is_refused() {
        let text = "a <!-- comment @Ana 2026-11-24T10:02Z: x b";
        assert!(comment_on_by(text, "a <!-- comment @Ana 2026-11-24T10:02Z: x", "y", ANA).is_err());
    }

    #[test]
    fn shown_author_treats_none_empty_and_unknown_as_unknown() {
        assert_eq!(shown_author(&None), None);
        assert_eq!(shown_author(&Some(String::new())), None);
        assert_eq!(shown_author(&Some("  ".into())), None);
        assert_eq!(shown_author(&Some("unknown".into())), None);
        assert_eq!(shown_author(&Some("Unknown".into())), None);
        assert_eq!(shown_author(&Some("UNKNOWN".into())), None);
        assert_eq!(shown_author(&Some(" Ana ".into())), Some("Ana"));
    }

    #[test]
    fn a_renamed_header_keeps_its_escaped_arrow_as_it_is() {
        let names = |n: &str| (vec![format!("Row \"Léa{n}\"")], Vec::new());
        let line = "<!-- comment @Ana 2026-11-24T10:02Z: Row \"Léa\": a \\↳ b -->";
        let rows = [Some(0)];
        let mut once =
            crate::markdown::retarget(&[line], &names(""), &names("2"), (&rows, &[]), false);
        let once = once.swap_remove(0).unwrap();
        assert_eq!(
            once,
            "<!-- comment @Ana 2026-11-24T10:02Z: Row \"Léa2\": a \\↳ b -->"
        );
        let mut twice = crate::markdown::retarget(
            &[once.as_str()],
            &names("2"),
            &names("3"),
            (&rows, &[]),
            false,
        );
        assert_eq!(
            twice.swap_remove(0).unwrap(),
            "<!-- comment @Ana 2026-11-24T10:02Z: Row \"Léa3\": a \\↳ b -->"
        );
    }
}
