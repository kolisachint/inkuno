//! Pending changes: an agent's proposed text written into the note as marked areas, until a person
//! accepts or rejects each one. The marks are CriticMarkup in block form, line by line:
//!
//! ```text
//! unchanged line
//! {--
//! old line 1
//! --}
//! {++
//! new line 1
//! ++}
//! unchanged line
//! ```
//!
//! One area is an optional removed block followed at once by an optional added block. A marker
//! line stands alone: its four strings are the only lines that count as marks, and only outside a
//! code fence (see below). Pure: no disk here.
//!
//! This is the form the editor shows. On disk the marks are not written: the note's file holds its
//! own text (`split`: the removed lines, without the added ones), and each area is an `Entry` kept
//! beside the note; `compose` writes the marks back from the text and the entries.
//!
//! Line breaks at the end of a text: a text may lack its final line break. When an area ends the
//! text, the line break just before its close mark (`\n`, or `\r\n` in a CRLF text) is not content
//! on either side: accept and reject take it off. So a reject gives back the note's text exactly,
//! and so does an accept of the agent's text. Where a line break had to be added between two lines,
//! it stays in the output. `propose` first gives the agent's text the note's final line break: it
//! adds one when the note has one, and takes one off when the note has none.
//!
//! Code fences: a fenced code block opens at a line of three or more backticks or tildes, and
//! closes at a line of the same character, at least as many. A mark line inside a fence is plain
//! text and starts no area. `propose` never puts a mark line inside a block: a change inside a fence
//! grows to the whole fenced block, and its marks stand outside the fence.

use std::fmt;
use std::ops::Range;

use crate::assistant;
use crate::merge;

const OPEN_DEL: &str = "{--";
const CLOSE_DEL: &str = "--}";
const OPEN_ADD: &str = "{++";
const CLOSE_ADD: &str = "++}";

/// One pending area of a text. The ranges are byte ranges of the text it was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    /// From the start of the first marker line to the end of the last one, with its line break.
    pub range: Range<usize>,
    /// The removed lines, without their marker lines. Empty when nothing is removed.
    pub removed: Range<usize>,
    /// The added lines, without their marker lines. Empty when nothing is added.
    pub added: Range<usize>,
}

/// Why a proposed change is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The proposed text is the one the agent read: there is nothing to change.
    NothingChanged,
    /// The note has pending changes already: they must be accepted or rejected first.
    AlreadyPending,
    /// A line of the text is one of the marks, so the marks would not be told apart from the text.
    HoldsMarks,
    /// A person changed the lines the agent changed, since the agent read them. `line` is the
    /// 1-based line of the current text.
    HumanChanged { line: usize },
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::NothingChanged => {
                write!(
                    f,
                    "nothing changed: the new text is the same as the old one"
                )
            }
            Refusal::AlreadyPending => write!(
                f,
                "the note has pending changes: accept or reject them in Inkuno first"
            ),
            Refusal::HoldsMarks => write!(
                f,
                "a line of the text is a pending-change mark ({{-- --}} {{++ ++}}): remove it first"
            ),
            Refusal::HumanChanged { line } => write!(
                f,
                "line {line} was changed since the agent read the note; read it again and retry"
            ),
        }
    }
}

/// A line of a text: its bytes with the line break, and the line without it.
struct Row<'a> {
    start: usize,
    end: usize,
    body: &'a str,
    /// Whether the line has its line break (only the last line of a text can lack it).
    broken: bool,
}

/// The lines of `text`, each with its byte range.
fn rows(text: &str) -> Vec<Row<'_>> {
    let mut out = Vec::new();
    let mut start = 0;
    for piece in text.split_inclusive('\n') {
        let end = start + piece.len();
        out.push(Row {
            start,
            end,
            body: bare(piece),
            broken: piece.ends_with('\n'),
        });
        start = end;
    }
    out
}

/// A line without its line break (`\n` or `\r\n`). A piece with no break is returned as it is.
fn bare(piece: &str) -> &str {
    match piece.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => piece,
    }
}

fn is_mark(body: &str) -> bool {
    matches!(body, OPEN_DEL | CLOSE_DEL | OPEN_ADD | CLOSE_ADD)
}

/// The row of the close marker `close` of the block opened at row `open`, or `None` when another
/// mark comes first or the text ends before it.
fn close_row(rows: &[Row], open: usize, close: &str) -> Option<usize> {
    for (k, row) in rows.iter().enumerate().skip(open + 1) {
        if row.body == close {
            return Some(k);
        }
        if is_mark(row.body) {
            return None;
        }
    }
    None
}

/// The area that starts at row `at`, and the row after it. `None` when row `at` does not start one.
fn area_from(text: &str, rows: &[Row], at: usize) -> Option<(Area, usize)> {
    let mut del: Option<(usize, usize)> = None;
    let mut add: Option<(usize, usize)> = None;
    let next = match rows[at].body {
        OPEN_DEL => {
            let close = close_row(rows, at, CLOSE_DEL)?;
            del = Some((at, close));
            let mut next = close + 1;
            // An added block must follow at once; if it is malformed, the area is the removed block.
            if rows.get(next).is_some_and(|r| r.body == OPEN_ADD)
                && let Some(close) = close_row(rows, next, CLOSE_ADD)
            {
                add = Some((next, close));
                next = close + 1;
            }
            next
        }
        OPEN_ADD => {
            let close = close_row(rows, at, CLOSE_ADD)?;
            add = Some((at, close));
            close + 1
        }
        _ => return None,
    };
    let first = del.or(add)?.0;
    let start = rows[first].start;
    let end = rows[next - 1].end;
    // The area ends the text when its last mark has no line break.
    let ends_text = !rows[next - 1].broken;
    let side = |block: Option<(usize, usize)>, empty_at: usize| -> Range<usize> {
        match block {
            None => empty_at..empty_at,
            Some((open, close)) => {
                let from = rows[open].end;
                let mut to = rows[close].start;
                // The area ends the text: the line break before a close mark is not content.
                if ends_text && to > from {
                    let bytes = text.as_bytes();
                    if to - from >= 2 && &bytes[to - 2..to] == b"\r\n" {
                        to -= 2;
                    } else if bytes[to - 1] == b'\n' {
                        to -= 1;
                    }
                }
                from..to
            }
        }
    };
    let area = Area {
        range: start..end,
        removed: side(del, start),
        added: side(add, end),
    };
    Some((area, next))
}

/// Every pending area of `text`, in order. An unclosed or malformed mark is plain text, and so is a
/// mark inside a code fence.
pub fn areas(text: &str) -> Vec<Area> {
    let rows = rows(text);
    let mut found = Vec::new();
    // The code fence that is open, if any. Only the rows outside areas move it.
    let mut fence: Option<(char, usize)> = None;
    let mut at = 0;
    while at < rows.len() {
        if let Some(open) = fence {
            if fence_closes(rows[at].body, open) {
                fence = None;
            }
            at += 1;
            continue;
        }
        match area_from(text, &rows, at) {
            Some((area, next)) => {
                found.push(area);
                at = next;
            }
            None => {
                fence = fence_opens(rows[at].body);
                at += 1;
            }
        }
    }
    found
}

/// Whether `text` has a pending change.
pub fn has_pending(text: &str) -> bool {
    !areas(text).is_empty()
}

/// The character and length of the code fence that `body` opens, if it opens one: three or more
/// backticks or tildes after up to three spaces. A backtick fence's info string holds no backtick.
fn fence_opens(body: &str) -> Option<(char, usize)> {
    let rest = body.trim_start_matches(' ');
    if body.len() - rest.len() > 3 {
        return None;
    }
    let ch = rest.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = rest.chars().take_while(|c| *c == ch).count();
    if len < 3 || (ch == '`' && rest[len..].contains('`')) {
        return None;
    }
    Some((ch, len))
}

/// Whether `body` closes the fence `open`: up to three spaces, then only its character, at least as
/// many as it opened with, then spaces and tabs.
fn fence_closes(body: &str, open: (char, usize)) -> bool {
    let rest = body.trim_start_matches(' ');
    if body.len() - rest.len() > 3 {
        return false;
    }
    let (ch, len) = open;
    let rest = rest.trim_end_matches([' ', '\t']);
    rest.len() >= len && rest.chars().all(|c| c == ch)
}

/// Whether a line of `text` outside a code fence is one of the four marks.
fn holds_marks(text: &str) -> bool {
    let mut fence: Option<(char, usize)> = None;
    for row in rows(text) {
        match fence {
            Some(open) => {
                if fence_closes(row.body, open) {
                    fence = None;
                }
            }
            None => {
                if is_mark(row.body) {
                    return true;
                }
                fence = fence_opens(row.body);
            }
        }
    }
    false
}

/// The index of the area whose range contains byte `offset`.
pub fn area_at(text: &str, offset: usize) -> Option<usize> {
    areas(text)
        .iter()
        .position(|area| area.range.start <= offset && offset < area.range.end)
}

/// Whether a line break starts `text`: `\n` or `\r\n`.
fn starts_with_break(text: &str) -> bool {
    text.starts_with('\n') || text.starts_with("\r\n")
}

/// Whether the edit that replaces the bytes `range` of `text` by `inserted` is refused. The one
/// rule of the note: the editor and the command line both ask it, and nothing else decides.
///
/// Refused when the edit changes a byte of an area, marker lines included. An edit that only
/// meets an area's edge is refused when it would leave a marker line without its own line:
/// - it ends where the area starts (an insertion, or a deletion, right before it): `inserted`
///   must end with a line break, or be empty with `range` starting a line (or the text);
/// - it starts where the area ends: the area's last line must keep its line end, so the area
///   must end with a line break, or `inserted` must start with one.
///
/// An insertion strictly inside an area is refused; an edit elsewhere is allowed.
///
/// Also refused when the edit would change the note's set of areas: a fence opener typed above an
/// area, or a closing fence line deleted above it, hides the area (its marks become plain text),
/// and a new mark pair typed in a note that has areas makes a new one. An edit that keeps every area
/// as it was (the same areas, in order, with the same lines) is allowed.
pub fn refuses(text: &str, range: Range<usize>, inserted: &str) -> bool {
    if range.is_empty() && inserted.is_empty() {
        return false;
    }
    let before = areas(text);
    if before
        .iter()
        .any(|area| edge_refused(text, area, &range, inserted))
    {
        return true;
    }
    if before.is_empty() {
        return false;
    }
    let edited = format!("{}{}{}", &text[..range.start], inserted, &text[range.end..]);
    !same_areas(text, &before, &edited)
}

/// Whether the edit that replaces `range` of `text` by `inserted` changes the bytes of `area`, or
/// meets its edge in a way that would leave a marker line without its own line.
fn edge_refused(text: &str, area: &Area, range: &Range<usize>, inserted: &str) -> bool {
    let r = &area.range;
    let changes = if range.is_empty() {
        r.start < range.start && range.start < r.end
    } else {
        range.start < r.end && r.start < range.end
    };
    if changes {
        return true;
    }
    if range.end == r.start {
        let line_start = range.start == 0 || text[..range.start].ends_with('\n');
        let kept = inserted.ends_with('\n') || (inserted.is_empty() && line_start);
        if !kept {
            return true;
        }
    }
    if range.start == r.end {
        let kept = text[r.clone()].ends_with('\n') || starts_with_break(inserted);
        if !kept {
            return true;
        }
    }
    false
}

/// Whether `edited` has the areas of `text`, in order: as many, and each with the same whole text,
/// removed lines and added lines. A line break at the end of a text does not count: an area that
/// ends the text takes its break in or out when a line is typed after it.
fn same_areas(text: &str, before: &[Area], edited: &str) -> bool {
    let after = areas(edited);
    before.len() == after.len()
        && before.iter().zip(&after).all(|(old, new)| {
            bare(&text[old.range.clone()]) == bare(&edited[new.range.clone()])
                && bare(&text[old.removed.clone()]) == bare(&edited[new.removed.clone()])
                && bare(&text[old.added.clone()]) == bare(&edited[new.added.clone()])
        })
}

/// Whether rewriting a note's text from `text` into `fresh` (its links renamed) changes a pending
/// change. Each changed line is one splice, checked with `refuses` as an edit is. So a link outside
/// every area does not count, however many links lie around the areas. A text whose line count
/// changed is one splice. The command line and the app both ask it before they rename a note.
pub fn rewrite_refused(text: &str, fresh: &str) -> bool {
    if !has_pending(text) {
        return false;
    }
    let before: Vec<&str> = text.split_inclusive('\n').collect();
    let after: Vec<&str> = fresh.split_inclusive('\n').collect();
    if before.len() != after.len() {
        return assistant::edit_between(text, fresh)
            .is_some_and(|(range, inserted)| refuses(text, range, &inserted));
    }
    let mut at = 0;
    for (old, new) in before.iter().zip(&after) {
        if old != new
            && let Some((range, inserted)) = assistant::edit_between(old, new)
            && refuses(text, at + range.start..at + range.end, &inserted)
        {
            return true;
        }
        at += old.len();
    }
    false
}

/// `text` with area `index` replaced by `keep` (a range of the text): the area's marker lines and
/// the lines of the other side go. An index with no area gives the text unchanged.
fn resolve_one(text: &str, index: usize, keep: fn(&Area) -> Range<usize>) -> String {
    match areas(text).get(index) {
        Some(area) => {
            let kept = &text[keep(area)];
            format!(
                "{}{}{}",
                &text[..area.range.start],
                kept,
                &text[area.range.end..]
            )
        }
        None => text.to_string(),
    }
}

/// `text` with every area replaced by the lines `keep` names.
fn resolve_all(text: &str, keep: fn(&Area) -> Range<usize>) -> String {
    let mut out = text.to_string();
    for area in areas(text).iter().rev() {
        out.replace_range(area.range.clone(), &text[keep(area)]);
    }
    out
}

fn added_of(area: &Area) -> Range<usize> {
    area.added.clone()
}

fn removed_of(area: &Area) -> Range<usize> {
    area.removed.clone()
}

/// The most bytes of the note's text on each side of an entry kept to find it again: the line on
/// that side, cut to this length.
const CONTEXT: usize = 120;

/// One pending change kept beside its note (in the `.inkuno` folder, never in the `.md`). The note
/// holds `old` at byte `at`: the lines the change removes, which are still the note's text. `new`
/// is what it becomes once accepted. `id` and `author` are kept as they were written.
///
/// `at` is only a hint: the note may change outside Inkuno after the entry was saved. `before` and
/// `after` are the lines around the anchor when it was saved (see `CONTEXT`). `compose` finds the
/// entry again by them, and leaves it out when neither is found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Entry {
    pub id: u32,
    pub author: String,
    pub at: usize,
    pub old: String,
    pub new: String,
    pub before: String,
    pub after: String,
}

/// The note text as it is on disk, and the pending changes beside it: `display` (the text with
/// its marks, as the editor shows it) is split into the note's own text, the removed lines
/// (`reject_all`) and one entry per area. An entry takes the `id` and `author` of the `known`
/// entry with the same `old` and `new`, if there is one; otherwise it is numbered after the
/// largest id known, with no author. The note text has no mark in it. Each entry carries the text
/// around its anchor, so `compose` can find it again.
pub fn split(display: &str, known: &[Entry]) -> (String, Vec<Entry>) {
    let (base, mut entries) = unmark(display);
    let mut used = vec![false; known.len()];
    let mut next = known.iter().map(|e| e.id).max().unwrap_or(0);
    for entry in &mut entries {
        let same = known
            .iter()
            .enumerate()
            .find(|(i, k)| !used[*i] && k.old == entry.old && k.new == entry.new);
        match same {
            Some((i, k)) => {
                used[i] = true;
                entry.id = k.id;
                entry.author = k.author.clone();
            }
            None => {
                next += 1;
                entry.id = next;
            }
        }
        entry.before = context_before(&base, entry.at);
        entry.after = context_after(&base, entry.at + entry.old.len());
    }
    (base, entries)
}

/// The note's own text and one entry per area of `display`, without ids or context.
fn unmark(display: &str) -> (String, Vec<Entry>) {
    let mut base = String::with_capacity(display.len());
    let mut entries: Vec<Entry> = Vec::new();
    let mut last = 0;
    for area in areas(display) {
        base.push_str(&display[last..area.range.start]);
        let at = base.len();
        let (old, new) = (&display[area.removed.clone()], &display[area.added.clone()]);
        base.push_str(old);
        entries.push(Entry {
            at,
            old: old.to_string(),
            new: new.to_string(),
            ..Entry::default()
        });
        last = area.range.end;
    }
    base.push_str(&display[last..]);
    (base, entries)
}

/// The line of `base` before `at`, its break included and cut to `CONTEXT` bytes (from the end, on a
/// character boundary). Empty at the start of the text.
fn context_before(base: &str, at: usize) -> String {
    if at == 0 {
        return String::new();
    }
    let start = base.as_bytes()[..at - 1]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    let mut from = start.max(at.saturating_sub(CONTEXT));
    while !base.is_char_boundary(from) {
        from += 1;
    }
    base.get(from..at).unwrap_or_default().to_string()
}

/// The line of `base` from `end`, its break included and cut to `CONTEXT` bytes (on a character
/// boundary). Empty at the end of the text.
fn context_after(base: &str, end: usize) -> String {
    let line_end = base
        .get(end..)
        .and_then(|rest| rest.find('\n'))
        .map_or(base.len(), |i| end + i + 1);
    let mut to = line_end.min(end + CONTEXT).min(base.len());
    while !base.is_char_boundary(to) {
        to -= 1;
    }
    base.get(end..to).unwrap_or_default().to_string()
}

/// Whether a line of `text` is one of the four marks, inside a code fence or not.
fn has_mark_line(text: &str) -> bool {
    rows(text).iter().any(|row| is_mark(row.body))
}

/// The byte positions of `base` where a line starts outside every code fence, the end of the text
/// included when it is outside one too. Sorted, and every one is a character boundary.
fn clean_starts(base: &str) -> Vec<usize> {
    let mut out = Vec::new();
    let mut fence: Option<(char, usize)> = None;
    for row in rows(base) {
        match fence {
            None => {
                out.push(row.start);
                fence = fence_opens(row.body);
            }
            Some(open) => {
                if fence_closes(row.body, open) {
                    fence = None;
                }
            }
        }
    }
    if fence.is_none() {
        out.push(base.len());
    }
    out
}

/// The offset of the candidate nearest to `at`.
fn nearest(at: usize, candidates: impl Iterator<Item = usize>) -> Option<usize> {
    candidates.min_by_key(|p| p.abs_diff(at))
}

/// Where `entry` is in `base`, or `None` when it cannot be placed cleanly. A removed text must be
/// whole lines outside code fences, and an insertion must sit at a line start outside them; so a
/// mark never splits a line or lands in a fence. With context, the place is one where the text
/// before or after the anchor still holds (both, when both are kept), the nearest to `at`. Without
/// any (an entry saved before context was kept), the removed text at `at`, or the nearest one, and
/// an insertion only at `at`.
fn place(base: &str, clean: &[usize], entry: &Entry) -> Option<usize> {
    if has_mark_line(&entry.old) || has_mark_line(&entry.new) {
        return None;
    }
    let len = entry.old.len();
    let at = entry.at.min(base.len());
    let candidates = clean.iter().copied().filter(|&p| {
        entry.old.is_empty()
            || (base
                .get(p..)
                .is_some_and(|rest| rest.starts_with(&entry.old))
                && clean.binary_search(&(p + len)).is_ok())
    });
    if entry.before.is_empty() && entry.after.is_empty() {
        return match entry.old.is_empty() {
            true => clean.binary_search(&at).is_ok().then_some(at),
            false => nearest(at, candidates),
        };
    }
    // A removed text that is in the note once is its place, whatever became of its context.
    if !entry.old.is_empty() {
        let mut found = candidates.clone();
        let first = found.next();
        if first.is_some() && found.next().is_none() {
            return first;
        }
    }
    // Each side holds when the context is still there on that side of the candidate.
    let found: Vec<(usize, bool, bool)> = candidates
        .filter_map(|p| {
            let before = !entry.before.is_empty()
                && base.get(..p).is_some_and(|t| t.ends_with(&entry.before));
            let after = !entry.after.is_empty()
                && base
                    .get(p + len..)
                    .is_some_and(|t| t.starts_with(&entry.after));
            (before || after).then_some((p, before, after))
        })
        .collect();
    let full = found
        .iter()
        .filter(|&&(_, b, a)| (entry.before.is_empty() || b) && (entry.after.is_empty() || a));
    nearest(at, full.map(|&(p, _, _)| p)).or_else(|| nearest(at, found.iter().map(|&(p, _, _)| p)))
}

/// Where each entry of `entries` is placed in `base`, if it is: the `(offset, index)` pairs of the
/// placed ones, in order. Entries are added one at a time, and an entry that would make the text
/// `split` does not read back as `base` with these entries is left out.
fn placements(base: &str, entries: &[Entry]) -> Vec<(usize, usize)> {
    let clean = clean_starts(base);
    let mut found: Vec<(usize, usize)> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| !(e.old.is_empty() && e.new.is_empty()))
        .filter_map(|(i, e)| place(base, &clean, e).map(|at| (at, i)))
        .collect();
    found.sort();
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for candidate in found {
        kept.push(candidate);
        if !reads_back(base, &build(base, &kept, entries), &kept, entries) {
            kept.pop();
        }
    }
    kept
}

/// Whether `text` reads back, through `unmark`, as `base` with the `kept` entries at their places.
fn reads_back(base: &str, text: &str, kept: &[(usize, usize)], entries: &[Entry]) -> bool {
    let (back, got) = unmark(text);
    back == base
        && got.len() == kept.len()
        && got
            .iter()
            .zip(kept)
            .all(|(g, &(at, i))| g.at == at && g.old == entries[i].old && g.new == entries[i].new)
}

/// The text with the `placed` entries (ordered, as `placements` gives them) written as areas.
fn build(base: &str, placed: &[(usize, usize)], entries: &[Entry]) -> String {
    let eol = if base.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = String::with_capacity(base.len() + 64);
    let mut last = 0;
    for &(at, i) in placed {
        let e = &entries[i];
        let end = at + e.old.len();
        if at < last {
            continue;
        }
        out.push_str(&base[last..at]);
        start_line(&mut out, eol);
        // The area ends a text that has no final line break: its last mark has none either.
        let ends_text = end == base.len() && !base.ends_with('\n');
        if !e.old.is_empty() {
            let omit = ends_text && e.new.is_empty();
            out.push_str(&block("{--", &[e.old.as_str()], "--}", omit, eol));
        }
        if !e.new.is_empty() {
            out.push_str(&block("{++", &[e.new.as_str()], "++}", ends_text, eol));
        }
        last = end;
    }
    out.push_str(&base[last..]);
    out
}

/// The text with its pending changes written as areas, as the editor shows them: the inverse of
/// `split`. An entry that cannot be placed cleanly, or whose text is gone from `base`, is left out.
pub fn compose(base: &str, entries: &[Entry]) -> String {
    build(base, &placements(base, entries), entries)
}

/// Whether each of `entries` is placed in `base`, as `compose` shows it. A placed entry's marks are
/// in the composed text; a left-out one is kept beside the note, shown nowhere.
pub fn placed(base: &str, entries: &[Entry]) -> Vec<bool> {
    let kept = placements(base, entries);
    (0..entries.len())
        .map(|i| kept.iter().any(|&(_, k)| k == i))
        .collect()
}

/// A fragment of the editor's text as it is copied: the marks go, the removed lines go, the added
/// lines stay, so that a copy never holds a mark and is the text the note will have once accepted.
pub fn plain(fragment: &str) -> String {
    let mut out = String::with_capacity(fragment.len());
    let mut removed = false;
    for piece in fragment.split_inclusive('\n') {
        match bare(piece) {
            OPEN_DEL => removed = true,
            CLOSE_DEL => removed = false,
            OPEN_ADD | CLOSE_ADD => {}
            _ if removed => {}
            _ => out.push_str(piece),
        }
    }
    out
}

/// Area `index` accepted: its removed lines go, its added lines stay, the marks go.
pub fn accept(text: &str, index: usize) -> String {
    resolve_one(text, index, added_of)
}

/// Area `index` rejected: its added lines go, its removed lines stay, the marks go.
pub fn reject(text: &str, index: usize) -> String {
    resolve_one(text, index, removed_of)
}

/// Every area accepted: the agent's text.
pub fn accept_all(text: &str) -> String {
    resolve_all(text, added_of)
}

/// Every area rejected: the person's text.
pub fn reject_all(text: &str) -> String {
    resolve_all(text, removed_of)
}

/// Adds a line break when the text so far ends without one, so that a mark starts its own line.
fn start_line(out: &mut String, eol: &str) {
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(eol);
    }
}

/// One block: its open mark, its lines, its close mark. `omit_break` leaves off the line break
/// after the close mark (it ends the text), and then a line break is always written before the
/// close mark, which accept and reject take off again.
fn block(open: &str, lines: &[&str], close: &str, omit_break: bool, eol: &str) -> String {
    let mut out = format!("{open}{eol}");
    for line in lines {
        out.push_str(line);
    }
    if omit_break || !out.ends_with('\n') {
        out.push_str(eol);
    }
    out.push_str(close);
    if !omit_break {
        out.push_str(eol);
    }
    out
}

/// The text with the agent's change to `base` written as pending areas on top of `current`.
///
/// `base` is the text the agent read, `current` the note now, `proposed` the agent's full new text.
/// A change of the agent that touches lines a person changed is refused. Otherwise the result has
/// every agent change as an area, placed where it belongs in `current`.
pub fn propose(base: &str, current: &str, proposed: &str) -> Result<String, Refusal> {
    if has_pending(current) {
        return Err(Refusal::AlreadyPending);
    }
    if holds_marks(current) || holds_marks(proposed) {
        return Err(Refusal::HoldsMarks);
    }
    // The marks and the added breaks use the note's own line break: CRLF if the text has one.
    let eol = if current.contains("\r\n") || proposed.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let normal = final_break_as(current, proposed, eol);
    let proposed = normal.as_str();
    if proposed == base {
        return Err(Refusal::NothingChanged);
    }
    let (b, c, p) = (
        merge::lines(base),
        merge::lines(current),
        merge::lines(proposed),
    );
    let human = merge::hunks(&b, &c);
    let agent = merge::hunks(&b, &p);
    for a in &agent {
        let clash = human
            .iter()
            .find(|h| a.base.start <= h.base.end && h.base.start <= a.base.end);
        if let Some(h) = clash {
            let line = h.side.start.min(c.len().saturating_sub(1)) + 1;
            return Err(Refusal::HumanChanged { line });
        }
    }
    // Where a base line sits in `current`: the person's changes before it move it.
    let shift = |x: usize| -> isize {
        human
            .iter()
            .filter(|h| h.base.end <= x)
            .map(|h| h.side.len() as isize - h.base.len() as isize)
            .sum()
    };
    // `m` is `current` with every agent hunk applied: the merged text. Each hunk is placed in
    // `current` (`cr`) and in `m` (`mr`).
    let mut m: Vec<&str> = Vec::new();
    let mut placed: Vec<(Range<usize>, Range<usize>)> = Vec::new();
    let mut at = 0;
    for a in &agent {
        let cs = (a.base.start as isize + shift(a.base.start)) as usize;
        let ce = cs + a.base.len();
        m.extend_from_slice(&c[at..cs]);
        let ms = m.len();
        m.extend_from_slice(&p[a.side.clone()]);
        placed.push((cs..ce, ms..m.len()));
        at = ce;
    }
    m.extend_from_slice(&c[at..]);
    // The cuts: where `current` and `m` are in step. Every position of an unchanged run is one, and
    // so are the two ends of the text.
    let mut cuts: Vec<(usize, usize)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    for (cr, mr) in &placed {
        for t in 0..=(cr.start - i) {
            cuts.push((i + t, j + t));
        }
        (i, j) = (cr.end, mr.end);
    }
    for t in 0..=(c.len() - i) {
        cuts.push((i + t, j + t));
    }
    cuts.dedup();
    // A cut is clean where neither text is inside a code fence. Every area lies between two clean
    // cuts, so no area starts or ends inside a fence.
    let (inside_c, inside_m) = (fence_positions(&c), fence_positions(&m));
    let clean: Vec<(usize, usize)> = cuts
        .into_iter()
        .filter(|&(i, j)| {
            (i == 0 && j == 0) || (i == c.len() && j == m.len()) || (!inside_c[i] && !inside_m[j])
        })
        .collect();
    let mut out = String::new();
    for pair in clean.windows(2) {
        let ((i, j), (i2, j2)) = (pair[0], pair[1]);
        let (removed, added) = (&c[i..i2], &m[j..j2]);
        if removed == added {
            for line in removed {
                out.push_str(line);
            }
            continue;
        }
        if removed.iter().chain(added).any(|line| is_mark(bare(line))) {
            return Err(Refusal::HoldsMarks);
        }
        // This area ends the text when nothing of `current` follows it.
        let ends = i2 == c.len();
        let omit = ends
            && if added.is_empty() {
                !ends_broken(removed)
            } else {
                !ends_broken(added)
            };
        if !removed.is_empty() {
            start_line(&mut out, eol);
            out.push_str(&block(
                OPEN_DEL,
                removed,
                CLOSE_DEL,
                omit && added.is_empty(),
                eol,
            ));
        }
        if !added.is_empty() {
            start_line(&mut out, eol);
            out.push_str(&block(OPEN_ADD, added, CLOSE_ADD, omit, eol));
        }
    }
    Ok(out)
}

/// Whether the last line of `lines` has its line break.
fn ends_broken(lines: &[&str]) -> bool {
    lines.last().is_some_and(|line| line.ends_with('\n'))
}

/// `proposed` with its final line break as `current` has it: one added (`eol`) when `current` has
/// one, one taken off when it has none. An empty text has no final break to change, and an empty
/// `current` keeps `proposed` as given.
fn final_break_as(current: &str, proposed: &str, eol: &str) -> String {
    if current.is_empty()
        || proposed.is_empty()
        || current.ends_with('\n') == proposed.ends_with('\n')
    {
        return proposed.to_string();
    }
    if current.ends_with('\n') {
        format!("{proposed}{eol}")
    } else {
        let cut = proposed
            .strip_suffix("\r\n")
            .or_else(|| proposed.strip_suffix('\n'));
        cut.unwrap_or(proposed).to_string()
    }
}

/// For each position before a line of `lines`, and after the last, whether it is inside an open
/// code fence of the text. A position after an opening line is inside; one after the closing line
/// is outside.
fn fence_positions(lines: &[&str]) -> Vec<bool> {
    let mut inside = Vec::with_capacity(lines.len() + 1);
    let mut fence: Option<(char, usize)> = None;
    for line in lines {
        inside.push(fence.is_some());
        let body = bare(line);
        fence = match fence {
            Some(open) if fence_closes(body, open) => None,
            Some(open) => Some(open),
            None => fence_opens(body),
        };
    }
    inside.push(fence.is_some());
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checks that rejecting every area gives `current` and accepting gives `merged`.
    fn both_ways(marked: &str, current: &str, merged: &str) {
        assert_eq!(reject_all(marked), current, "reject of {marked:?}");
        assert_eq!(accept_all(marked), merged, "accept of {marked:?}");
    }

    #[test]
    fn a_change_is_one_area_with_both_blocks() {
        let text = "keep\n{--\nold\n--}\n{++\nnew\n++}\nend\n";
        let found = areas(text);
        assert_eq!(found.len(), 1);
        assert_eq!(&text[found[0].removed.clone()], "old\n");
        assert_eq!(&text[found[0].added.clone()], "new\n");
        assert_eq!(
            &text[found[0].range.clone()],
            "{--\nold\n--}\n{++\nnew\n++}\n"
        );
    }

    #[test]
    fn a_pure_insertion_has_only_the_added_block() {
        let text = "a\n{++\nnew\n++}\nb\n";
        let found = areas(text);
        assert_eq!(found.len(), 1);
        assert!(found[0].removed.is_empty());
        assert_eq!(&text[found[0].added.clone()], "new\n");
        assert_eq!(&text[found[0].range.clone()], "{++\nnew\n++}\n");
    }

    #[test]
    fn a_pure_deletion_has_only_the_removed_block() {
        let text = "a\n{--\ngone\n--}\nb\n";
        let found = areas(text);
        assert_eq!(found.len(), 1);
        assert_eq!(&text[found[0].removed.clone()], "gone\n");
        assert!(found[0].added.is_empty());
        assert_eq!(&text[found[0].range.clone()], "{--\ngone\n--}\n");
    }

    #[test]
    fn an_area_range_takes_the_line_break_of_its_last_mark() {
        let text = "x\n{++\ny\n++}";
        let found = areas(text);
        assert_eq!(found[0].range, 2..text.len());
    }

    #[test]
    fn a_text_without_marks_has_no_areas() {
        assert!(areas("plain\ntext\n").is_empty());
        assert!(areas("").is_empty());
        assert!(!has_pending("plain\n"));
        assert!(has_pending("{++\nx\n++}\n"));
    }

    #[test]
    fn an_unclosed_mark_is_plain_text() {
        assert!(areas("{++\nnew\n").is_empty());
        assert!(areas("{--\nold\n").is_empty());
        assert!(areas("a\n++}\nb\n").is_empty());
    }

    #[test]
    fn a_mark_inside_a_block_makes_the_block_malformed() {
        // A removed block cut by an added opener is not an area; the opener is plain text.
        let text = "{--\nold\n{++\nnew\n++}\n";
        let found = areas(text);
        assert_eq!(found.len(), 1);
        assert_eq!(&text[found[0].added.clone()], "new\n");
        assert!(found[0].removed.is_empty());
    }

    #[test]
    fn a_removed_block_without_its_added_block_is_its_own_area() {
        let text = "{--\nold\n--}\n{++\nnever closed\n";
        let found = areas(text);
        assert_eq!(found.len(), 1);
        assert_eq!(&text[found[0].removed.clone()], "old\n");
        assert!(found[0].added.is_empty());
        assert_eq!(found[0].range, 0.."{--\nold\n--}\n".len());
    }

    #[test]
    fn two_areas_are_both_found_in_order() {
        let text = "{++\nA\n++}\nmid\n{--\nB\n--}\n";
        let found = areas(text);
        assert_eq!(found.len(), 2);
        assert!(found[0].range.end <= found[1].range.start);
        assert_eq!(&text[found[1].removed.clone()], "B\n");
    }

    #[test]
    fn accepting_one_area_leaves_the_other() {
        let text = "{++\nA\n++}\nmid\n{--\nB\n--}\n";
        assert_eq!(accept(text, 0), "A\nmid\n{--\nB\n--}\n");
        assert_eq!(reject(text, 0), "mid\n{--\nB\n--}\n");
        assert_eq!(accept(text, 1), "{++\nA\n++}\nmid\n");
        assert_eq!(reject(text, 1), "{++\nA\n++}\nmid\nB\n");
        // An index with no area changes nothing.
        assert_eq!(accept(text, 5), text);
    }

    #[test]
    fn accept_all_and_reject_all_give_both_texts() {
        let text = "t\n{--\nold\n--}\n{++\nnew\n++}\nm\n{++\nadded\n++}\nz\n";
        both_ways(text, "t\nold\nm\nz\n", "t\nnew\nm\nadded\nz\n");
    }

    #[test]
    fn area_at_finds_the_area_of_an_offset() {
        let text = "a\n{++\nx\n++}\nb\n{--\ny\n--}\n";
        let second = text.find("{--").unwrap();
        assert_eq!(area_at(text, 0), None);
        assert_eq!(area_at(text, 2), Some(0));
        assert_eq!(area_at(text, text.find("x").unwrap()), Some(0));
        assert_eq!(area_at(text, second), Some(1));
        assert_eq!(area_at(text, text.len()), None);
    }

    /// Where `needle` first occurs in `text`.
    fn at(text: &str, needle: &str) -> usize {
        text.find(needle).unwrap()
    }

    #[test]
    fn an_edit_inside_an_area_is_refused() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let inside = at(text, "yz");
        assert!(refuses(text, inside..inside + 1, "q"));
        assert!(refuses(text, inside..inside, "q"));
    }

    #[test]
    fn an_edit_of_a_marker_line_is_refused() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let mark = at(text, "++}");
        assert!(refuses(text, mark..mark + 1, "+"));
        let open = at(text, "{++");
        assert!(refuses(text, open..open + 1, "{"));
    }

    #[test]
    fn a_letter_typed_at_the_start_of_the_marker_line_is_refused() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let start = at(text, "{++");
        assert!(refuses(text, start..start, "x"));
    }

    #[test]
    fn a_line_break_typed_at_the_start_of_the_marker_line_is_allowed() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let start = at(text, "{++");
        assert!(!refuses(text, start..start, "\n"));
        assert!(!refuses(text, start..start, "\r\n"));
    }

    #[test]
    fn a_letter_typed_on_the_line_above_the_area_is_allowed() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        assert!(!refuses(text, 1..1, "x"));
    }

    #[test]
    fn a_letter_typed_after_the_area_line_break_is_allowed() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let after = at(text, "b\n");
        assert!(!refuses(text, after..after, "x"));
    }

    #[test]
    fn a_letter_typed_at_the_end_without_a_final_break_is_refused() {
        let text = "a\n{++\nx\n++}";
        assert!(refuses(text, text.len()..text.len(), "x"));
        assert!(refuses(text, text.len()..text.len(), "y z"));
    }

    #[test]
    fn a_line_break_typed_at_the_end_without_a_final_break_is_allowed() {
        let text = "a\n{++\nx\n++}";
        assert!(!refuses(text, text.len()..text.len(), "\n"));
        assert!(!refuses(text, text.len()..text.len(), "\n\nfoo\n"));
        assert!(!refuses(text, text.len()..text.len(), "\r\nfoo"));
    }

    #[test]
    fn a_letter_typed_at_the_end_with_a_final_break_is_allowed() {
        let text = "a\n{++\nx\n++}\n";
        assert!(!refuses(text, text.len()..text.len(), "x"));
    }

    #[test]
    fn a_note_that_starts_with_an_area_refuses_a_letter_at_byte_zero() {
        let text = "{++\nx\n++}\nb\n";
        assert!(refuses(text, 0..0, "y"));
        assert!(!refuses(text, 0..0, "y\n"));
    }

    #[test]
    fn deleting_the_whole_line_above_an_area_is_allowed() {
        let text = "a\nb\n{++\nx\n++}\n";
        let line = at(text, "b\n")..at(text, "{++");
        assert!(!refuses(text, line, ""));
    }

    #[test]
    fn deleting_the_break_of_the_line_above_an_area_is_refused() {
        let text = "a\nb\n{++\nx\n++}\n";
        let brk = at(text, "\n{++");
        assert!(refuses(text, brk..brk + 1, ""));
    }

    #[test]
    fn replacing_the_line_above_an_area_keeping_its_break_is_allowed() {
        let text = "a\nb\n{++\nx\n++}\n";
        let line = at(text, "b\n")..at(text, "{++");
        assert!(!refuses(text, line.clone(), "c\n"));
        assert!(refuses(text, line, "c"));
    }

    #[test]
    fn an_edit_elsewhere_in_the_note_is_allowed() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        assert!(!refuses(text, 0..0, "y"));
        let b = at(text, "b\n");
        assert!(!refuses(text, b..b + 1, "c"));
    }

    #[test]
    fn an_edit_that_spans_an_area_is_refused() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        assert!(refuses(text, 0..text.len(), ""));
    }

    #[test]
    fn a_crlf_text_keeps_its_marker_lines_alone() {
        let text = "a\r\n{++\r\nx\r\n++}\r\nb\r\n";
        let start = at(text, "{++");
        assert!(refuses(text, start..start, "y"));
        assert!(!refuses(text, start..start, "y\r\n"));
        let after = at(text, "b\r\n");
        assert!(!refuses(text, after..after, "c"));
        assert!(!refuses(text, 0..start, ""));
    }

    #[test]
    fn a_crlf_text_without_a_final_break_keeps_its_line_end() {
        let text = "a\r\n{++\r\nx\r\n++}";
        assert!(refuses(text, text.len()..text.len(), "x"));
        assert!(!refuses(text, text.len()..text.len(), "\r\nfoo"));
    }

    #[test]
    fn an_empty_edit_is_never_refused() {
        let text = "a\n{++\nxyz\n++}";
        assert!(!refuses(text, 2..2, ""));
        assert!(!refuses(text, text.len()..text.len(), ""));
    }

    #[test]
    fn a_note_without_areas_refuses_nothing() {
        assert!(!refuses("plain\n", 2..2, "x"));
        assert!(!refuses("", 0..0, "x"));
    }

    #[test]
    fn a_fence_opener_typed_above_an_area_is_refused() {
        let text = "a\n{--\nold\n--}\n";
        assert!(refuses(text, 2..2, "```\n"));
    }

    #[test]
    fn deleting_a_closing_fence_line_above_an_area_is_refused() {
        let text = "```\ncode\n```\n{--\nold\n--}\n";
        let closer = at(text, "```\n{");
        assert!(refuses(text, closer..closer + 4, ""));
    }

    #[test]
    fn typing_a_fence_opener_and_its_closer_above_an_area_is_allowed() {
        let text = "a\n{--\nold\n--}\n";
        assert!(!refuses(text, 2..2, "```\n```\n"));
    }

    #[test]
    fn a_new_mark_pair_typed_above_an_area_is_refused() {
        let text = "a\n{++\nx\n++}\n";
        assert!(refuses(text, 2..2, "{--\nold\n--}\n"));
    }

    #[test]
    fn a_fence_opener_typed_below_the_last_area_is_allowed() {
        let text = "a\n{++\nx\n++}\nb\n";
        assert!(!refuses(text, text.len()..text.len(), "```\n"));
    }

    #[test]
    fn ordinary_edits_above_between_and_below_areas_are_allowed() {
        let text = "a\n{++\nx\n++}\nmid\n{--\nold\n--}\nb\n";
        assert!(!refuses(text, 0..0, "y"));
        let mid = at(text, "mid") + 1;
        assert!(!refuses(text, mid..mid, "q"));
        let below = at(text, "b\n") + 1;
        assert!(!refuses(text, below..below, "z"));
    }

    #[test]
    fn a_rewritten_link_inside_an_area_is_refused_and_one_outside_is_not() {
        let text = "[[Old]] x\n{++\n[[Old]]\n++}\n";
        let outside = "[[New]] x\n{++\n[[Old]]\n++}\n";
        let inside = "[[Old]] x\n{++\n[[New]]\n++}\n";
        assert!(!rewrite_refused(text, outside));
        assert!(rewrite_refused(text, inside));
        assert!(!rewrite_refused("[[Old]]\n", "[[New]]\n"));
    }

    #[test]
    fn propose_refuses_while_a_change_is_pending() {
        let current = "a\n{++\nx\n++}\n";
        assert_eq!(propose("a\n", current, "b\n"), Err(Refusal::AlreadyPending));
    }

    #[test]
    fn propose_refuses_a_mark_line_in_the_proposed_text() {
        assert_eq!(propose("a\n", "a\n", "a\n{--\n"), Err(Refusal::HoldsMarks));
        assert_eq!(propose("a\n", "a\n", "x\n--}\n"), Err(Refusal::HoldsMarks));
    }

    #[test]
    fn propose_refuses_a_mark_line_already_in_the_note() {
        assert_eq!(propose("a\n", "a\n++}\n", "b\n"), Err(Refusal::HoldsMarks));
    }

    #[test]
    fn propose_refuses_a_text_nothing_changed_in() {
        assert_eq!(
            propose("same\n", "same\n", "same\n"),
            Err(Refusal::NothingChanged)
        );
    }

    #[test]
    fn propose_marks_the_agent_change_when_base_is_current() {
        let marked = propose("a\nb\nc\n", "a\nb\nc\n", "a\nB\nc\n").unwrap();
        assert_eq!(marked, "a\n{--\nb\n--}\n{++\nB\n++}\nc\n");
        both_ways(&marked, "a\nb\nc\n", "a\nB\nc\n");
    }

    #[test]
    fn propose_keeps_a_change_of_the_person_and_marks_the_agent_one() {
        let base = "t\nm\nb\n";
        let current = "T!\nm\nb\n";
        let proposed = "t\nm\nB!\n";
        let marked = propose(base, current, proposed).unwrap();
        both_ways(&marked, current, "T!\nm\nB!\n");
    }

    #[test]
    fn propose_refuses_a_change_on_lines_the_person_changed() {
        let base = "a\nb\nc\n";
        let current = "a\nmine\nc\n";
        let proposed = "a\ntheirs\nc\n";
        assert_eq!(
            propose(base, current, proposed),
            Err(Refusal::HumanChanged { line: 2 })
        );
    }

    #[test]
    fn propose_refuses_an_insertion_next_to_a_person_change() {
        let base = "a\nb\n";
        let current = "a\nmine\nb\n";
        let proposed = "a\nnew\nb\n";
        assert!(matches!(
            propose(base, current, proposed),
            Err(Refusal::HumanChanged { .. })
        ));
    }

    #[test]
    fn a_refusal_names_its_reason_in_one_sentence() {
        assert!(
            Refusal::AlreadyPending
                .to_string()
                .contains("accept or reject")
        );
        assert!(
            Refusal::HumanChanged { line: 7 }
                .to_string()
                .starts_with("line 7 ")
        );
        assert!(Refusal::NothingChanged.to_string().contains("same"));
        assert!(Refusal::HoldsMarks.to_string().contains("mark"));
    }

    #[test]
    fn invariants_hold_for_a_change_on_the_first_line() {
        let marked = propose("a\nb\n", "a\nb\n", "A\nb\n").unwrap();
        both_ways(&marked, "a\nb\n", "A\nb\n");
    }

    #[test]
    fn invariants_hold_for_a_change_on_the_last_line_with_a_final_break() {
        let marked = propose("a\nb\n", "a\nb\n", "a\nB\n").unwrap();
        both_ways(&marked, "a\nb\n", "a\nB\n");
    }

    #[test]
    fn invariants_hold_for_an_insertion_at_the_end_with_a_final_break() {
        let marked = propose("a\nb\n", "a\nb\n", "a\nb\nc\n").unwrap();
        both_ways(&marked, "a\nb\n", "a\nb\nc\n");
    }

    #[test]
    fn invariants_hold_for_an_insertion_at_the_start_of_an_empty_note() {
        let marked = propose("", "", "x\n").unwrap();
        both_ways(&marked, "", "x\n");
    }

    #[test]
    fn invariants_hold_for_a_deletion_of_everything() {
        let marked = propose("a\nb\n", "a\nb\n", "").unwrap();
        both_ways(&marked, "a\nb\n", "");
    }

    #[test]
    fn a_deletion_at_the_end_of_a_text_without_final_break_is_exact_both_ways() {
        let marked = propose("a\nb", "a\nb", "a").unwrap();
        both_ways(&marked, "a\nb", "a");
    }

    #[test]
    fn invariants_hold_for_an_added_last_line_without_break() {
        // The agent's text lacks the final break, but the note has one: the text gets it.
        let marked = propose("a\n", "a\n", "a\nb").unwrap();
        both_ways(&marked, "a\n", "a\nb\n");
    }

    #[test]
    fn a_text_without_a_final_break_is_exact_both_ways_when_changed_at_its_end() {
        let marked = propose("one", "one", "two").unwrap();
        both_ways(&marked, "one", "two");
    }

    #[test]
    fn crlf_breaks_are_kept_as_they_are() {
        let marked = propose("a\r\nb\r\n", "a\r\nb\r\n", "a\r\nB\r\n").unwrap();
        assert_eq!(marked, "a\r\n{--\r\nb\r\n--}\r\n{++\r\nB\r\n++}\r\n");
        both_ways(&marked, "a\r\nb\r\n", "a\r\nB\r\n");
    }

    #[test]
    fn a_text_with_unicode_lines_keeps_its_bytes() {
        let marked = propose("Ünï\n日本\n", "Ünï\n日本\n", "Ünï\n🦀\n").unwrap();
        both_ways(&marked, "Ünï\n日本\n", "Ünï\n🦀\n");
    }

    #[test]
    fn an_agent_change_next_to_a_person_change_keeps_both() {
        let base = "l1\nl2\nl3\nl4\n";
        let current = "L1!\nl2\nl3\nl4\n";
        let proposed = "l1\nl2\nl3\nL4!\n";
        let marked = propose(base, current, proposed).unwrap();
        both_ways(&marked, current, "L1!\nl2\nl3\nL4!\n");
    }

    #[test]
    fn marks_inside_a_backtick_fence_are_plain_text() {
        let text = "a\n```\n{++\nx\n++}\n```\nb\n";
        assert!(areas(text).is_empty());
        assert!(!has_pending(text));
        let x = at(text, "x\n");
        assert!(!refuses(text, x..x + 1, "y"));
    }

    #[test]
    fn marks_inside_a_tilde_fence_are_plain_text() {
        let text = "a\n~~~\n{--\nold\n--}\n~~~\nb\n";
        assert!(areas(text).is_empty());
        assert!(!has_pending(text));
        let old = at(text, "old");
        assert!(!refuses(text, old..old + 1, "o"));
    }

    #[test]
    fn a_real_area_before_and_after_a_fence_is_found_and_the_fence_is_not() {
        let text = "{++\nA\n++}\n```\n{--\nx\n--}\n```\n{--\nB\n--}\n";
        let found = areas(text);
        assert_eq!(found.len(), 2);
        assert_eq!(&text[found[0].added.clone()], "A\n");
        assert_eq!(&text[found[1].removed.clone()], "B\n");
        assert_eq!(found[1].range.end, text.len());
    }

    #[test]
    fn a_mark_like_line_in_a_fence_that_is_never_closed_is_not_an_area() {
        let text = "a\n```\n{++\nx\n++}\n";
        assert!(areas(text).is_empty());
        assert!(!has_pending(text));
    }

    #[test]
    fn propose_succeeds_when_the_note_documents_the_marks_in_a_fence_and_the_change_is_elsewhere() {
        let base = "intro\n```\n{--\nold\n--}\n```\nend\n";
        let proposed = "Intro\n```\n{--\nold\n--}\n```\nend\n";
        let marked = propose(base, base, proposed).unwrap();
        assert_eq!(areas(&marked).len(), 1);
        both_ways(&marked, base, proposed);
    }

    #[test]
    fn a_change_inside_a_fence_is_one_area_around_the_whole_block() {
        let base = "a\n```\nold\n```\nb\n";
        let proposed = "a\n```\nnew\n```\nb\n";
        let marked = propose(base, base, proposed).unwrap();
        assert_eq!(
            marked,
            "a\n{--\n```\nold\n```\n--}\n{++\n```\nnew\n```\n++}\nb\n"
        );
        assert_eq!(areas(&marked).len(), 1);
        both_ways(&marked, base, proposed);
    }

    #[test]
    fn after_accepting_an_earlier_area_a_later_fence_area_is_still_found() {
        let base = "x\n```\nold\n```\nb\nq\n";
        let proposed = "X\n```\nnew\n```\nb\nq\n";
        let marked = propose(base, base, proposed).unwrap();
        assert_eq!(areas(&marked).len(), 2);
        let once = accept(&marked, 0);
        assert_eq!(areas(&once).len(), 1);
        assert_eq!(accept(&once, 0), accept_all(&marked));
        both_ways(&marked, base, proposed);
    }

    #[test]
    fn a_new_fenced_block_is_one_added_area() {
        let base = "a\nb\n";
        let proposed = "a\n```\ncode\n```\nb\n";
        let marked = propose(base, base, proposed).unwrap();
        assert_eq!(marked, "a\n{++\n```\ncode\n```\n++}\nb\n");
        assert_eq!(areas(&marked).len(), 1);
        both_ways(&marked, base, proposed);
    }

    #[test]
    fn a_removed_fenced_block_is_one_removed_area() {
        let base = "a\n```\ncode\n```\nb\n";
        let proposed = "a\nb\n";
        let marked = propose(base, base, proposed).unwrap();
        assert_eq!(marked, "a\n{--\n```\ncode\n```\n--}\nb\n");
        assert_eq!(areas(&marked).len(), 1);
        both_ways(&marked, base, proposed);
    }

    #[test]
    fn changes_to_two_fenced_blocks_are_two_areas() {
        let base = "```\n1\n```\nmid\n```\n2\n```\n";
        let proposed = "```\n1b\n```\nmid\n```\n2b\n```\n";
        let marked = propose(base, base, proposed).unwrap();
        assert_eq!(areas(&marked).len(), 2);
        both_ways(&marked, base, proposed);
    }

    #[test]
    fn a_changed_fenced_block_holding_a_mark_line_is_refused() {
        let base = "a\n```\n{++\nx\n++}\n```\nb\n";
        let proposed = "a\n```\n{++\ny\n++}\n```\nb\n";
        assert_eq!(propose(base, base, proposed), Err(Refusal::HoldsMarks));
    }

    #[test]
    fn a_new_fenced_block_documenting_marks_is_refused() {
        assert_eq!(
            propose("a\n", "a\n", "a\n```\n{--\n```\n"),
            Err(Refusal::HoldsMarks)
        );
    }

    #[test]
    fn a_proposal_that_differs_only_by_the_final_break_is_nothing_changed() {
        assert_eq!(
            propose("a\nb\n", "a\nb\n", "a\nb"),
            Err(Refusal::NothingChanged)
        );
        assert_eq!(
            propose("a\nb", "a\nb", "a\nb\n"),
            Err(Refusal::NothingChanged)
        );
        assert_eq!(
            propose("a\r\nb\r\n", "a\r\nb\r\n", "a\r\nb"),
            Err(Refusal::NothingChanged)
        );
    }

    #[test]
    fn an_empty_note_takes_the_proposed_text_with_or_without_a_final_break() {
        both_ways(&propose("", "", "x").unwrap(), "", "x");
        both_ways(&propose("", "", "x\n").unwrap(), "", "x\n");
    }

    /// `text` with its final line break as `current` has it: the expected accept of a proposal.
    fn brought_to(current: &str, text: &str, eol: &str) -> String {
        let cut = text
            .strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .unwrap_or(text);
        if current.ends_with('\n') {
            format!("{cut}{eol}")
        } else {
            cut.to_string()
        }
    }

    #[test]
    fn the_final_line_break_is_exact_in_every_case() {
        for eol in ["\n", "\r\n"] {
            let tail = |broken: bool| if broken { eol } else { "" };
            for current_broken in [true, false] {
                for proposed_broken in [true, false] {
                    let current = format!("a{eol}b{}", tail(current_broken));
                    let changes = [
                        format!("a{eol}B{}", tail(proposed_broken)),
                        format!("a{}", tail(proposed_broken)),
                        format!("a{eol}b{eol}c{}", tail(proposed_broken)),
                    ];
                    for proposed in changes {
                        let marked = propose(&current, &current, &proposed).unwrap();
                        assert_eq!(reject_all(&marked), current, "reject of {marked:?}");
                        assert_eq!(
                            accept_all(&marked),
                            brought_to(&current, &proposed, eol),
                            "accept of {marked:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_change_is_split_into_the_note_text_and_an_entry_and_composed_back() {
        // Each shape: a change in the middle, at the end without a final break, a pure insertion,
        // a pure removal, two changes, CRLF.
        let cases = [
            "keep\n{--\nold\n--}\n{++\nnew\n++}\nend\n",
            "a\n{--\nb\n--}\n{++\nc\n++}",
            "a\n{++\nadded\n++}\nb\n",
            "a\n{--\ngone\n--}\nb\n",
            "x\n{--\n1\n--}\n{++\n2\n++}\nm\n{--\n3\n--}\n{++\n4\n++}\n",
            "x\r\n{--\r\n1\r\n--}\r\n{++\r\n2\r\n++}\r\nend\r\n",
        ];
        for display in cases {
            let (base, entries) = split(display, &[]);
            assert_eq!(base, reject_all(display), "{display:?}");
            assert_eq!(compose(&base, &entries), display, "{display:?}");
            assert!(!base.contains("{--") && !base.contains("{++"), "{base:?}");
        }
        let (base, entries) = split("keep\n{--\nold\n--}\n{++\nnew\n++}\nend\n", &[]);
        assert_eq!(base, "keep\nold\nend\n");
        assert_eq!(
            entries,
            [Entry {
                id: 1,
                author: String::new(),
                at: 5,
                old: "old\n".into(),
                new: "new\n".into(),
                before: "keep\n".into(),
                after: "end\n".into(),
            }]
        );
    }

    #[test]
    fn a_split_keeps_the_id_and_author_of_a_known_entry() {
        let display = "a\n{--\nold\n--}\n{++\nnew\n++}\nb\n{++\nfresh\n++}\n";
        let known = [Entry {
            id: 7,
            author: "assistant".into(),
            at: 2,
            old: "old\n".into(),
            new: "new\n".into(),
            ..Entry::default()
        }];
        let (_, entries) = split(display, &known);
        assert_eq!(
            (entries[0].id, entries[0].author.as_str()),
            (7, "assistant")
        );
        assert_eq!((entries[1].id, entries[1].author.as_str()), (8, ""));
    }

    #[test]
    fn compose_finds_a_moved_removed_text_and_leaves_out_a_gone_one() {
        let entry = Entry {
            id: 1,
            author: String::new(),
            at: 0,
            old: "two\n".into(),
            new: "TWO\n".into(),
            ..Entry::default()
        };
        // The note gained a line above: the entry follows its text.
        assert_eq!(
            compose("zero\none\ntwo\n", std::slice::from_ref(&entry)),
            "zero\none\n{--\ntwo\n--}\n{++\nTWO\n++}\n"
        );
        // The text is gone: nothing is marked, the note is as it is.
        assert_eq!(compose("zero\n", &[entry]), "zero\n");
    }

    #[test]
    fn a_copy_of_marked_text_holds_no_mark() {
        assert_eq!(plain("a\n{--\nold\n--}\n{++\nnew\n++}\nb\n"), "a\nnew\nb\n");
        assert_eq!(plain("{--\nold\n--}\n"), "");
        assert_eq!(plain("plain\n"), "plain\n");
    }

    /// The text `marked` (an area per change) is split into, and the entries it gives.
    fn read_back(marked: &str) -> (String, Vec<Entry>) {
        split(marked, &[])
    }

    #[test]
    fn an_insertion_follows_its_line_after_an_outside_edit_above_it() {
        let (base, entries) = read_back("a\n{++\nX\n++}\nb\nc\n");
        assert_eq!(base, "a\nb\nc\n");
        assert_eq!(
            (entries[0].before.as_str(), entries[0].after.as_str()),
            ("a\n", "b\n")
        );
        // A line is added above: the stored offset 2 is now inside the first line.
        assert_eq!(
            compose("zero\na\nb\nc\n", &entries),
            "zero\na\n{++\nX\n++}\nb\nc\n"
        );
    }

    #[test]
    fn a_removed_text_that_is_in_the_note_once_is_found_after_a_relink_of_another_line() {
        let (base, entries) = read_back("See [[Old]].\n{--\nend\n--}\n");
        assert_eq!(base, "See [[Old]].\nend\n");
        // The link above changed length: the removed line is still there, once.
        assert_eq!(
            compose("See [[Longer-name-here]].\nend\n", &entries),
            "See [[Longer-name-here]].\n{--\nend\n--}\n"
        );
    }

    #[test]
    fn an_insertion_is_placed_by_the_line_after_when_the_line_before_was_relinked() {
        let (_, entries) = read_back("See [[Old]].\nend\n{++\nadded\n++}\nfoot\n");
        assert_eq!(
            compose("See [[Longer-name-here]].\nend\nfoot\n", &entries),
            "See [[Longer-name-here]].\nend\n{++\nadded\n++}\nfoot\n"
        );
    }

    #[test]
    fn a_repeated_removed_line_follows_its_own_context() {
        let (base, entries) = read_back("x\nend\ny\n{--\nend\n--}\n");
        assert_eq!(base, "x\nend\ny\nend\n");
        assert_eq!(
            compose("new\nx\nend\ny\nend\n", &entries),
            "new\nx\nend\ny\n{--\nend\n--}\n"
        );
    }

    #[test]
    fn a_stale_insertion_inside_a_character_is_left_out_and_never_splits_it() {
        // Offset 1 is inside the two bytes of `é`: no panic, no mark, the text as it is.
        let entry = Entry {
            at: 1,
            new: "x\n".into(),
            ..Entry::default()
        };
        assert_eq!(compose("é\n", std::slice::from_ref(&entry)), "é\n");
        assert_eq!(placed("é\n", std::slice::from_ref(&entry)), [false]);
    }

    #[test]
    fn a_text_with_multi_byte_characters_around_the_anchor_is_found_again() {
        let (base, entries) = read_back("Ünï\n{++\nnew\n++}\nend\n");
        assert_eq!(base, "Ünï\nend\n");
        assert_eq!(
            compose("日本\nÜnï\nend\n", &entries),
            "日本\nÜnï\n{++\nnew\n++}\nend\n"
        );
    }

    #[test]
    fn crlf_anchors_are_found_again_and_keep_their_line_breaks() {
        let (_, removal) = read_back("a\r\n{--\r\nb\r\n--}\r\n");
        assert_eq!(
            compose("x\r\na\r\nb\r\n", &removal),
            "x\r\na\r\n{--\r\nb\r\n--}\r\n"
        );
        let (_, insertion) = read_back("a\r\n{++\r\nX\r\n++}\r\nb\r\n");
        assert_eq!(
            compose("z\r\na\r\nb\r\n", &insertion),
            "z\r\na\r\n{++\r\nX\r\n++}\r\nb\r\n"
        );
    }

    #[test]
    fn a_change_is_never_placed_inside_a_code_fence_or_mid_line() {
        let fenced = "```\nx\n```\n";
        let inside = Entry {
            at: 4,
            old: "x\n".into(),
            new: "y\n".into(),
            ..Entry::default()
        };
        assert_eq!(compose(fenced, std::slice::from_ref(&inside)), fenced);
        let mid = Entry {
            at: 0,
            old: "two\n".into(),
            new: "TWO\n".into(),
            ..Entry::default()
        };
        assert_eq!(compose("network two\n", &[mid]), "network two\n");
    }

    #[test]
    fn an_entry_holding_a_mark_line_is_left_out() {
        let entry = Entry {
            at: 0,
            old: "a\n".into(),
            new: "{--\n".into(),
            ..Entry::default()
        };
        assert_eq!(compose("a\n", std::slice::from_ref(&entry)), "a\n");
    }

    /// Whatever is placed is written back exactly: `split` of a composed text is its base, with no
    /// mark of ours in it. Every shape of line, code fences, inline code and both line breaks, with
    /// the note changed outside Inkuno after each save.
    #[test]
    fn save_writes_back_the_clean_text_for_any_placement_and_any_outside_change() {
        const LINES: [&str; 15] = [
            "# Heading",
            "plain prose line",
            "- bullet item",
            "1. numbered item",
            "> quoted line",
            "- [ ] task item",
            "",
            "```",
            "~~~",
            "    indented code",
            "inline `{==x==}{>>y<<}` here",
            "{== comment ==}{>> note <<} text",
            "<!-- html comment -->",
            "{++ inline, not a mark line ++}",
            "a line with the words {-- inside --}",
        ];
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let (mut changes, mut outside_placed) = (0, 0);
        let mut rng = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for round in 0..400 {
            let eol = if round % 3 == 0 { "\r\n" } else { "\n" };
            let mut base = String::new();
            for _ in 0..1 + rng(9) {
                base.push_str(LINES[rng(LINES.len())]);
                base.push_str(eol);
            }
            if round % 4 == 0 {
                while base.ends_with(['\n', '\r']) {
                    base.pop();
                }
            }
            // Areas whose marks sit on clean line starts, each with whole lines and breaks.
            let clean = clean_starts(&base);
            let mut marked = String::new();
            let mut last = 0;
            let mut cut = 0;
            while cut + 1 < clean.len() {
                if rng(3) != 0 {
                    cut += 1;
                    continue;
                }
                let at = clean[cut];
                let to = clean[cut + rng(clean.len() - cut - 1)];
                let (old, next) = (&base[at..to], to);
                let mut new = String::new();
                for _ in 0..rng(3) {
                    new.push_str(LINES[rng(LINES.len())]);
                    new.push_str(eol);
                }
                if old.is_empty() && new.is_empty() {
                    cut += 1;
                    continue;
                }
                marked.push_str(&base[last..at]);
                if !old.is_empty() {
                    marked.push_str(&format!("{{--{eol}{old}--}}{eol}"));
                }
                if !new.is_empty() {
                    marked.push_str(&format!("{{++{eol}{new}++}}{eol}"));
                }
                last = next;
                cut = clean.binary_search(&next).unwrap_or(cut) + 1;
            }
            marked.push_str(&base[last..]);

            let (back, entries) = read_back(&marked);
            changes += entries.len();
            assert_eq!(back, base, "round {round}: {marked:?}");
            let composed = compose(&base, &entries);
            assert_eq!(composed, marked, "round {round}");
            let (again, _) = read_back(&composed);
            assert_eq!(again, base, "round {round}");

            // The note changed outside: whatever is placed still reads back as the note.
            let outside = format!("outside line{eol}{base}");
            let shown = compose(&outside, &entries);
            let (written, _) = read_back(&shown);
            assert_eq!(written, outside, "round {round}, outside: {shown:?}");
            let kept = placed(&outside, &entries).iter().filter(|p| **p).count();
            assert_eq!(areas(&shown).len(), kept, "round {round}");
            outside_placed += kept;
        }
        // The rounds exercise real placements, both with the note as saved and after a change.
        assert!(
            changes > 200 && outside_placed > 200,
            "{changes} {outside_placed}"
        );
    }
}
