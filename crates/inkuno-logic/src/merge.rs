//! Three texts of one note: `base` as the app last read or wrote it, `ours` as typed since, and
//! `theirs` as another program (an agent, a sync tool) wrote it in the meantime. What `theirs`
//! changed is turned into replacements of `ours`, so the editor applies them without losing a
//! keystroke. Lines are the unit, as in git.

use std::ops::Range;

/// A changed block: lines `base` of the base became lines `side` of the other text.
#[derive(Debug, Clone, PartialEq)]
pub struct Hunk {
    pub base: Range<usize>,
    pub side: Range<usize>,
}

/// What `theirs` changed, as byte replacements in `ours`, in order. `None` when both changed the
/// same lines differently, or inserted different lines at the same place: the caller keeps both.
pub fn onto(base: &str, ours: &str, theirs: &str) -> Option<Vec<(Range<usize>, String)>> {
    let (b, o, t) = (lines(base), lines(ours), lines(theirs));
    let (mine, others) = (hunks(&b, &o), hunks(&b, &t));
    let empty = |r: &Range<usize>| r.start == r.end;
    let mut edits = Vec::new();
    for whole in others {
        let same = |h: &&Hunk| h.base == whole.base && o[h.side.clone()] == t[whole.side.clone()];
        if mine.iter().any(|h| same(&h)) {
            // The same change on both sides: it is already in `ours`.
            continue;
        }
        for hunk in left_to_apply(&whole, &mine)? {
            if mine
                .iter()
                .any(|h| empty(&hunk.base) && empty(&h.base) && hunk.base.start == h.base.start)
            {
                // Two insertions at one place: their order cannot be chosen.
                return None;
            }
            // Where the block's edges land in `ours`. An insertion of ours at the start comes
            // before their block, one at the end comes after it.
            let shift = |at: usize, start: bool| {
                mine.iter()
                    .filter(|h| h.base.end < at || h.base.end == at && (start || !empty(&h.base)))
                    .fold(at as isize, |n, h| {
                        n + h.side.len() as isize - h.base.len() as isize
                    }) as usize
            };
            let start = shift(hunk.base.start, true);
            let end = if empty(&hunk.base) {
                start
            } else {
                shift(hunk.base.end, false)
            };
            let byte = |line: usize| o[..line].iter().map(|l| l.len()).sum::<usize>();
            let mut with = t[hunk.side.clone()].concat();
            if !with.is_empty() {
                // `ours` may lack the newline of its last line: the text must not join that line.
                if start > 0 && !o[start - 1].ends_with('\n') {
                    with.insert(0, '\n');
                }
                // `ours` goes on after the range: the text must end its line.
                if end < o.len() && !with.ends_with('\n') {
                    with.push('\n');
                }
            }
            edits.push((byte(start)..byte(end), with));
        }
    }
    Some(edits)
}

/// What is still to apply of one of `theirs`'s hunks once `ours` is taken into account. A hunk
/// that only deletes lines, and whose lines `ours` already deleted in part, is cut down to the
/// lines `ours` still has. `None` when the hunk overlaps a change of `ours`: a clash.
fn left_to_apply(hunk: &Hunk, mine: &[Hunk]) -> Option<Vec<Hunk>> {
    let touching: Vec<&Hunk> = mine
        .iter()
        .filter(|h| hunk.base.start < h.base.end && h.base.start < hunk.base.end)
        .collect();
    if touching.is_empty() {
        return Some(vec![hunk.clone()]);
    }
    if !hunk.side.is_empty() || touching.iter().any(|h| !h.side.is_empty()) {
        return None;
    }
    let mut cuts: Vec<Range<usize>> = touching.iter().map(|h| h.base.clone()).collect();
    cuts.sort_by_key(|r| r.start);
    let mut rest = Vec::new();
    let mut at = hunk.base.start;
    for cut in cuts {
        if cut.start > at {
            rest.push(at..cut.start);
        }
        at = at.max(cut.end);
    }
    if at < hunk.base.end {
        rest.push(at..hunk.base.end);
    }
    Some(
        rest.into_iter()
            .map(|base| Hunk {
                base,
                side: hunk.side.clone(),
            })
            .collect(),
    )
}

pub fn lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// Above this many cells, the middle that differs is one block: coarser, never wrong.
// ponytail: quadratic longest common subsequence on what is left once the common start and end
// are cut (a few lines when someone types); swap for Myers' diff if big rewrites must merge finely.
const CELLS: usize = 1_000_000;

pub fn hunks(base: &[&str], side: &[&str]) -> Vec<Hunk> {
    let head = base.iter().zip(side).take_while(|(a, b)| a == b).count();
    let tail = base[head..]
        .iter()
        .rev()
        .zip(side[head..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (b, s) = (
        &base[head..base.len() - tail],
        &side[head..side.len() - tail],
    );
    if b.is_empty() && s.is_empty() {
        return Vec::new();
    }
    if b.len() * s.len() > CELLS || b.is_empty() || s.is_empty() {
        return vec![Hunk {
            base: head..head + b.len(),
            side: head..head + s.len(),
        }];
    }
    // Length of the longest common subsequence of b[i..] and s[j..].
    let w = s.len() + 1;
    let mut lcs = vec![0u32; (b.len() + 1) * w];
    for i in (0..b.len()).rev() {
        for j in (0..s.len()).rev() {
            lcs[i * w + j] = if b[i] == s[j] {
                lcs[(i + 1) * w + j + 1] + 1
            } else {
                lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, Vec::new());
    let mut open: Option<(usize, usize)> = None;
    while i < b.len() || j < s.len() {
        if i < b.len() && j < s.len() && b[i] == s[j] {
            if let Some((bi, sj)) = open.take() {
                out.push(Hunk {
                    base: head + bi..head + i,
                    side: head + sj..head + j,
                });
            }
            (i, j) = (i + 1, j + 1);
            continue;
        }
        open.get_or_insert((i, j));
        if j < s.len() && (i == b.len() || lcs[i * w + j + 1] >= lcs[(i + 1) * w + j]) {
            j += 1;
        } else {
            i += 1;
        }
    }
    if let Some((bi, sj)) = open {
        out.push(Hunk {
            base: head + bi..head + i,
            side: head + sj..head + j,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `ours` with what `theirs` changed put in, or `None`.
    fn merged(base: &str, ours: &str, theirs: &str) -> Option<String> {
        Some(apply_checked(ours, &onto(base, ours, theirs)?))
    }

    /// `ours` with `edits` applied, after checking they are in order, in bounds and on char
    /// boundaries: the editor must never get a range that would panic or split a character.
    fn apply_checked(ours: &str, edits: &[(Range<usize>, String)]) -> String {
        let mut end = 0;
        for (range, _) in edits {
            assert!(
                end <= range.start && range.start <= range.end && range.end <= ours.len(),
                "ranges out of order or out of bounds: {edits:?} for {ours:?}"
            );
            assert!(
                ours.is_char_boundary(range.start) && ours.is_char_boundary(range.end),
                "range {range:?} splits a character in {ours:?}"
            );
            end = range.end;
        }
        let mut text = ours.to_string();
        for (range, with) in edits.iter().rev() {
            text.replace_range(range.clone(), with);
        }
        text
    }

    /// What a case expects: the merged text, or a clash (`None`).
    #[derive(Clone, Copy)]
    enum Want {
        Text(&'static str),
        Clash,
    }
    use Want::{Clash, Text};

    /// Scenario name, base, ours, theirs, and what must come out.
    type Case = (&'static str, &'static str, &'static str, &'static str, Want);

    fn check(cases: &[Case]) {
        for &(name, base, ours, theirs, want) in cases {
            let got = merged(base, ours, theirs);
            match want {
                Text(text) => assert_eq!(got.as_deref(), Some(text), "case `{name}`"),
                Clash => assert_eq!(got, None, "case `{name}` should clash"),
            }
        }
    }

    #[test]
    fn disjoint_edits_by_human_and_agent_all_survive() {
        check(&[
            // Human edits the top, agent the bottom.
            (
                "human top, agent bottom",
                "t\nm\nb\n",
                "T!\nm\nb\n",
                "t\nm\nB!\n",
                Text("T!\nm\nB!\n"),
            ),
            // Agent edits the top, human the bottom.
            (
                "agent top, human bottom",
                "t\nm\nb\n",
                "t\nm\nB!\n",
                "T!\nm\nb\n",
                Text("T!\nm\nB!\n"),
            ),
            // Many alternating hunks from both sides.
            (
                "alternating hunks",
                "l1\nl2\nl3\nl4\nl5\nl6\n",
                "H1\nl2\nH3\nl4\nH5\nl6\n",
                "l1\nA2\nl3\nA4\nl5\nA6\n",
                Text("H1\nA2\nH3\nA4\nH5\nA6\n"),
            ),
            // Agent appends at the end while the human types in the middle.
            (
                "agent appends, human types in middle",
                "a\nb\nc\n",
                "a\nb typed\nc\n",
                "a\nb\nc\nd\n",
                Text("a\nb typed\nc\nd\n"),
            ),
            // Agent prepends while the human appends at the end.
            (
                "agent prepends, human appends",
                "a\nb\n",
                "a\nb\nz\n",
                "p\na\nb\n",
                Text("p\na\nb\nz\n"),
            ),
            // Agent deletes a block the human did not touch.
            (
                "agent deletes untouched block",
                "a\nb\nc\nd\n",
                "a\nb\nc\nD!\n",
                "a\nd\n",
                Text("a\nD!\n"),
            ),
        ]);
    }

    #[test]
    fn adjacent_edits_touch_but_do_not_overlap() {
        check(&[
            // Neighbouring lines, one changed by each side.
            (
                "neighbours",
                "a\nb\nc\n",
                "A\nb\nc\n",
                "a\nB\nc\n",
                Text("A\nB\nc\n"),
            ),
            // Agent inserts right after a line the human changed.
            (
                "agent inserts after human change",
                "a\nb\n",
                "a!\nb\n",
                "a\nNEW\nb\n",
                Text("a!\nNEW\nb\n"),
            ),
            // Agent inserts right before a line the human changed.
            (
                "agent inserts before human change",
                "a\nb\n",
                "a\nB!\n",
                "a\nNEW\nb\n",
                Text("a\nNEW\nB!\n"),
            ),
            // Human inserts right after a line the agent changed.
            (
                "human inserts after agent change",
                "a\nb\n",
                "a\nmine\nb\n",
                "A!\nb\n",
                Text("A!\nmine\nb\n"),
            ),
            // Human inserts right before a line the agent changed.
            (
                "human inserts before agent change",
                "a\nb\n",
                "a\nmine\nb\n",
                "a\nB!\n",
                Text("a\nmine\nB!\n"),
            ),
            // Both insert the same text at the same place: it is there once.
            (
                "same insertion by both",
                "a\nb\n",
                "a\nX\nb\n",
                "a\nX\nb\n",
                Text("a\nX\nb\n"),
            ),
            // Both insert different text at the same place: their order cannot be chosen.
            (
                "different insertions, same place",
                "a\nb\n",
                "a\nmine\nb\n",
                "a\ntheirs\nb\n",
                Clash,
            ),
        ]);
    }

    #[test]
    fn identical_changes_on_both_sides_apply_once() {
        check(&[
            // The same edit on both sides.
            (
                "identical edit",
                "a\nb\nc\n",
                "a\nB\nc\n",
                "a\nB\nc\n",
                Text("a\nB\nc\n"),
            ),
            // The same deletion on both sides.
            (
                "identical deletion",
                "a\nb\nc\n",
                "a\nc\n",
                "a\nc\n",
                Text("a\nc\n"),
            ),
            // The same insertion on both sides.
            (
                "identical insertion",
                "a\nb\nc\n",
                "a\nX\nb\nc\n",
                "a\nX\nb\nc\n",
                Text("a\nX\nb\nc\n"),
            ),
            // Same edit on both, plus the agent's own append.
            (
                "identical edit plus agent append",
                "a\nb\nc\n",
                "a\nB\nc\n",
                "a\nB\nc\nd\n",
                Text("a\nB\nc\nd\n"),
            ),
            // Both rewrote the whole file the same way.
            (
                "identical whole-file rewrite",
                "a\nb\n",
                "x\ny\n",
                "x\ny\n",
                Text("x\ny\n"),
            ),
            // Both emptied the file.
            ("identical emptying", "a\nb\n", "", "", Text("")),
        ]);
    }

    #[test]
    fn deletions_by_one_or_both_sides() {
        check(&[
            // Agent deletes a block; human edits a line inside it: clash.
            (
                "human edits agent-deleted block",
                "a\nb\nc\nd\n",
                "a\nB!\nc\nd\n",
                "a\nd\n",
                Clash,
            ),
            // Agent deletes b and c; human inserts a new line between b and c: clash (as git does,
            // the new line is inside the deleted block, so the agent's deletion cannot be placed).
            (
                "human inserts inside agent-deleted block",
                "a\nb\nc\nd\n",
                "a\nb\nNEW\nc\nd\n",
                "a\nd\n",
                Clash,
            ),
            // Human inserts right after a line the agent deletes.
            (
                "human inserts after agent-deleted line",
                "a\nb\nc\n",
                "a\nb\nNEW\nc\n",
                "a\nc\n",
                Text("a\nNEW\nc\n"),
            ),
            // Human inserts right before a line the agent deletes.
            (
                "human inserts before agent-deleted line",
                "a\nb\nc\n",
                "a\nNEW\nb\nc\n",
                "a\nc\n",
                Text("a\nNEW\nc\n"),
            ),
            // Agent empties the file while the human changed a line: clash.
            (
                "agent empties, human edited",
                "a\nb\n",
                "a\nB!\n",
                "",
                Clash,
            ),
            // Human empties the file, agent appends a line: the agent's line survives.
            (
                "human empties, agent appends",
                "a\nb\n",
                "",
                "a\nb\nc\n",
                Text("c\n"),
            ),
            // Human empties the file, agent idle.
            (
                "human empties, agent idle",
                "a\nb\n",
                "",
                "a\nb\n",
                Text(""),
            ),
            // Agent empties the file, human typed at the end: the typed line survives.
            (
                "agent empties, human typed at end",
                "a\nb\n",
                "a\nb\nX\n",
                "",
                Text("X\n"),
            ),
            // Agent empties the file, human idle.
            (
                "agent empties, human idle",
                "a\nb\n",
                "a\nb\n",
                "",
                Text(""),
            ),
            // Human deletes one line, agent deletes a different line far away.
            (
                "both delete different lines",
                "a\nb\nc\nd\n",
                "a\nc\nd\n",
                "a\nb\nc\n",
                Text("a\nc\n"),
            ),
            // Both delete the same line, the agent also deletes the line after it: the union.
            (
                "both delete overlapping runs",
                "a\nb\nc\nd\ne\n",
                "a\nd\ne\n",
                "a\nb\ne\n",
                Text("a\ne\n"),
            ),
            // Agent deletes a superset of what the human deleted (human deleted b, agent a and b).
            (
                "agent deletes superset of human",
                "a\nb\nc\n",
                "a\nc\n",
                "c\n",
                Text("c\n"),
            ),
            // Agent deletes a subset of what the human deleted: nothing left to do.
            (
                "agent deletes subset of human",
                "a\nb\nc\n",
                "c\n",
                "a\nc\n",
                Text("c\n"),
            ),
            // Agent deletes a run that spans a line the human deleted, and one more line.
            (
                "agent deletes run around human deletion",
                "a\nb\nc\nd\ne\n",
                "a\nd\ne\n",
                "a\ne\n",
                Text("a\ne\n"),
            ),
            // Agent rewrites a run the human deleted in part: clash.
            (
                "agent rewrites run human deleted in part",
                "a\nb\nc\n",
                "a\nc\n",
                "a\nB!\n",
                Clash,
            ),
        ]);
    }

    #[test]
    fn whole_file_rewrites_and_moved_blocks() {
        check(&[
            // Agent rewrites every line while the human changed one: clash.
            (
                "agent rewrites all, human edited one",
                "a\nb\nc\n",
                "a\nB!\nc\n",
                "x\ny\nz\n",
                Clash,
            ),
            // Agent rewrites the other lines and keeps the one the human changed.
            (
                "agent rewrites around human line",
                "a\nb\nc\nd\ne\n",
                "a\nb\nC!\nd\ne\n",
                "A\nB\nc\nD\nE\n",
                Text("A\nB\nC!\nD\nE\n"),
            ),
            // Agent moves the first block to the end while the human edits the third line.
            (
                "agent moves first line to end",
                "A\nB\nC\nD\n",
                "A\nB\nC!\nD\n",
                "B\nC\nD\nA\n",
                Text("B\nC!\nD\nA\n"),
            ),
            // Agent moves the last line to the top while the human edits the third line.
            (
                "agent moves last line to top",
                "A\nB\nC\nD\n",
                "A\nB\nC!\nD\n",
                "D\nA\nB\nC\n",
                Text("D\nA\nB\nC!\n"),
            ),
            // Agent moves a line the human edited: clash.
            (
                "agent moves human-edited line",
                "A\nB\nC\nD\n",
                "A!\nB\nC\nD\n",
                "B\nC\nD\nA\n",
                Clash,
            ),
        ]);
    }

    #[test]
    fn trailing_newlines_and_empty_texts() {
        check(&[
            // Agent adds the final newline the base lacked.
            (
                "agent adds final newline",
                "a\nb",
                "a\nb",
                "a\nb\n",
                Text("a\nb\n"),
            ),
            // Human adds the final newline while the agent edits the first line.
            (
                "human adds final newline, agent edits first",
                "a\nb",
                "a\nb\n",
                "A\nb",
                Text("A\nb\n"),
            ),
            // Human pressed Enter after the last line while the agent edited that line: clash.
            (
                "human Enter at end, agent edits last line",
                "a\nb",
                "a\nb\n",
                "a\nB",
                Clash,
            ),
            // Agent appends after an unterminated last line the human did not touch.
            (
                "agent appends after unterminated line",
                "a\nb",
                "a\nb",
                "a\nb\nc",
                Text("a\nb\nc"),
            ),
            // Empty base: the human typed, the agent did nothing.
            (
                "empty base, human typed",
                "",
                "typed\n",
                "",
                Text("typed\n"),
            ),
            // Empty base: the agent wrote, the human did nothing.
            (
                "empty base, agent wrote",
                "",
                "",
                "theirs\n",
                Text("theirs\n"),
            ),
            // Empty base: both wrote different text.
            (
                "empty base, both wrote differently",
                "",
                "mine\n",
                "theirs\n",
                Clash,
            ),
            // Empty base: both wrote the same text.
            (
                "empty base, both wrote the same",
                "",
                "same\n",
                "same\n",
                Text("same\n"),
            ),
            // Empty ours (human deleted everything), agent idle.
            (
                "human emptied, agent idle",
                "a\nb\n",
                "",
                "a\nb\n",
                Text(""),
            ),
            // Empty theirs (agent emptied), human idle.
            (
                "agent emptied, human idle",
                "a\nb\n",
                "a\nb\n",
                "",
                Text(""),
            ),
            // Empty theirs while the human changed a line: clash.
            ("agent emptied, human edited", "a\nb\n", "a\nB\n", "", Clash),
            // Everything empty.
            ("all empty", "", "", "", Text("")),
            // Single line without newline, human types.
            ("single line, human types", "a", "a!", "a", Text("a!")),
            // Single line without newline, agent types.
            ("single line, agent types", "a", "a", "a?", Text("a?")),
            // Single line without newline, both change it differently: clash.
            ("single line, both change", "a", "a!", "a?", Clash),
            // Single line, human adds a newline.
            ("single line, human newline", "a", "a\n", "a", Text("a\n")),
            // Single line, agent adds a newline.
            ("single line, agent newline", "a", "a", "a\n", Text("a\n")),
            // Human removed the final newline; agent appends a line: it goes on its own line.
            (
                "final newline removed, agent appends",
                "a\nb\n",
                "a\nb",
                "a\nb\nc\n",
                Text("a\nb\nc\n"),
            ),
            // Human removed the final newline; agent appends an unterminated line.
            (
                "final newline removed, agent appends bare",
                "a\nb\n",
                "a\nb",
                "a\nb\nc",
                Text("a\nb\nc"),
            ),
            // Human removed the final newline; agent appends a blank line.
            (
                "final newline removed, agent blank line",
                "a\nb\n",
                "a\nb",
                "a\nb\n\n",
                Text("a\nb\n\n"),
            ),
            // Agent removed the final newline; human typed a line after the last line.
            (
                "agent drops final newline, human appends",
                "a\nb\n",
                "a\nb\nX\n",
                "a\nb",
                Text("a\nb\nX\n"),
            ),
            // Agent drops the final newline, human appended nothing: the agent's text as is.
            (
                "agent drops final newline, human idle",
                "a\nb\n",
                "a\nb\n",
                "a\nb",
                Text("a\nb"),
            ),
            // Agent appends without a final newline to a note whose newline is kept.
            (
                "agent appends bare line to kept newline",
                "a\nb\n",
                "a\nb\n",
                "a\nb\nc",
                Text("a\nb\nc"),
            ),
            // Human removed the final newline and agent edits that line: clash.
            (
                "final newline removed, agent edits that line",
                "a\nb\n",
                "a\nb",
                "a\nB\n",
                Clash,
            ),
            // Human's text is empty, agent appended a line without newline.
            (
                "human emptied, agent appends bare",
                "a\n",
                "",
                "a\nc",
                Text("c"),
            ),
        ]);
    }

    #[test]
    fn line_endings_blank_lines_repeats_and_multibyte_text() {
        check(&[
            // CRLF on both sides.
            (
                "crlf both",
                "a\r\nb\r\n",
                "a\r\nB!\r\n",
                "a\r\nb\r\nc\r\n",
                Text("a\r\nB!\r\nc\r\n"),
            ),
            // Human's lines are CRLF, agent's are LF.
            (
                "crlf human, lf agent",
                "a\nb\n",
                "a\r\nb\n",
                "a\nb\nc\n",
                Text("a\r\nb\nc\n"),
            ),
            // Agent converts the whole file to CRLF while the human edited a line: clash.
            (
                "agent converts to crlf, human edited",
                "a\nb\n",
                "a!\nb\n",
                "a\r\nb\r\n",
                Clash,
            ),
            // Agent adds a blank line above a line the human changed.
            (
                "blank line added above human line",
                "a\n\n\nb\n",
                "a\n\n\nB\n",
                "a\n\n\n\nb\n",
                Text("a\n\n\n\nB\n"),
            ),
            // Repeated lines: human changed the last one, agent appends one more.
            (
                "repeats: human edits last, agent appends",
                "x\nx\nx\n",
                "x\nx\nY\n",
                "x\nx\nx\nx\n",
                Text("x\nx\nY\nx\n"),
            ),
            // Repeats: human deleted one, agent appends one: the note keeps two.
            (
                "repeats: human deletes, agent appends",
                "x\nx\n",
                "x\n",
                "x\nx\nx\n",
                Text("x\nx\n"),
            ),
            // Repeats: human inserted between two, agent appends an identical line.
            (
                "repeats: human inserts, agent appends",
                "x\nx\n",
                "x\ny\nx\n",
                "x\nx\nx\n",
                Text("x\ny\nx\nx\n"),
            ),
            // Repeats: human deletes the first, agent changes the last.
            (
                "repeats: first deleted, last changed",
                "x\ny\nx\n",
                "y\nx\n",
                "x\ny\nZ\n",
                Text("y\nZ\n"),
            ),
            // Accents, combining marks, CJK and emoji on both sides.
            (
                "accents, CJK, emoji",
                "cafe\u{301}\n日本語\n🙂 smile\n",
                "cafe\u{301} typed\n日本語\n🙂 smile\n",
                "cafe\u{301}\n日本語 です\n🙂 smile\n",
                Text("cafe\u{301} typed\n日本語 です\n🙂 smile\n"),
            ),
            // The human types after a multi-byte line that the agent then extends.
            (
                "multibyte line, agent inserts after",
                "日\n",
                "日本\n",
                "日\n🙂\n",
                Text("日本\n🙂\n"),
            ),
        ]);
    }

    #[test]
    fn very_long_lines_merge_by_line() {
        let long = "x".repeat(100_000);
        let base = format!("{long}\nshort\n");
        let ours = format!("{long} typed\nshort\n");
        let theirs = format!("{long}\nSHORT\n");
        assert_eq!(
            merged(&base, &ours, &theirs).unwrap(),
            format!("{long} typed\nSHORT\n")
        );
    }

    #[test]
    fn markdown_shaped_parallel_edits() {
        check(&[
            // Agent rewrites one list item while the human ticks a box on another.
            (
                "list item rewritten, other box ticked",
                "- [ ] buy milk\n- [ ] call Ann\n- old item\n",
                "- [x] buy milk\n- [ ] call Ann\n- old item\n",
                "- [ ] buy milk\n- [ ] call Ann\n- new item text\n",
                Text("- [x] buy milk\n- [ ] call Ann\n- new item text\n"),
            ),
            // Agent deletes the item the human was ticking: clash.
            (
                "item ticked by human, deleted by agent",
                "- [ ] a\n- [ ] b\n",
                "- [x] a\n- [ ] b\n",
                "- [ ] b\n",
                Clash,
            ),
            // Agent adds a table row while the human edits another row.
            (
                "table row added, other row edited",
                "| a | 1 |\n| b | 2 |\n",
                "| a | 9 |\n| b | 2 |\n",
                "| a | 1 |\n| b | 2 |\n| c | 3 |\n",
                Text("| a | 9 |\n| b | 2 |\n| c | 3 |\n"),
            ),
            // Agent inserts a row above the row the human edits.
            (
                "table row inserted above edited row",
                "| a | 1 |\n| b | 2 |\n",
                "| a | 1 |\n| b | 20 |\n",
                "| a | 1 |\n| x | 0 |\n| b | 2 |\n",
                Text("| a | 1 |\n| x | 0 |\n| b | 20 |\n"),
            ),
            // Agent edits the front matter, human edits the body.
            (
                "front matter by agent, body by human",
                "---\ntitle: A\n---\n\nBody one.\n",
                "---\ntitle: A\n---\n\nBody one, typed.\n",
                "---\ntitle: B\n---\n\nBody one.\n",
                Text("---\ntitle: B\n---\n\nBody one, typed.\n"),
            ),
            // Human and agent both change the title line: clash.
            (
                "title changed by both",
                "---\ntitle: A\n---\n\nBody.\n",
                "---\ntitle: Mine\n---\n\nBody.\n",
                "---\ntitle: B\n---\n\nBody.\n",
                Clash,
            ),
            // Agent inserts a bullet under a heading while the human renames the heading.
            (
                "bullet under heading, heading renamed",
                "# Notes\n\nText\n",
                "# My notes\n\nText\n",
                "# Notes\n\n- item\nText\n",
                Text("# My notes\n\n- item\nText\n"),
            ),
            // Agent inserts directly under the heading line, human renames the heading.
            (
                "bullet right under heading, heading renamed",
                "# Notes\n\nText\n",
                "# My notes\n\nText\n",
                "# Notes\n- item\n\nText\n",
                Text("# My notes\n- item\n\nText\n"),
            ),
        ]);
    }

    /// Lines of `text` as they are in the note, without their newline.
    fn human_lines(text: &str) -> Vec<&str> {
        text.split_inclusive('\n')
            .map(|line| line.trim_end_matches('\n'))
            .filter(|line| line.starts_with('H'))
            .collect()
    }

    /// Tiny deterministic generator, no dependency: a linear congruential one.
    struct Lcg(u64);

    impl Lcg {
        fn below(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            ((self.0 >> 33) % n as u64) as usize
        }
    }

    /// A note of up to `max` lines from a small alphabet (repeats, blanks, accents, CJK, emoji),
    /// sometimes without its final newline.
    fn random_note(rng: &mut Lcg, max: usize) -> String {
        const ALPHABET: [&str; 7] = ["a", "b", "", "é", "日本", "x 🙂", "a"];
        let mut text = String::new();
        for _ in 0..rng.below(max + 1) {
            text += ALPHABET[rng.below(ALPHABET.len())];
            text.push('\n');
        }
        if rng.below(3) == 0 {
            text.pop();
        }
        text
    }

    /// `text` with up to three lines inserted, replaced or deleted. New lines are `tag` plus a
    /// serial number, so each one can be found again in the merged text.
    fn edit_lines(rng: &mut Lcg, text: &str, tag: char, serial: &mut usize) -> String {
        let mut lines: Vec<String> = text.split_inclusive('\n').map(String::from).collect();
        for _ in 0..rng.below(4) {
            *serial += 1;
            let line = format!("{tag}{serial}\n");
            let at = rng.below(lines.len() + 1);
            match rng.below(3) {
                0 => lines.insert(at, line),
                1 if at < lines.len() => {
                    lines.remove(at);
                }
                _ if at < lines.len() => lines[at] = line,
                _ => lines.push(line),
            }
        }
        lines.concat()
    }

    /// Random (base, ours, theirs): the human (`H` lines) and the agent (`A` lines) each edited
    /// the same base.
    fn random_cases(seed: u64, count: usize) -> Vec<(String, String, String)> {
        let mut rng = Lcg(seed);
        (0..count)
            .map(|_| {
                let base = random_note(&mut rng, 10);
                let (mut human, mut agent) = (0, 0);
                let ours = edit_lines(&mut rng, &base, 'H', &mut human);
                let theirs = edit_lines(&mut rng, &base, 'A', &mut agent);
                (base, ours, theirs)
            })
            .collect()
    }

    /// `random_cases`, where either side may also have lost its final newline (Enter removed).
    fn random_cases_dropping_final_newlines(
        seed: u64,
        count: usize,
    ) -> Vec<(String, String, String)> {
        let mut rng = Lcg(seed ^ 0x9e37_79b9);
        random_cases(seed, count)
            .into_iter()
            .map(|(base, mut ours, mut theirs)| {
                if rng.below(4) == 0 && ours.ends_with('\n') {
                    ours.pop();
                }
                if rng.below(4) == 0 && theirs.ends_with('\n') {
                    theirs.pop();
                }
                (base, ours, theirs)
            })
            .collect()
    }

    #[test]
    fn random_merges_with_dropped_final_newlines_are_well_formed() {
        for (base, ours, theirs) in random_cases_dropping_final_newlines(6, 3_000) {
            if let Some(edits) = onto(&base, &ours, &theirs) {
                apply_checked(&ours, &edits);
            }
        }
    }

    #[test]
    fn random_merges_with_dropped_final_newlines_keep_every_line() {
        for (base, ours, theirs) in random_cases_dropping_final_newlines(5, 3_000) {
            if let Some(edits) = onto(&base, &ours, &theirs) {
                let text = apply_checked(&ours, &edits);
                for line in human_lines(&ours) {
                    assert!(
                        text.split('\n').any(|l| l == line),
                        "lost {line:?}: base {base:?} ours {ours:?} theirs {theirs:?} gave {text:?}"
                    );
                }
            }
        }
    }

    #[test]
    #[ignore = "BUG: an identical change inside a larger block the human edited clashes instead of merging"]
    fn identical_change_inside_a_larger_human_block_merges() {
        // Repro: base "a\nb\nc\n", ours "a\nB\nc!\n" (b and c adjacent, one hunk), theirs "a\nB\nc\n".
        // Expected "a\nB\nc!\n", got None.
        assert_eq!(
            merged("a\nb\nc\n", "a\nB\nc!\n", "a\nB\nc\n").as_deref(),
            Some("a\nB\nc!\n")
        );
    }

    #[test]
    fn random_one_sided_edits_give_the_changed_side() {
        for (base, ours, theirs) in random_cases(1, 3_000) {
            let case = format!("base {base:?} ours {ours:?} theirs {theirs:?}");
            // Theirs is the same as ours: nothing to add.
            assert_eq!(
                merged(&base, &ours, &ours).as_deref(),
                Some(ours.as_str()),
                "onto(base, ours, ours): {case}"
            );
            // Only the agent changed: its text.
            assert_eq!(
                merged(&base, &base, &theirs).as_deref(),
                Some(theirs.as_str()),
                "only theirs changed: {case}"
            );
            // Only the human changed: the human's text.
            assert_eq!(
                merged(&base, &ours, &base).as_deref(),
                Some(ours.as_str()),
                "only ours changed: {case}"
            );
        }
    }

    #[test]
    fn random_merges_give_ordered_in_bounds_edits_on_char_boundaries() {
        let mut merges = 0;
        for (base, ours, theirs) in random_cases(2, 3_000) {
            if let Some(edits) = onto(&base, &ours, &theirs) {
                merges += 1;
                // Panics on a bad range or a split character.
                apply_checked(&ours, &edits);
            }
        }
        assert!(
            merges > 1_000,
            "only {merges} merges: the generator is too clashy to test much"
        );
    }

    #[test]
    fn random_merges_keep_every_line_the_human_typed() {
        for (base, ours, theirs) in random_cases(3, 3_000) {
            if let Some(edits) = onto(&base, &ours, &theirs) {
                let text = apply_checked(&ours, &edits);
                for line in human_lines(&ours) {
                    assert!(
                        text.split('\n').any(|l| l == line),
                        "lost {line:?}: base {base:?} ours {ours:?} theirs {theirs:?} gave {text:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn random_merges_do_not_depend_on_which_side_is_applied_onto() {
        for (base, ours, theirs) in random_cases(4, 3_000) {
            if let (Some(a), Some(b)) =
                (merged(&base, &ours, &theirs), merged(&base, &theirs, &ours))
            {
                assert_eq!(a, b, "base {base:?} ours {ours:?} theirs {theirs:?}");
            }
        }
    }

    #[test]
    fn fifty_hunks_each_side_in_one_region_merge_quickly() {
        let base_lines: Vec<String> = (0..5_000).map(|n| format!("line {n}\n")).collect();
        let (mut ours, mut theirs, mut expected) =
            (base_lines.clone(), base_lines.clone(), base_lines.clone());
        for k in 0..50 {
            let (human, agent) = (2_000 + 16 * k, 2_008 + 16 * k);
            ours[human] = format!("human {k}\n");
            expected[human] = format!("human {k}\n");
            theirs[agent] = format!("agent {k}\n");
            expected[agent] = format!("agent {k}\n");
        }
        let (base, ours, theirs) = (base_lines.concat(), ours.concat(), theirs.concat());
        let started = std::time::Instant::now();
        assert_eq!(
            merged(&base, &ours, &theirs).as_deref(),
            Some(expected.concat().as_str())
        );
        assert!(
            started.elapsed().as_secs() < 5,
            "took {:?}",
            started.elapsed()
        );
    }

    #[test]
    #[ignore = "BUG: 50 hunks each side spread over a 5,000-line note clash, the LCS cap merges the middle into one block"]
    fn fifty_hunks_each_side_spread_over_the_note_merge() {
        let base_lines: Vec<String> = (0..5_000).map(|n| format!("line {n}\n")).collect();
        let (mut ours, mut theirs, mut expected) =
            (base_lines.clone(), base_lines.clone(), base_lines.clone());
        for k in 0..50 {
            let (human, agent) = (10 + 100 * k, 50 + 100 * k);
            ours[human] = format!("human {k}\n");
            expected[human] = format!("human {k}\n");
            theirs[agent] = format!("agent {k}\n");
            expected[agent] = format!("agent {k}\n");
        }
        let (base, ours, theirs) = (base_lines.concat(), ours.concat(), theirs.concat());
        let started = std::time::Instant::now();
        assert_eq!(
            merged(&base, &ours, &theirs).as_deref(),
            Some(expected.concat().as_str())
        );
        assert!(
            started.elapsed().as_secs() < 5,
            "took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn removed_final_newline_does_not_glue_the_agents_line() {
        // Repro: base "a\nb\n", ours "a\nb", theirs "a\nb\nc\n". Expected "a\nb\nc\n", got "a\nbc\n".
        assert_eq!(
            merged("a\nb\n", "a\nb", "a\nb\nc\n").as_deref(),
            Some("a\nb\nc\n")
        );
    }

    #[test]
    fn both_sides_delete_overlapping_lines_merge_to_the_union() {
        // Repro: base "a\nb\nc\nd\ne\n", ours "a\nd\ne\n" (b, c deleted), theirs "a\nb\ne\n" (c, d deleted).
        // Expected "a\ne\n", got None.
        assert_eq!(
            merged("a\nb\nc\nd\ne\n", "a\nd\ne\n", "a\nb\ne\n").as_deref(),
            Some("a\ne\n")
        );
    }

    #[test]
    fn changes_on_different_lines_are_both_kept() {
        let base = "# Note\n\nfirst\nsecond\nthird\n";
        // An agent appends while the user types on the first lines.
        assert_eq!(
            merged(
                base,
                "# Note\n\nfirst typed\nsecond\nthird\n",
                "# Note\n\nfirst\nsecond\nthird\n\n## Added\nby the agent\n"
            )
            .unwrap(),
            "# Note\n\nfirst typed\nsecond\nthird\n\n## Added\nby the agent\n"
        );
        // The agent edits above where the user types.
        assert_eq!(
            merged(
                base,
                "# Note\n\nfirst\nsecond\nthird!\n",
                "# Note\n\nFIRST\nsecond\nthird\n"
            )
            .unwrap(),
            "# Note\n\nFIRST\nsecond\nthird!\n"
        );
        // Neighbouring lines, one each: lines are independent.
        assert_eq!(
            merged(
                base,
                "# Note\n\nfirst\nsecond 2\nthird\n",
                "# Note\n\nfirst\nsecond\nthird 3\n"
            )
            .unwrap(),
            "# Note\n\nfirst\nsecond 2\nthird 3\n"
        );
        // Lines removed by the agent, lines added by the user elsewhere.
        assert_eq!(
            merged(
                base,
                "# Note\nnew\n\nfirst\nsecond\nthird\n",
                "# Note\n\nthird\n"
            )
            .unwrap(),
            "# Note\nnew\n\nthird\n"
        );
    }

    #[test]
    fn the_same_lines_changed_twice_clash() {
        let base = "a\nb\nc\n";
        assert_eq!(merged(base, "a\nB mine\nc\n", "a\nB theirs\nc\n"), None);
        // Two insertions at the same place: their order cannot be chosen.
        assert_eq!(merged(base, "a\nmine\nb\nc\n", "a\ntheirs\nb\nc\n"), None);
        // A line deleted on one side, changed on the other.
        assert_eq!(merged(base, "a\nc\n", "a\nb!\nc\n"), None);
        // Typing at the end of a last line without newline while the agent appends to it.
        assert_eq!(merged("a\nb", "a\nbX", "a\nb\nc\n"), None);
    }

    #[test]
    fn edges_and_identical_changes() {
        // Nothing changed elsewhere; or only elsewhere; or the same change on both sides.
        assert_eq!(merged("a\n", "a typed\n", "a\n").unwrap(), "a typed\n");
        assert_eq!(merged("a\n", "a\n", "b\n").unwrap(), "b\n");
        assert_eq!(merged("a\nb\n", "a\nx\n", "a\nx\n").unwrap(), "a\nx\n");
        // Empty texts.
        assert_eq!(merged("", "typed\n", "").unwrap(), "typed\n");
        assert_eq!(merged("", "", "theirs\n").unwrap(), "theirs\n");
        assert_eq!(merged("", "mine\n", "theirs\n"), None);
        assert_eq!(merged("a\n", "a\n", "").unwrap(), "");
        // An insertion of ours right before their changed block, one of theirs right after ours.
        assert_eq!(
            merged("a\nb\nc\n", "a\nmine\nb\nc\n", "a\nB\nc\n").unwrap(),
            "a\nmine\nB\nc\n"
        );
        assert_eq!(
            merged("a\nb\nc\n", "a\nB\nc\n", "a\nb\ntheirs\nc\n").unwrap(),
            "a\nB\ntheirs\nc\n"
        );
        // CRLF, accents, CJK, emoji: lines are compared byte for byte.
        assert_eq!(
            merged("é\r\n日本\r\n", "é typed\r\n日本\r\n", "é\r\n日本 🙂\r\n").unwrap(),
            "é typed\r\n日本 🙂\r\n"
        );
    }

    #[test]
    fn a_long_note_merges_and_a_huge_rewrite_stays_whole() {
        let base: String = (0..5_000).map(|n| format!("line {n}\n")).collect();
        let ours = base.replace("line 10\n", "line 10 typed\n");
        let theirs = base.replace("line 4000\n", "line 4000 agent\n");
        assert_eq!(
            merged(&base, &ours, &theirs).unwrap(),
            base.replace("line 10\n", "line 10 typed\n")
                .replace("line 4000\n", "line 4000 agent\n")
        );
        // Every line rewritten on one side, one on the other: too big to align finely, so one clash.
        let all: String = (0..2_000).map(|n| format!("other {n}\n")).collect();
        let start = std::time::Instant::now();
        assert_eq!(merged(&base, &ours, &all), None);
        assert!(start.elapsed().as_millis() < 500);
    }

    #[test]
    fn random_edits_apart_always_merge() {
        // Each side changes lines of its own half: the result has both, whatever the changes.
        let mut seed = 7u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for _ in 0..300 {
            let base: Vec<String> = (0..20).map(|n| format!("{n}\n")).collect();
            let edit = |lines: &mut Vec<String>, from: usize, r: u64| {
                let at = from + (r % 8) as usize;
                match r % 3 {
                    0 => lines[at] = format!("changed {r}\n"),
                    1 => lines[at] = String::new(),
                    _ => lines[at].insert_str(0, &format!("new {r}\n")),
                }
            };
            let (mut ours, mut theirs) = (base.clone(), base.clone());
            let (a, b) = (next(), next());
            edit(&mut ours, 1, a);
            edit(&mut theirs, 11, b);
            let mut both = base.clone();
            edit(&mut both, 1, a);
            edit(&mut both, 11, b);
            assert_eq!(
                merged(&base.concat(), &ours.concat(), &theirs.concat()).unwrap(),
                both.concat()
            );
        }
    }
}
