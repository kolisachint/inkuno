//! The nesting of list items, read in one pass: each laid out line keeps its list context (the
//! items above it that it can nest in, and whether the lines above it are indented code), so a
//! relayout reads each line once instead of scanning back to the start of its list.

use super::layout::Row;
use crate::logic::markdown::{self as md, Kind};
use std::ops::Range;
use std::rc::Rc;

/// An item on the stack of the items above a line: its indent in columns, its depth (how many
/// items it is nested in) and the item it is nested in.
struct Item {
    columns: usize,
    depth: usize,
    within: Option<Rc<Item>>,
}

/// What a line reads from the lines above it. Kept with each laid out row: a zone of the text
/// stops growing as soon as the nesting it leaves matches the one the old layout had there.
#[derive(Clone)]
pub(super) struct Nesting {
    /// The items the next line can nest in, innermost first: each one is less indented than the
    /// item it is nested in, and its depth is how many items it is nested in.
    items: Option<Rc<Item>>,
    /// The last non-blank line is indented code: the block goes on with indented lines.
    pub(super) indented: bool,
    /// The line above is blank, or there is none.
    pub(super) starts: bool,
    /// Depth of the block quote the last line is in (0: none). A block quote's content reads as its
    /// own line, so its items nest among the quote's lines only.
    pub(super) quote: usize,
    /// The last line is a quoted paragraph or item: a paragraph line after it is a lazy continuation.
    pub(super) open: bool,
}

impl Default for Nesting {
    /// The start of the text: no item, no code, and a blank line above.
    fn default() -> Self {
        Self {
            items: None,
            indented: false,
            starts: true,
            quote: 0,
            open: false,
        }
    }
}

/// The innermost item indented less than `columns`: the one a line that size is nested in.
fn enclosing(items: &Option<Rc<Item>>, columns: usize) -> Option<Rc<Item>> {
    let mut top = items.clone();
    while let Some(item) = top {
        if item.columns < columns {
            return Some(item);
        }
        top = item.within.clone();
    }
    None
}

impl Nesting {
    /// The nesting the rows before `index` leave for the line at `index`: none at the start.
    pub(super) fn before(rows: &[Row], index: usize) -> Nesting {
        index
            .checked_sub(1)
            .map_or_else(Nesting::default, |i| rows[i].nesting.clone())
    }

    /// The nesting all the rows leave for the line after them.
    pub(super) fn after(rows: &[Row]) -> Nesting {
        Nesting::before(rows, rows.len())
    }

    /// Nesting level of an item indented by `columns`: the number of items it is nested in.
    /// Equal to `md::list_depth` for the lines above.
    pub(super) fn depth_of(&self, columns: usize) -> usize {
        enclosing(&self.items, columns).map_or(0, |item| item.depth + 1)
    }

    /// The item a continuation line indented by `columns` belongs to: the nearest item above
    /// that is indented no more than it, and its depth.
    fn owner(&self, columns: usize) -> Option<usize> {
        let mut top = self.items.as_deref();
        while let Some(item) = top {
            if item.columns <= columns {
                return Some(item.depth);
            }
            top = item.within.as_deref();
        }
        None
    }

    /// Depth of the item a paragraph line continues, if it is one: the same as
    /// `md::continuation_depth` for the lines above.
    pub(super) fn continuation(&self, line: &str) -> Option<usize> {
        let columns = md::indent_columns(line);
        if columns < 2 || line.trim().is_empty() || md::classify(line, false).0 != Kind::Para {
            return None;
        }
        self.owner(columns)
    }

    /// Whether a line starts indented code, given the context: four columns of indent, and either
    /// the block goes on from indented code, or the line follows a blank line with no item above
    /// it that it could continue (as `md::starts_indented_code`).
    pub(super) fn starts_code(&self, line: &str) -> bool {
        md::indent_columns(line) >= 4
            && !line.trim().is_empty()
            && (self.indented || self.starts && self.items.is_none())
    }

    /// Advances over `line`, which is indented code when `code`.
    pub(super) fn advance(&mut self, line: &str, code: bool) {
        // A quote's content is read as the line it holds: its items are indented from its own start.
        let quoted = md::quote(line);
        let body = quoted.map_or(line, |q| &line[q.at..]);
        let blank = body.trim().is_empty();
        match md::classify(body, false).0 {
            Kind::Bullet | Kind::Ordered | Kind::Task(_) => {
                let columns = md::indent_columns(body);
                let within = enclosing(&self.items, columns);
                let depth = within.as_ref().map_or(0, |item| item.depth + 1);
                self.items = Some(Rc::new(Item {
                    columns,
                    depth,
                    within,
                }));
            }
            // An unindented line that is not an item ends the list.
            _ if !blank && md::indent_columns(body) == 0 => self.items = None,
            _ => {}
        }
        match quoted {
            Some(q) => {
                self.quote = q.depth;
                self.open = !blank && matches!(q.inner, Kind::Para | Kind::Bullet | Kind::Ordered | Kind::Task(_));
            }
            // A paragraph line after a quoted paragraph is its lazy continuation: it stays in the quote.
            None if !blank && self.open && md::classify(line, false).0 == Kind::Para => {}
            None => {
                self.quote = 0;
                self.open = false;
            }
        }
        if !blank {
            self.indented = code;
        }
        self.starts = blank;
    }

    /// Whether two nestings read the same for the lines below them.
    pub(super) fn same(&self, other: &Nesting) -> bool {
        self.indented == other.indented
            && self.starts == other.starts
            && self.quote == other.quote
            && self.open == other.open
            && same_items(&self.items, &other.items)
    }

    /// Depth of the innermost item at or above the line (`-1`: none), for the layout's signature.
    #[cfg(test)]
    pub(super) fn innermost(&self) -> i64 {
        self.items.as_ref().map_or(-1, |item| item.depth as i64)
    }
}

/// Grows `zone` (old rows) until the nesting it leaves is the one the old layout had at its end:
/// the rows after it are then reused as they are. `text` is the new text of the zone, and `line`
/// gives the new text of an old row.
pub(super) fn settle<'a>(
    rows: &[Row],
    zone: &mut Range<usize>,
    text: &str,
    line: impl Fn(usize) -> &'a str,
) {
    let mut nesting = Nesting::before(rows, zone.start);
    for l in text.split('\n') {
        nesting.advance(l, false);
    }
    while zone.end < rows.len() && !nesting.same(&rows[zone.end - 1].nesting) {
        nesting.advance(line(zone.end), false);
        zone.end += 1;
    }
}

/// Whether two stacks of items have the same indents: the same nesting, then.
fn same_items(a: &Option<Rc<Item>>, b: &Option<Rc<Item>>) -> bool {
    let (mut a, mut b) = (a.as_deref(), b.as_deref());
    loop {
        match (a, b) {
            (None, None) => return true,
            (Some(x), Some(y)) => {
                if std::ptr::eq(x, y) {
                    return true;
                }
                if x.columns != y.columns {
                    return false;
                }
                a = x.within.as_deref();
                b = y.within.as_deref();
            }
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;
    use crate::logic::prefs::Prefs;
    use gpui::WindowAppearance;
    use std::time::Instant;

    /// Lines from a small alphabet: items at several indents, paragraphs, blank and indented
    /// lines, a table line. Each line's context, read forward, matches what the old readers
    /// (`md::list_depth`, `md::continuation_depth`, `md::starts_indented_code`) read back from
    /// the lines above it.
    #[test]
    fn the_context_reads_what_the_pure_readers_read() {
        let pool = [
            "- a",
            "  - b",
            "    - c",
            "1. d",
            "  2) e",
            "- [ ] f",
            "  - [x] g",
            "h",
            "  i",
            "    j",
            "",
            "   ",
            "  - k",
            "      - l",
            "| t |",
            "    code",
            "- m",
            "  n",
        ];
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _ in 0..500 {
            let lines: Vec<&str> = (0..40).map(|_| pool[next(pool.len())]).collect();
            let mut nesting = Nesting::default();
            // The old layout's own state: whether the previous non-blank line was code, and
            // whether the previous line is blank (or there is none).
            let (mut indented, mut starts) = (false, true);
            for (i, &line) in lines.iter().enumerate() {
                let above = || lines[..i].iter().rev().copied();
                let blank = line.trim().is_empty();
                let columns = md::indent_columns(line);
                let old_code = !blank
                    && columns >= 4
                    && (indented || starts && md::starts_indented_code(line, above()));
                assert_eq!(
                    old_code,
                    nesting.starts_code(line) && !blank,
                    "code at {i}: {line:?}"
                );
                if matches!(
                    md::classify(line, false).0,
                    Kind::Bullet | Kind::Ordered | Kind::Task(_)
                ) {
                    assert_eq!(
                        md::list_depth(columns, above()),
                        nesting.depth_of(columns),
                        "depth at {i}: {line:?}"
                    );
                }
                assert_eq!(
                    md::continuation_depth(line, above()),
                    nesting.continuation(line),
                    "continuation at {i}: {line:?}"
                );
                nesting.advance(line, old_code);
                if !blank {
                    indented = old_code;
                }
                starts = blank;
            }
        }
    }

    /// Lines of a note, and the start of each.
    fn starts(text: &str) -> Vec<usize> {
        let mut out = vec![0];
        out.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        out
    }

    fn laid_out<'a>(
        cx: &'a mut gpui::TestAppContext,
        text: &str,
    ) -> (gpui::Entity<Editor>, &'a mut gpui::VisualTestContext) {
        cx.update(crate::ui::assets::init_fonts);
        let theme = Theme::of(&Prefs::parse(""), WindowAppearance::Light);
        let (editor, cx) = cx.add_window_view(|_, cx| Editor::new(theme, cx));
        editor.update(cx, |e, cx| e.load(text.to_string(), 0, cx));
        cx.run_until_parked();
        let _ = cx.update(|window, cx| window.draw(cx));
        cx.run_until_parked();
        (editor, cx)
    }

    /// Lays the note out again, after an edit.
    fn draw(cx: &mut gpui::VisualTestContext) {
        let _ = cx.update(|window, cx| window.draw(cx));
        cx.run_until_parked();
    }

    /// The bullet shape of the item starting at `start`.
    fn bullet(
        editor: &gpui::Entity<Editor>,
        cx: &mut gpui::VisualTestContext,
        start: usize,
    ) -> Bullet {
        editor.read_with(cx, |e, _| {
            let row = e.rows.iter().find(|r| r.start == start).unwrap();
            row.shaped.lead.as_ref().unwrap().bullet.as_ref().unwrap().0
        })
    }

    /// An item indented by two columns under the item above, after a line that is no item:
    /// the lines above decide its depth, here an unindented line that is not an item ends the list.
    #[gpui::test]
    fn an_item_nests_under_the_item_above_once_its_line_is_indented(cx: &mut gpui::TestAppContext) {
        let text = "- a\nb\n  - b2\n";
        let (editor, cx) = laid_out(cx, text);
        let item = starts(text)[2];
        assert_eq!(
            bullet(&editor, cx, item),
            Bullet::Disc,
            "the list ended at `b`"
        );
        // The cursor is on `b` (the line showing its source): the zone laid out again then holds no
        // item, which would re-lay the items below it. Two spaces before `b`: `- b2` nests in `a`.
        let b = starts(text)[1];
        editor.update(cx, |e, cx| e.load(text.to_string(), b, cx));
        draw(cx);
        editor.update(cx, |e, cx| e.edit(b..b, "  ", cx));
        draw(cx);
        let edited = editor.read_with(cx, |e, _| e.text().to_string());
        assert_eq!(bullet(&editor, cx, starts(&edited)[2]), Bullet::Ring);
    }

    /// A blank line that turns into text ends the list: the items below it go back to depth 0.
    #[gpui::test]
    fn text_on_a_blank_line_ends_the_list_below_it(cx: &mut gpui::TestAppContext) {
        let text = "- a\n\n  - b\n";
        let (editor, cx) = laid_out(cx, text);
        let item = starts(text)[2];
        assert_eq!(bullet(&editor, cx, item), Bullet::Ring, "b nests in a");
        let blank = starts(text)[1];
        editor.update(cx, |e, cx| e.load(text.to_string(), blank, cx));
        draw(cx);
        editor.update(cx, |e, cx| e.edit(blank..blank, "x", cx));
        draw(cx);
        let edited = editor.read_with(cx, |e, _| e.text().to_string());
        assert_eq!(edited, "- a\nx\n  - b\n");
        assert_eq!(bullet(&editor, cx, starts(&edited)[2]), Bullet::Disc);
    }

    /// An indented line after a paragraph is a lazy continuation, not code: typing at its end does
    /// not lay out the whole note again.
    #[gpui::test]
    fn a_lazy_paragraph_is_not_relaid_whole(cx: &mut gpui::TestAppContext) {
        let text = "Intro\n    more\n";
        let (editor, cx) = laid_out(cx, text);
        let (full, reused, _) = editor.read_with(cx, |e, _| e.layouts());
        let end = text.find("more").unwrap() + "more".len();
        editor.update(cx, |e, cx| e.edit(end..end, "x", cx));
        draw(cx);
        let (full2, reused2, _) = editor.read_with(cx, |e, _| e.layouts());
        assert_eq!(full2, full, "no full layout");
        assert_eq!(reused2, reused + 1, "an incremental one");
    }

    /// The checkbox of an ordered task flips from the keyboard, with the cursor on its text.
    #[gpui::test]
    fn an_ordered_task_toggles_from_the_keyboard(cx: &mut gpui::TestAppContext) {
        let text = "1. [ ] x\n";
        let (editor, cx) = laid_out(cx, text);
        editor.update(cx, |e, cx| {
            e.place_cursor(text.find('x').unwrap(), cx);
            e.toggle_task(cx);
        });
        draw(cx);
        let edited = editor.read_with(cx, |e, _| e.text().to_string());
        assert_eq!(edited, "1. [x] x\n");
        editor.update(cx, |e, cx| {
            e.toggle_task(cx);
        });
        let back = editor.read_with(cx, |e, _| e.text().to_string());
        assert_eq!(back, "1. [ ] x\n");
    }

    /// Random edits to a note of nested lists, blank lines, indented lines and paragraphs: each
    /// reused layout is compared with a full one (`Editor::layout` does it under test).
    #[gpui::test]
    fn random_list_edits_match_a_full_layout(cx: &mut gpui::TestAppContext) {
        let mut text = String::from(
            "# Outline\n\n- a\n  - a1\n    - a11\n  - a2\n\nIntro\n    lazy\n\n1. one\n   2. two\n- [ ] task\n  - [x] done\n\n    code\n- z\n",
        );
        let (editor, cx) = laid_out(cx, &text);
        let mut seed = 0x9E37_79B9_7F4A_7C15u64;
        let mut next = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        let pieces = ["  ", "- ", "x", "\n", " ", "1. ", "\n\n", "    ", "- [ ] "];
        for _ in 0..300 {
            let len = text.len();
            let at = next(len + 1);
            let range = if next(3) == 0 {
                at..(at + 1 + next(3)).min(len)
            } else {
                at..at
            };
            // A deletion of a few bytes, or an insertion of a piece of list syntax.
            let piece = if range.is_empty() {
                pieces[next(pieces.len())]
            } else {
                ""
            };
            editor.update(cx, |e, cx| e.edit(range.clone(), piece, cx));
            draw(cx);
            text = editor.read_with(cx, |e, _| e.text().to_string());
        }
        assert!(
            editor.read_with(cx, |e, _| e.layouts().1) > 0,
            "some incremental layouts"
        );
    }

    /// Layout of an 8,000-line outline (items alternate between depth 0 and depth 1), then typing
    /// in its first item. Measures, not checks: run by hand with
    /// `cargo test --release --locked -p inkuno --bin inkuno outline_scale -- --ignored --nocapture`.
    #[gpui::test]
    #[ignore]
    fn outline_scale(cx: &mut gpui::TestAppContext) {
        // Tests compare each reused layout with a full one: not here, we measure.
        unsafe { std::env::set_var("INKUNO_NO_COMPARE", "1") };
        cx.update(crate::ui::assets::init_fonts);
        let theme = Theme::of(&Prefs::parse(""), WindowAppearance::Light);
        let (editor, cx) = cx.add_window_view(|_, cx| Editor::new(theme, cx));
        let text: String = (0..4000)
            .map(|i| format!("- item {i}\n  - sub {i}\n"))
            .collect();
        // Timed from the edit to the drawn frame: `update` may already flush the draw.
        let start = Instant::now();
        editor.update(cx, |e, cx| e.load(text, 0, cx));
        cx.run_until_parked();
        let _ = cx.update(|window, cx| window.draw(cx));
        cx.run_until_parked();
        println!("load and layout of 8,000 lines: {:?}", start.elapsed());
        // The first item starts at byte 2, after its `- `.
        for i in 0..3 {
            let start = Instant::now();
            editor.update(cx, |e, cx| e.edit(2..2, "x", cx));
            let _ = cx.update(|window, cx| window.draw(cx));
            cx.run_until_parked();
            println!(
                "typing in the first item, keystroke {i}: {:?}",
                start.elapsed()
            );
        }
        let (full, reused, _) = editor.read_with(cx, |e, _| e.layouts());
        println!(
            "layouts: {full} full, {reused} reused, rows: {}",
            editor.read_with(cx, |e, _| e.rows.len())
        );
    }
}
