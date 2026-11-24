//! Line numbers of a note: the 1-based line of a byte offset in the file, and the digits the gutter needs.
//! The numbers are the lines of the `.md` on disk, so a line the editor hides keeps its number and leaves a gap
//! (app-ui.md §23.7). The Review feed and the editor's gutter both number through `FileLines`.

use std::ops::Range;

/// The lines of a note as the file holds them. The text may show more than the file: the marks of pending
/// changes and their added lines are `hidden` (`review::hidden`). A hidden line breaks nothing in the file, so it
/// adds no number, and a line that starts in hidden text has none of its own.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct FileLines {
    /// Each line of the text: where it starts, and its file line; None when it starts in hidden text.
    starts: Vec<(usize, Option<usize>)>,
    /// The bytes of the line breaks the file holds, in order.
    breaks: Vec<usize>,
}

impl FileLines {
    /// The file lines of `text`, leaving out the bytes of `hidden`.
    pub fn new(text: &str, hidden: &[Range<usize>]) -> Self {
        let hidden = merged(hidden);
        let in_hidden = |at: usize| {
            let i = hidden.partition_point(|r| r.end <= at);
            hidden.get(i).is_some_and(|r| r.start <= at)
        };
        let mut starts = Vec::new();
        let mut breaks = Vec::new();
        let mut start = 0;
        loop {
            let number = (!in_hidden(start)).then(|| breaks.len() + 1);
            starts.push((start, number));
            match text[start..].find('\n') {
                Some(i) => {
                    let at = start + i;
                    if !in_hidden(at) {
                        breaks.push(at);
                    }
                    start = at + 1;
                }
                None => break,
            }
        }
        Self { starts, breaks }
    }

    /// How many lines the file has (the last one may be empty, as an editor shows it).
    pub fn count(&self) -> usize {
        self.breaks.len() + 1
    }

    /// The 1-based file line of byte `at` of the text: the line breaks the file holds before it, plus one. An
    /// offset in hidden text takes the number of the file line it falls on; past the end, the last line.
    pub fn line_of(&self, at: usize) -> usize {
        self.breaks.partition_point(|&brk| brk < at) + 1
    }

    /// The file line a display line starting at `start` shows: None when `start` begins no line of the text, or
    /// the line is hidden (a mark or an added line).
    pub fn number_of_line(&self, start: usize) -> Option<usize> {
        let i = self.starts.binary_search_by_key(&start, |&(s, _)| s).ok()?;
        self.starts[i].1
    }
}

/// The ranges sorted by start, with the ones that overlap or touch joined; empty ranges dropped.
fn merged(ranges: &[Range<usize>]) -> Vec<Range<usize>> {
    let mut sorted: Vec<Range<usize>> = ranges.iter().filter(|r| !r.is_empty()).cloned().collect();
    sorted.sort_by_key(|r| r.start);
    let mut out: Vec<Range<usize>> = Vec::new();
    for range in sorted {
        match out.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => out.push(range),
        }
    }
    out
}

/// The characters the widest of the numbers `1..=lines` takes: the width of the gutter, in digits (at least one).
pub fn digits(lines: usize) -> usize {
    lines.max(1).to_string().len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pending::areas;
    use crate::review::hidden;

    #[test]
    fn without_hidden_text_the_lines_are_counted_plainly() {
        let lines = FileLines::new("ab\ncd\nef", &[]);
        assert_eq!(lines.count(), 3);
        let numbers: Vec<usize> = (0..=8).map(|i| lines.line_of(i)).collect();
        assert_eq!(numbers, [1, 1, 1, 2, 2, 2, 3, 3, 3]);
        assert_eq!(lines.number_of_line(0), Some(1));
        assert_eq!(lines.number_of_line(3), Some(2));
        assert_eq!(lines.number_of_line(1), None, "not a line start");
    }

    #[test]
    fn a_trailing_newline_makes_an_empty_last_line() {
        let lines = FileLines::new("a\n", &[]);
        assert_eq!(lines.count(), 2);
        assert_eq!(lines.line_of(2), 2);
        assert_eq!(lines.number_of_line(2), Some(2));
        assert_eq!(FileLines::new("", &[]).count(), 1);
    }

    #[test]
    fn an_offset_past_the_end_is_on_the_last_line() {
        assert_eq!(FileLines::new("a\nb", &[]).line_of(99), 2);
    }

    #[test]
    fn hidden_text_adds_no_line_and_its_lines_have_no_number() {
        // Line 1 plain, then a change: its marker line, removed "old", marker, added "new", marker.
        let text = "a\n{--\nold\n--}\n{++\nnew\n++}\nend";
        let hidden = hidden(&areas(text));
        let lines = FileLines::new(text, &hidden);
        // On disk: `a`, `old`, `end`.
        assert_eq!(lines.number_of_line(0), Some(1));
        assert_eq!(lines.number_of_line(2), None, "the marker line");
        assert_eq!(lines.number_of_line(text.find("old").unwrap()), Some(2));
        assert_eq!(lines.number_of_line(text.find("--}").unwrap()), None);
        assert_eq!(
            lines.number_of_line(text.find("new").unwrap()),
            None,
            "an added line"
        );
        assert_eq!(lines.number_of_line(text.find("end").unwrap()), Some(3));
        assert_eq!(lines.line_of(text.find("end").unwrap()), 3);
        assert_eq!(lines.count(), 3);
        // A change's anchor (its first marker) is the file line its removed text is on.
        assert_eq!(lines.line_of(2), 2);
    }

    #[test]
    fn overlapping_hidden_ranges_are_one() {
        // `b` and the break after it, and `c`, are hidden: the file holds `a` alone.
        let lines = FileLines::new("a\nb\nc", &[2..4, 3..5]);
        assert_eq!(lines.number_of_line(2), None);
        assert_eq!(lines.number_of_line(4), None);
        assert_eq!(lines.count(), 2);
    }

    #[test]
    fn digits_fit_the_widest_number() {
        assert_eq!(digits(0), 1);
        assert_eq!(digits(1), 1);
        assert_eq!(digits(9), 1);
        assert_eq!(digits(10), 2);
        assert_eq!(digits(999), 3);
        assert_eq!(digits(1000), 4);
    }
}
