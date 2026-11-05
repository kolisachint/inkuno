//! Read-only table block (plan task 4.3). Owner: task 4.3.
//!
//! When the caret is outside a table block, `blocks::item_element` draws it with [`TableElement`]
//! as a grid. When the caret is inside, the source text is drawn instead, so the table stays
//! editable. [`parse_table`] and [`max_widths`] are pure and unit-tested on every platform.
//!
//! Cells render inline markdown (bold, italic, code, strikethrough, links) with the same runs
//! the editor uses for paragraphs: [`cell_visible`] splits each cell into [`RunStyle`]s, hides
//! the markers (the caret is always outside a drawn table) and [`element::text_run`] turns them
//! into GPUI runs. Inline `$...$` stays plain text while math is on hold.
//!
//! A table has no text lines in the layout map, so a click on it puts the caret at the start of
//! the block (see `cursor::offset_at`). The caret then moves into the block and shows the source.

use std::ops::Range;
use std::panic::Location;

use gpui::{
    fill, point, px, relative, size, App, Bounds, ContentMask, ElementId, Entity, GlobalElementId,
    InspectorElementId, IntoElement, LayoutId, Pixels, ShapedLine, SharedString, Style, TextAlign,
    TextRun, Window,
};

use md_core::inline::{hidden_ranges, inline_spans};
use md_core::{InlineSpan, SpanKind};

use crate::blocks::basic::{line_runs, BlockStyle, RunStyle, PARAGRAPH};
use crate::editor::Editor;
use crate::element::{text_run, LayoutSnapshot};
use crate::inline_render::{visible_run_lengths, visible_text};
use crate::selection::{block_coverage, BlockCoverage};

/// Padding inside a cell, left and right, and above and below the text, in pixels.
const CELL_PAD_X: f32 = 8.0;
const CELL_PAD_Y: f32 = 4.0;
/// Thickness of the thin grid lines and of the line under the header row, in pixels.
const LINE_THIN: f32 = 1.0;
const LINE_THICK: f32 = 2.0;

/// Column alignment from the delimiter row (`:---`, `:---:`, `---:`, `---`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Unset,
    Left,
    Center,
    Right,
}

/// A parsed GFM table. Cell text has no outer spaces and escaped pipes (`\|`) are unescaped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub header: Vec<String>,
    /// One entry per column, the same length as `header`.
    pub align: Vec<Alignment>,
    /// Body rows, each padded or cut to the header's column count.
    pub rows: Vec<Vec<String>>,
}

/// Parse `source` (the text of one table block) as a GFM table. Returns `None` when the first
/// two non-blank lines are not a header and a delimiter row with the same number of cells.
///
/// Leading and trailing pipes are optional. Blank lines are skipped. A body row with fewer cells
/// than the header is padded with empty cells, and extra cells are dropped, as GFM does.
pub fn parse_table(source: &str) -> Option<Table> {
    let mut lines = source.lines().filter(|line| !line.trim().is_empty());
    let header = split_row(lines.next()?);
    let align = split_row(lines.next()?)
        .iter()
        .map(|cell| alignment_of(cell))
        .collect::<Option<Vec<_>>>()?;
    if header.is_empty() || align.len() != header.len() {
        return None;
    }
    let width = header.len();
    let rows = lines
        .map(|line| {
            let mut cells = split_row(line);
            cells.resize(width, String::new());
            cells
        })
        .collect();
    Some(Table {
        header,
        align,
        rows,
    })
}

/// Split one table row into cells. Handles optional leading and trailing pipes, and `\|` as a
/// literal pipe inside a cell. Each cell is trimmed.
pub(crate) fn split_row(line: &str) -> Vec<String> {
    let mut text = line.trim();
    if let Some(rest) = text.strip_prefix('|') {
        text = rest;
    }
    if text.ends_with('|') && !text.ends_with("\\|") {
        text = &text[..text.len() - 1];
    }
    let mut cells = Vec::new();
    let mut cell = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if chars.peek() == Some(&'|') => {
                cell.push('|');
                chars.next();
            }
            '|' => {
                cells.push(cell.trim().to_string());
                cell.clear();
            }
            _ => cell.push(ch),
        }
    }
    cells.push(cell.trim().to_string());
    cells
}

/// Alignment of one delimiter cell, or `None` when the cell is not a delimiter (`:` alone, or
/// text other than dashes and colons).
fn alignment_of(cell: &str) -> Option<Alignment> {
    let left = cell.starts_with(':');
    let rest = if left { &cell[1..] } else { cell };
    let right = rest.ends_with(':');
    let dashes = if right { &rest[..rest.len() - 1] } else { rest };
    if dashes.is_empty() || !dashes.bytes().all(|byte| byte == b'-') {
        return None;
    }
    Some(match (left, right) {
        (true, true) => Alignment::Center,
        (true, false) => Alignment::Left,
        (false, true) => Alignment::Right,
        (false, false) => Alignment::Unset,
    })
}

/// Inline spans of one cell, with offsets relative to the cell. Math spans are dropped, so
/// inline `$...$` in cells stays plain text while math is on hold.
pub(crate) fn cell_spans(cell: &str) -> Vec<InlineSpan> {
    inline_spans(cell, 0)
        .into_iter()
        .filter(|span| span.kind != SpanKind::Math)
        .collect()
}

/// Visible text of one cell and the runs that cover it. The markers of every span are hidden,
/// since the caret is always outside a drawn table. The runs tile the visible text with no gaps
/// or overlaps, styled the same way paragraph runs are ([`line_runs`]). An empty cell gives no
/// runs.
pub(crate) fn cell_visible(cell: &str) -> (String, Vec<RunStyle>) {
    if cell.is_empty() {
        return (String::new(), Vec::new());
    }
    let spans = cell_spans(cell);
    let runs = line_runs(cell, 0, &spans);
    let hidden = hidden_ranges(&spans, None);
    let (visible, clipped) = visible_text(cell, 0, &hidden);
    let lens: Vec<usize> = runs.iter().map(|run| run.range.len()).collect();
    let mut start = 0;
    let mut out = Vec::new();
    for (index, len) in visible_run_lengths(&lens, 0, &clipped) {
        let mut run = runs[index].clone();
        run.range = start..start + len;
        start += len;
        out.push(run);
    }
    (visible, out)
}

/// The widest value in each column of `rows`, where `rows[r][c]` is the width of one cell.
/// Rows may be ragged: a missing cell counts as 0. Pure, unit-tested.
pub fn max_widths(rows: &[Vec<f32>]) -> Vec<f32> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    (0..columns)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .fold(0.0_f32, |widest, &width| widest.max(width))
        })
        .collect()
}

/// Horizontal offset of a cell's text inside a column of `column_width`, for `text_width`.
/// Pure, unit-tested.
pub(crate) fn text_offset(align: Alignment, column_width: f32, text_width: f32) -> f32 {
    let free = (column_width - text_width).max(0.0);
    match align {
        Alignment::Center => free / 2.0,
        Alignment::Right => free,
        Alignment::Unset | Alignment::Left => 0.0,
    }
}

/// Shaped cells and geometry of one table. Built in `request_layout` and completed in `prepaint`.
#[derive(Default)]
pub struct Grid {
    /// Shaped text of each cell, `[row][column]`. Row 0 is the header.
    cells: Vec<Vec<ShapedLine>>,
    /// Height of one row, in pixels, padding included.
    row_height: f32,
    /// Line height of the cell text, in pixels.
    line_height: Pixels,
    /// Column width from the widest cell, before any shrinking, in pixels.
    natural_widths: Vec<f32>,
    /// `(x offset from the left edge, width)` of each column after fitting the editor width.
    columns: Vec<(f32, f32)>,
}

/// Draws one table block as a grid. Each column is as wide as its widest cell, shrunk when the
/// table is wider than the editor. The header row is bold and has a heavier line under it.
pub struct TableElement {
    editor: Entity<Editor>,
    /// Byte range of the table block in the buffer.
    source: Range<usize>,
    table: Table,
}

impl TableElement {
    /// A grid for `table`, which was parsed from the text at `source`.
    pub fn new(editor: Entity<Editor>, source: Range<usize>, table: Table) -> Self {
        Self {
            editor,
            source,
            table,
        }
    }
}

impl IntoElement for TableElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for TableElement {
    type RequestLayoutState = Grid;
    type PrepaintState = Grid;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let text_style = window.text_style();
        let font = text_style.font();
        let font_size = text_style.font_size.to_pixels(window.rem_size());
        let theme = crate::theme::current(cx);
        let line_height = window.line_height();
        let header = BlockStyle {
            bold: true,
            ..PARAGRAPH
        };

        let mut cells = Vec::with_capacity(self.table.rows.len() + 1);
        let all_rows = std::iter::once(&self.table.header).chain(self.table.rows.iter());
        for (row_index, row) in all_rows.enumerate() {
            let style = if row_index == 0 { &header } else { &PARAGRAPH };
            let mut shaped_row = Vec::with_capacity(row.len());
            for text in row {
                let (visible, runs) = cell_visible(text);
                let text_runs: Vec<TextRun> = runs
                    .iter()
                    .map(|run| text_run(run, style, &font, &theme))
                    .collect();
                shaped_row.push(window.text_system().shape_line(
                    SharedString::from(visible),
                    font_size,
                    &text_runs,
                    None,
                ));
            }
            cells.push(shaped_row);
        }

        let widths: Vec<Vec<f32>> = cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|shaped| f32::from(shaped.width) + 2.0 * CELL_PAD_X)
                    .collect()
            })
            .collect();
        let natural_widths = max_widths(&widths);
        let row_height = f32::from(line_height) + 2.0 * CELL_PAD_Y;

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(row_height * cells.len() as f32).into();
        let grid = Grid {
            cells,
            row_height,
            line_height,
            natural_widths,
            columns: Vec::new(),
        };
        (window.request_layout(style, [], cx), grid)
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let mut grid = std::mem::take(request_layout);
        let available = f32::from(bounds.size.width);
        let natural_total: f32 = grid.natural_widths.iter().sum();
        let scale = if natural_total > available && natural_total > 0.0 {
            available / natural_total
        } else {
            1.0
        };
        let mut x = 0.0;
        grid.columns = grid
            .natural_widths
            .iter()
            .map(|width| {
                let column = (x, width * scale);
                x += width * scale;
                column
            })
            .collect();
        grid
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let grid = std::mem::take(prepaint);
        let left = bounds.left();
        let top = bounds.top();
        let table_width = grid.columns.last().map_or(0.0, |(x, width)| x + width);
        let row_count = grid.cells.len();
        let row_height = grid.row_height;

        let theme = crate::theme::current(cx);
        // A selection touching this block highlights the whole block (task: selection over
        // rendered tables and images). Full and partial overlap highlight alike; a caret
        // alone is `BlockCoverage::None`. The quad goes over the content because the
        // header fill is opaque, and it uses the same `selection` token as paragraphs.
        let covered = {
            let editor = self.editor.read(cx);
            block_coverage(editor.selection.selection, self.source.clone()) != BlockCoverage::None
        };
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            // Header background, then the grid lines: a line above each row, one below the last
            // row, and a vertical line at each column edge. The line under the header is heavier.
            let header_height = px(row_height);
            window.paint_quad(fill(
                Bounds::new(point(left, top), size(px(table_width), header_height)),
                theme.table_header_bg,
            ));
            for row in 0..=row_count {
                let thickness = if row == 1 { LINE_THICK } else { LINE_THIN };
                let y = top + px(row_height * row as f32);
                window.paint_quad(fill(
                    Bounds::new(point(left, y), size(px(table_width), px(thickness))),
                    theme.table_border,
                ));
            }
            let column_edges = grid
                .columns
                .iter()
                .map(|(x, _)| *x)
                .chain(std::iter::once(table_width));
            let table_height = row_height * row_count as f32;
            for edge in column_edges {
                window.paint_quad(fill(
                    Bounds::new(
                        point(left + px(edge), top),
                        size(px(LINE_THIN), px(table_height)),
                    ),
                    theme.table_border,
                ));
            }

            // Cell text, aligned inside its column.
            for (row_index, row) in grid.cells.iter().enumerate() {
                let y = top + px(row_height * row_index as f32 + CELL_PAD_Y);
                for (column_index, shaped) in row.iter().enumerate() {
                    let Some(&(x, width)) = grid.columns.get(column_index) else {
                        continue;
                    };
                    let align = self
                        .table
                        .align
                        .get(column_index)
                        .copied()
                        .unwrap_or(Alignment::Unset);
                    let text_width = f32::from(shaped.width);
                    let inner = (width - 2.0 * CELL_PAD_X).max(0.0);
                    let offset = text_offset(align, inner, text_width);
                    let origin = point(left + px(x + CELL_PAD_X + offset), y);
                    if let Err(err) =
                        shaped.paint(origin, grid.line_height, TextAlign::Left, None, window, cx)
                    {
                        eprintln!("md_editor: failed to paint table cell: {err}");
                    }
                }
            }
            if covered {
                window.paint_quad(fill(bounds, theme.selection));
            }
        });

        // Keep the block's box for hit testing. It has no text lines, so a click on it places the
        // caret at the start of the block (`cursor::offset_at`).
        let source = self.source.clone();
        let line_height = grid.line_height;
        self.editor.update(cx, |editor, _cx| {
            editor.layouts.insert(
                source,
                LayoutSnapshot {
                    lines: Vec::new(),
                    bounds,
                    line_height,
                    revealed: false,
                },
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(texts: &[&str]) -> Vec<String> {
        texts.iter().map(|text| text.to_string()).collect()
    }

    #[test]
    fn parses_header_alignment_and_rows() {
        let table = parse_table(
            "| Name | Count | Notes |\n|:-----|:-----:|------:|\n| Apples | 3 | fresh |\n| Pears | 12 | ripe |\n",
        )
        .expect("a table");
        assert_eq!(table.header, cells(&["Name", "Count", "Notes"]));
        assert_eq!(
            table.align,
            vec![Alignment::Left, Alignment::Center, Alignment::Right]
        );
        assert_eq!(
            table.rows,
            vec![
                cells(&["Apples", "3", "fresh"]),
                cells(&["Pears", "12", "ripe"])
            ]
        );
    }

    #[test]
    fn leading_and_trailing_pipes_are_optional() {
        let table = parse_table("a | b\n--|--\n1 | 2").expect("a table");
        assert_eq!(table.header, cells(&["a", "b"]));
        assert_eq!(table.align, vec![Alignment::Unset, Alignment::Unset]);
        assert_eq!(table.rows, vec![cells(&["1", "2"])]);
    }

    #[test]
    fn escaped_pipe_stays_inside_its_cell() {
        let table = parse_table("| a | b |\n| - | - |\n| x \\| y | z |\n").expect("a table");
        assert_eq!(table.rows, vec![cells(&["x | y", "z"])]);
        let table = parse_table("| a |\n| - |\n| tail \\|\n").expect("a table");
        assert_eq!(table.rows, vec![cells(&["tail |"])]);
    }

    #[test]
    fn ragged_rows_are_padded_and_extra_cells_dropped() {
        let table = parse_table("| a | b | c |\n|---|---|---|\n| 1 |\n| 1 | 2 | 3 | 4 |\n")
            .expect("a table");
        assert_eq!(table.rows[0], cells(&["1", "", ""]));
        assert_eq!(table.rows[1], cells(&["1", "2", "3"]));
    }

    #[test]
    fn non_tables_are_rejected() {
        assert_eq!(parse_table("just text"), None);
        assert_eq!(
            parse_table("| a | b |\n| - |\n"),
            None,
            "cell counts differ"
        );
        assert_eq!(
            parse_table("| a |\n| : |\n"),
            None,
            "a lone colon is no delimiter"
        );
        assert_eq!(
            parse_table("| a |\n| x |\n"),
            None,
            "text in the delimiter row"
        );
        assert_eq!(parse_table(""), None);
    }

    #[test]
    fn max_widths_take_the_widest_cell_per_column() {
        let rows = vec![vec![10.0, 5.0], vec![3.0, 20.0, 7.0], vec![12.0]];
        assert_eq!(max_widths(&rows), vec![12.0, 20.0, 7.0]);
        assert_eq!(max_widths(&[]), Vec::<f32>::new());
    }

    /// Runs of `cell`: the visible text with its markers hidden, and the styles of each run.
    fn visible_runs(cell: &str) -> (String, Vec<RunStyle>) {
        cell_visible(cell)
    }

    /// Assert that `runs` tile `visible` exactly, with no gaps or overlaps.
    fn assert_covers(runs: &[RunStyle], visible: &str) {
        let mut next = 0;
        for run in runs {
            assert_eq!(run.range.start, next, "gap or overlap in {runs:?}");
            assert!(run.range.start < run.range.end, "empty run in {runs:?}");
            next = run.range.end;
        }
        assert_eq!(next, visible.len(), "runs do not reach the end");
    }

    #[test]
    fn cell_styles_bold_italic_code_strike_and_link() {
        let cases = [
            ("**bold**", "bold", (true, false, false, false, false)),
            ("*it*", "it", (false, true, false, false, false)),
            ("`code`", "code", (false, false, true, false, false)),
            ("~~gone~~", "gone", (false, false, false, true, false)),
        ];
        for (cell, visible, style) in cases {
            let (text, runs) = visible_runs(cell);
            assert_eq!(text, visible, "cell {cell:?}");
            assert_eq!(runs.len(), 1, "cell {cell:?}: one run, got {runs:?}");
            let run = &runs[0];
            assert_eq!(
                (run.bold, run.italic, run.code, run.strike, run.link),
                style,
                "cell {cell:?}"
            );
            assert_eq!(run.range, 0..visible.len());
        }
        let (text, runs) = visible_runs("[docs](https://example.com)");
        assert_eq!(text, "docs");
        assert_eq!(runs.len(), 1);
        assert!(runs[0].link);
        assert_eq!(runs[0].range, 0..4);
    }

    #[test]
    fn cell_runs_tile_the_visible_text() {
        let (visible, runs) = visible_runs("a **b** and `c`");
        assert_eq!(visible, "a b and c");
        assert_covers(&runs, &visible);
        assert!(runs
            .iter()
            .any(|run| run.bold && &visible[run.range.clone()] == "b"));
        assert!(runs
            .iter()
            .any(|run| run.code && &visible[run.range.clone()] == "c"));
    }

    #[test]
    fn cell_math_stays_plain_text() {
        let (visible, runs) = visible_runs("$x$");
        assert_eq!(visible, "$x$", "math markers stay visible");
        assert_covers(&runs, &visible);
        assert!(
            runs.iter().all(|run| !run.bold
                && !run.italic
                && !run.code
                && !run.strike
                && !run.link),
            "no styling, got {runs:?}"
        );
        assert!(cell_spans("$x$").is_empty());
    }

    #[test]
    fn escaped_pipe_inside_code_stays_one_styled_cell() {
        let table = parse_table("| `a\\|b` | c |\n|---|---|\n").expect("a table");
        assert_eq!(table.header, cells(&["`a|b`", "c"]));
        let (visible, runs) = visible_runs(&table.header[0]);
        assert_eq!(visible, "a|b");
        assert_eq!(runs.len(), 1);
        assert!(runs[0].code, "got {runs:?}");
        assert_eq!(runs[0].range, 0..3);
    }

    #[test]
    fn empty_cell_has_no_runs() {
        assert_eq!(cell_visible(""), (String::new(), Vec::new()));
    }

    #[test]
    fn text_offset_follows_the_alignment() {
        assert_eq!(text_offset(Alignment::Unset, 100.0, 40.0), 0.0);
        assert_eq!(text_offset(Alignment::Left, 100.0, 40.0), 0.0);
        assert_eq!(text_offset(Alignment::Center, 100.0, 40.0), 30.0);
        assert_eq!(text_offset(Alignment::Right, 100.0, 40.0), 60.0);
        assert_eq!(
            text_offset(Alignment::Right, 10.0, 40.0),
            0.0,
            "never negative"
        );
    }
}
