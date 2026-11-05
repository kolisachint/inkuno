//! Inline reveal (plan task 4.4, DESIGN §4.1 and §4.2): which source bytes are hidden for a
//! caret, and how source offsets map to the visible text of a line.
//!
//! Pure functions with no GPUI types, so they are unit-tested on every platform. The element
//! (`element.rs`) shapes each line without its hidden bytes and keeps the hidden ranges of the
//! line in its `LineLayout`. Caret x, hit testing, selection rectangles and IME bounds then
//! convert between source and visible offsets with [`source_to_display`] and
//! [`display_to_source`].
//!
//! A *visible* offset is a byte offset into the text that is shaped: the line with its hidden
//! bytes removed, relative to the start of the line. A *source* offset is an absolute byte
//! offset in the document.

use std::ops::Range;

use md_core::block::split_blocks;
use md_core::inline::{hidden_ranges, inline_spans};
use md_core::BlockKind;

use crate::blocks::basic::shows_inline_styles;
use crate::blocks::list::parse_list_lines;

/// Hidden byte ranges of one top-level block, as absolute offsets, sorted and merged.
///
/// `block_text` is the block's text without its trailing newline and `base` is its offset in
/// the document. `caret` is the document caret. `None`, or a caret outside the block, means the
/// block is not revealed, so:
/// - its block markers are hidden: the `#`s and the space after a heading, the `>` and the
///   space after each blockquote line, and the whole text of a thematic break (drawn as a rule);
/// - the inline markers of every span are hidden, except those of spans that contain the caret
///   (DESIGN §4.1). Only blocks that show inline styles have inline spans.
///
/// A list block hides the markers of each line whose caret is not on it (task 4.2). An item
/// line with the caret keeps its marker and hides only its indentation, since the element draws
/// it at its depth.
///
/// A block containing the caret keeps all its markers visible.
pub fn block_hidden_ranges(
    block_text: &str,
    base: usize,
    kind: &BlockKind,
    caret: Option<usize>,
) -> Vec<Range<usize>> {
    let end = base + block_text.len();
    let caret = caret.filter(|caret| (base..=end).contains(caret));
    let mut hidden = Vec::new();
    if caret.is_none() {
        match kind {
            BlockKind::Heading(_) => hidden.extend(heading_marker(block_text, base)),
            BlockKind::BlockQuote => hidden.extend(quote_markers(block_text, base)),
            BlockKind::ThematicBreak => hidden.push(base..end),
            _ => {}
        }
    }
    if matches!(kind, BlockKind::List) {
        hidden.extend(list_hidden_ranges(block_text, base, caret));
    }
    if shows_inline_styles(kind) {
        hidden.extend(hidden_ranges(&inline_spans(block_text, base), caret));
    }
    merge_ranges(hidden)
}

/// The hidden ranges of a list block, one per line (task 4.2). `caret` is already inside the
/// block or `None`. A line holds the caret when the caret is in its source, end included.
fn list_hidden_ranges(text: &str, base: usize, caret: Option<usize>) -> Vec<Range<usize>> {
    let mut hidden = Vec::new();
    let mut line_start = 0;
    for (line, item) in text.split('\n').zip(parse_list_lines(text, base)) {
        let abs = base + line_start;
        let revealed = caret.is_some_and(|caret| (abs..=abs + line.len()).contains(&caret));
        if item.marker.is_some() && revealed {
            // The indentation goes; the marker stays in the source.
            let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
            hidden.push(abs..abs + indent);
        } else {
            hidden.push(item.marker_range);
        }
        line_start += line.len() + 1;
    }
    hidden
}

/// Hidden byte ranges of every top-level block of `text`, as absolute offsets, sorted and
/// merged. It is the union of [`block_hidden_ranges`] over the blocks, computed the same way the
/// element computes them for each block, so caret movement sees what is drawn.
pub fn document_hidden_ranges(text: &str, caret: Option<usize>) -> Vec<Range<usize>> {
    let mut hidden = Vec::new();
    for block in split_blocks(text) {
        let mut range = block.range.clone();
        if text[range.clone()].ends_with('\n') {
            range.end -= 1;
        }
        hidden.extend(block_hidden_ranges(
            &text[range.clone()],
            range.start,
            &block.kind,
            caret,
        ));
    }
    merge_ranges(hidden)
}

/// The visible text of one source line and the hidden ranges that fall in it.
///
/// `line` starts at absolute offset `line_base`. `hidden` may cover more than the line. The
/// returned ranges are the hidden ranges clipped to the line, absolute, sorted and merged.
pub fn visible_text(
    line: &str,
    line_base: usize,
    hidden: &[Range<usize>],
) -> (String, Vec<Range<usize>>) {
    let clipped = clip_ranges(hidden, line_base..line_base + line.len());
    let mut text = String::with_capacity(line.len());
    let mut pos = line_base;
    for range in &clipped {
        text.push_str(&line[pos - line_base..range.start - line_base]);
        pos = range.end;
    }
    text.push_str(&line[pos - line_base..]);
    (text, clipped)
}

/// The visible offset for source offset `source` in a line that starts at `line_base`.
///
/// Every hidden byte before `source` is removed. An offset inside a hidden range maps to the
/// position where that range collapsed, so it is the same as the offset at its start.
pub fn source_to_display(source: usize, line_base: usize, hidden: &[Range<usize>]) -> usize {
    let removed: usize = hidden
        .iter()
        .map(|range| overlap(range, line_base..source.max(line_base)))
        .sum();
    source.saturating_sub(line_base) - removed
}

/// The source offset for visible offset `display` in a line that starts at `line_base`.
///
/// The result is never strictly inside a hidden range. A visible offset at a collapse point,
/// where a hidden range was removed, has two source offsets, the start and the end of that
/// range. `prefer_after` picks the end, otherwise the start.
pub fn display_to_source(
    display: usize,
    line_base: usize,
    hidden: &[Range<usize>],
    prefer_after: bool,
) -> usize {
    let mut removed = 0;
    for range in hidden {
        let start = range.start.max(line_base);
        if start >= range.end {
            continue;
        }
        // The visible offset where this range collapses.
        let collapse = (start - line_base) - removed;
        if collapse < display {
            removed += range.end - start;
        } else if collapse == display {
            return if prefer_after { range.end } else { start };
        } else {
            break;
        }
    }
    line_base + display + removed
}

/// Visible lengths of the runs of a line. `lens` are the lengths of consecutive runs that cover
/// the line from `line_base`. Returns `(run index, visible length)` for each run that has visible
/// bytes, so that a run that is entirely hidden is dropped.
pub fn visible_run_lengths(
    lens: &[usize],
    line_base: usize,
    hidden: &[Range<usize>],
) -> Vec<(usize, usize)> {
    let mut pos = line_base;
    let mut out = Vec::with_capacity(lens.len());
    for (index, &len) in lens.iter().enumerate() {
        let run = pos..pos + len;
        let hidden_bytes: usize = hidden.iter().map(|range| overlap(range, run.clone())).sum();
        let visible = len - hidden_bytes;
        if visible > 0 {
            out.push((index, visible));
        }
        pos += len;
    }
    out
}

/// Merge overlapping and adjacent ranges. Empty ranges are dropped. The result is sorted.
pub(crate) fn merge_ranges(mut ranges: Vec<Range<usize>>) -> Vec<Range<usize>> {
    ranges.retain(|range| range.start < range.end);
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    merged
}

/// Ranges clipped to `window`, empty ones dropped, merged and sorted.
fn clip_ranges(ranges: &[Range<usize>], window: Range<usize>) -> Vec<Range<usize>> {
    merge_ranges(
        ranges
            .iter()
            .map(|range| range.start.max(window.start)..range.end.min(window.end))
            .collect(),
    )
}

/// Number of bytes in both ranges.
fn overlap(a: &Range<usize>, b: Range<usize>) -> usize {
    let start = a.start.max(b.start);
    let end = a.end.min(b.end);
    end.saturating_sub(start)
}

/// The `#`s of an ATX heading and the spaces or tabs after them, as an absolute range. Leading
/// spaces are kept visible.
fn heading_marker(text: &str, base: usize) -> Option<Range<usize>> {
    let rest = text.trim_start_matches(' ');
    let start = text.len() - rest.len();
    let hashes = rest.bytes().take_while(|&byte| byte == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let spaces = rest.as_bytes()[hashes..]
        .iter()
        .take_while(|&&byte| byte == b' ' || byte == b'\t')
        .count();
    let marker_start = base + start;
    Some(marker_start..marker_start + hashes + spaces)
}

/// The `>` markers of each blockquote line, each with the one space after it, as absolute
/// ranges. Nested markers (`> > `) are one range per line.
fn quote_markers(text: &str, base: usize) -> Vec<Range<usize>> {
    let mut markers = Vec::new();
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let indent = line.len() - line.trim_start_matches(' ').len();
        let rest = &line.as_bytes()[indent..];
        let mut len = 0;
        while len < rest.len() && rest[len] == b'>' {
            len += 1;
            if len < rest.len() && rest[len] == b' ' {
                len += 1;
            }
        }
        if len > 0 {
            let start = base + line_start + indent;
            markers.push(start..start + len);
        }
        line_start += line.len();
    }
    markers
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Visible text of a whole single-line `text` at base 0, with the hidden ranges of its
    /// block at `caret`.
    fn visible_of(text: &str, kind: &BlockKind, caret: Option<usize>) -> String {
        let hidden = block_hidden_ranges(text, 0, kind, caret);
        visible_text(text, 0, &hidden).0
    }

    #[test]
    fn strong_markers_hide_when_the_caret_is_outside() {
        let text = "a **b** c";
        assert_eq!(
            block_hidden_ranges(text, 0, &BlockKind::Paragraph, Some(0)),
            vec![2..4, 5..7]
        );
        assert_eq!(visible_of(text, &BlockKind::Paragraph, Some(0)), "a b c");
        assert_eq!(visible_of(text, &BlockKind::Paragraph, None), "a b c");
    }

    #[test]
    fn strong_markers_show_when_the_caret_is_inside() {
        let text = "a **b** c";
        // Inside the span, and at its edges (the span range is inclusive).
        for caret in [3, 4, 6, 7] {
            assert!(
                block_hidden_ranges(text, 0, &BlockKind::Paragraph, Some(caret)).is_empty(),
                "caret {caret}"
            );
        }
        assert_eq!(
            visible_of(text, &BlockKind::Paragraph, Some(4)),
            "a **b** c"
        );
    }

    #[test]
    fn each_span_type_hides_its_markers() {
        let cases = [
            ("*it*", vec![0..1, 3..4]),
            ("`x`", vec![0..1, 2..3]),
            ("~~gone~~", vec![0..2, 6..8]),
            ("$x$", vec![0..1, 2..3]),
            ("[t](u)", vec![0..1, 2..6]),
            ("![a](s)", vec![0..2, 3..7]),
        ];
        for (text, expected) in cases {
            assert_eq!(
                block_hidden_ranges(text, 0, &BlockKind::Paragraph, None),
                expected,
                "span in {text:?}"
            );
            // The same markers are visible with the caret inside the span.
            assert!(
                block_hidden_ranges(text, 0, &BlockKind::Paragraph, Some(1)).is_empty(),
                "caret inside {text:?} reveals its markers"
            );
        }
    }

    #[test]
    fn link_shows_its_text_and_hides_brackets_and_url() {
        let text = "[docs](https://example.com) x";
        assert_eq!(visible_of(text, &BlockKind::Paragraph, Some(28)), "docs x");
        assert_eq!(
            visible_of(text, &BlockKind::Paragraph, Some(1)),
            "[docs](https://example.com) x",
            "the caret inside the link shows it whole"
        );
    }

    #[test]
    fn heading_hides_its_marker_only_when_the_caret_is_not_in_the_block() {
        assert_eq!(
            block_hidden_ranges("# T", 0, &BlockKind::Heading(1), None),
            vec![0..2]
        );
        assert_eq!(visible_of("# T", &BlockKind::Heading(1), None), "T");
        assert!(block_hidden_ranges("# T", 0, &BlockKind::Heading(1), Some(2)).is_empty());
        assert!(block_hidden_ranges("# T", 0, &BlockKind::Heading(1), Some(0)).is_empty());
        // A caret in another block does not reveal this one.
        assert_eq!(
            block_hidden_ranges("# T", 10, &BlockKind::Heading(1), Some(2)),
            vec![10..12]
        );
    }

    #[test]
    fn heading_markers_are_absolute_and_handle_levels() {
        assert_eq!(
            block_hidden_ranges("### Deep", 20, &BlockKind::Heading(3), None),
            vec![20..24]
        );
        assert_eq!(
            block_hidden_ranges("#nospace", 0, &BlockKind::Heading(1), None),
            vec![0..1],
            "a `#` with no space is only its own marker"
        );
    }

    #[test]
    fn blockquote_hides_each_line_marker() {
        let text = "> a\n> b";
        assert_eq!(
            block_hidden_ranges(text, 0, &BlockKind::BlockQuote, None),
            vec![0..2, 4..6]
        );
        assert_eq!(visible_of(text, &BlockKind::BlockQuote, None), "a\nb");
        assert!(block_hidden_ranges(text, 0, &BlockKind::BlockQuote, Some(3)).is_empty());
    }

    #[test]
    fn thematic_break_hides_its_text() {
        assert_eq!(
            block_hidden_ranges("---", 5, &BlockKind::ThematicBreak, None),
            vec![5..8]
        );
        assert_eq!(visible_of("---", &BlockKind::ThematicBreak, None), "");
        assert!(block_hidden_ranges("---", 0, &BlockKind::ThematicBreak, Some(1)).is_empty());
    }

    #[test]
    fn code_fences_and_tables_are_not_inline_marked() {
        assert!(block_hidden_ranges("`x`", 0, &BlockKind::CodeFence, None).is_empty());
        assert!(block_hidden_ranges("**a**", 0, &BlockKind::Table, None).is_empty());
    }

    #[test]
    fn visible_text_clips_hidden_ranges_to_the_line() {
        // Line 2 of "ab\n**cd**" starts at 3, so the markers 3..5 and 7..9 are its own.
        let hidden = vec![3..5, 7..9];
        let (text, clipped) = visible_text("**cd**", 3, &hidden);
        assert_eq!(text, "cd");
        assert_eq!(clipped, vec![3..5, 7..9]);
        // A range that starts before the line is clipped to it.
        let whole = 0..4;
        let (text, clipped) = visible_text("cd", 3, std::slice::from_ref(&whole));
        assert_eq!(text, "d");
        assert_eq!(clipped, vec![3..4]);
    }

    #[test]
    fn source_to_display_skips_hidden_bytes() {
        let hidden = [2..4, 5..7];
        assert_eq!(source_to_display(0, 0, &hidden), 0);
        assert_eq!(
            source_to_display(2, 0, &hidden),
            2,
            "start of the first marker"
        );
        assert_eq!(source_to_display(4, 0, &hidden), 2, "after it");
        assert_eq!(source_to_display(5, 0, &hidden), 3, "after 'b'");
        assert_eq!(
            source_to_display(7, 0, &hidden),
            3,
            "after the second marker"
        );
        assert_eq!(source_to_display(8, 0, &hidden), 4);
        assert_eq!(source_to_display(9, 0, &hidden), 5);
    }

    #[test]
    fn source_to_display_respects_the_line_base() {
        // The line starts at 10 and its marker is at 12..14.
        let marker = 12..14;
        let hidden = std::slice::from_ref(&marker);
        assert_eq!(source_to_display(10, 10, hidden), 0);
        assert_eq!(source_to_display(14, 10, hidden), 2);
        assert_eq!(source_to_display(15, 10, hidden), 3);
    }

    #[test]
    fn display_to_source_chooses_a_side_at_collapse_points() {
        let hidden = [2..4, 5..7];
        // Visible 2 is before 'b': the start of the marker, or its end.
        assert_eq!(display_to_source(2, 0, &hidden, false), 2);
        assert_eq!(display_to_source(2, 0, &hidden, true), 4);
        // Visible 3 is after 'b': before the closing marker, or after it.
        assert_eq!(display_to_source(3, 0, &hidden, false), 5);
        assert_eq!(display_to_source(3, 0, &hidden, true), 7);
        assert_eq!(display_to_source(4, 0, &hidden, false), 8);
        assert_eq!(display_to_source(0, 0, &hidden, false), 0);
        assert_eq!(display_to_source(0, 0, &hidden, true), 0);
    }

    #[test]
    fn display_to_source_is_never_inside_a_hidden_range() {
        let hidden = [2..4, 5..7];
        for display in 0..=5 {
            for prefer_after in [false, true] {
                let source = display_to_source(display, 0, &hidden, prefer_after);
                assert!(
                    hidden.iter().all(|r| !(r.start < source && source < r.end)),
                    "display {display} gave {source}"
                );
            }
        }
    }

    #[test]
    fn source_display_roundtrip_on_visible_positions() {
        // "a **b** c **dd**" with the markers hidden; every visible offset round-trips.
        let text = "a **b** c **dd**";
        let hidden = block_hidden_ranges(text, 0, &BlockKind::Paragraph, Some(0));
        let (visible, _) = visible_text(text, 0, &hidden);
        for display in 0..=visible.len() {
            for prefer_after in [false, true] {
                let source = display_to_source(display, 0, &hidden, prefer_after);
                assert_eq!(
                    source_to_display(source, 0, &hidden),
                    display,
                    "display {display}, source {source}"
                );
            }
        }
        // And every source caret stop, outside the hidden ranges, round-trips too.
        for source in 0..=text.len() {
            if hidden.iter().any(|r| r.start < source && source < r.end) {
                continue;
            }
            let display = source_to_display(source, 0, &hidden);
            let back = display_to_source(display, 0, &hidden, false);
            assert_eq!(
                source_to_display(back, 0, &hidden),
                display,
                "source {source}"
            );
        }
    }

    #[test]
    fn source_display_roundtrip_with_a_hidden_block_marker_and_a_nonzero_base() {
        // A heading "## Title" at document offset 7.
        let hidden = block_hidden_ranges("## Title", 7, &BlockKind::Heading(2), None);
        assert_eq!(hidden, vec![7..10]);
        assert_eq!(source_to_display(10, 7, &hidden), 0);
        assert_eq!(display_to_source(0, 7, &hidden, false), 7);
        assert_eq!(display_to_source(0, 7, &hidden, true), 10);
        assert_eq!(display_to_source(5, 7, &hidden, false), 15);
    }

    #[test]
    fn visible_run_lengths_drop_hidden_bytes_and_empty_runs() {
        // Runs "a " (2), "**" (2), "b" (1), "**" (2), " c" (2) on a line at base 0.
        let lens = [2, 2, 1, 2, 2];
        let hidden = [2..4, 5..7];
        assert_eq!(
            visible_run_lengths(&lens, 0, &hidden),
            vec![(0, 2), (2, 1), (4, 2)]
        );
        // With nothing hidden every run keeps its length.
        assert_eq!(
            visible_run_lengths(&lens, 0, &[]),
            vec![(0, 2), (1, 2), (2, 1), (3, 2), (4, 2)]
        );
    }

    #[test]
    fn document_hidden_ranges_follow_the_caret_block() {
        let text = "# T\n\nA **b**\n";
        // Caret on the bold text: the heading marker hides, the span's markers show.
        let caret_in_paragraph = text.find('b').unwrap();
        assert_eq!(
            document_hidden_ranges(text, Some(caret_in_paragraph)),
            vec![0..2]
        );
        // Caret in the heading: its marker shows, the bold markers hide.
        assert_eq!(document_hidden_ranges(text, Some(1)), vec![7..9, 10..12]);
        // No caret: every marker hides.
        assert_eq!(document_hidden_ranges(text, None), vec![0..2, 7..9, 10..12]);
    }

    #[test]
    fn document_hidden_ranges_of_a_rule_and_quote() {
        let text = "> q\n\n---\n";
        assert_eq!(
            document_hidden_ranges(text, Some(9)),
            vec![0..2, 5..8],
            "the quote marker and the rule text hide while the caret is on the blank line"
        );
        assert_eq!(
            document_hidden_ranges(text, Some(0)),
            vec![5..8],
            "the caret in the quote shows its marker, the rule still hides"
        );
    }

    #[test]
    fn caret_steps_skip_hidden_markers() {
        use md_core::display_map::{Direction, DisplayMap};

        // "a **b** c" with the markers hidden. The caret never rests strictly inside a marker
        // (offsets 3 and 6), and each step passes exactly one visible character.
        let text = "a **b** c";
        let map = DisplayMap::new(text.len(), document_hidden_ranges(text, Some(0)));
        assert_eq!(
            map.move_caret(text, 1, Direction::Right),
            2,
            "over the space"
        );
        assert_eq!(
            map.move_caret(text, 2, Direction::Right),
            5,
            "over 'b', past the marker"
        );
        assert_eq!(
            map.move_caret(text, 5, Direction::Right),
            8,
            "over the space after it"
        );
        assert_eq!(
            map.move_caret(text, 8, Direction::Left),
            7,
            "back over the space"
        );
        assert_eq!(map.move_caret(text, 5, Direction::Left), 4, "back over 'b'");
        assert_eq!(
            map.move_caret(text, 4, Direction::Left),
            1,
            "past the opening marker, over the space"
        );
    }

    #[test]
    fn list_markers_hide_on_every_line_without_the_caret() {
        let text = "- a\n  - b\n1. c";
        assert_eq!(
            block_hidden_ranges(text, 0, &BlockKind::List, None),
            vec![0..2, 4..8, 10..13]
        );
        assert_eq!(visible_of(text, &BlockKind::List, None), "a\nb\nc");
    }

    #[test]
    fn list_caret_line_keeps_its_marker_and_hides_its_indent() {
        let text = "- a\n  - b\n1. c";
        // Caret on the nested item: its indent hides, its marker shows. Other lines hide.
        assert_eq!(
            block_hidden_ranges(text, 0, &BlockKind::List, Some(8)),
            vec![0..2, 4..6, 10..13]
        );
        // Caret on the first item keeps its marker, the others hide theirs.
        assert_eq!(
            block_hidden_ranges(text, 0, &BlockKind::List, Some(0)),
            vec![4..8, 10..13]
        );
    }

    #[test]
    fn list_continuation_indent_hides_even_with_the_caret_on_it() {
        let text = "- a\n  more";
        assert_eq!(
            block_hidden_ranges(text, 0, &BlockKind::List, Some(7)),
            vec![0..2, 4..6],
        );
    }

    #[test]
    fn list_task_box_hides_with_its_marker() {
        let text = "- [x] done";
        assert_eq!(visible_of(text, &BlockKind::List, None), "done");
        assert_eq!(
            visible_of(text, &BlockKind::List, Some(10)),
            "- [x] done",
            "the caret line keeps its marker"
        );
    }

    #[test]
    fn multibyte_text_keeps_byte_offsets() {
        // "日 **本**" : the markers are at 4..6 and 9..11 (the kanji are 3 bytes each).
        let text = "日 **本**";
        let hidden = block_hidden_ranges(text, 0, &BlockKind::Paragraph, Some(0));
        assert_eq!(hidden, vec![4..6, 9..11]);
        let (visible, _) = visible_text(text, 0, &hidden);
        assert_eq!(visible, "日 本");
        assert_eq!(source_to_display(9, 0, &hidden), 7);
    }
}
