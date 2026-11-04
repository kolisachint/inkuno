//! Inline span model (task 2.5).
//!
//! Finds the inline spans of one block (strong, emphasis, strikethrough, inline
//! code, link, image, inline math) with the pulldown-cmark offset iterator, and
//! derives each span's marker ranges and content range from the source text.
//! Marker ranges are what the display map hides while the caret is outside the
//! span (DESIGN §4.1).

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

use crate::{InlineSpan, SpanKind};

/// An open container on the parser stack. Every start tag gets a frame so that
/// start and end events stay paired; only span kinds carry a `kind`.
struct Frame {
    kind: Option<SpanKind>,
    start: usize,
    /// Largest source offset reached by a descendant event. For links and
    /// images this is where the link text ends, so the closing `]` is searched
    /// from here.
    text_end: usize,
}

/// Find inline spans in `text` (one block's source). `base` is added to every returned offset (the block's start offset in the document). Spans are returned sorted by range.start, outer before inner when nested.
pub fn inline_spans(text: &str, base: usize) -> Vec<InlineSpan> {
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_MATH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS;
    let bytes = text.as_bytes();

    let mut spans = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();

    for (event, range) in Parser::new_ext(text, options).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                let kind = span_kind(&tag);
                let open_len = match kind {
                    Some(SpanKind::Link) => 1,
                    Some(SpanKind::Image) => 2,
                    _ => 0,
                };
                stack.push(Frame {
                    kind,
                    start: range.start,
                    text_end: range.start + open_len,
                });
            }
            Event::End(_) => {
                let Some(frame) = stack.pop() else { continue };
                if let Some(kind) = frame.kind {
                    if let Some(span) = close_span(bytes, kind, &frame, range.clone()) {
                        spans.push(span);
                    }
                }
                extend_ancestors(&mut stack, range.end);
            }
            Event::Code(_) => {
                if let Some(span) = delimited_span(bytes, SpanKind::Code, b'`', range.clone()) {
                    spans.push(span);
                }
                extend_ancestors(&mut stack, range.end);
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                if let Some(span) = delimited_span(bytes, SpanKind::Math, b'$', range.clone()) {
                    spans.push(span);
                }
                extend_ancestors(&mut stack, range.end);
            }
            _ => extend_ancestors(&mut stack, range.end),
        }
    }

    // Outer spans first: ascending start, then descending end.
    spans.sort_by(|a, b| {
        a.range
            .start
            .cmp(&b.range.start)
            .then(b.range.end.cmp(&a.range.end))
    });

    if base != 0 {
        for span in &mut spans {
            shift(span, base);
        }
    }
    spans
}

/// Spans whose markers should be visible for a caret at absolute `caret` (caret inside span means range.start <= caret <= range.end).
pub fn revealed(spans: &[InlineSpan], caret: usize) -> impl Iterator<Item = &InlineSpan> {
    spans.iter().filter(move |span| contains_caret(span, caret))
}

/// All marker ranges that should be hidden for this caret (markers of spans not containing the caret), sorted, merged.
pub fn hidden_ranges(spans: &[InlineSpan], caret: Option<usize>) -> Vec<Range<usize>> {
    let mut ranges: Vec<Range<usize>> = spans
        .iter()
        .filter(|span| !caret.is_some_and(|c| contains_caret(span, c)))
        .flat_map(|span| span.markers.iter().cloned())
        .filter(|range| range.start < range.end)
        .collect();
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

fn contains_caret(span: &InlineSpan, caret: usize) -> bool {
    span.range.start <= caret && caret <= span.range.end
}

fn span_kind(tag: &Tag) -> Option<SpanKind> {
    match tag {
        Tag::Strong => Some(SpanKind::Strong),
        Tag::Emphasis => Some(SpanKind::Emphasis),
        Tag::Strikethrough => Some(SpanKind::Strikethrough),
        // Autolinks (`<http://..>`) have no bracketed text and no closing `]`,
        // so they are skipped.
        Tag::Link { link_type, .. }
            if !matches!(link_type, LinkType::Autolink | LinkType::Email) =>
        {
            Some(SpanKind::Link)
        }
        Tag::Image { .. } => Some(SpanKind::Image),
        _ => None,
    }
}

/// Keep every ancestor's `text_end` at or beyond `end`.
fn extend_ancestors(stack: &mut [Frame], end: usize) {
    for frame in stack.iter_mut() {
        frame.text_end = frame.text_end.max(end);
    }
}

/// Build a span for a finished strong, emphasis, strikethrough, link or image frame.
fn close_span(
    bytes: &[u8],
    kind: SpanKind,
    frame: &Frame,
    range: Range<usize>,
) -> Option<InlineSpan> {
    let (start, end) = (frame.start, range.end);
    match kind {
        SpanKind::Strong => {
            let (markers, content) = fixed_width(bytes, start..end, 2, b"*_")?;
            Some(make(kind, start..end, markers, content))
        }
        SpanKind::Emphasis => {
            let (markers, content) = fixed_width(bytes, start..end, 1, b"*_")?;
            Some(make(kind, start..end, markers, content))
        }
        SpanKind::Strikethrough => delimited_span(bytes, kind, b'~', start..end),
        SpanKind::Link | SpanKind::Image => {
            let open = if kind == SpanKind::Image { 2 } else { 1 };
            let close = closing_bracket(bytes, frame.text_end, end)?;
            if close < start + open {
                return None;
            }
            Some(make(
                kind,
                start..end,
                vec![start..start + open, close..end],
                start + open..close,
            ))
        }
        _ => None,
    }
}

/// Span whose markers are a run of `ch` at each end (inline code, math, strikethrough).
fn delimited_span(bytes: &[u8], kind: SpanKind, ch: u8, range: Range<usize>) -> Option<InlineSpan> {
    let width = bytes[range.start..range.end]
        .iter()
        .take_while(|&&b| b == ch)
        .count();
    if width == 0 {
        return None;
    }
    let (markers, content) = fixed_width(bytes, range.clone(), width, &[ch])?;
    Some(make(kind, range, markers, content))
}

/// Markers are `width` bytes at each end of `range`; every marker byte must be one of `allowed`.
fn fixed_width(
    bytes: &[u8],
    range: Range<usize>,
    width: usize,
    allowed: &[u8],
) -> Option<(Vec<Range<usize>>, Range<usize>)> {
    if range.end - range.start < 2 * width {
        return None;
    }
    let head = &bytes[range.start..range.start + width];
    let tail = &bytes[range.end - width..range.end];
    let ok = |run: &[u8]| {
        run.iter().all(|b| allowed.contains(b)) && run.windows(2).all(|w| w[0] == w[1])
    };
    if !ok(head) || !ok(tail) {
        return None;
    }
    let markers = vec![
        range.start..range.start + width,
        range.end - width..range.end,
    ];
    let content = range.start + width..range.end - width;
    Some((markers, content))
}

/// Offset of the `]` that closes link text, given the end of the text. Only
/// whitespace may sit between the last text event and the bracket.
fn closing_bracket(bytes: &[u8], from: usize, end: usize) -> Option<usize> {
    let mut i = from;
    while i < end && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    (i < end && bytes[i] == b']').then_some(i)
}

fn make(
    kind: SpanKind,
    range: Range<usize>,
    markers: Vec<Range<usize>>,
    content: Range<usize>,
) -> InlineSpan {
    InlineSpan {
        kind,
        range,
        markers,
        content,
    }
}

fn shift(span: &mut InlineSpan, base: usize) {
    span.range = span.range.start + base..span.range.end + base;
    for marker in &mut span.markers {
        *marker = marker.start + base..marker.end + base;
    }
    span.content = span.content.start + base..span.content.end + base;
}

#[cfg(test)]
mod tests {
    use super::*;

    type Case = (
        &'static str,
        SpanKind,
        Range<usize>,
        [Range<usize>; 2],
        Range<usize>,
    );

    /// One row per span kind: source, kind, range, markers, content (base 0).
    const CASES: &[Case] = &[
        ("**bold**", SpanKind::Strong, 0..8, [0..2, 6..8], 2..6),
        ("__b__", SpanKind::Strong, 0..5, [0..2, 3..5], 2..3),
        ("*em*", SpanKind::Emphasis, 0..4, [0..1, 3..4], 1..3),
        ("_e_", SpanKind::Emphasis, 0..3, [0..1, 2..3], 1..2),
        ("~~s~~", SpanKind::Strikethrough, 0..5, [0..2, 3..5], 2..3),
        ("~s~", SpanKind::Strikethrough, 0..3, [0..1, 2..3], 1..2),
        ("`c`", SpanKind::Code, 0..3, [0..1, 2..3], 1..2),
        ("``a`b``", SpanKind::Code, 0..7, [0..2, 5..7], 2..5),
        ("`` a ``", SpanKind::Code, 0..7, [0..2, 5..7], 2..5),
        ("$x$", SpanKind::Math, 0..3, [0..1, 2..3], 1..2),
        ("$$x$$", SpanKind::Math, 0..5, [0..2, 3..5], 2..3),
        ("[t](u)", SpanKind::Link, 0..6, [0..1, 2..6], 1..2),
        ("![a](s)", SpanKind::Image, 0..7, [0..2, 3..7], 2..3),
        ("**日本**", SpanKind::Strong, 0..10, [0..2, 8..10], 2..8),
        ("*é*", SpanKind::Emphasis, 0..4, [0..1, 3..4], 1..3),
        ("[日本](ü)", SpanKind::Link, 0..12, [0..1, 7..12], 1..7),
        ("![日](ü)", SpanKind::Image, 0..10, [0..2, 5..10], 2..5),
    ];

    fn assert_case(case: &Case, base: usize) {
        let (src, kind, range, markers, content) = case;
        let spans = inline_spans(src, base);
        assert_eq!(
            spans.len(),
            1,
            "{src:?}: expected exactly one span, got {spans:?}"
        );
        let span = &spans[0];
        let shifted = |r: &Range<usize>| r.start + base..r.end + base;
        assert_eq!(span.kind, *kind, "{src:?} kind");
        assert_eq!(span.range, shifted(range), "{src:?} range (base {base})");
        assert_eq!(
            span.markers,
            vec![shifted(&markers[0]), shifted(&markers[1])],
            "{src:?} markers (base {base})"
        );
        assert_eq!(
            span.content,
            shifted(content),
            "{src:?} content (base {base})"
        );
    }

    #[test]
    fn table_ranges_markers_and_content() {
        for case in CASES {
            assert_case(case, 0);
        }
    }

    #[test]
    fn table_with_base_offset() {
        for case in CASES {
            assert_case(case, 100);
        }
    }

    #[test]
    fn nested_strong_contains_emphasis() {
        let spans = inline_spans("**bold *em* x**", 0);
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert_eq!(spans[0].kind, SpanKind::Strong);
        assert_eq!(spans[0].range, 0..15);
        assert_eq!(spans[0].markers, vec![0..2, 13..15]);
        assert_eq!(spans[0].content, 2..13);
        assert_eq!(spans[1].kind, SpanKind::Emphasis);
        assert_eq!(spans[1].range, 7..11);
        assert_eq!(spans[1].markers, vec![7..8, 10..11]);
        assert_eq!(spans[1].content, 8..10);
    }

    #[test]
    fn nested_strong_inside_link() {
        let spans = inline_spans("[**b**](u)", 0);
        assert_eq!(spans.len(), 2, "{spans:?}");
        assert_eq!(spans[0].kind, SpanKind::Link);
        assert_eq!(spans[0].range, 0..10);
        assert_eq!(spans[0].markers, vec![0..1, 6..10]);
        assert_eq!(spans[0].content, 1..6);
        assert_eq!(spans[1].kind, SpanKind::Strong);
        assert_eq!(spans[1].range, 1..6);
        assert_eq!(spans[1].markers, vec![1..3, 4..6]);
        assert_eq!(spans[1].content, 3..4);
    }

    #[test]
    fn nested_spans_with_base_offset() {
        let spans = inline_spans("[**b**](u)", 100);
        assert_eq!(spans[0].range, 100..110);
        assert_eq!(spans[0].markers, vec![100..101, 106..110]);
        assert_eq!(spans[0].content, 101..106);
        assert_eq!(spans[1].range, 101..106);
        assert_eq!(spans[1].markers, vec![101..103, 104..106]);
        assert_eq!(spans[1].content, 103..104);
    }

    #[test]
    fn link_text_with_escaped_bracket_and_code() {
        // The escaped `\]` and the bracket inside code must not close the link.
        let src = r"[a\]b `]` c](u)";
        let spans = inline_spans(src, 0);
        let link = spans
            .iter()
            .find(|s| s.kind == SpanKind::Link)
            .expect("link span");
        assert_eq!(link.range, 0..src.len());
        assert_eq!(&src[link.markers[1].clone()], "](u)");
    }

    #[test]
    fn empty_link_text() {
        let spans = inline_spans("[](u)", 0);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].markers, vec![0..1, 1..5]);
        assert_eq!(spans[0].content, 1..1);
    }

    #[test]
    fn autolink_is_skipped_without_panic() {
        assert!(inline_spans("<http://example.com>", 0).is_empty());
    }

    #[test]
    fn plain_text_and_empty_input() {
        assert!(inline_spans("", 0).is_empty());
        assert!(inline_spans("no spans here", 7).is_empty());
    }

    #[test]
    fn spans_sorted_by_start() {
        let spans = inline_spans("*a* and **b**", 0);
        let starts: Vec<usize> = spans.iter().map(|s| s.range.start).collect();
        assert_eq!(starts, vec![0, 8]);
        assert_eq!(spans[0].kind, SpanKind::Emphasis);
        assert_eq!(spans[1].kind, SpanKind::Strong);
    }

    /// "a **b** c": strong at 2..7, markers 2..4 and 5..7, content 4..5.
    const REVEAL_TEXT: &str = "a **b** c";

    #[test]
    fn caret_inside_strong_reveals_it() {
        let spans = inline_spans(REVEAL_TEXT, 0);
        let revealed_spans: Vec<_> = revealed(&spans, 4).collect();
        assert_eq!(revealed_spans.len(), 1);
        assert_eq!(revealed_spans[0].kind, SpanKind::Strong);
        assert!(hidden_ranges(&spans, Some(4)).is_empty());
    }

    #[test]
    fn caret_at_span_boundaries_counts_as_inside() {
        let spans = inline_spans(REVEAL_TEXT, 0);
        assert_eq!(revealed(&spans, 2).count(), 1);
        assert_eq!(revealed(&spans, 7).count(), 1);
    }

    #[test]
    fn caret_outside_hides_both_marker_runs() {
        let spans = inline_spans(REVEAL_TEXT, 0);
        assert_eq!(revealed(&spans, 0).count(), 0);
        assert_eq!(hidden_ranges(&spans, Some(0)), vec![2..4, 5..7]);
        assert_eq!(hidden_ranges(&spans, Some(8)), vec![2..4, 5..7]);
    }

    #[test]
    fn caret_none_hides_all_markers() {
        let spans = inline_spans(REVEAL_TEXT, 0);
        assert_eq!(hidden_ranges(&spans, None), vec![2..4, 5..7]);
    }

    #[test]
    fn caret_uses_absolute_offsets_with_base() {
        let spans = inline_spans(REVEAL_TEXT, 100);
        assert_eq!(revealed(&spans, 104).count(), 1);
        assert!(hidden_ranges(&spans, Some(104)).is_empty());
        assert_eq!(hidden_ranges(&spans, Some(100)), vec![102..104, 105..107]);
    }

    #[test]
    fn nested_reveal_follows_caret() {
        let spans = inline_spans("**bold *em* x**", 0);
        // Inside the emphasis: both spans revealed, nothing hidden.
        assert_eq!(revealed(&spans, 9).count(), 2);
        assert!(hidden_ranges(&spans, Some(9)).is_empty());
        // Inside the strong but outside the emphasis: only the emphasis markers hide.
        assert_eq!(revealed(&spans, 3).count(), 1);
        assert_eq!(hidden_ranges(&spans, Some(3)), vec![7..8, 10..11]);
        // No caret: everything hidden.
        assert_eq!(
            hidden_ranges(&spans, None),
            vec![0..2, 7..8, 10..11, 13..15]
        );
    }

    #[test]
    fn multibyte_strong_reveal() {
        // "x **日本**": strong 2..12, markers 2..4 and 10..12, content 4..10.
        let spans = inline_spans("x **日本**", 0);
        assert_eq!(spans[0].range, 2..12);
        assert_eq!(spans[0].content, 4..10);
        assert_eq!(revealed(&spans, 5).count(), 1);
        assert!(hidden_ranges(&spans, Some(5)).is_empty());
        assert_eq!(hidden_ranges(&spans, Some(0)), vec![2..4, 10..12]);
    }

    #[test]
    fn hidden_ranges_are_sorted_and_merged() {
        let make = |markers: Vec<Range<usize>>| InlineSpan {
            kind: SpanKind::Strong,
            range: 0..10,
            markers,
            content: 2..8,
        };
        let spans = vec![
            make(vec![4..6, 0..2]),
            make(vec![2..3, 6..8]),
            make(vec![5..7, 9..10]),
        ];
        assert_eq!(hidden_ranges(&spans, None), vec![0..3, 4..8, 9..10]);
    }
}
