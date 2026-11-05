//! Plain-text element for the editor (plan task 3.1). Owner: task 3.1.
//!
//! `TextElement` draws a byte range of the buffer as plain text, one shaped line per source
//! line. It is a custom GPUI `Element` (DESIGN §7). `Editor::render` creates it for the whole
//! buffer (HOOK 3.1); the block list (3.5) creates one per block.

use std::ops::Range;
use std::panic::Location;
use std::rc::Rc;

use gpui::{
    fill, point, px, relative, rgb, size, App, Bounds, ElementId, Entity, Font, FontFallbacks,
    FontStyle, FontWeight, GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId,
    Pixels, Point, ShapedLine, StrikethroughStyle, Style, TextAlign, TextRun, UnderlineStyle,
    Window,
};

use md_core::display_map::{Bias, DisplayMap};
use md_core::inline::inline_spans;
use md_core::BlockKind;
use md_render::highlight::Token;

use crate::blocks::basic::{
    block_style, line_runs, shows_inline_styles, BlockStyle, Decoration, RunStyle, TextColor,
};
use crate::blocks::code_fence::{
    self, token_color, CODE_BLOCK_BACKGROUND, CODE_BLOCK_RADIUS_PX, CODE_PAD_X_PX, CODE_PAD_Y_PX,
};
use crate::blocks::list::{self, ListGutter, ListLine, ListMarker};
use crate::cursor::{self, hit_test};
use crate::editor::Editor;
use crate::ime;
use crate::inline_render::{
    block_hidden_ranges, display_to_source, source_to_display, visible_run_lengths, visible_text,
};
use crate::selection;

/// Monospace family for inline code: the macOS system face, or the common Linux one. The
/// fallbacks are tried when the family is missing.
#[cfg(target_os = "macos")]
const CODE_FAMILY: &str = "Menlo";
#[cfg(not(target_os = "macos"))]
const CODE_FAMILY: &str = "DejaVu Sans Mono";
const CODE_FALLBACKS: &[&str] = &["Consolas", "Liberation Mono", "Courier New"];
/// Body family on Linux (task 4.4). GPUI maps the default `.SystemUIFont` to "IBM Plex Sans",
/// which is often not installed, and the fallback then has one regular face, so bold and italic
/// runs look regular. Liberation Sans is widely packaged and has regular, bold, italic and
/// bold-italic faces. DejaVu Sans has no italic face on some installs, so it is not used. macOS
/// keeps the system font.
#[cfg(not(target_os = "macos"))]
const BODY_FAMILY: Option<&str> = Some("Liberation Sans");
#[cfg(target_os = "macos")]
const BODY_FAMILY: Option<&str> = None;
/// Colours of the block styles (DESIGN §9 will move these into the theme).
const LINK_COLOR: Hsla = Hsla {
    h: 0.61,
    s: 0.75,
    l: 0.50,
    a: 1.0,
};
const MUTED_COLOR: Hsla = Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.45,
    a: 1.0,
};
const MARKER_COLOR: Hsla = Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.70,
    a: 1.0,
};
const CODE_BACKGROUND: Hsla = Hsla {
    h: 0.0,
    s: 0.0,
    l: 0.50,
    a: 0.15,
};
/// Width of the blockquote bar, and the thickness of a thematic break, in pixels.
const QUOTE_BAR_WIDTH: f32 = 3.0;
const RULE_THICKNESS: f32 = 1.0;

/// Byte range of each line in `text`, split on `\n`. Ranges exclude the `\n`. A trailing
/// newline gives a final empty range. Pure function, unit-tested in 3.1.
pub fn line_ranges(text: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, byte) in text.bytes().enumerate() {
        if byte == b'\n' {
            ranges.push(start..index);
            start = index + 1;
        }
    }
    ranges.push(start..text.len());
    ranges
}

/// One laid-out source line.
///
/// Written during paint by `TextElement` and read by selection (3.2), hit testing (3.2) and IME
/// bounds (3.4). The shaped text is the line without its hidden bytes (task 4.4), so a source
/// offset is converted with [`LineLayout::source_to_shaped`] before it indexes `shaped`, and an
/// index is converted back with [`LineLayout::shaped_to_source`].
#[derive(Clone)]
pub struct LineLayout {
    /// Byte range of the line in the buffer, excluding its `\n`.
    pub source: Range<usize>,
    /// The shaped text of the line: its visible bytes only.
    pub shaped: ShapedLine,
    /// Top-left corner of the line in window coordinates.
    pub origin: Point<Pixels>,
    /// Hidden byte ranges of the line, absolute and inside `source` (task 4.4).
    pub hidden: Vec<Range<usize>>,
    /// The bullet, number, tick or checkbox drawn in the gutter of a list line whose markers
    /// are hidden (task 4.2). Clicks on a checkbox are found through it.
    pub gutter: Option<ListGutter>,
}

impl LineLayout {
    /// The index into `shaped` for source offset `source` of this line.
    pub fn source_to_shaped(&self, source: usize) -> usize {
        source_to_display(source, self.source.start, &self.hidden)
    }

    /// The source offset for index `index` into `shaped`. `Bias::Left` gives the start of a
    /// hidden range at a collapse point, `Bias::Right` its end.
    pub fn shaped_to_source(&self, index: usize, bias: Bias) -> usize {
        display_to_source(index, self.source.start, &self.hidden, bias == Bias::Right)
    }

    /// The x position, relative to the line origin, of source offset `source`.
    pub fn x_for_source(&self, source: usize) -> Pixels {
        self.shaped.x_for_index(self.source_to_shaped(source))
    }
}

/// Geometry of one painted block: the lines of one `TextElement`.
#[derive(Default, Clone)]
pub struct LayoutSnapshot {
    pub lines: Vec<LineLayout>,
    /// Bounds of the element in window coordinates.
    pub bounds: Bounds<Pixels>,
    /// Height of one line.
    pub line_height: Pixels,
    /// Whether the caret is in this block, so its markers show (task 4.4). A thematic break
    /// draws its rule only while it is not revealed, since the rule would cross its text.
    pub revealed: bool,
}

/// One painted block. `source` is the block's byte range in the buffer (its trailing newline
/// excluded), the same range its `TextElement` draws.
#[derive(Clone)]
pub struct BlockLayout {
    pub source: Range<usize>,
    pub snapshot: LayoutSnapshot,
}

impl BlockLayout {
    /// The line that contains byte `offset`. A line break belongs to the line before it, so the
    /// offset at the end of a line is found on that line.
    pub fn line_for_offset(&self, offset: usize) -> Option<&LineLayout> {
        self.snapshot
            .lines
            .iter()
            .find(|line| line.source.start <= offset && offset <= line.source.end)
    }
}

/// Layouts of the visible blocks from the most recent frame, held by `Editor` (field
/// `layouts`), in buffer order.
///
/// Every block is painted by its own `TextElement`. `TextElement::paint` inserts its block
/// here, and `BlockHost` clears the map at the start of each frame (3.5). A block scrolled out
/// of view has no entry, so offsets there have no position until they are scrolled back in.
/// Points and offsets are mapped through this map, never through one block's layout.
#[derive(Default, Clone)]
pub struct Layouts {
    blocks: Vec<BlockLayout>,
}

impl Layouts {
    /// Forget every block. Called at the start of each frame.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }

    /// Record a painted block. Blocks are kept in buffer order whatever the paint order is. A
    /// block painted twice in one frame replaces its earlier entry.
    pub fn insert(&mut self, source: Range<usize>, snapshot: LayoutSnapshot) {
        let index = self
            .blocks
            .partition_point(|block| block.source.start < source.start);
        let block = BlockLayout { source, snapshot };
        match self.blocks.get(index) {
            Some(existing) if existing.source.start == block.source.start => {
                self.blocks[index] = block;
            }
            _ => self.blocks.insert(index, block),
        }
    }

    /// The visible block whose source contains byte `offset`, if the block is on screen.
    pub fn block_for_offset(&self, offset: usize) -> Option<&BlockLayout> {
        index_containing(&self.blocks, |block| block.source.clone(), offset)
            .map(|index| &self.blocks[index])
    }

    /// The visible block under window-space height `y`. Above the first block or below the last
    /// one this is that end block, and between two blocks it is the nearer one.
    pub fn block_for_y(&self, y: Pixels) -> Option<&BlockLayout> {
        index_for_y(&self.blocks, f32::from(y)).map(|index| &self.blocks[index])
    }

    /// The source offset under `point` (window coordinates), found in the block under the
    /// point, or in the nearest visible block when the point is between blocks or past them.
    /// Returns 0 when no block is visible.
    pub fn offset_at(&self, display: &DisplayMap, point: Point<Pixels>) -> usize {
        match self.block_for_y(point.y) {
            Some(block) => hit_test(&block.snapshot, display, point),
            None => 0,
        }
    }
}

/// Index of the item whose range contains `offset`. `items` must be in buffer order, with
/// ranges that do not overlap. An offset at the end of an item's range belongs to that item,
/// since it is the position of the line break after it. Pure, unit-tested.
pub(crate) fn index_containing<T>(
    items: &[T],
    range_of: impl Fn(&T) -> Range<usize>,
    offset: usize,
) -> Option<usize> {
    let count = items.partition_point(|item| range_of(item).start <= offset);
    let index = count.checked_sub(1)?;
    (offset <= range_of(&items[index]).end).then_some(index)
}

/// Index of the block at height `y`, for blocks stacked in buffer order. Inside a block this is
/// that block. Above the first block it is 0, below the last it is the last index, and between
/// two blocks it is the nearer one. Pure, unit-tested.
pub(crate) fn index_for_y(blocks: &[BlockLayout], y: f32) -> Option<usize> {
    let top = |block: &BlockLayout| f32::from(block.snapshot.bounds.top());
    let bottom = |block: &BlockLayout| f32::from(block.snapshot.bounds.bottom());
    if blocks.is_empty() {
        return None;
    }
    // The first block whose bottom is below `y`.
    let after = blocks.partition_point(|block| bottom(block) <= y);
    if after == blocks.len() {
        return Some(blocks.len() - 1);
    }
    if top(&blocks[after]) <= y {
        return Some(after);
    }
    if after == 0 {
        return Some(0);
    }
    let previous = after - 1;
    if y - bottom(&blocks[previous]) <= top(&blocks[after]) - y {
        Some(previous)
    } else {
        Some(after)
    }
}

/// Paints `source` (a byte range of the buffer) as plain text.
///
/// Each block (one list item, see `block_list.rs`) gets its own `TextElement`. Paint order
/// within one block:
/// 1. `selection::paint_selection`, for the part of the selection in this block
/// 2. the block decoration (`paint_decoration`: blockquote bar, thematic break)
/// 3. the text lines, each with `ime::decorate_runs` applied to its runs
/// 4. `cursor::paint_caret`, which draws only when the caret is in this block
///
/// Each line's runs come from `blocks::basic::line_runs` (inline styles, task 4.1). The block's
/// `BlockStyle` sets the font scale, indent and colour, and scales the line height.
///
/// Mouse handlers and the input handler are not registered here. `BlockHost` registers them
/// once for the whole list. After the text is painted, the element inserts this block's
/// `LayoutSnapshot` into `Editor::layouts`.
pub struct TextElement {
    editor: Entity<Editor>,
    /// Byte range of the buffer to draw.
    source: Range<usize>,
    /// Kind of the block. It decides the markers and inline spans (task 4.4).
    kind: BlockKind,
    /// How the block is drawn (task 4.1): font scale, bold, colour, indent and decoration.
    style: BlockStyle,
    /// Whether the inline spans of the block are drawn with their styles (task 4.1).
    inline_styles: bool,
}

impl TextElement {
    /// The fence lines of this block that take no height and are not drawn: the fence lines of a
    /// code fence while the caret is outside it (task 5.3). Empty for every other block.
    fn collapsed_lines(&self, text: &str, caret: usize) -> Vec<Range<usize>> {
        let revealed = self.source.start <= caret && caret <= self.source.end;
        if self.kind == BlockKind::CodeFence && !revealed {
            code_fence::fence_lines(text, self.source.start)
        } else {
            Vec::new()
        }
    }

    /// An element that draws `source` of the editor's buffer as a block of `kind`. The style and
    /// the inline spans follow the kind (task 4.1). Markers are hidden unless the caret is in
    /// the block or in their span (task 4.4).
    pub fn new(editor: Entity<Editor>, source: Range<usize>, kind: BlockKind) -> Self {
        Self {
            editor,
            source,
            style: block_style(&kind),
            inline_styles: shows_inline_styles(&kind),
            kind,
        }
    }
}

/// The font of body text: the window's font, with the body family on Linux (see `BODY_FAMILY`).
fn body_font(mut font: Font) -> Font {
    if let Some(family) = BODY_FAMILY {
        font.family = family.into();
    }
    font
}

/// The monospace font of code: `base` with the code family and its fallbacks.
fn code_font(base: &Font) -> Font {
    let mut font = base.clone();
    font.family = CODE_FAMILY.into();
    font.fallbacks = Some(FontFallbacks::from_fonts(
        CODE_FALLBACKS.iter().map(|name| name.to_string()).collect(),
    ));
    font
}

/// The runs of one source line of a code fence block (task 5.3). `line` is absolute and
/// `code_base` is the absolute start of the highlighted code. A fence line is muted. A code line
/// takes the colour of each token, and plain text the base colour `base_color`.
fn code_runs(
    line: Range<usize>,
    is_fence: bool,
    code_base: usize,
    tokens: &[Token],
    font: &Font,
    base_color: Hsla,
) -> Vec<TextRun> {
    let font = code_font(font);
    let run = |len: usize, color: Hsla| TextRun {
        len,
        font: font.clone(),
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    if is_fence {
        return vec![run(line.len(), MUTED_COLOR)];
    }
    code_fence::line_segments(line, code_base, tokens)
        .into_iter()
        .map(|(len, kind)| {
            let color = kind.map_or(base_color, |kind| Hsla::from(rgb(token_color(kind))));
            run(len, color)
        })
        .collect()
}

/// Paint the background of a code block: a light rounded box behind the whole block (task 5.3).
fn paint_code_background(bounds: Bounds<Pixels>, window: &mut Window) {
    let background = Hsla::from(rgb(CODE_BLOCK_BACKGROUND));
    window.paint_quad(fill(bounds, background).corner_radii(px(CODE_BLOCK_RADIUS_PX)));
}

/// Convert one styled part of a line into a GPUI run. `base` is the font and `base_color` the
/// colour of plain text. `style` supplies the block's bold and colour.
fn text_run(run: &RunStyle, style: &BlockStyle, base: &Font, base_color: Hsla) -> TextRun {
    let mut font = base.clone();
    if run.bold || style.bold {
        font.weight = FontWeight::BOLD;
    }
    if run.italic {
        font.style = FontStyle::Italic;
    }
    if run.code {
        font = code_font(&font);
    }
    let color = if run.link {
        LINK_COLOR
    } else {
        match style.color {
            TextColor::Normal => base_color,
            TextColor::Muted => MUTED_COLOR,
            TextColor::Link => LINK_COLOR,
            TextColor::Marker => MARKER_COLOR,
        }
    };
    TextRun {
        len: run.range.len(),
        font,
        color,
        background_color: run.code.then_some(CODE_BACKGROUND),
        underline: run.link.then_some(UnderlineStyle {
            thickness: px(1.),
            color: Some(LINK_COLOR),
            wavy: false,
        }),
        strikethrough: run.strike.then_some(StrikethroughStyle {
            thickness: px(1.),
            color: Some(color),
        }),
    }
}

/// Split `runs` (relative to the line) at `from`, the start of the text of a checked task item,
/// and mark each run with whether it is after that point (task 4.2). `None` marks no run.
fn split_done(runs: Vec<RunStyle>, from: Option<usize>) -> Vec<(RunStyle, bool)> {
    let Some(from) = from else {
        return runs.into_iter().map(|run| (run, false)).collect();
    };
    let mut out = Vec::with_capacity(runs.len() + 1);
    for run in runs {
        if run.range.start < from && from < run.range.end {
            let mut head = run.clone();
            head.range.end = from;
            let mut tail = run;
            tail.range.start = from;
            out.push((head, false));
            out.push((tail, true));
        } else {
            let done = run.range.start >= from;
            out.push((run, done));
        }
    }
    out
}

/// Paint the block's decoration: a bar on the left for a blockquote, a line across for a
/// thematic break. Drawn before the text, so the text sits on top.
fn paint_decoration(decoration: Decoration, snapshot: &LayoutSnapshot, window: &mut Window) {
    let height = snapshot.line_height * snapshot.lines.len() as f32;
    match decoration {
        Decoration::None => {}
        Decoration::QuoteBar => {
            let bar = Bounds::new(
                point(snapshot.bounds.left(), snapshot.bounds.top()),
                size(px(QUOTE_BAR_WIDTH), height),
            );
            window.paint_quad(fill(bar, MUTED_COLOR));
        }
        Decoration::Rule if !snapshot.revealed => {
            let top = snapshot.bounds.top() + snapshot.line_height * 0.5;
            let rule = Bounds::new(
                point(snapshot.bounds.left(), top),
                size(snapshot.bounds.size.width, px(RULE_THICKNESS)),
            );
            window.paint_quad(fill(rule, MARKER_COLOR));
        }
        Decoration::Rule => {}
    }
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl gpui::Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = LayoutSnapshot;

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
        let editor = self.editor.read(cx);
        let text = editor.buffer().slice(self.source.clone());
        let caret = editor.selection.selection.head;
        let collapsed = self.collapsed_lines(&text, caret);
        let line_count = line_ranges(&text).len() - collapsed.len();
        let line_height = window.line_height() * self.style.font_scale;
        let padding = if self.kind == BlockKind::CodeFence {
            px(2. * CODE_PAD_Y_PX)
        } else {
            px(0.)
        };

        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = (line_height * line_count as f32 + padding).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let text = editor.buffer().slice(self.source.clone());
        let marked = editor.ime.marked.clone();
        // The caret decides what is revealed: a block that holds it shows its block markers, and
        // a span that holds it shows its inline markers (task 4.4). Other markers are hidden.
        let caret = editor.selection.selection.head;
        let revealed = self.source.start <= caret && caret <= self.source.end;
        let hidden = block_hidden_ranges(&text, self.source.start, &self.kind, Some(caret));
        // A code fence (task 5.3): its fence lines, and the parsed fence for the code runs.
        let is_code = self.kind == BlockKind::CodeFence;
        let collapsed = self.collapsed_lines(&text, caret);
        let fence = if is_code {
            code_fence::parse_fence(&text, self.source.start)
        } else {
            None
        };

        let style = window.text_style();
        let font = body_font(style.font());
        let font_size = style.font_size.to_pixels(window.rem_size()) * self.style.font_scale;
        let color = style.color;
        let line_height = window.line_height() * self.style.font_scale;
        // Inline spans of the whole block, with absolute offsets, so that a span that crosses a
        // line break is styled on each of its lines.
        let spans = if self.inline_styles {
            inline_spans(&text, self.source.start)
        } else {
            Vec::new()
        };
        let indent = px(self.style.indent_px);
        // The highlighted tokens of the code, from the editor's cache (task 5.3). The editor is
        // no longer borrowed here, so it can be updated.
        let code_tokens: Rc<[Token]> = match &fence {
            Some(fence) => {
                let code =
                    &text[fence.code.start - self.source.start..fence.code.end - self.source.start];
                let lang = fence.language();
                self.editor
                    .update(cx, |editor, _cx| editor.code_cache.tokens(code, lang))
            }
            None => Rc::from(Vec::new()),
        };
        // Code blocks are inset from their background (task 5.3).
        let pad_x = if is_code { CODE_PAD_X_PX } else { 0. };
        let pad_y = if is_code { CODE_PAD_Y_PX } else { 0. };
        // The items of a list block, one per line (task 4.2). Empty for other blocks.
        let items = if self.kind == BlockKind::List {
            list::parse_list_lines(&text, self.source.start)
        } else {
            Vec::new()
        };
        let marker_text = list::MarkerText {
            font: font.clone(),
            size: font_size,
            color: MUTED_COLOR,
        };
        let done_style = BlockStyle {
            color: TextColor::Muted,
            ..self.style
        };

        let mut lines = Vec::new();
        let mut top = bounds.top() + px(pad_y);
        for (index, range) in line_ranges(&text).into_iter().enumerate() {
            // Line ranges are relative to `text`, which starts at `source.start` in the buffer.
            let source = (self.source.start + range.start)..(self.source.start + range.end);
            // A hidden fence line takes no height (task 5.3).
            if collapsed.contains(&source) {
                continue;
            }
            let line = &text[range];
            // A list line shows its markers only while the caret is on it (task 4.2).
            let item = items.get(index);
            let line_revealed = source.start <= caret && caret <= source.end;
            let text_x = match item {
                Some(item) => list::text_indent(item, line_revealed),
                None => f32::from(indent) + pad_x,
            };
            let line_origin = point(bounds.left() + px(text_x), top);
            // A checked task item is muted and struck through, from its text on.
            let done_from = match item {
                Some(ListLine {
                    marker: Some(ListMarker::Task { checked: true }),
                    text_start,
                    ..
                }) => Some(*text_start - source.start),
                _ => None,
            };
            let styled: Vec<TextRun> = match &fence {
                Some(fence) => {
                    let is_fence =
                        fence.open_line == source || fence.close_line.as_ref() == Some(&source);
                    code_runs(
                        source.clone(),
                        is_fence,
                        fence.code.start,
                        &code_tokens,
                        &font,
                        color,
                    )
                }
                None => split_done(line_runs(line, source.start, &spans), done_from)
                    .into_iter()
                    .map(|(mut run, done)| {
                        if done {
                            run.strike = true;
                            text_run(&run, &done_style, &font, color)
                        } else {
                            text_run(&run, &self.style, &font, color)
                        }
                    })
                    .collect(),
            };
            // Zero-length runs are dropped, so an empty line is shaped with no runs. The runs
            // still cover the source line here, so the IME marked range is applied first.
            let runs: Vec<TextRun> = ime::decorate_runs(styled, source.clone(), marked.clone())
                .into_iter()
                .filter(|run| run.len > 0)
                .collect();
            // Only the visible bytes are shaped (task 4.4). A run that loses all its bytes is dropped.
            let (visible, hidden_line) = visible_text(line, source.start, &hidden);
            let lens: Vec<usize> = runs.iter().map(|run| run.len).collect();
            let runs: Vec<TextRun> = visible_run_lengths(&lens, source.start, &hidden_line)
                .into_iter()
                .map(|(index, len)| TextRun {
                    len,
                    ..runs[index].clone()
                })
                .collect();
            let shaped = window
                .text_system()
                .shape_line(visible.into(), font_size, &runs, None);
            // The marker of an item whose markers are hidden goes in the gutter (task 4.2).
            let gutter = match item {
                Some(item) if item.marker.is_some() && !line_revealed => Some(list::gutter(
                    item,
                    line,
                    source.start,
                    line_origin,
                    line_height,
                    &marker_text,
                    window,
                )),
                _ => None,
            };
            lines.push(LineLayout {
                source,
                shaped,
                origin: line_origin,
                hidden: hidden_line,
                gutter,
            });
            top += line_height;
        }

        LayoutSnapshot {
            lines,
            bounds,
            line_height,
            revealed,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let snapshot = std::mem::take(prepaint);
        let editor = self.editor.clone();
        let focus = editor.read(cx).focus_handle.clone();
        let focused = focus.is_focused(window);

        // 0. The background of a code block, behind the selection and the text (task 5.3).
        if self.kind == BlockKind::CodeFence {
            paint_code_background(snapshot.bounds, window);
        }

        // 1. Selection behind the text, for the part of it that lies in this block.
        selection::paint_selection(editor.read(cx), &snapshot, window);

        // 2. The block decoration (blockquote bar, thematic break), behind the text.
        paint_decoration(self.style.decoration, &snapshot, window);

        // 3. The text, one line at a time. Run backgrounds (inline code) go first, since
        // `ShapedLine::paint` draws the glyphs only.
        for line in &snapshot.lines {
            if let Err(err) = line.shaped.paint_background(
                line.origin,
                snapshot.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            ) {
                eprintln!(
                    "md_editor: failed to paint background {:?}: {err}",
                    line.source
                );
            }
        }
        for line in &snapshot.lines {
            if let Err(err) = line.shaped.paint(
                line.origin,
                snapshot.line_height,
                TextAlign::Left,
                None,
                window,
                cx,
            ) {
                eprintln!("md_editor: failed to paint line {:?}: {err}", line.source);
            }
        }

        // 3b. List bullets, numbers and checkboxes in the gutter of their lines (task 4.2).
        for line in &snapshot.lines {
            if let Some(gutter) = &line.gutter {
                gutter.paint(snapshot.line_height, window, cx);
            }
        }

        // 4. The caret, only when it is in this block.
        cursor::paint_caret(editor.read(cx), &snapshot, focused, window);

        // Keep this block's geometry for hit testing, caret reveal and IME bounds.
        let source = self.source.clone();
        editor.update(cx, |editor, _cx| editor.layouts.insert(source, snapshot));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{px, size, Bounds};

    /// A block with no shaped lines whose box is `top..top + height` in window coordinates.
    /// The mapping helpers only read `source` and `bounds`.
    fn block(source: Range<usize>, top: f32, height: f32) -> BlockLayout {
        BlockLayout {
            source,
            snapshot: LayoutSnapshot {
                bounds: Bounds::new(point(px(0.), px(top)), size(px(100.), px(height))),
                ..Default::default()
            },
        }
    }

    #[test]
    fn index_containing_finds_the_item_for_each_offset() {
        // "ab" 0..2, a blank line 3..3, "cd" 4..6, with the line breaks at 2, 3 and 6.
        let items = [0..2, 3..3, 4..6];
        assert_eq!(index_containing(&items, |r| r.clone(), 0), Some(0));
        assert_eq!(
            index_containing(&items, |r| r.clone(), 2),
            Some(0),
            "its line break"
        );
        assert_eq!(
            index_containing(&items, |r| r.clone(), 3),
            Some(1),
            "blank line"
        );
        assert_eq!(index_containing(&items, |r| r.clone(), 4), Some(2));
        assert_eq!(
            index_containing(&items, |r| r.clone(), 6),
            Some(2),
            "end of text"
        );
        assert_eq!(
            index_containing(&items, |r| r.clone(), 7),
            None,
            "past the end"
        );
    }

    #[test]
    fn index_containing_skips_offscreen_items() {
        // Only the items 0..2 and 6..8 are visible; offsets in the hidden 3..5 have no item.
        let visible = [(0..2, 0), (6..8, 3)];
        let range_of = |item: &(Range<usize>, usize)| item.0.clone();
        assert_eq!(index_containing(&visible, range_of, 1), Some(0));
        assert_eq!(index_containing(&visible, range_of, 4), None);
        assert_eq!(index_containing(&visible, range_of, 7), Some(1));
        assert_eq!(
            index_containing::<(Range<usize>, usize)>(&[], range_of, 0),
            None
        );
    }

    #[test]
    fn index_for_y_inside_above_below_and_between_blocks() {
        // Blocks at 0..20, 20..60 and 60..80 (stacked, as the list lays them out).
        let blocks = [
            block(0..2, 0., 20.),
            block(3..5, 20., 40.),
            block(6..9, 60., 20.),
        ];
        assert_eq!(index_for_y(&blocks, 0.), Some(0));
        assert_eq!(index_for_y(&blocks, 19.9), Some(0));
        assert_eq!(index_for_y(&blocks, 20.), Some(1));
        assert_eq!(index_for_y(&blocks, 79.), Some(2));
        // Above the first block and below the last clamp to the end blocks.
        assert_eq!(index_for_y(&blocks, -50.), Some(0));
        assert_eq!(index_for_y(&blocks, 500.), Some(2));
        assert_eq!(index_for_y(&[], 10.), None);
    }

    #[test]
    fn index_for_y_between_blocks_takes_the_nearer_one() {
        // A 10 px gap between 0..20 and 30..50.
        let blocks = [block(0..2, 0., 20.), block(3..5, 30., 20.)];
        assert_eq!(index_for_y(&blocks, 22.), Some(0));
        assert_eq!(
            index_for_y(&blocks, 27.),
            Some(1),
            "closer to the second block"
        );
        assert_eq!(
            index_for_y(&blocks, 25.),
            Some(0),
            "exactly halfway goes up"
        );
    }

    #[test]
    fn layouts_insert_keeps_buffer_order_and_replaces_duplicates() {
        let mut layouts = Layouts::default();
        layouts.insert(4..6, LayoutSnapshot::default());
        layouts.insert(0..2, LayoutSnapshot::default());
        layouts.insert(2..4, LayoutSnapshot::default());
        let starts: Vec<usize> = layouts.blocks.iter().map(|b| b.source.start).collect();
        assert_eq!(starts, vec![0, 2, 4]);
        layouts.insert(2..9, LayoutSnapshot::default());
        assert_eq!(layouts.blocks.len(), 3, "same start replaces the entry");
        assert_eq!(layouts.blocks[1].source, 2..9);
        layouts.clear();
        assert!(layouts.block_for_offset(0).is_none());
    }

    #[test]
    fn line_ranges_of_small_inputs() {
        assert_eq!(line_ranges(""), vec![0..0]);
        assert_eq!(line_ranges("a"), vec![0..1]);
        assert_eq!(line_ranges("a\n"), vec![0..1, 2..2]);
        assert_eq!(line_ranges("a\n\nb"), vec![0..1, 2..2, 3..4]);
    }

    #[test]
    fn line_ranges_use_byte_offsets() {
        // "é" is 2 bytes, "日" is 3 bytes, "🎉" is 4 bytes.
        let text = "é\n日本\n🎉";
        let ranges = line_ranges(text);
        assert_eq!(ranges, vec![0..2, 3..9, 10..14]);
        let lines: Vec<&str> = ranges.iter().map(|r| &text[r.clone()]).collect();
        assert_eq!(lines, vec!["é", "日本", "🎉"]);
    }

    #[test]
    fn line_ranges_of_100_lines() {
        let text: String = (1..=100).map(|i| format!("line {i}\n")).collect();
        let ranges = line_ranges(&text);
        assert_eq!(
            ranges.len(),
            101,
            "a trailing newline gives a final empty line"
        );
        assert_eq!(&text[ranges[0].clone()], "line 1");
        assert_eq!(&text[ranges[99].clone()], "line 100");
        assert_eq!(&text[ranges[100].clone()], "");

        let no_trailing: String = (1..=100)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ranges = line_ranges(&no_trailing);
        assert_eq!(ranges.len(), 100);
        for (i, range) in ranges.iter().enumerate() {
            assert_eq!(&no_trailing[range.clone()], format!("line {}", i + 1));
        }
    }
}
