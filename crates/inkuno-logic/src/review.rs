//! The Review feed of a note (app-ui.md §23): what the right panel lists for it, and in what order.
//! Pure: the text, the pending changes and the clock are given by the caller.

use std::ops::Range;

use crate::markdown;
use crate::pending::{Area, Entry};
use crate::revision;

/// What a feed item is: a comment, or a pending change by its edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Comment,
    Replace,
    Insert,
    Delete,
}

/// The kind of a pending change from its two sides: `Insert` when nothing is removed, `Delete` when
/// nothing is added, `Replace` when both sides have text.
pub fn change_kind(old: &str, new: &str) -> Kind {
    match (old.is_empty(), new.is_empty()) {
        (true, false) => Kind::Insert,
        (false, true) => Kind::Delete,
        _ => Kind::Replace,
    }
}

/// Orders `items` as the feed lists them: the ones without a time first, in document order, then the
/// timed ones, oldest first. `time` is the UTC minute of an item (`2026-11-24T10:02Z`), `at` its
/// byte position in the note.
pub fn sort<T>(items: &mut [T], time: impl Fn(&T) -> Option<&str>, at: impl Fn(&T) -> usize) {
    items.sort_by(|a, b| time(a).cmp(&time(b)).then_with(|| at(a).cmp(&at(b))));
}

/// What the grouping of a feed reads of one entry: whether it is a pending change, who by, and the store
/// call that proposed it (`Entry::run`, `None` when it has none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot<'a> {
    pub change: bool,
    pub author: Option<&'a str>,
    pub run: Option<u64>,
}

/// The runs of entries the feed shows, each as one card: a run is two or more pending changes in a row with the
/// same known run (one Improve proposes them together, in one store call) by the same author. An entry with no
/// run is never grouped. Every other entry is a run of its own. The runs cover `slots` in order.
pub fn runs(slots: &[Slot]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    while start < slots.len() {
        let first = slots[start];
        let mut end = start + 1;
        if first.change && first.run.is_some() {
            while slots.get(end).is_some_and(|next| {
                next.change && next.run == first.run && next.author == first.author
            }) {
                end += 1;
            }
        }
        out.push(start..end);
        start = end;
    }
    out
}

/// The bytes of `areas` that the file does not hold: the marker lines and the added lines of each
/// pending change. Its removed lines stay in the file.
pub fn hidden(areas: &[Area]) -> Vec<Range<usize>> {
    areas
        .iter()
        .flat_map(|area| {
            [
                area.range.start..area.removed.start,
                area.removed.end..area.range.end,
            ]
        })
        .collect()
}

/// The label of the heading that byte `at` is under: the last heading above it. `heads` is
/// `markdown::headings` of the text, read once by the caller. None before the first heading.
pub fn section<'a>(heads: &[(u8, &'a str, usize)], at: usize) -> Option<&'a str> {
    heads
        .iter()
        .rev()
        .find(|(_, _, start)| *start <= at)
        .map(|(_, label, _)| *label)
}

/// The headings of `text`, as `section` takes them.
pub fn headings(text: &str) -> Vec<(u8, &str, usize)> {
    markdown::headings(text)
}

/// Seconds from the UTC minute `time` to `now` (Unix seconds). None when `time` is not a minute.
pub fn seconds_since(now: i64, time: &str) -> Option<i64> {
    Some(now - revision::unix_of(time)?)
}

/// The saved entry of the pending change `old` to `new`, from the list `entries`: its author and its
/// `created` minute. None when no entry has these two sides.
pub fn entry_of<'a>(entries: &'a [Entry], old: &str, new: &str) -> Option<&'a Entry> {
    let same = |a: &str, b: &str| a.trim_end_matches('\n') == b.trim_end_matches('\n');
    entries
        .iter()
        .find(|entry| same(&entry.old, old) && same(&entry.new, new))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_is_an_insert_a_delete_or_a_replace_by_its_sides() {
        assert_eq!(change_kind("", "new\n"), Kind::Insert);
        assert_eq!(change_kind("old\n", ""), Kind::Delete);
        assert_eq!(change_kind("old\n", "new\n"), Kind::Replace);
    }

    #[test]
    fn untimed_items_come_first_in_document_order_then_by_time() {
        // (time, at)
        let mut items = vec![
            (Some("2026-11-24T10:05Z"), 40),
            (None, 90),
            (Some("2026-11-24T09:00Z"), 70),
            (None, 10),
        ];
        sort(&mut items, |i| i.0, |i| i.1);
        let order: Vec<usize> = items.iter().map(|i| i.1).collect();
        assert_eq!(order, vec![10, 90, 70, 40]);
    }

    #[test]
    fn the_feed_numbers_a_line_as_the_file_does() {
        // Line 1 plain, then a change: its marker line, removed "b", marker, added "c", marker.
        let text = "a\n{--\nb\n--}\n{++\nc\n++}\nd\ne";
        let hidden = hidden(&crate::pending::areas(text));
        let lines = crate::lines::FileLines::new(text, &hidden);
        // `d` is the 8th line of the editor's text; on disk the change is `b` alone, so it is the 3rd.
        assert_eq!(lines.line_of(text.find('d').unwrap()), 3);
        assert_eq!(lines.line_of(0), 1);
        // A change is anchored on the line of its first marker: the file line its removed text is on.
        assert_eq!(lines.line_of(text.find("{--").unwrap()), 2);
    }

    #[test]
    fn the_section_is_the_nearest_heading_above() {
        let text = "# Top\nintro\n## Sub\ntext\nend";
        let heads = headings(text);
        assert_eq!(section(&heads, 1), Some("Top"));
        assert_eq!(section(&heads, text.find("text").unwrap()), Some("Sub"));
        assert_eq!(section(&headings("no heading"), 3), None);
    }

    #[test]
    fn a_time_is_read_back_and_measured_from_now() {
        let now = 1_000_000_000;
        assert_eq!(seconds_since(now, "2001-09-09T01:46Z"), Some(40));
        assert_eq!(seconds_since(now, "not a time"), None);
    }

    #[test]
    fn a_change_is_found_by_its_sides_in_the_saved_list() {
        let entry = Entry {
            old: "old\n".into(),
            new: "new\n".into(),
            created: Some("2026-11-24T10:02Z".into()),
            ..Entry::default()
        };
        let entries = [entry];
        let found = entry_of(&entries, "old", "new").expect("the entry");
        assert_eq!(found.created.as_deref(), Some("2026-11-24T10:02Z"));
        assert!(entry_of(&entries, "old", "other").is_none());
    }

    #[test]
    fn a_run_of_changes_from_one_store_call_is_one_card() {
        let change = |author, run| Slot {
            change: true,
            author: Some(author),
            run: Some(run),
        };
        let slots = [
            change("assistant", 1),
            change("assistant", 1),
            change("assistant", 1),
            change("Bo", 1),
            change("Bo", 2),
        ];
        assert_eq!(runs(&slots), [0..3, 3..4, 4..5]);
    }

    #[test]
    fn two_runs_in_the_same_minute_are_two_cards() {
        let change = |run| Slot {
            change: true,
            author: Some("assistant"),
            run: Some(run),
        };
        assert_eq!(
            runs(&[change(1), change(1), change(2), change(2)]),
            [0..2, 2..4],
            "two Improve runs in one minute stay apart"
        );
        assert_eq!(
            runs(&[change(7), change(7)]),
            vec![0..2],
            "one run is one card"
        );
    }

    #[test]
    fn a_single_change_and_a_comment_are_not_grouped() {
        let change = Slot {
            change: true,
            author: Some("assistant"),
            run: Some(1),
        };
        let comment = Slot {
            change: false,
            author: Some("assistant"),
            run: Some(1),
        };
        let no_run = Slot {
            change: true,
            author: Some("assistant"),
            run: None,
        };
        assert_eq!(runs(&[change, comment, change]), [0..1, 1..2, 2..3]);
        assert_eq!(runs(&[no_run, no_run]), [0..1, 1..2], "no run, no group");
        assert!(runs(&[]).is_empty());
    }
}
