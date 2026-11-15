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
//! line stands alone: its four strings are the only lines that count as marks. Pure: no disk here.
//!
//! Line breaks at the end of a text: a text may lack its final line break. A marker line that ends
//! the whole text has no line break, and the line break written just before it is not content: it
//! is taken off again on accept and reject. Where a line break had to be added between two lines
//! (a content line without one, or a line that follows an unbroken line), it stays in the output.
//!
//! Known limit: a text without a final line break that a change touches at its end gets one break
//! more on reject, when the change also has an added block. The same marks come from the text with
//! that break, so the two cannot be told apart. Accept is exact in every case.

use std::fmt;
use std::ops::Range;

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
    let side = |block: Option<(usize, usize)>, empty_at: usize| -> Range<usize> {
        match block {
            None => empty_at..empty_at,
            Some((open, close)) => {
                let from = rows[open].end;
                let mut to = rows[close].start;
                // The close marker ends the text: the line break before it is not content.
                if !rows[close].broken && to > from {
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

/// Every pending area of `text`, in order. An unclosed or malformed mark is plain text.
pub fn areas(text: &str) -> Vec<Area> {
    let rows = rows(text);
    let mut found = Vec::new();
    let mut at = 0;
    while at < rows.len() {
        match area_from(text, &rows, at) {
            Some((area, next)) => {
                found.push(area);
                at = next;
            }
            None => at += 1,
        }
    }
    found
}

/// Whether `text` has a pending change.
pub fn has_pending(text: &str) -> bool {
    !areas(text).is_empty()
}

/// Whether a line of `text` is one of the four marks.
fn holds_marks(text: &str) -> bool {
    rows(text).iter().any(|row| is_mark(row.body))
}

/// The index of the area whose range contains byte `offset`.
pub fn area_at(text: &str, offset: usize) -> Option<usize> {
    areas(text)
        .iter()
        .position(|area| area.range.start <= offset && offset < area.range.end)
}

/// Whether an edit of the bytes `range` would change any byte of an area, marker lines included.
/// An empty range is an insertion point: inserting exactly at an area's start or end is allowed.
pub fn touches(text: &str, range: Range<usize>) -> bool {
    areas(text).iter().any(|area| {
        let r = &area.range;
        if range.is_empty() {
            r.start < range.start && range.start < r.end
        } else {
            range.start < r.end && r.start < range.end
        }
    })
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
    // The marks and the added breaks use the note's own line break: CRLF if the text has one.
    let eol = if c.iter().chain(p.iter()).any(|line| line.ends_with("\r\n")) {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = String::new();
    let mut at = 0;
    for (k, a) in agent.iter().enumerate() {
        let cs = (a.base.start as isize + shift(a.base.start)) as usize;
        let ce = cs + a.base.len();
        for line in &c[at..cs] {
            out.push_str(line);
        }
        at = ce;
        let removed = &c[cs..ce];
        let added = &p[a.side.clone()];
        // This area ends the text when nothing of `current` follows it.
        let ends = k + 1 == agent.len() && ce == c.len();
        let omit = ends
            && if added.is_empty() {
                !current.ends_with('\n')
            } else {
                !proposed.ends_with('\n')
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
    for line in &c[at..] {
        out.push_str(line);
    }
    Ok(out)
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

    #[test]
    fn an_edit_inside_an_area_touches_it() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let inside = text.find("yz").unwrap();
        assert!(touches(text, inside..inside + 1));
        assert!(touches(text, inside..inside));
    }

    #[test]
    fn an_edit_on_a_marker_line_touches_the_area() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let mark = text.find("++}").unwrap();
        assert!(touches(text, mark..mark + 1));
        let open = text.find("{++").unwrap();
        assert!(touches(text, open..open + 1));
    }

    #[test]
    fn an_insertion_at_an_area_edge_does_not_touch_it() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let start = text.find("{++").unwrap();
        let end = text.find("b\n").unwrap();
        assert!(!touches(text, start..start));
        assert!(!touches(text, end..end));
        assert!(!touches(text, 0..start));
        assert!(!touches(text, end..text.len()));
    }

    #[test]
    fn an_edit_that_only_meets_an_area_edge_does_not_touch_it() {
        let text = "a\n{++\nxyz\n++}\nb\n";
        let start = text.find("{++").unwrap();
        assert!(!touches(text, 0..start));
        let end = text.find("b\n").unwrap();
        assert!(!touches(text, end..end + 1));
    }

    #[test]
    fn an_insertion_in_plain_text_does_not_touch_any_area() {
        let text = "a\n{++\nx\n++}\nb\n";
        assert!(!touches(
            text,
            text.find('b').unwrap()..text.find('b').unwrap()
        ));
        assert!(!touches("plain\n", 2..2));
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
    fn a_deletion_at_the_end_of_a_text_without_final_break_is_known_to_gain_a_break() {
        // Format limit: "a\nb" and "a\nb\n" changed the same way give the same marks, so reject
        // cannot tell them apart. Accept is exact; reject keeps the text plus its last break.
        let marked = propose("a\nb", "a\nb", "a").unwrap();
        assert_eq!(accept_all(&marked), "a");
        assert_eq!(reject_all(&marked), "a\nb\n");
    }

    #[test]
    fn invariants_hold_for_an_added_last_line_without_break() {
        let marked = propose("a\n", "a\n", "a\nb").unwrap();
        both_ways(&marked, "a\n", "a\nb");
    }

    #[test]
    fn a_text_with_no_line_break_gains_one_on_reject_when_changed_at_its_end() {
        // The same format limit as above: accept is exact, reject adds the last break.
        let marked = propose("one", "one", "two").unwrap();
        assert_eq!(accept_all(&marked), "two");
        assert_eq!(reject_all(&marked), "one\n");
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
}
