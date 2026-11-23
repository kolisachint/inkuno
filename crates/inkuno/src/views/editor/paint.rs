//! Painting the laid-out text, and drawing Mermaid and SVG figures in the background (`Job`).

use super::*;

/// A figure drawn off the UI thread: laying out a large Mermaid diagram
/// can take a second, and resvg as much for a text-heavy SVG.
#[derive(Clone)]
pub(super) struct Job {
    pub(super) key: u64,
    /// The key of the source alone: one figure at a time per source.
    pub(super) family: u64,
    /// The SVG to draw; `None` if the source is invalid.
    pub(super) make: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    /// Display width at most, screen density, and background.
    pub(super) room: f32,
    pub(super) density: f32,
    pub(super) bg: [u8; 3],
}

impl Job {
    pub(super) fn id(&self) -> (u64, u32, u32, [u8; 3]) {
        (
            self.key,
            self.room.to_bits(),
            self.density.to_bits(),
            self.bg,
        )
    }
}

impl Hash for Job {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl gpui::Asset for Job {
    type Source = Self;
    type Output = Drawing;

    // The future must not borrow `cx`: an `async fn` would.
    #[allow(clippy::manual_async_fn)]
    fn load(job: Self, _: &mut App) -> impl Future<Output = Drawing> + Send + 'static {
        async move {
            let tree = svg::parse(&(job.make)()?)?;
            let (w, h) = (tree.size().width(), tree.size().height());
            // No finer than the screen at the largest display width: a very wide
            // or very tall diagram weighs no more than what is seen of it.
            let scale = svg::scale_for(w, h, job.density * (job.room / w).min(1.));
            let bytes = svg::bmp(
                &tree,
                scale,
                (0, 0, (w * scale).ceil() as u32, (h * scale).ceil() as u32),
                job.bg,
            )?;
            Some((
                Arc::new(Image::from_bytes(ImageFormat::Bmp, bytes)),
                size(px(w), px(h)),
            ))
        }
    }
}

/// The figure of `job` if it is ready; otherwise it is drawn (`waiting`), and the note will be
/// laid out again. Only one at a time per source (`before`: the ones being waited for): going
/// quickly through themes does not start one drawing per theme.
pub(super) fn drawn(
    job: Job,
    before: &[Job],
    waiting: &mut Vec<Job>,
    window: &mut Window,
    cx: &mut App,
) -> Option<Drawing> {
    if let Some(running) = before
        .iter()
        .find(|j| j.family == job.family && j.id() != job.id())
    {
        if window.use_asset::<Job>(running, cx).is_none() {
            waiting.push(running.clone());
            return None;
        }
        cx.remove_asset::<Job>(running);
    }
    let drawing = window.use_asset::<Job>(&job, cx);
    // Ready when its image is decoded: until then, the old version stays on screen.
    let ready = match &drawing {
        Some(Some((image, _))) => image.clone().use_render_image(window, cx).is_some(),
        Some(None) => true,
        None => false,
    };
    if !ready {
        waiting.push(job);
        return None;
    }
    // The figure is kept by the editor: gpui no longer has to hold it.
    cx.remove_asset::<Job>(&job);
    drawing
}

pub(super) struct EditorElement(pub(super) Entity<Editor>);

impl IntoElement for EditorElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for EditorElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let resized = self.0.update(cx, |editor, cx| {
            let before = editor.viewport.size.width;
            editor.layout(bounds, window, cx);
            before != editor.viewport.size.width
        });
        // The formatting bar's place follows the note's width: the next frame moves it. Asked for once this one
        // is drawn, since gpui forgets a view notified while it draws.
        if resized {
            let editor = self.0.clone();
            cx.defer(move |cx| editor.update(cx, |_, cx| cx.notify()));
        }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.0.read(cx).focus.clone();
        window.handle_input(&focus, ElementInputHandler::new(bounds, self.0.clone()), cx);
        let focused = focus.is_focused(window);
        self.0.update(cx, |editor, cx| {
            editor.paint(focused, window, cx);
            if let Some((hand, hitbox)) = editor.hand.zip(editor.view_hitbox.as_ref()) {
                window.set_cursor_style(hand, hitbox);
            }
        });
    }
}

impl Editor {
    /// The chip of a folded code block: its language, on a small plate at the top right of the block.
    /// The word-wrap switch: a quiet chip with a return arrow, dark while the block wraps.
    fn paint_wrap_chip(&self, bounds: Bounds<Pixels>, on: bool, window: &mut Window, cx: &mut App) {
        let t = self.theme;
        let s = t.size / crate::logic::prefs::Prefs::SIZE;
        window.paint_quad(fill(bounds, t.gutter).corner_radii(Corners::all(px(4. * s))));
        let run = TextRun {
            len: "↩".len(),
            font: font(sans()),
            color: if on { t.text } else { t.soft },
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let glyph =
            window
                .text_system()
                .shape_line(SharedString::from("↩"), px(13. * s), &[run], None);
        let at = bounds.center() - point(glyph.width / 2., px(8. * s));
        glyph.paint(at, px(16. * s), window, cx).ok();
    }

    fn paint_chip(&self, row: &Row, window: &mut Window, cx: &mut App) {
        let t = self.theme;
        let s = t.size / crate::logic::prefs::Prefs::SIZE;
        let bounds = self.chip_bounds(row);
        window.paint_quad(fill(bounds, t.gutter).corner_radii(Corners::all(px(4. * s))));
        let label = self.chip_label(row);
        let run = TextRun {
            len: label.len(),
            font: font(sans()),
            color: t.soft,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let name =
            window
                .text_system()
                .shape_line(SharedString::from(label), px(11. * s), &[run], None);
        let at = bounds.origin + point(px(8. * s), px(2. * s));
        name.paint(at, px(16. * s), window, cx).ok();
    }

    pub(super) fn paint(&self, focused: bool, window: &mut Window, cx: &mut App) {
        let (t, o, w) = (self.theme, self.origin, self.width);
        let cursor = self.cursor();
        let sel = &self.sel;
        let label = |text: &str, color, window: &mut Window| {
            let run = TextRun {
                len: text.len(),
                font: font(sans()),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            window.text_system().shape_line(
                SharedString::from(text.to_string()),
                px(14.),
                &[run],
                None,
            )
        };
        for (i, row) in self.rows.iter().enumerate() {
            let top = o.y + row.y;
            if top + row.height < self.viewport.top() || top > self.viewport.bottom() {
                continue;
            }
            let text_top = top + row.pad;
            let end = row.start + row.len;
            let has_cursor = (row.start..=end).contains(&cursor);
            let clear = t.px(3.);
            let block = |x: Pixels, y: Pixels, w: Pixels, h: Pixels| {
                Bounds::new(point(o.x + x, y), size(w, h))
            };
            match row.kind {
                Kind::Code | Kind::Fence if row.gutter > px(0.) => {
                    // Quiet: a box with a 1 px line (the top and bottom only on its head and foot), and its line
                    // numbers in a gutter; the fences show their source on the live block alone.
                    let (y, h) = shape_span(row, top, clear);
                    let s = t.size / crate::logic::prefs::Prefs::SIZE;
                    let (radius, one) = (px(8. * s), px(1.));
                    let edge = |on: bool| if on { one } else { px(0.) };
                    let rim = gpui::Edges {
                        top: edge(row.head),
                        right: one,
                        bottom: edge(row.foot),
                        left: one,
                    };
                    window.paint_quad(quad(
                        block(px(0.), y, w, h),
                        corners(radius, row),
                        t.code_bg,
                        rim,
                        t.code_line,
                        BorderStyle::default(),
                    ));
                    let gutter = row.gutter;
                    let lip = |on: bool| if on { radius - one } else { px(0.) };
                    window.paint_quad(
                        fill(block(px(1.), y, gutter - one, h), t.gutter).corner_radii(Corners {
                            top_left: lip(row.head),
                            bottom_left: lip(row.foot),
                            top_right: px(0.),
                            bottom_right: px(0.),
                        }),
                    );
                    window.paint_quad(fill(block(gutter - one, y, one, h), t.code_line));
                    if row.number > 0 {
                        let digits = row.number.to_string();
                        let size = px(13. * s);
                        let run = TextRun {
                            len: digits.len(),
                            font: font(mono()),
                            color: t.gutter_num,
                            background_color: None,
                            underline: None,
                            strikethrough: None,
                        };
                        let num = window.text_system().shape_line(
                            SharedString::from(digits),
                            size,
                            &[run],
                            None,
                        );
                        let at = o.x + gutter - px(10. * s) - num.width;
                        num.paint(point(at, text_top), row.lh, window, cx).ok();
                    }
                }
                // A diagram off the caret is its view alone: no band behind it.
                Kind::Code | Kind::Fence if !row.fold.whole() => {
                    let (y, h) = shape_span(row, top, clear);
                    window.paint_quad(
                        fill(block(px(0.), y, w, h), t.code_bg)
                            .corner_radii(corners(t.band(), row)),
                    )
                }
                Kind::Quote => {
                    // A callout is its tinted shape alone, lined up with the code blocks; a plain quote keeps a thin neutral bar.
                    if let Some(tint) = row.tint {
                        // 7 % of the tint, a 1 px line of 22 %, and the alert's icon on its first line (not live).
                        let s = t.size / crate::logic::prefs::Prefs::SIZE;
                        let (y, h) = callout_span(row, top, CALLOUT_ROOM * s);
                        let one = px(1.);
                        let rim = gpui::Edges {
                            top: if row.head { one } else { px(0.) },
                            right: one,
                            bottom: if row.foot { one } else { px(0.) },
                            left: one,
                        };
                        window.paint_quad(quad(
                            block(px(0.), y, w, h),
                            corners(t.block_radius(), row),
                            tint.opacity(0.07),
                            rim,
                            // 22 % of the tint over the page, so the rim is not drawn darker over the fill.
                            t.bg.blend(tint.opacity(0.22)),
                            BorderStyle::default(),
                        ));
                        // A quote inside the callout: its own bars, from the second level on.
                        for level in 1..row.bars() {
                            window.paint_quad(
                                fill(
                                    block(px(-2. + 10. * level as f32), y, px(3.), h),
                                    t.quote_bar,
                                )
                                .corner_radii(corners(px(1.5), row)),
                            );
                        }
                        if row.head && !row.live {
                            let icon = px(16. * s);
                            let at = Bounds::new(
                                point(o.x + px(14. * s), text_top + (row.lh - icon) / 2.),
                                size(icon, icon),
                            );
                            window
                                .paint_svg(
                                    at,
                                    "info.svg".into(),
                                    TransformationMatrix::unit(),
                                    tint,
                                    cx,
                                )
                                .ok();
                        }
                    } else {
                        // One bar per level of the quote, 10 px apart: the outer one at -14 px.
                        for level in 0..row.bars() {
                            window.paint_quad(
                                fill(
                                    block(px(-14. + 10. * level as f32), top, px(3.), row.height),
                                    t.quote_bar,
                                )
                                .corner_radii(corners(px(1.5), row)),
                            );
                        }
                    }
                }
                Kind::Rule if !row.live => {
                    let mid = top + row.height / 2.;
                    match t.dots {
                        // Quiet: dots of 1.6 px on a 6 px pitch, centred across the width (at size 15).
                        Some(dots) => {
                            let scale = t.size / crate::logic::prefs::Prefs::SIZE;
                            let (d, pitch) = (px(1.6 * scale), px(6. * scale));
                            let count = ((w - d) / pitch).floor() as usize + 1;
                            let start = (w - (pitch * (count - 1) as f32 + d)) / 2.;
                            for i in 0..count {
                                let x = start + pitch * i as f32;
                                window.paint_quad(
                                    fill(block(x, mid - d / 2., d, d), dots).corner_radii(d / 2.),
                                );
                            }
                        }
                        None => window.paint_quad(fill(block(px(0.), mid, w, px(1.)), t.border)),
                    }
                    continue;
                }
                _ => {}
            }
            // A table wider than the page is cut at its edges.
            let frame = (row.kind == Kind::Table)
                .then(|| {
                    self.tables
                        .iter()
                        .find(|(range, _)| range.contains(&row.start))
                        .map(|(_, frame)| *frame)
                })
                .flatten();
            let (left, wide) = frame.map_or((o.x, w), |f| (f.left(), f.size.width));
            let mask = (row.kind == Kind::Table).then(|| ContentMask {
                bounds: Bounds::new(point(left - px(2.), top), size(wide + px(4.), row.height)),
            });
            if row.hidden() {
                continue;
            }
            // A pending change: its lines lie on a quiet wash, green for an addition, red for a removal.
            let wash = match row.pend {
                Pend::Added => Some(t.added),
                Pend::Removed => Some(t.removed),
                Pend::Plain | Pend::Mark => None,
            };
            if let Some(wash) = wash {
                window.paint_quad(fill(block(px(-12.), top, w + px(24.), row.height), wash));
            }
            window.with_content_mask(mask, |window| {
                // In a table, the header background goes under the selection.
                let table_top = point(o.x - row.dx, top);
                // In a pending change the header keeps the wash, not its own band.
                if let (Some(grid), None) = (&row.grid, wash) {
                    // The table ends here when nothing but dash lines follows.
                    let ends = self.rows[i + 1..]
                        .iter()
                        .take_while(|r| r.kind == Kind::Table)
                        .all(Row::hidden);
                    grid.paint_back(
                        Bounds::new(point(left, top), size(wide, row.height - row.gap)),
                        t.block_radius(),
                        ends,
                        &t,
                        window,
                    );
                }
                // Background of a passage of text on this line: the selection, the found passages.
                // `edge`: a 1 px outline around it, for the current found passage.
                let mark_edged =
                    |range: &Range<usize>, color: Hsla, edge: Option<Hsla>, window: &mut Window| {
                        let a = row.pos(range.start.max(row.start) - row.start);
                        let b = row.pos(range.end.min(end) - row.start);
                        let tail = if range.end > end { px(6.) } else { px(0.) };
                        // Rounded at the ends of the passage only: the lines of a passage over several lines join.
                        let r = t.small();
                        let (none, all) = (px(0.), Corners::all(r));
                        let (first, last) = (
                            Corners {
                                bottom_left: none,
                                bottom_right: none,
                                ..all
                            },
                            Corners {
                                top_left: none,
                                top_right: none,
                                ..all
                            },
                        );
                        let mut rect =
                            |x: Pixels, y: Pixels, w: Pixels, h: Pixels, round: Corners<Pixels>| {
                                match edge {
                                    Some(edge) => window.paint_quad(quad(
                                        block(x, text_top + y, w, h),
                                        round,
                                        color,
                                        px(1.),
                                        edge,
                                        BorderStyle::default(),
                                    )),
                                    None => window.paint_quad(
                                        fill(block(x, text_top + y, w, h), color)
                                            .corner_radii(round),
                                    ),
                                }
                            };
                        if let Some(grid) = &row.grid {
                            for (x, y, w) in grid.spans(
                                range.start.max(row.start) - row.start,
                                range.end.min(end) - row.start,
                                row.lh,
                            ) {
                                rect(x - row.dx, y, w, row.lh, all);
                            }
                        } else if a.y == b.y {
                            rect(a.x, a.y, b.x - a.x + tail, row.lh, all);
                        } else {
                            rect(a.x, a.y, w - a.x, row.lh, first);
                            rect(
                                px(0.),
                                a.y + row.lh,
                                w,
                                b.y - a.y - row.lh,
                                Corners::default(),
                            );
                            rect(px(0.), b.y, b.x + tail, row.lh, last);
                        }
                    };
                let mark = |range: &Range<usize>, color: Hsla, window: &mut Window| {
                    mark_edged(range, color, None, window)
                };
                // The pieces of a passage on this line (`r`, bytes of the line): one per line of a wrap, or one per cell.
                let spans_of = |r: Range<usize>| -> Vec<(Pixels, Pixels, Pixels)> {
                    // At a wrap, the offset of a line's first byte reads as the end of the line before: take the line
                    // of the byte next to it.
                    let (a, inner) = (row.pos(r.start), row.pos(r.start + 1));
                    let a = if a.y == inner.y {
                        a
                    } else {
                        point(px(0.), inner.y)
                    };
                    let (b, last) = (row.pos(r.end), row.pos(r.end - 1));
                    // The passage ends the line, the offset after it reads as the next line's: it ends one letter after the last.
                    let b = if b.y == last.y {
                        b
                    } else {
                        let before = row.pos(r.end.saturating_sub(2));
                        point(
                            last.x
                                + if before.y == last.y && last.x > before.x {
                                    last.x - before.x
                                } else {
                                    t.px(8.)
                                },
                            last.y,
                        )
                    };
                    if let Some(grid) = &row.grid {
                        grid.spans(r.start, r.end, row.lh)
                            .into_iter()
                            .map(|(x, y, w)| (x - row.dx, y, w))
                            .collect()
                    } else if a.y == b.y {
                        vec![(a.x, a.y, b.x - a.x)]
                    } else {
                        vec![(a.x, a.y, w - a.x), (px(0.), b.y, b.x)]
                    }
                };
                // Tags are pills: the accent at 10 % behind the text, a little wider and shorter than the line, so
                // that the text and the line's height stay where they are. A tag cut by a wrap is one pill per line.
                if !matches!(row.kind, Kind::Code | Kind::Fence | Kind::Meta | Kind::Rule) {
                    let (pad, tall, lift) = (t.px(3.), row.lh * 0.78, row.lh * 0.11);
                    for r in md::tags(&self.content[row.start..end], row.kind == Kind::Table) {
                        for (x, y, width) in spans_of(r) {
                            window.paint_quad(
                                fill(
                                    block(x - pad, text_top + y + lift, width + pad * 2., tall),
                                    crate::ui::widgets::pill_tint(t),
                                )
                                .corner_radii(t.small()),
                            );
                        }
                    }
                    // Quiet: inline code is a chip, 4 px out on each side of the code (the text keeps its place), with a 1 px rim.
                    if let Some(chip) = t.chip {
                        let scale = t.size / crate::logic::prefs::Prefs::SIZE;
                        let (pad, tall, lift) = (px(4. * scale), row.lh * 0.82, row.lh * 0.09);
                        let flags = md::flags(&self.content[row.start..end], 0);
                        let mut i = 0;
                        while i < flags.len() {
                            if flags[i] & md::CODE == 0 {
                                i += 1;
                                continue;
                            }
                            let start = i;
                            while i < flags.len() && flags[i] & md::CODE != 0 {
                                i += 1;
                            }
                            for (x, y, width) in spans_of(start..i) {
                                window.paint_quad(quad(
                                    block(x - pad, text_top + y + lift, width + pad * 2., tall),
                                    px(4. * scale),
                                    chip,
                                    px(scale),
                                    t.chip_line,
                                    BorderStyle::default(),
                                ));
                            }
                        }
                    }
                }
                // Highlighted text (`==text==`), under the selection; matches and comments go over it.
                if !matches!(row.kind, Kind::Code | Kind::Fence | Kind::Meta | Kind::Rule) {
                    for (r, color) in
                        md::highlights(&self.content[row.start..end], row.kind == Kind::Table)
                    {
                        mark(
                            &(row.start + r.start..row.start + r.end),
                            t.highlight(color),
                            window,
                        );
                    }
                }
                if !sel.is_empty() && sel.start <= end && sel.end > row.start {
                    mark(sel, t.selection, window);
                }
                for (other, _) in self.more.iter().filter(|(other, _)| {
                    !other.is_empty() && other.start <= end && other.end > row.start
                }) {
                    mark(other, t.selection, window);
                }
                // Text that carries a comment (v2 `.cm`): a yellow wash, a 1.5 px gold underline under each piece of
                // it on the line, and a 6 px gold dot in the right margin of the line.
                if !matches!(row.kind, Kind::Code | Kind::Fence) {
                    for (_, noted, _) in md::comments(&self.content[row.start..end]) {
                        mark(
                            &(row.start + noted.start..row.start + noted.end),
                            crate::ui::widgets::passage_tint(t),
                            window,
                        );
                        // A comment on its own has no passage to underline.
                        if !noted.is_empty() {
                            for (x, y, width) in spans_of(noted.clone()) {
                                window.paint_quad(fill(
                                    block(x, text_top + y + row.lh * 0.88, width, px(1.5)),
                                    t.comment_gold,
                                ));
                            }
                        }
                        window.paint_quad(
                            fill(
                                block(
                                    w + px(12.),
                                    text_top + row.lh * 0.5 - px(3.),
                                    px(6.),
                                    px(6.),
                                ),
                                t.comment_gold,
                            )
                            .corner_radii(px(3.)),
                        );
                    }
                }
                if let Some(find) = &self.find {
                    let from = find.hits.partition_point(|hit| hit.end <= row.start);
                    for (i, hit) in find
                        .hits
                        .iter()
                        .enumerate()
                        .skip(from)
                        .take_while(|(_, hit)| hit.start < end)
                    {
                        // The current one is outlined rather than darker: the text over it must read.
                        mark_edged(hit, t.selection, (i == find.at).then_some(t.accent), window);
                    }
                }
                if let Some(grid) = &row.grid {
                    // Scrolled sideways, any column can be under the word-wrap switch: the header's
                    // text stops short of it.
                    let under_switch =
                        (grid.is_header() && grid.total() > wide + px(0.5)).then(|| ContentMask {
                            bounds: Bounds::new(
                                point(left, top),
                                size(wide - COPY_SIZE - px(8.), row.height),
                            ),
                        });
                    window.with_content_mask(under_switch, |window| {
                        grid.paint_front(table_top, row.height - row.gap, row.lh, &t, window, cx);
                    });
                } else {
                    // The lead is drawn even on an empty line: a list item's bullet or box is there.
                    let lead = row.shaped.lead.as_ref();
                    if let Some(lead) = lead {
                        let left = o.x - row.dx;
                        if let Some(glyph) = &lead.line {
                            glyph
                                .paint(
                                    point(left + lead.dx, text_top + lead.dy),
                                    row.lh,
                                    TextAlign::Left,
                                    None,
                                    window,
                                    cx,
                                )
                                .ok();
                        }
                        if let Some((shape, x)) = &lead.bullet {
                            // A bullet, centered like a box on the first line: a disc, a ring
                            // (a 1px stroke) or a square, in the text colour.
                            let side = x.end - x.start;
                            let mid = text_top + row.lh * 0.5 + px(t.size * 0.1);
                            let bounds = Bounds::new(
                                point(left + x.start, mid - side / 2.),
                                size(side, side),
                            );
                            let (radius, fill, edge) = match shape {
                                Bullet::Disc => (side / 2., t.text, px(0.)),
                                Bullet::Ring => (side / 2., Hsla::transparent_black(), px(1.)),
                                // The square has a 1 px radius at size 15.
                                Bullet::Square => (
                                    px(t.size / crate::logic::prefs::Prefs::SIZE),
                                    t.text,
                                    px(0.),
                                ),
                            };
                            window.paint_quad(quad(
                                bounds,
                                radius,
                                fill,
                                edge,
                                t.text,
                                BorderStyle::default(),
                            ));
                        }
                        if let Some(check) = &lead.check {
                            let side = check.x.end - check.x.start;
                            // Centred on the x-height of the first line: the middle of the lowercase letters is about
                            // a tenth of an em below the middle of the line, in Quiet as elsewhere.
                            let mid = text_top + row.lh * 0.5 + px(t.size * 0.1);
                            // The box sits on whole pixels, so that its 1 px border is one line, not two.
                            let origin = point(left + check.x.start, mid - side / 2.);
                            let origin = point(
                                px(f32::from(origin.x).round()),
                                px(f32::from(origin.y).round()),
                            );
                            let bounds = Bounds::new(origin, size(side, side));
                            let scale = t.size / crate::logic::prefs::Prefs::SIZE;
                            let (radius, edge) = (px(5. * scale), px(scale));
                            if check.done {
                                window.paint_quad(quad(
                                    bounds,
                                    radius,
                                    t.accent,
                                    edge,
                                    t.accent,
                                    BorderStyle::default(),
                                ));
                                // A check mark in two strokes, centered in the box; Quiet's span about 13/17 of it.
                                let at = |x: f32, y: f32| {
                                    point(bounds.origin.x + side * x, bounds.origin.y + side * y)
                                };
                                let tick = [(0.13, 0.53), (0.40, 0.79), (0.89, 0.21)];
                                let mut mark = gpui::PathBuilder::stroke(edge * 1.3);
                                mark.move_to(at(tick[0].0, tick[0].1));
                                mark.line_to(at(tick[1].0, tick[1].1));
                                mark.line_to(at(tick[2].0, tick[2].1));
                                if let Ok(path) = mark.build() {
                                    window.paint_path(path, gpui::white());
                                }
                            } else {
                                // The box is filled with the page.
                                let fill = t.bg;
                                window.paint_quad(quad(
                                    bounds,
                                    radius,
                                    fill,
                                    edge,
                                    t.todo_box,
                                    BorderStyle::default(),
                                ));
                            }
                        }
                    }
                    if let Some(line) = row.shaped.line.as_ref().filter(|_| !row.src_hidden()) {
                        let at = lead.map_or(px(0.), |l| l.width);
                        // A code line scrolls under its gutter, which stays in view: its text is cut there.
                        let mask = (row.kind == Kind::Code).then(|| ContentMask {
                            bounds: Bounds::new(
                                point(o.x + row.gutter, top),
                                size(self.width - row.gutter, row.height),
                            ),
                        });
                        window.with_content_mask(mask, |window| {
                            line.paint(
                                point(o.x - row.dx + at, text_top),
                                row.lh,
                                TextAlign::Left,
                                None,
                                window,
                                cx,
                            )
                            .ok();
                        });
                    }
                }
                // Off the caret, a code block's language shows on a chip at its top right.
                if row.opens && row.fold.edges() {
                    self.paint_chip(row, window, cx);
                }
                if focused && sel.is_empty() && has_cursor {
                    let p = row.pos(cursor - row.start);
                    let caret = block(p.x, text_top + p.y + row.lh * 0.14, px(2.), row.lh * 0.72);
                    window.paint_quad(fill(caret, t.accent));
                }
                // The other cursors, when the note has input.
                for (other, back) in self.more.iter().filter(|_| focused && row.grid.is_none()) {
                    let at = if *back { other.start } else { other.end };
                    if other.is_empty() && (row.start..=end).contains(&at) {
                        let p = row.pos(at - row.start);
                        let caret =
                            block(p.x, text_top + p.y + row.lh * 0.14, px(2.), row.lh * 0.72);
                        window.paint_quad(fill(caret, t.accent));
                    }
                }
            });
            // Inline formulas: each figure over the gap its source left in the line, centred on the math axis.
            for inline in &row.inline {
                let p = row.pos(inline.at);
                let top = text_top + p.y + row.lh / 2. + px(0.2 * t.size) - inline.size.height / 2.;
                let at = block(
                    p.x - inline.room,
                    top,
                    inline.size.width,
                    inline.size.height,
                );
                window
                    .paint_image(at, Corners::default(), inline.image.clone(), 0, false)
                    .ok();
            }
            // Footnote references off the caret: their numbers, raised over the gap their source left.
            for foot in &row.shaped.foots {
                let p = row.pos(foot.at);
                let run = TextRun {
                    len: foot.number.len(),
                    font: font(sans()),
                    color: t.accent,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let number = window.text_system().shape_line(
                    SharedString::from(foot.number.clone()),
                    foot.size,
                    &[run],
                    None,
                );
                // The body's baseline: `paint` centres the text's ascent and descent in the line height.
                let baseline = match row.shaped.line.as_ref().map(|l| &l.unwrapped_layout) {
                    Some(body) => {
                        text_top + p.y + (row.lh - body.ascent - body.descent) / 2. + body.ascent
                    }
                    None => text_top + p.y + row.lh * 0.8,
                };
                let at = point(o.x + p.x, baseline - foot.raise - number.ascent);
                number
                    .paint(at, number.ascent + number.descent, window, cx)
                    .ok();
            }
            if let Some((image, s)) = &row.image {
                let at = block(px(0.), o.y + row.image_top(), s.width, s.height);
                // Radius 8 and a 1 px ring over the edge (both at size 15).
                let radius = px(8. * t.size / crate::logic::prefs::Prefs::SIZE);
                window
                    .paint_image(at, Corners::all(radius), image.clone(), 0, false)
                    .ok();
                // A picture keeps its ring; a diagram or a formula has none.
                if !row.drawn {
                    window.paint_quad(quad(
                        at,
                        radius,
                        Hsla::transparent_black(),
                        px(1.),
                        t.ring,
                        BorderStyle::default(),
                    ));
                }
            } else if let Some(frame) = &row.frame {
                let at = block(
                    px(0.),
                    o.y + row.image_top(),
                    frame.size.width,
                    frame.size.height,
                );
                window.paint_quad(
                    fill(at, t.panel)
                        .corner_radii(px(8.))
                        .border_widths(px(1.))
                        .border_color(t.border),
                );
                label(&frame_text(frame), t.soft, window)
                    .paint(
                        point(o.x + px(10.), o.y + row.image_top() + px(8.)),
                        BROKEN,
                        window,
                        cx,
                    )
                    .ok();
            } else if let Some(said) = row.broken {
                label(said, t.soft, window)
                    .paint(point(o.x, o.y + row.image_top()), BROKEN, window, cx)
                    .ok();
            }
            // "Enlarge" button, on the picture, formula or figure the pointer is over.
            if self.big_hover == Some(row.start)
                && let Some(bounds) = self.big_bounds(row)
            {
                let over = bounds.contains(&self.pointer);
                window.paint_quad(
                    fill(bounds, if over { t.border } else { t.panel })
                        .corner_radii(px(6.))
                        .border_widths(px(1.))
                        .border_color(t.border),
                );
                let inset = (COPY_SIZE - px(16.)) / 2.;
                let at = Bounds::new(bounds.origin + point(inset, inset), size(px(16.), px(16.)));
                window
                    .paint_svg(
                        at,
                        "c-zoom-in.svg".into(),
                        TransformationMatrix::unit(),
                        t.text,
                        cx,
                    )
                    .ok();
                if let Some((_, hitbox)) = self.big_hitboxes.iter().find(|(s, _)| *s == row.start) {
                    window.set_cursor_style(CursorStyle::PointingHand, hitbox);
                }
            }
            // Copy button of the block: the same icon as for copying the note.
            if row.opens {
                let bounds = self.copy_bounds(row);
                // The buttons show while the pointer is over the block (v2 `.code:hover .copy`), or
                // while the copied check mark is shown.
                let shown =
                    self.copied == Some(row.start) || self.code_at(self.pointer) == Some(row.start);
                if self.hover == Some(row.start) {
                    window.paint_quad(fill(bounds, t.border).corner_radii(px(6.)));
                }
                let icon = if self.copied == Some(row.start) {
                    "check.svg"
                } else {
                    "copy.svg"
                };
                let inset = (COPY_SIZE - px(16.)) / 2.;
                let at = Bounds::new(bounds.origin + point(inset, inset), size(px(16.), px(16.)));
                if shown {
                    window
                        .paint_svg(at, icon.into(), TransformationMatrix::unit(), t.text, cx)
                        .ok();
                }
                if let Some((_, hitbox)) = self.copy_hitboxes.iter().find(|(s, _)| *s == row.start)
                {
                    window.set_cursor_style(CursorStyle::PointingHand, hitbox);
                }
                if shown && self.views(row) {
                    let bounds = self.view_bounds(row);
                    if self.view_hover == Some(row.start) {
                        window.paint_quad(fill(bounds, t.border).corner_radii(px(6.)));
                    }
                    let at =
                        Bounds::new(bounds.origin + point(inset, inset), size(px(16.), px(16.)));
                    window
                        .paint_svg(
                            at,
                            "c-zoom-in.svg".into(),
                            TransformationMatrix::unit(),
                            t.text,
                            cx,
                        )
                        .ok();
                    if let Some((_, hitbox)) =
                        self.view_hitboxes.iter().find(|(s, _)| *s == row.start)
                    {
                        window.set_cursor_style(CursorStyle::PointingHand, hitbox);
                    }
                }
            }
        }
        if self.content.is_empty() {
            // The rest of the empty page (date, keys, notes to reread) is in `Shell::render_empty`.
            label(tr("Write here…", "Écrivez ici…"), t.soft, window)
                .paint(point(o.x + px(8.), o.y + px(3.)), px(22.), window, cx)
                .ok();
        }
        // A thin scrollbar under each table too wide for the page; stronger under the pointer.
        for bar in &self.thumbs {
            // Stronger while its thumb is dragged, and under the pointer.
            let dragged = self.bar_drag.is_some_and(|(start, _)| start == bar.start);
            let alpha = if dragged {
                0.9
            } else if bar.frame.contains(&self.pointer) {
                0.7
            } else {
                0.4
            };
            window.paint_quad(fill(bar.thumb, t.dim.opacity(alpha)).corner_radii(px(3.)));
        }
        // Word-wrap switches: of each fenced code block (off by default), of each table (on by default).
        for row in self.rows.iter().filter(|r| self.wraps_code(r)) {
            let bounds = self.view_bounds(row);
            if bounds.bottom() < self.viewport.top() || bounds.top() > self.viewport.bottom() {
                continue;
            }
            let on = self.wraps.get(&row.start).copied().unwrap_or(false);
            self.paint_wrap_chip(bounds, on, window, cx);
        }
        for (range, frame) in &self.tables {
            let bounds = Self::table_wrap_bounds(frame);
            if bounds.bottom() < self.viewport.top() || bounds.top() > self.viewport.bottom() {
                continue;
            }
            let on = self.wraps.get(&range.start).copied().unwrap_or(true);
            self.paint_wrap_chip(bounds, on, window, cx);
        }
        // The line where files dragged onto the note would land.
        // The edge of each table: a hairline in the header's tint, rounded like it, over the lines.
        for (meta, (_, frame)) in self.table_meta.iter().zip(&self.tables) {
            let (Some(first), Some(last)) = (
                self.rows.get(meta.rows.start),
                self.rows.get(meta.rows.end - 1),
            ) else {
                continue;
            };
            let (top, bottom) = (o.y + first.y, o.y + last.y + last.height - last.gap);
            if first.grid.is_none() || bottom < self.viewport.top() || top > self.viewport.bottom()
            {
                continue;
            }
            let area = Bounds::new(
                point(frame.left(), top),
                size(frame.size.width, bottom - top),
            );
            window.paint_quad(quad(
                area,
                t.block_radius(),
                Hsla::transparent_black(),
                px(1.),
                t.table_line,
                BorderStyle::default(),
            ));
        }
        if let Some(mark) = self.drop_mark
            && let Some(last) = self.rows.last()
        {
            let y = match self.row_at(mark).filter(|_| mark < self.content.len()) {
                Some(r) => self.rows[r].y,
                None => last.y + last.height,
            };
            let line = Bounds::new(
                point(o.x - px(6.), o.y + y - px(1.)),
                size(w + px(12.), px(2.)),
            );
            window.paint_quad(fill(line, t.accent).corner_radii(px(1.)));
        }
        for (n, (range, frame)) in self.tables.iter().enumerate() {
            let hover = self.table_hover.filter(|(at, _)| *at == range.start);
            if hover.is_none() && !(focused && (range.start..=range.end).contains(&cursor)) {
                continue;
            }
            for (i, bar) in plus_bars(*frame).into_iter().enumerate() {
                let hot = hover.is_some_and(|(_, over)| over == Some(i));
                window.paint_quad(
                    fill(bar, if hot { t.selection } else { t.code_bg }).corner_radii(px(4.)),
                );
                let icon =
                    Bounds::new(bar.center() - point(px(5.), px(5.)), size(px(10.), px(10.)));
                let color = if hot { t.accent } else { t.soft };
                window
                    .paint_svg(
                        icon,
                        "plus.svg".into(),
                        TransformationMatrix::unit(),
                        color,
                        cx,
                    )
                    .ok();
                if let Some(hitbox) = self.table_hitboxes.get(n * 2 + i) {
                    window.set_cursor_style(CursorStyle::PointingHand, hitbox);
                }
            }
        }
        // Table handles: a three-dot badge on the column and row the pointer
        // hovers (and on what was grabbed), what they grabbed, and the line where the dragged thing drops.
        for geo in &self.geos {
            let hot = self
                .part_hover
                .filter(|(at, _)| *at == geo.start)
                .map(|(_, part)| part);
            let (near, pick) = (geo.near(self.pointer), self.live_pick(geo.start));
            if let Some(pick) = pick {
                let area = geo.cells(&pick.rows, &pick.cols).intersect(&geo.frame);
                window.paint_quad(quad(
                    area,
                    px(2.),
                    t.selection,
                    px(1.),
                    t.accent.opacity(0.7),
                    BorderStyle::default(),
                ));
            }
            // What carries a comment: a cell is tinted and marked in its corner, a row, a column or the
            // table is outlined; the comment the cursor is in is drawn stronger.
            for target in &geo.notes.found {
                let active = geo.notes.active == Some(*target);
                let (rows, cols) = (0..geo.tall(), 0..geo.cols());
                let area = match *target {
                    Target::Row(v) => geo.cells(&(v..v + 1), &cols),
                    Target::Col(c) => geo.cells(&rows, &(c..c + 1)),
                    Target::Table => geo.frame,
                    Target::Cell(v, c) => geo.cells(&(v..v + 1), &(c..c + 1)),
                }
                .intersect(&geo.frame);
                // Scrolled out of the frame: nothing to draw.
                if area.size.width < px(14.) || area.size.height < px(8.) {
                    continue;
                }
                if let Target::Cell(..) = target {
                    window.paint_quad(fill(
                        area,
                        t.accent.opacity(if active { 0.3 } else { 0.14 }),
                    ));
                    let dot = Bounds::new(
                        point(area.right() - px(9.), area.top() + px(3.)),
                        size(px(6.), px(6.)),
                    );
                    window.paint_quad(fill(dot, t.accent).corner_radii(px(3.)));
                } else {
                    let (back, width) = if active {
                        (t.accent.opacity(0.1), px(2.))
                    } else {
                        (t.accent.opacity(0.), px(1.))
                    };
                    window.paint_quad(quad(
                        area,
                        px(2.),
                        back,
                        width,
                        t.accent.opacity(if active { 0.9 } else { 0.55 }),
                        BorderStyle::default(),
                    ));
                }
            }
            let held = |part: Part| {
                pick.is_some_and(|p| {
                    let (all_rows, all_cols) =
                        (p.rows.len() == geo.tall(), p.cols.len() == geo.cols());
                    match part {
                        Part::Col(c) => all_rows && p.cols.contains(&c),
                        Part::Row(r) => all_cols && p.rows.contains(&r),
                        Part::Corner => all_rows && all_cols,
                        _ => false,
                    }
                })
            };
            for (part, zone) in geo.parts() {
                if !geo.visible(part, near, pick) {
                    continue;
                }
                if matches!(part, Part::JoinCol(_) | Part::JoinRow(_)) {
                    if hot == Some(part) {
                        let dot = Bounds::new(
                            zone.center() - point(px(7.), px(7.)),
                            size(px(14.), px(14.)),
                        );
                        window.paint_quad(fill(dot, t.accent).corner_radii(px(7.)));
                        let icon = Bounds::new(
                            zone.center() - point(px(5.), px(5.)),
                            size(px(10.), px(10.)),
                        );
                        window
                            .paint_svg(
                                icon,
                                "plus.svg".into(),
                                TransformationMatrix::unit(),
                                t.bg,
                                cx,
                            )
                            .ok();
                    }
                    continue;
                }
                // At rest: a light background and a thin rule; under the pointer: translucent accent; grabbed: the accent.
                let (back, edge, dots) = if held(part) {
                    (t.accent, t.accent, t.bg)
                } else if hot == Some(part) {
                    (t.selection, t.accent.opacity(0.6), t.accent)
                } else {
                    (t.bg, t.border, t.soft)
                };
                window.paint_quad(quad(
                    zone,
                    zone.size.width.min(zone.size.height) / 2.,
                    back,
                    px(1.),
                    edge,
                    BorderStyle::default(),
                ));
                // Three dots along the handle; the corner has none.
                if part != Part::Corner {
                    let lengthwise = zone.size.width > zone.size.height;
                    for k in [-1., 0., 1.] {
                        let step = px(4.) * k;
                        let center = if lengthwise {
                            zone.center() + point(step, px(0.))
                        } else {
                            zone.center() + point(px(0.), step)
                        };
                        let dot = Bounds::new(center - point(px(1.), px(1.)), size(px(2.), px(2.)));
                        window.paint_quad(fill(dot, dots).corner_radii(px(1.)));
                    }
                }
            }
            // A mark on the handle of what carries a comment, shown or not.
            for target in &geo.notes.found {
                let part = match *target {
                    Target::Row(v) => Part::Row(v),
                    Target::Col(c) => Part::Col(c),
                    Target::Table => Part::Corner,
                    Target::Cell(..) => continue,
                };
                if let Some((_, zone)) = geo.parts().into_iter().find(|(p, _)| *p == part) {
                    let dot = Bounds::new(
                        point(zone.right() - px(5.), zone.top() - px(3.)),
                        size(px(8.), px(8.)),
                    );
                    window.paint_quad(quad(
                        dot,
                        px(4.),
                        t.accent,
                        px(1.),
                        t.bg,
                        BorderStyle::default(),
                    ));
                }
            }
            if let Some(Moving {
                table,
                part,
                to: Some(b),
                ..
            }) = &self.moving
                && *table == geo.start
            {
                let f = geo.frame;
                let line = match part {
                    Part::Col(_) => Bounds::new(
                        point(geo.xs[*b] - px(1.), f.top()),
                        size(px(2.), f.size.height),
                    ),
                    _ => Bounds::new(
                        point(f.left(), geo.ys[*b] - px(1.)),
                        size(f.size.width, px(2.)),
                    ),
                };
                window.paint_quad(fill(line, t.accent));
            }
        }
        // The block being dragged: tinted, and a line where it would land.
        if let Some(drag) = self.handles.drag.as_ref().filter(|drag| drag.moved)
            && let Some(y) = drag.to.and_then(|to| self.line_top(to))
        {
            let line = Bounds::new(
                point(self.origin.x - px(8.), y - px(1.)),
                size(self.width + px(16.), px(2.)),
            );
            window.paint_quad(fill(line, t.accent).corner_radii(px(1.)));
        }
        // The handle of the block under the pointer, and its "+".
        if let Some((grip, plus)) = self.handle_bounds().filter(|_| !self.readonly) {
            let on = |b: Bounds<Pixels>| {
                b.contains(&self.pointer) || (self.handles.drag.is_some() && b == grip)
            };
            for (spot, icon) in [(Some(grip), "grip.svg"), (plus, "plus.svg")] {
                let Some(spot) = spot else { continue };
                if on(spot) {
                    window.paint_quad(fill(spot, t.border).corner_radii(px(4.)));
                }
                let mark = Bounds::new(
                    spot.center() - point(px(7.), px(7.)),
                    size(px(14.), px(14.)),
                );
                window
                    .paint_svg(
                        mark,
                        icon.into(),
                        TransformationMatrix::unit(),
                        if on(spot) { t.text } else { t.soft },
                        cx,
                    )
                    .ok();
            }
            if let Some((grip, plus)) = &self.handles.hitboxes {
                window.set_cursor_style(
                    if self.handles.drag.is_some() {
                        CursorStyle::ClosedHand
                    } else {
                        CursorStyle::OpenHand
                    },
                    grip,
                );
                if let Some(plus) = plus {
                    window.set_cursor_style(CursorStyle::PointingHand, plus);
                }
            }
        }
        // Table picker: the grid, and the chosen size under it.
        if focused
            && let Some((cols, rows)) = self.grid
            && let Some(at) = self.popup_at(Self::grid_size())
        {
            let panel = Bounds::new(at, Self::grid_size());
            window.paint_quad(quad(
                panel,
                px(8.),
                t.panel,
                px(1.),
                t.border,
                BorderStyle::default(),
            ));
            for (col, row) in (0..GRID * GRID).map(|i| (i % GRID, i / GRID)) {
                let corner = at
                    + point(
                        px(8.) + GRID_STEP * col as f32,
                        px(8.) + GRID_STEP * row as f32,
                    );
                let on = col < cols && row < rows;
                let (inside, edge) = if on {
                    (t.selection, t.accent)
                } else {
                    (t.code_bg, t.border)
                };
                let cell = Bounds::new(corner, size(px(16.), px(16.)));
                window.paint_quad(quad(
                    cell,
                    px(3.),
                    inside,
                    px(1.),
                    edge,
                    BorderStyle::default(),
                ));
            }
            if let Some(hitbox) = &self.grid_hitbox {
                window.set_cursor_style(CursorStyle::PointingHand, hitbox);
            }
            let size = format!("{cols} × {rows}");
            label(&size, t.text, window)
                .paint(
                    point(at.x + px(8.), panel.bottom() - px(24.)),
                    px(20.),
                    window,
                    cx,
                )
                .ok();
            let hint = label(tr("or type 12x5", "ou tapez 12x5"), t.soft, window);
            hint.paint(
                point(
                    panel.right() - px(8.) - hint.width,
                    panel.bottom() - px(24.),
                ),
                px(20.),
                window,
                cx,
            )
            .ok();
        }
    }
}

/// The corners a line of a block rounds: the top ones if it starts the block, the bottom ones if it ends it.
fn corners(radius: Pixels, row: &Row) -> Corners<Pixels> {
    let (top, bottom) = (
        if row.head { radius } else { px(0.) },
        if row.foot { radius } else { px(0.) },
    );
    Corners {
        top_left: top,
        top_right: top,
        bottom_right: bottom,
        bottom_left: bottom,
    }
}

/// Where a line of a filled block is drawn, from the top of the line and for its height: the first and last
/// lines stay `clear` of their edge, so that two blocks one after the other do not touch.
/// A callout's box: the block gap above its label stays outside it, so the room of the box (`room`) is
/// its padding; the box runs to the bottom of the last line's room.
fn callout_span(row: &Row, top: Pixels, room: Pixels) -> (Pixels, Pixels) {
    let gap = if row.head {
        (row.pad - room).max(px(0.))
    } else {
        px(0.)
    };
    (top + gap, row.height - gap)
}

fn shape_span(row: &Row, top: Pixels, clear: Pixels) -> (Pixels, Pixels) {
    let (above, below) = (
        if row.head { clear } else { px(0.) },
        if row.foot { clear } else { px(0.) },
    );
    (top + above, row.height - above - below)
}
