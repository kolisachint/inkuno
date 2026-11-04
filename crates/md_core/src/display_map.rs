//! Display map from source to visible text (task 2.6, DESIGN §3.2 and §4.1).
//!
//! The display string is the source text with a set of hidden byte ranges
//! removed. The hidden ranges normally come from
//! [`crate::inline::hidden_ranges`]: the markers of every inline span that does
//! not contain the caret. Caret movement and hit-testing work on the display
//! map, so they skip hidden ranges.
//!
//! Terms used below:
//! - A *collapse point* is the display position where a hidden range was
//!   removed. The source offsets `start..end` of a hidden range all map to it.
//! - A *caret stop* is a source offset that is not strictly inside a hidden
//!   range (`start < offset < end`).

use std::ops::Range;

use crate::inline;
use crate::InlineSpan;

/// Which side of a collapse point a display position resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bias {
    /// The start of the hidden range that collapsed here.
    Left,
    /// The end of the hidden range that collapsed here (first visible byte after it).
    Right,
}

/// Caret movement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

/// Source-to-display mapping for one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisplayMap {
    source_len: usize,
    /// Sorted, non-overlapping, non-adjacent, non-empty, inside `0..source_len`.
    hidden: Vec<Range<usize>>,
    /// `prefix[i]` is the total length of `hidden[..i]`. Has `hidden.len() + 1` entries.
    prefix: Vec<usize>,
}

impl DisplayMap {
    /// Build a map for a source of `source_len` bytes with the given hidden ranges.
    ///
    /// `hidden` may be unsorted and overlapping. It is normalized: ranges are
    /// clipped to `0..source_len`, empty ones are dropped, and the rest are
    /// sorted and merged (adjacent ranges merge too).
    pub fn new(source_len: usize, hidden: Vec<Range<usize>>) -> Self {
        let mut clipped: Vec<Range<usize>> = hidden
            .into_iter()
            .map(|r| r.start.min(source_len)..r.end.min(source_len))
            .filter(|r| r.start < r.end)
            .collect();
        clipped.sort_by_key(|r| (r.start, r.end));

        let mut merged: Vec<Range<usize>> = Vec::with_capacity(clipped.len());
        for r in clipped {
            match merged.last_mut() {
                Some(last) if r.start <= last.end => last.end = last.end.max(r.end),
                _ => merged.push(r),
            }
        }

        let mut prefix = Vec::with_capacity(merged.len() + 1);
        let mut total = 0;
        prefix.push(0);
        for r in &merged {
            total += r.end - r.start;
            prefix.push(total);
        }

        Self {
            source_len,
            hidden: merged,
            prefix,
        }
    }

    /// Build the map for `text` with the caret at `caret` (`None` hides every marker).
    pub fn for_text(text: &str, caret: Option<usize>) -> Self {
        let spans: Vec<InlineSpan> = inline::inline_spans(text, 0);
        let hidden = inline::hidden_ranges(&spans, caret);
        Self::new(text.len(), hidden)
    }

    /// Normalized hidden ranges.
    pub fn hidden(&self) -> &[Range<usize>] {
        &self.hidden
    }

    /// Length of the display string in bytes.
    pub fn display_len(&self) -> usize {
        self.source_len - self.total_hidden()
    }

    /// True when `offset` is in `start..end` of some hidden range.
    pub fn is_hidden(&self, offset: usize) -> bool {
        let i = self.hidden.partition_point(|r| r.end <= offset);
        self.hidden.get(i).is_some_and(|r| r.start <= offset)
    }

    /// Source offset to display offset. An offset inside a hidden range maps
    /// to the display position where that range collapsed.
    pub fn to_display(&self, source: usize) -> usize {
        let source = source.min(self.source_len);
        // Ranges that end at or before `source` are removed in full.
        let i = self.hidden.partition_point(|r| r.end <= source);
        let mut removed = self.prefix[i];
        // At most one range straddles `source`: its part before `source` is removed too.
        if let Some(r) = self.hidden.get(i) {
            if r.start < source {
                removed += source - r.start;
            }
        }
        source - removed
    }

    /// Display offset to source offset. At a collapse point, `Bias::Left`
    /// returns the hidden range's start and `Bias::Right` its end.
    pub fn to_source(&self, display: usize, bias: Bias) -> usize {
        let display = display.min(self.display_len());
        let n = self.hidden.len();
        // Collapse points are strictly increasing, so binary search applies.
        // Number of ranges whose collapse point lies before `display`.
        let (mut lo, mut hi) = (0, n);
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.collapse_point(mid) < display {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let i = lo;
        if i < n && self.collapse_point(i) == display {
            return match bias {
                Bias::Left => self.hidden[i].start,
                Bias::Right => self.hidden[i].end,
            };
        }
        display + self.prefix[i]
    }

    /// Nearest caret stop for hit-testing. A source offset strictly inside a
    /// hidden range snaps to the closer end of that range; ties go to the end.
    pub fn clip(&self, source: usize) -> usize {
        let source = source.min(self.source_len);
        match self.strictly_hidden_range(source) {
            Some(r) => {
                if source - r.start < r.end - source {
                    r.start
                } else {
                    r.end
                }
            }
            None => source,
        }
    }

    /// Move the caret one visible character in `dir`, skipping hidden ranges.
    ///
    /// The result is the nearest caret stop whose display position differs
    /// from the current one in that direction, so one keystroke passes exactly
    /// one visible character, and the caret never lands strictly inside a
    /// hidden range. Moving right from before a hidden `**` in `a **bold**` lands
    /// after the `b`. Returns `caret` unchanged at the document ends.
    ///
    /// `text` must be the text this map was built from (same length).
    pub fn move_caret(&self, text: &str, caret: usize, dir: Direction) -> usize {
        debug_assert_eq!(text.len(), self.source_len, "text does not match the map");
        let mut caret = caret.min(self.source_len);
        // A caret strictly inside a hidden range starts from that range's edge
        // in the direction of travel. Both edges share one display position.
        if let Some(r) = self.strictly_hidden_range(caret) {
            caret = match dir {
                Direction::Left => r.start,
                Direction::Right => r.end,
            };
        }
        let here = self.to_display(caret);
        match dir {
            Direction::Right => (caret + 1..=self.source_len)
                .find(|&q| {
                    text.is_char_boundary(q)
                        && !self.is_strictly_hidden(q)
                        && self.to_display(q) > here
                })
                .unwrap_or(caret),
            Direction::Left => (0..caret)
                .rev()
                .find(|&q| {
                    text.is_char_boundary(q)
                        && !self.is_strictly_hidden(q)
                        && self.to_display(q) < here
                })
                .unwrap_or(caret),
        }
    }

    fn total_hidden(&self) -> usize {
        self.prefix[self.hidden.len()]
    }

    /// Display position where hidden range `k` collapsed.
    fn collapse_point(&self, k: usize) -> usize {
        self.hidden[k].start - self.prefix[k]
    }

    fn is_strictly_hidden(&self, offset: usize) -> bool {
        self.strictly_hidden_range(offset).is_some()
    }

    fn strictly_hidden_range(&self, offset: usize) -> Option<&Range<usize>> {
        let i = self.hidden.partition_point(|r| r.end <= offset);
        self.hidden
            .get(i)
            .filter(|r| r.start < offset && offset < r.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// "a **bold** c": markers 2..4 and 8..10, visible text "a bold c".
    const T: &str = "a **bold** c";

    fn sample() -> DisplayMap {
        DisplayMap::for_text(T, Some(0))
    }

    #[test]
    fn for_text_hides_both_marker_runs() {
        let d = sample();
        assert_eq!(d.hidden(), &[2..4, 8..10]);
        assert_eq!(d.display_len(), 8);
    }

    #[test]
    fn caret_inside_span_hides_nothing() {
        let d = DisplayMap::for_text(T, Some(4));
        assert!(d.hidden().is_empty());
        assert_eq!(d.display_len(), T.len());
    }

    #[test]
    fn caret_none_hides_all_markers() {
        let d = DisplayMap::for_text(T, None);
        assert_eq!(d.hidden(), &[2..4, 8..10]);
    }

    #[test]
    fn empty_hidden_set_is_identity() {
        let d = DisplayMap::new(5, vec![]);
        assert_eq!(d.display_len(), 5);
        for i in 0..=5 {
            assert_eq!(d.to_display(i), i);
            assert_eq!(d.to_source(i, Bias::Left), i);
            assert_eq!(d.to_source(i, Bias::Right), i);
            assert_eq!(d.clip(i), i);
            assert!(!d.is_hidden(i));
        }
        assert_eq!(d.move_caret("hello", 0, Direction::Right), 1);
        assert_eq!(d.move_caret("hello", 5, Direction::Left), 4);
    }

    #[test]
    fn new_normalizes_unsorted_overlapping_and_clipped_ranges() {
        // 2..4 and 3..6 overlap, 5..7 and 6..8 overlap the chain, 4..4 and 8..3 are
        // empty or inverted, 9..20 is clipped to 9..10.
        let empty = std::ops::Range { start: 4, end: 4 };
        let inverted = std::ops::Range { start: 8, end: 3 };
        let d = DisplayMap::new(10, vec![9..20, 5..7, empty, 3..6, inverted, 2..4, 6..8]);
        assert_eq!(d.hidden(), &[2..8, 9..10]);
        assert_eq!(d.display_len(), 3);
    }

    #[test]
    fn adjacent_ranges_merge() {
        let d = DisplayMap::new(10, vec![5..7, 1..3, 3..5]);
        assert_eq!(d.hidden().len(), 1);
        assert_eq!(d.hidden()[0], 1..7);
        assert_eq!(d.display_len(), 4);
    }

    #[test]
    fn ranges_past_source_end_are_dropped() {
        let d = DisplayMap::new(4, vec![6..9, 10..12]);
        assert!(d.hidden().is_empty());
        assert_eq!(d.display_len(), 4);
    }

    #[test]
    fn to_display_at_boundaries() {
        let d = sample();
        // Visible: a(0) space(1) b(4) o(5) l(6) d(7) space(10) c(11).
        let expected = [0, 1, 2, 2, 2, 3, 4, 5, 6, 6, 6, 7, 8];
        for (source, display) in expected.iter().enumerate() {
            assert_eq!(d.to_display(source), *display, "source {source}");
        }
    }

    #[test]
    fn to_source_at_collapse_points() {
        let d = sample();
        assert_eq!(d.to_source(2, Bias::Left), 2);
        assert_eq!(d.to_source(2, Bias::Right), 4);
        assert_eq!(d.to_source(6, Bias::Left), 8);
        assert_eq!(d.to_source(6, Bias::Right), 10);
    }

    #[test]
    fn to_source_between_collapse_points() {
        let d = sample();
        assert_eq!(d.to_source(0, Bias::Left), 0);
        assert_eq!(d.to_source(1, Bias::Right), 1);
        assert_eq!(d.to_source(3, Bias::Left), 5);
        assert_eq!(d.to_source(7, Bias::Right), 11);
    }

    #[test]
    fn to_source_at_document_ends() {
        let d = sample();
        assert_eq!(d.to_source(0, Bias::Right), 0);
        assert_eq!(d.to_source(d.display_len(), Bias::Left), T.len());
        assert_eq!(d.to_source(d.display_len(), Bias::Right), T.len());
    }

    #[test]
    fn hidden_range_at_document_start_and_end() {
        let d = DisplayMap::new(4, vec![0..1, 3..4]);
        assert_eq!(d.display_len(), 2);
        assert_eq!(d.to_display(0), 0);
        assert_eq!(d.to_display(4), 2);
        assert_eq!(d.to_source(0, Bias::Left), 0);
        assert_eq!(d.to_source(0, Bias::Right), 1);
        assert_eq!(d.to_source(2, Bias::Left), 3);
        assert_eq!(d.to_source(2, Bias::Right), 4);
    }

    #[test]
    fn is_hidden_is_half_open() {
        let d = sample();
        assert!(!d.is_hidden(1));
        assert!(d.is_hidden(2));
        assert!(d.is_hidden(3));
        assert!(!d.is_hidden(4));
        assert!(d.is_hidden(9));
        assert!(!d.is_hidden(10));
    }

    #[test]
    fn clip_snaps_inside_hidden_to_closer_end() {
        let d = sample();
        // Inside the first "**" (2..4): offset 3 is equidistant, so it goes to the end.
        assert_eq!(d.clip(3), 4);
        // Inside the second "**" (8..10): offset 9 ties, so it goes to the end.
        assert_eq!(d.clip(9), 10);
        // Asymmetric range: 0..5 with offset 1 is nearer the start, offset 3 the end.
        let first: Range<usize> = 0..5;
        let e = DisplayMap::new(8, vec![first]);
        assert_eq!(e.clip(1), 0);
        assert_eq!(e.clip(3), 5);
    }

    #[test]
    fn clip_keeps_caret_stops_and_clamps() {
        let d = sample();
        assert_eq!(d.clip(0), 0);
        assert_eq!(d.clip(2), 2);
        assert_eq!(d.clip(4), 4);
        assert_eq!(d.clip(10), 10);
        assert_eq!(d.clip(T.len()), T.len());
        assert_eq!(d.clip(1000), T.len());
    }

    #[test]
    fn move_right_passes_one_visible_char() {
        let d = sample();
        let next = d.move_caret(T, 2, Direction::Right);
        assert_eq!(next, 5);
        assert_eq!(d.to_display(next), d.to_display(2) + 1);
        assert!(!d.is_hidden(next));
    }

    #[test]
    fn move_left_passes_one_visible_char() {
        let d = sample();
        assert_eq!(d.move_caret(T, 5, Direction::Left), 4);
        assert_eq!(d.move_caret(T, 4, Direction::Left), 1);
    }

    #[test]
    fn move_across_closing_markers() {
        let d = sample();
        // From after 'd' (display 6) moving right passes the space and reaches display 7.
        assert_eq!(d.move_caret(T, 8, Direction::Right), 11);
        assert_eq!(d.move_caret(T, 11, Direction::Left), 10);
    }

    #[test]
    fn move_at_document_ends_is_unchanged() {
        let d = sample();
        assert_eq!(d.move_caret(T, 0, Direction::Left), 0);
        assert_eq!(d.move_caret(T, T.len(), Direction::Right), T.len());
        assert_eq!(d.move_caret(T, 0, Direction::Right), 1);
        assert_eq!(d.move_caret(T, T.len(), Direction::Left), 11);
    }

    #[test]
    fn move_over_trailing_hidden_range_stops_at_visible_end() {
        let tail: Range<usize> = 1..3;
        let d = DisplayMap::new(3, vec![tail]);
        // "a" is visible; the trailing 2 bytes are hidden.
        assert_eq!(d.move_caret("abc", 0, Direction::Right), 1);
        assert_eq!(d.move_caret("abc", 1, Direction::Right), 1);
        assert_eq!(d.move_caret("abc", 3, Direction::Left), 0);
    }

    #[test]
    fn move_from_inside_hidden_range_starts_at_edge() {
        // "wxyz" with 1..3 hidden: display is "wz".
        let middle: Range<usize> = 1..3;
        let d = DisplayMap::new(4, vec![middle]);
        assert_eq!(d.move_caret("wxyz", 2, Direction::Left), 0);
        assert_eq!(d.move_caret("wxyz", 2, Direction::Right), 4);
    }

    #[test]
    fn move_handles_multibyte_text() {
        // "é**x**": é is 0..2, markers 2..4 and 5..7, x at 4..5.
        let t = "é**x**";
        let d = DisplayMap::for_text(t, Some(0));
        assert_eq!(d.hidden(), &[2..4, 5..7]);
        assert_eq!(d.display_len(), 3);
        // Right from 0 passes é; the next caret stop is 2 (display 2).
        assert_eq!(d.move_caret(t, 0, Direction::Right), 2);
        // Right from 2 passes x; the next caret stop is 5 (display 3).
        assert_eq!(d.move_caret(t, 2, Direction::Right), 5);
        // Left from 7 passes x and lands before it (display 2, source 4).
        assert_eq!(d.move_caret(t, 7, Direction::Left), 4);
        assert_eq!(d.to_display(4), 2);
    }

    #[test]
    fn multibyte_round_trip() {
        let t = "日本 **語** é";
        let d = DisplayMap::for_text(t, Some(0));
        for s in (0..=t.len()).filter(|&s| t.is_char_boundary(s)) {
            if d.hidden().iter().any(|r| r.start < s && s < r.end) {
                continue;
            }
            let disp = d.to_display(s);
            let back = d.to_source(disp, Bias::Left);
            assert!(
                back == s || d.to_source(disp, Bias::Right) == s,
                "source {s}"
            );
        }
    }

    #[test]
    fn caret_inside_span_hit_test_stays_visible_after_move() {
        let d = sample();
        for start in 0..=T.len() {
            if !T.is_char_boundary(start) {
                continue;
            }
            for dir in [Direction::Left, Direction::Right] {
                let next = d.move_caret(T, start, dir);
                assert!(
                    !d.hidden().iter().any(|r| r.start < next && next < r.end),
                    "{start} {dir:?} -> {next}"
                );
            }
        }
    }
}
