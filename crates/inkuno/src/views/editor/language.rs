//! The language picker of a folded code block: a popup under its chip that rewrites the first word of
//! the opening fence. Typing filters the list; Up, Down, Enter and Escape work, and so does the mouse.

use super::*;
use gpui::{Div, MouseButton, MouseDownEvent};

/// Languages offered, in the order of the list.
pub(super) const LANGUAGES: [&str; 24] = [
    "text",
    "rust",
    "python",
    "javascript",
    "typescript",
    "json",
    "bash",
    "sh",
    "html",
    "css",
    "sql",
    "yaml",
    "toml",
    "markdown",
    "go",
    "c",
    "cpp",
    "java",
    "kotlin",
    "swift",
    "ruby",
    "php",
    "mermaid",
    "math",
];

/// Choices shown at once in the picker; the rest scroll.
const SHOWN: usize = 10;

/// A picker open on the folded code block whose opening line starts at `fence`.
#[derive(Default)]
pub(super) struct LangPick {
    pub(super) fence: usize,
    pub(super) query: String,
    pub(super) index: usize,
}

/// The languages whose name contains the query: those that start with it first, in the list's order.
pub(super) fn matching(query: &str) -> Vec<&'static str> {
    let q = query.trim().to_lowercase();
    let (mut first, rest): (Vec<_>, Vec<_>) = LANGUAGES
        .iter()
        .copied()
        .filter(|lang| lang.contains(q.as_str()))
        .partition(|lang| lang.starts_with(q.as_str()));
    first.extend(rest);
    first
}

/// An opening fence line with the first word of its info string replaced by `lang`. The indentation,
/// the fence characters and the attributes after the word stay as they are; a fence with no language
/// gets one. None if the line is not an opening fence.
pub(super) fn with_language(line: &str, lang: &str) -> Option<String> {
    let (fence, info) = md::fence_of(line)?;
    let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
    let marks = &line[..indent + fence.len];
    let attributes = info.find(char::is_whitespace).map_or("", |i| &info[i..]);
    Some(format!("{marks}{lang}{attributes}"))
}

impl Editor {
    /// Opens the picker on the folded block whose opening line starts at `fence`.
    pub(super) fn open_language(&mut self, fence: usize, cx: &mut Context<Self>) {
        self.lang = Some(LangPick {
            fence,
            ..LangPick::default()
        });
        self.lang_scroll.scroll_to_item(0);
        cx.notify();
    }

    /// Up and Down move the chosen language, which wraps round.
    pub(super) fn language_step(&mut self, down: bool, cx: &mut Context<Self>) {
        let n = matching(self.lang.as_ref().map_or("", |p| p.query.as_str())).len();
        let Some(pick) = self.lang.as_mut().filter(|_| n > 0) else {
            return cx.notify();
        };
        pick.index = if down {
            (pick.index + 1) % n
        } else {
            (pick.index + n - 1) % n
        };
        self.lang_scroll.scroll_to_item(pick.index);
        cx.notify();
    }

    /// Typed text goes to the filter, not to the note.
    pub(super) fn language_type(&mut self, text: &str, cx: &mut Context<Self>) {
        if let Some(pick) = self.lang.as_mut() {
            pick.query.push_str(text);
            pick.index = 0;
            self.lang_scroll.scroll_to_item(0);
        }
        cx.notify();
    }

    /// Backspace shortens the filter.
    pub(super) fn language_back(&mut self, cx: &mut Context<Self>) {
        if let Some(pick) = self.lang.as_mut() {
            pick.query.pop();
            pick.index = 0;
            self.lang_scroll.scroll_to_item(0);
        }
        cx.notify();
    }

    /// Enter chooses the language under the cursor of the list.
    pub(super) fn language_enter(&mut self, cx: &mut Context<Self>) {
        let chosen = self.lang.as_ref().and_then(|pick| {
            matching(&pick.query)
                .get(pick.index)
                .copied()
                .map(str::to_string)
        });
        match chosen {
            Some(lang) => self.language_pick(&lang, cx),
            None => {
                self.lang = None;
                cx.notify();
            }
        }
    }

    /// Writes `lang` as the language of the picker's block, as one undo step. The caret stays where it
    /// was: the block stays folded.
    pub(super) fn language_pick(&mut self, lang: &str, cx: &mut Context<Self>) {
        let Some(pick) = self.lang.take() else {
            return;
        };
        let line = self.line_range(pick.fence);
        let Some(new) = with_language(&self.content[line.clone()], lang) else {
            return cx.notify();
        };
        if new == self.content[line.clone()] {
            return cx.notify();
        }
        if self.touches_pending(&line, &new) {
            return self.refuse_pending(cx);
        }
        // One undo step: the step is opened here, and the splice joins it.
        self.push_undo(true);
        self.splice_now(line, &new, cx);
    }

    /// The popup under the chip of the picker's block, or above it when it does not fit below.
    pub(super) fn language_menu(&self, cx: &mut Context<Self>) -> Option<Div> {
        let t = self.theme;
        let pick = self.lang.as_ref()?;
        let row = self
            .rows
            .iter()
            .find(|r| r.start == pick.fence && r.opens)?;
        let chip = self.chip_bounds(row);
        let items = matching(&pick.query);
        let current = md::fence_of(&self.content[self.line_range(pick.fence)])
            .and_then(|(_, info)| info.split_whitespace().next())
            .unwrap_or("")
            .to_lowercase();
        let (width, row_h) = (t.px(200.), t.px(32.));
        let shown = items.len().clamp(1, SHOWN) as f32;
        let height = t.px(10.) + t.px(34.) + row_h * shown;
        let below = chip.bottom() + t.px(4.);
        let top = if below + height <= self.viewport.bottom() - FOOTER {
            below
        } else {
            chip.top() - height - t.px(4.)
        };
        let left = chip
            .left()
            .min(self.viewport.right() - width - t.px(8.))
            .max(self.viewport.left());
        let at = point(left, top) - self.viewport.origin;

        let field = div()
            .h(t.px(34.))
            .mx(t.px(5.))
            .px(t.px(8.))
            .flex()
            .items_center()
            .text_size(t.px(13.))
            .border_b_1()
            .border_color(t.border)
            .text_color(if pick.query.is_empty() { t.dim } else { t.text })
            .child(if pick.query.is_empty() {
                tr("Filter languages", "Filtrer les langages").to_string()
            } else {
                pick.query.clone()
            });
        let rows: Vec<_> = if items.is_empty() {
            vec![
                div()
                    .h(row_h)
                    .px(t.px(13.))
                    .flex()
                    .items_center()
                    .text_color(t.dim)
                    .child(tr("No language", "Aucun langage"))
                    .into_any_element(),
            ]
        } else {
            items
                .iter()
                .enumerate()
                .map(|(i, &lang)| {
                    let on = i == pick.index.min(items.len() - 1);
                    div()
                        .id(("lang-row", i))
                        .h(row_h)
                        .mx(t.px(5.))
                        .px(t.px(8.))
                        .rounded(t.small())
                        .flex()
                        .items_center()
                        .gap(t.px(8.))
                        .text_size(t.px(13.))
                        .text_color(t.text)
                        .when(on, |d| d.bg(t.selection))
                        .when(!on, |d| d.hover(|s| s.bg(t.hover)))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                this.language_pick(lang, cx)
                            }),
                        )
                        .child(div().flex_1().min_w_0().truncate().child(lang))
                        .child(
                            div()
                                .flex_none()
                                .font_family(crate::ui::assets::mono())
                                .text_size(t.px(12.))
                                .text_color(t.dim)
                                .when(lang == current, |d| d.child(tr("current", "actuel"))),
                        )
                        .into_any_element()
                })
                .collect()
        };
        Some(
            crate::ui::widgets::popover(t)
                .debug_selector(|| "lang-menu".into())
                .absolute()
                .left(at.x)
                .top(at.y)
                .w(width)
                .py(t.px(5.))
                .overflow_hidden()
                .text_size(t.px(13.))
                // A click on the picker is its own: it does not reach the note.
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(field)
                .child(crate::ui::components::scroll::scroll_area(
                    "lang-rows",
                    &self.lang_scroll,
                    t,
                    row_h * SHOWN as f32,
                    rows,
                )),
        )
    }

    /// Where the chip of the block starting at `start` is: a test clicks it.
    #[cfg(test)]
    pub fn chip_center(&self, start: usize) -> Option<Point<Pixels>> {
        let row = self
            .rows
            .iter()
            .find(|r| r.start == start && r.opens && r.fold.edges())?;
        Some(self.chip_bounds(row).center())
    }

    /// The filter of the open picker, for the tests.
    #[cfg(test)]
    pub fn language_filter(&self) -> Option<String> {
        self.lang.as_ref().map(|p| p.query.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_word_of_an_opening_fence_is_rewritten() {
        assert_eq!(with_language("```", "python").as_deref(), Some("```python"));
        assert_eq!(with_language("~~~", "python").as_deref(), Some("~~~python"));
        assert_eq!(
            with_language("```rust", "python").as_deref(),
            Some("```python")
        );
        assert_eq!(with_language("~~~rust", "go").as_deref(), Some("~~~go"));
        assert_eq!(
            with_language("```rust {.numberLines}", "python").as_deref(),
            Some("```python {.numberLines}")
        );
        assert_eq!(
            with_language("````  js title=\"a b\"", "ts").as_deref(),
            Some("````ts title=\"a b\"")
        );
        assert_eq!(
            with_language("  ```text", "bash").as_deref(),
            Some("  ```bash")
        );
        assert_eq!(with_language("``rust", "go"), None);
        assert_eq!(with_language("rust", "go"), None);
        assert_eq!(with_language("```rust`", "go"), None);
    }

    #[test]
    fn the_filter_matches_names_and_starts_first() {
        assert_eq!(matching("py"), vec!["python"]);
        assert_eq!(matching("")[0], "text");
        assert_eq!(matching("").len(), LANGUAGES.len());
        assert_eq!(matching("cp"), vec!["cpp"]);
        assert_eq!(matching("s")[..3], ["sh", "sql", "swift"]);
        assert_eq!(matching("script"), vec!["javascript", "typescript"]);
        assert!(matching("zig").is_empty());
    }
}
