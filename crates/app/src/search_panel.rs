//! Full-text search panel (task 9.4).
//!
//! The sidebar switches to search mode (ToggleSearch, Ctrl+Shift+F on Linux, Cmd+Shift+F on
//! macOS). The panel has a one-line query input on top and a results list below. Each result
//! is one matching line: the note name, the line number, and the line as a snippet with the
//! matches highlighted. The search reruns on every query change over the vault's notes, using
//! `workspace::search`. Results are capped at [`MAX_RESULTS`] lines.
//!
//! Keys while the panel has focus. Typed text goes to the query, Backspace deletes a character,
//! Enter opens the first result, and Escape closes the panel and returns focus to the editor.
//!
//! The pure helpers ([`snippet`], [`segments`], [`summary`]) take plain strings and ranges, so
//! they are unit-tested without a window.

use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;

use gpui::{
    actions, div, prelude::*, px, ClickEvent, Context, FocusHandle, FontWeight, IntoElement,
    KeyDownEvent,
};
use workspace::search::{search_with_report, SearchHit, SearchOptions};
use workspace::tree::Vault;

use crate::text_input;
use crate::workspace_view::{WorkspaceView, SIDEBAR_WIDTH};

actions!(search_panel, [ToggleSearch]);

/// The most result lines a search returns.
pub const MAX_RESULTS: usize = 200;
/// Characters of the line shown on each side of the first match.
const CONTEXT_CHARS: usize = 24;
const ELLIPSIS: &str = "\u{2026}";

/// A line shortened for display. `highlights` are byte ranges into `text`, in order and on
/// char boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snippet {
    pub text: String,
    pub highlights: Vec<Range<usize>>,
}

/// Shortens `line` to a window of about `context_chars` characters on each side of the first
/// match, and highlights every match that falls inside the window.
///
/// `matches` are byte ranges into `line`. Offsets that are out of range or not on a char
/// boundary are clamped outward to the nearest boundary, so the function never panics and
/// the highlights always index valid text. A missing first match gives the start of the line.
pub fn snippet(line: &str, matches: &[Range<usize>], context_chars: usize) -> Snippet {
    let total_len = line.len();
    let first = matches.first().cloned().unwrap_or(0..0);
    let start = floor_boundary(line, first.start.min(total_len));
    let end = ceil_boundary(line, first.end.min(total_len).max(start));

    let before = line[..start].chars().count();
    let anchor = line[start..end].chars().count();
    let total_chars = line.chars().count();
    let win_start = byte_of_char(line, before.saturating_sub(context_chars));
    let win_end = byte_of_char(line, (before + anchor + context_chars).min(total_chars));

    let prefix = if win_start > 0 { ELLIPSIS } else { "" };
    let suffix = if win_end < total_len { ELLIPSIS } else { "" };
    // Tabs become spaces. Both are one byte, so the offsets below do not move.
    let body = line[win_start..win_end].replace('\t', " ");
    let text = format!("{prefix}{body}{suffix}");

    let to_text = |offset: usize| prefix.len() + offset - win_start;
    let mut highlights = Vec::new();
    for m in matches {
        let s = floor_boundary(line, m.start.min(total_len));
        let e = ceil_boundary(line, m.end.min(total_len).max(s));
        if s >= win_start && e <= win_end && s < e {
            highlights.push(to_text(s)..to_text(e));
        }
    }
    highlights.sort_by_key(|r| r.start);
    highlights.dedup();
    Snippet { text, highlights }
}

/// Splits `text` into alternating plain and highlighted pieces, in order. Each piece is a byte
/// range into `text` and a flag that is true for a highlight. Highlights that overlap an earlier
/// one or fall off `text` are ignored.
pub fn segments(text: &str, highlights: &[Range<usize>]) -> Vec<(Range<usize>, bool)> {
    let mut out = Vec::new();
    let mut pos = 0;
    for h in highlights {
        let valid = h.start >= pos
            && h.start <= h.end
            && h.end <= text.len()
            && text.is_char_boundary(h.start)
            && text.is_char_boundary(h.end);
        if !valid {
            continue;
        }
        if h.start > pos {
            out.push((pos..h.start, false));
        }
        if h.start < h.end {
            out.push((h.clone(), true));
        }
        pos = h.end;
    }
    if pos < text.len() {
        out.push((pos..text.len(), false));
    }
    out
}

/// The snippet for one search hit. Match offsets are made relative to the line.
pub fn hit_snippet(hit: &SearchHit) -> Snippet {
    let line = hit.line.trim_end_matches('\r');
    let ranges: Vec<Range<usize>> = hit
        .matches
        .iter()
        .map(|m| {
            let s = m.byte_range.start.saturating_sub(hit.byte_offset) as usize;
            let e = m.byte_range.end.saturating_sub(hit.byte_offset) as usize;
            s..e
        })
        .collect();
    snippet(line, &ranges, CONTEXT_CHARS)
}

/// The caret offset for a hit: the start of its first match in the file.
pub fn hit_offset(hit: &SearchHit) -> usize {
    hit.matches
        .first()
        .map(|m| m.byte_range.start as usize)
        .unwrap_or(hit.byte_offset as usize)
}

/// The note's name as the sidebar shows it: the file name without `.md`.
pub fn note_label(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// One line for the result count, for example `3 lines in 2 notes`.
pub fn summary(hits: &[SearchHit], skipped: usize) -> String {
    let mut notes = HashSet::new();
    for hit in hits {
        notes.insert(&hit.path);
    }
    let lines = if hits.len() >= MAX_RESULTS {
        format!("first {MAX_RESULTS} lines")
    } else {
        plural(hits.len(), "line", "lines")
    };
    let mut text = format!("{lines} in {}", plural(notes.len(), "note", "notes"));
    if skipped > 0 {
        text.push_str(&format!(" \u{00B7} {skipped} unreadable"));
    }
    text
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

fn floor_boundary(s: &str, mut i: usize) -> usize {
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn ceil_boundary(s: &str, mut i: usize) -> usize {
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn byte_of_char(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len())
}

/// The search state held by the root view. The results come from the vault that is open in
/// the sidebar, so `refresh` takes that vault.
pub struct SearchPanel {
    pub query: String,
    pub hits: Vec<SearchHit>,
    /// Files that could not be read and were skipped.
    pub skipped: usize,
    pub error: Option<String>,
    pub focus: FocusHandle,
}

impl SearchPanel {
    pub fn new(focus: FocusHandle) -> Self {
        Self {
            query: String::new(),
            hits: Vec::new(),
            skipped: 0,
            error: None,
            focus,
        }
    }

    /// Reruns the search for the current query. Without a vault there are no results.
    pub fn refresh(&mut self, vault: Option<&Vault>) {
        self.hits.clear();
        self.skipped = 0;
        self.error = None;
        let Some(vault) = vault else { return };
        if self.query.is_empty() {
            return;
        }
        let opts = SearchOptions {
            case_insensitive: true,
            regex: false,
            max_results: Some(MAX_RESULTS),
        };
        match search_with_report(vault, &self.query, &opts) {
            Ok(report) => {
                self.hits = report.hits;
                self.skipped = report.skipped.len();
            }
            Err(err) => self.error = Some(err.to_string()),
        }
    }

    fn status(&self, has_vault: bool) -> String {
        if !has_vault {
            return "Open a folder to search its notes.".into();
        }
        if let Some(err) = &self.error {
            return format!("Search failed: {err}");
        }
        if self.query.is_empty() {
            return "Type to search. Enter opens the first result, Esc closes.".into();
        }
        if self.hits.is_empty() {
            return "No matches.".into();
        }
        summary(&self.hits, self.skipped)
    }
}

/// Renders the search panel in place of the folder tree.
pub fn render_panel(
    panel: &SearchPanel,
    has_vault: bool,
    cx: &mut Context<WorkspaceView>,
) -> impl IntoElement {
    let theme = md_editor::theme::current(cx);
    let input = text_input::field(&theme, "search-input", &panel.query, "Search notes", 28.)
        .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
            window.focus(&this.search.focus, cx);
        }));

    let rows = panel
        .hits
        .iter()
        .enumerate()
        .map(|(index, hit)| {
            let snip = hit_snippet(hit);
            let pieces = segments(&snip.text, &snip.highlights)
                .into_iter()
                .map(|(range, highlighted)| {
                    div()
                        .whitespace_nowrap()
                        .when(highlighted, |d| {
                            d.bg(theme.ui_match).font_weight(FontWeight::BOLD)
                        })
                        .child(snip.text[range].to_string())
                })
                .collect::<Vec<_>>();
            div()
                .id(("search-hit", index))
                .px_2()
                .py_1()
                .cursor_pointer()
                .hover(|style| style.bg(theme.ui_hover))
                .on_click(cx.listener(move |this, _event, window, cx| {
                    this.open_search_hit(index, window, cx);
                }))
                .child(
                    div()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.ui_text)
                        .child(format!("{} : {}", note_label(&hit.path), hit.line_number)),
                )
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .overflow_hidden()
                        .text_xs()
                        .text_color(theme.ui_text_secondary)
                        .children(pieces),
                )
        })
        .collect::<Vec<_>>();

    div()
        .id("search-panel")
        .h_full()
        .w(px(SIDEBAR_WIDTH))
        .flex_none()
        .flex()
        .flex_col()
        .bg(theme.ui_surface)
        .border_r_1()
        .border_color(theme.ui_border)
        .track_focus(&panel.focus)
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            this.search_key_down(event, window, cx);
        }))
        .child(div().p_2().child(input))
        .child(
            div()
                .px_2()
                .pb_1()
                .text_xs()
                .text_color(theme.ui_muted)
                .child(panel.status(has_vault)),
        )
        .child(
            div()
                .id("search-results")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .children(rows),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use workspace::search::MatchSpan;

    /// The text each highlight covers, for checking.
    fn highlighted(s: &Snippet) -> Vec<&str> {
        s.highlights.iter().map(|r| &s.text[r.clone()]).collect()
    }

    /// One range as a vector, for the single-match calls.
    fn one<T>(r: Range<T>) -> Vec<Range<T>> {
        vec![r]
    }

    /// A hit whose matches are the given (start, end) file byte pairs.
    fn hit(
        path: &str,
        line_number: u64,
        line: &str,
        byte_offset: u64,
        spans: &[(u64, u64)],
    ) -> SearchHit {
        SearchHit {
            path: PathBuf::from(path),
            line_number,
            line: line.to_string(),
            byte_offset,
            matches: spans
                .iter()
                .map(|&(start, end)| MatchSpan {
                    byte_range: start..end,
                    column: 1,
                })
                .collect(),
        }
    }

    #[test]
    fn short_line_is_shown_whole_with_one_highlight() {
        let line = "the cat sat";
        let s = snippet(line, &one(4..7), CONTEXT_CHARS);
        assert_eq!(s.text, "the cat sat");
        assert_eq!(s.highlights, vec![4..7]);
        assert_eq!(highlighted(&s), vec!["cat"]);
    }

    #[test]
    fn long_line_is_cut_around_the_match_with_ellipses() {
        let line = format!("{}needle{}", "a".repeat(100), "b".repeat(100));
        let s = snippet(&line, &one(100..106), 10);
        assert_eq!(
            s.text,
            format!(
                "{ELLIPSIS}{}needle{}{ELLIPSIS}",
                "a".repeat(10),
                "b".repeat(10)
            )
        );
        assert_eq!(highlighted(&s), vec!["needle"]);
    }

    #[test]
    fn match_at_start_has_no_leading_ellipsis() {
        let s = snippet("hello world and more text here", &one(0..5), 4);
        assert_eq!(s.text, "hello wor\u{2026}");
        assert_eq!(highlighted(&s), vec!["hello"]);
    }

    #[test]
    fn multibyte_text_is_cut_on_char_boundaries() {
        let line = "héllo wörld ünïcode 日本語 match 🎉 end";
        let start = line.find("match").unwrap();
        let s = snippet(line, &one(start..start + 5), 6);
        assert_eq!(highlighted(&s), vec!["match"]);
        assert!(s.text.contains("日本語"));
    }

    #[test]
    fn every_match_inside_the_window_is_highlighted() {
        let line = "one x two x three x four";
        let positions: Vec<Range<usize>> = line.match_indices('x').map(|(i, _)| i..i + 1).collect();
        let s = snippet(line, &positions, 100);
        assert_eq!(s.highlights.len(), 3);
        assert_eq!(highlighted(&s), vec!["x", "x", "x"]);
    }

    #[test]
    fn offsets_inside_a_character_are_widened_to_its_boundaries() {
        // 'é' is two bytes (1..3). A match 2..3 starts mid-character.
        let line = "aébc";
        let s = snippet(line, &one(2..3), 10);
        assert_eq!(s.highlights.len(), 1);
        assert!(s.text.is_char_boundary(s.highlights[0].start));
        assert!(s.text.is_char_boundary(s.highlights[0].end));
        assert_eq!(highlighted(&s), vec!["é"]);
    }

    #[test]
    fn out_of_range_match_does_not_panic() {
        let s = snippet("abc", &one(10..20), 5);
        assert_eq!(s.text, "abc");
        assert!(s.highlights.is_empty());
    }

    #[test]
    fn tabs_become_spaces_without_moving_highlights() {
        let s = snippet("a\tneedle", &one(2..8), 10);
        assert_eq!(s.text, "a needle");
        assert_eq!(highlighted(&s), vec!["needle"]);
    }

    #[test]
    fn segments_alternate_plain_and_highlighted() {
        let text = "ab cd ef";
        let segs = segments(text, &[0..2, 3..5]);
        assert_eq!(
            segs,
            vec![(0..2, true), (2..3, false), (3..5, true), (5..8, false)]
        );
    }

    #[test]
    fn segments_skip_bad_highlights() {
        let text = "abcdef";
        let segs = segments(text, &[2..4, 0..3, 9..12]);
        assert_eq!(segs, vec![(0..2, false), (2..4, true), (4..6, false)]);
    }

    #[test]
    fn hit_snippet_turns_file_offsets_into_line_offsets() {
        // The line starts at byte 100 of the file; the match is at file bytes 107..110.
        let h = hit("/v/a.md", 3, "xxxxxx abc", 100, &[(107, 110)]);
        let s = hit_snippet(&h);
        assert_eq!(s.text, "xxxxxx abc");
        assert_eq!(highlighted(&s), vec!["abc"]);
        assert_eq!(hit_offset(&h), 107);
    }

    #[test]
    fn hit_snippet_drops_carriage_return() {
        let h = hit("/v/a.md", 1, "find me\r", 0, &[(0, 4)]);
        let s = hit_snippet(&h);
        assert_eq!(s.text, "find me");
    }

    #[test]
    fn summary_counts_lines_notes_and_skipped_files() {
        let hits = vec![
            hit("/v/a.md", 1, "x", 0, &[(0, 1)]),
            hit("/v/a.md", 4, "x", 10, &[(10, 11)]),
            hit("/v/b.md", 2, "x", 20, &[(20, 21)]),
        ];
        assert_eq!(summary(&hits, 0), "3 lines in 2 notes");
        assert_eq!(
            summary(&hits[..1], 2),
            "1 line in 1 note \u{00B7} 2 unreadable"
        );
    }

    #[test]
    fn summary_marks_a_capped_result() {
        let hits: Vec<SearchHit> = (0..MAX_RESULTS as u64)
            .map(|i| hit("/v/a.md", i + 1, "x", i, &[(i, i + 1)]))
            .collect();
        assert_eq!(summary(&hits, 0), "first 200 lines in 1 note");
    }

    #[test]
    fn note_label_drops_the_extension() {
        assert_eq!(note_label(Path::new("/v/Alpha/a.md")), "a");
    }
}
