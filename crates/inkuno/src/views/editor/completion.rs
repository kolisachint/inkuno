//! Completion: wikilinks, images, `/` commands and `@` dates offered under the cursor.

use super::*;
use gpui::{Div, FontWeight};

/// Choices shown at once in the completion list.
pub(super) const SHOWN: usize = 8;

pub(super) type Command = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// What `/` offers, in the order of its menu: the Basic blocks (`BASIC`), then the More list. Only what
/// GitHub renders as Markdown. The `before` of a trigger (`[[`, `![[`, `@`) opens its picker, which
/// writes a link. The three alerts are typed only (`TYPED`).
pub(super) const COMMANDS: &[Command] = &[
    ("h1", "Heading 1", "Titre 1", "# ", ""),
    ("h2", "Heading 2", "Titre 2", "## ", ""),
    ("h3", "Heading 3", "Titre 3", "### ", ""),
    ("list", "Bullet list", "Liste à puces", "- ", ""),
    ("num", "Numbered list", "Liste numérotée", "1. ", ""),
    ("todo", "Task", "Tâche à cocher", "- [ ] ", ""),
    ("quote", "Quote", "Citation", "> ", ""),
    ("code", "Code block", "Bloc de code", "```", "\n\n```"),
    ("table", "Table", "Tableau", "", ""),
    ("note", "Panel: note", "Panneau : note", "> [!NOTE]\n> ", ""),
    ("warning", "Panel: warning", "Panneau : attention", "> [!WARNING]\n> ", ""),
    ("rule", "Divider", "Séparateur", "---\n", ""),
    ("link", "Link", "Lien", "[", "](https://)"),
    ("image", "Picture", "Image", "![[", ""),
    ("math", "Formula", "Formule", "$$\n", "\n$$"),
    ("mermaid", "Mermaid diagram", "Diagramme Mermaid", "```mermaid\n", "\n```"),
    ("linknote", "Link to note", "Lien vers une note", "[[", ""),
    ("linkdate", "Link to date", "Lien vers une date", "@", ""),
    ("tip", "Panel: tip", "Panneau : astuce", "> [!TIP]\n> ", ""),
    ("important", "Panel: important", "Panneau : important", "> [!IMPORTANT]\n> ", ""),
    ("caution", "Panel: danger", "Panneau : danger", "> [!CAUTION]\n> ", ""),
];

/// The Basic blocks of the slash menu's icon grid, in order.
pub(super) const BASIC: [&str; 8] = ["h1", "h2", "h3", "list", "num", "todo", "quote", "code"];

/// Typed only: not in the menu's list when nothing is typed after `/`.
pub(super) const TYPED: [&str; 3] = ["tip", "important", "caution"];

/// The icon of a slash command in the menu: a file of `ui::assets`, or none for the heading
/// commands, drawn as their letters.
pub(super) fn icon(name: &str) -> &'static str {
    match name {
        "list" => "list-bullet.svg",
        "num" => "list-number.svg",
        "todo" => "list-task.svg",
        "quote" => "quote.svg",
        "code" | "mermaid" => "code.svg",
        "table" => "table.svg",
        "note" | "tip" | "important" => "info.svg",
        "warning" | "caution" => "alert.svg",
        "rule" => "rule.svg",
        "link" => "link.svg",
        "image" => "image.svg",
        "math" => "math.svg",
        "linknote" => "note.svg",
        "linkdate" => "calendar.svg",
        _ => "",
    }
}

/// A day as its link names it: `Saturday, 21 November`; a written date as `2026-11-21`.
fn date_label(en: &str, date: md::Date) -> String {
    if en.is_empty() {
        return crate::lang::date_name(date);
    }
    let (_, day, month) = date;
    let weekday = en[..1].to_uppercase() + &en[1..];
    format!("{weekday}, {day} {}", md::MONTHS[month as usize - 1].0)
}

/// What completion offers: a name to put after `[[`, or a command.
#[derive(Clone)]
pub(super) enum Choice<'a> {
    Name(&'a str),
    Command(&'static Command),
    /// Day offered after `@`: its English and French name, its date.
    Date(&'static str, &'static str, md::Date),
}

impl Editor {
    /// Corner of a floating panel of size `panel`: below the cursor, or
    /// above if it does not fit.
    pub(super) fn popup_at(&self, panel: Size<Pixels>) -> Option<Point<Pixels>> {
        let c = self.cursor();
        let row = &self.rows[self.row_at(c)?];
        let p = row.pos(c - row.start);
        let top = self.origin.y + row.y + row.pad + p.y;
        let (below, above) = (top + row.lh + px(4.), top - panel.height - px(4.));
        let fits =
            below + panel.height <= self.viewport.bottom() - FOOTER || above < self.viewport.top();
        let x = (self.origin.x + p.x).min(self.viewport.right() - panel.width - px(8.));
        Some(point(
            x.max(self.viewport.left()),
            if fits { below } else { above },
        ))
    }

    /// Where the note `name` (or alias) is. A note that does not exist yet goes at the vault's root,
    /// where the daily note is created (`ops::daily_note`).
    pub(super) fn note_path(&self, name: &str) -> PathBuf {
        let wanted = name.to_lowercase();
        self.targets
            .iter()
            .find(|(known, _)| known.to_lowercase() == wanted)
            .map(|(_, path)| path.clone())
            .unwrap_or_else(|| {
                let root = self.dirs.last().cloned().unwrap_or_default();
                root.join(format!("{name}.md"))
            })
    }

    /// The Markdown link `[title](path)` to `to`, the path relative to the folder of the note being edited.
    pub(super) fn link_to(&self, title: &str, to: &Path) -> String {
        let from = self.dirs.first().map_or(Path::new(""), PathBuf::as_path);
        crate::logic::wiki::link_to(title, from, to)
    }

    /// Quiet's completion list, a popover above or below the cursor: after a bare `/` the Basic icon grid and
    /// the More list with right-aligned tokens; otherwise one list of what matches. `None` when it is not open.
    pub(super) fn completion_menu(&self) -> Option<Div> {
        let t = self.theme;
        if !t.quiet {
            return None;
        }
        let (start, items) = self.completion()?;
        let chosen = self.ac_index.min(items.len() - 1);
        let bare = start == self.cursor();
        let grid = |choice: &Choice| matches!(choice, Choice::Command((name, ..)) if bare && BASIC.contains(name));
        let (cell, row, width) = (t.px(38.), t.px(32.), t.px(260.));
        let basic: Vec<usize> = (0..items.len()).filter(|&i| grid(&items[i])).collect();
        let list: Vec<usize> = (0..items.len()).filter(|i| !basic.contains(i)).collect();
        // Padding 5 above and below; with the grid, two headings of 22 and the grid's 4 px of foot.
        let grid_h = if bare && !basic.is_empty() {
            t.px(48.) + cell * basic.len().div_ceil(4) as f32
        } else {
            t.px(0.)
        };
        let height = t.px(10.) + grid_h + row * list.len() as f32;
        let at = self.popup_at(size(width, height))?;
        let at = at - self.viewport.origin;
        let heading = |text: &'static str| {
            div()
                .h(t.px(22.))
                .px(t.px(13.))
                .pt(t.px(4.))
                .text_size(t.px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(t.soft)
                .child(text)
        };
        let shown = |i: usize| (items[i].clone(), i == chosen);
        let cells = basic.iter().map(|&i| {
            let (choice, on) = shown(i);
            let Choice::Command((name, ..)) = choice else {
                return div().into_any_element();
            };
            let mark = match *name {
                "h1" => div().text_size(t.px(12.)).font_weight(FontWeight::SEMIBOLD).child("H1").into_any_element(),
                "h2" => div().text_size(t.px(12.)).font_weight(FontWeight::SEMIBOLD).child("H2").into_any_element(),
                "h3" => div().text_size(t.px(12.)).font_weight(FontWeight::SEMIBOLD).child("H3").into_any_element(),
                _ => gpui::svg().path(icon(name)).size(t.px(18.)).text_color(t.soft).into_any_element(),
            };
            div()
                .id(("slash-cell", i))
                .debug_selector(move || format!("slash-{name}"))
                .w(relative(0.25))
                .h(cell)
                .rounded(t.small())
                .flex()
                .items_center()
                .justify_center()
                .text_color(t.soft)
                .when(on, |d| d.bg(t.selection))
                .when(!on, |d| d.hover(|s| s.bg(t.hover)))
                .child(mark)
                .into_any_element()
        });
        let rows = list.iter().map(|&i| {
            let (choice, on) = shown(i);
            let (icon_path, text, token) = match choice {
                Choice::Command((name, en, fr, ..)) => {
                    let token = match *name {
                        "rule" => "/hr".to_string(),
                        "linknote" => "[[".to_string(),
                        "linkdate" => "@".to_string(),
                        _ => format!("/{name}"),
                    };
                    (icon(name), tr(en, fr).to_string(), token)
                }
                Choice::Name(name) => (if self.images.contains_key(&name.to_lowercase()) { "image.svg" } else { "note.svg" }, name.to_string(), String::new()),
                Choice::Date(en, fr, date) => ("clock.svg", tr(en, fr).to_string(), crate::lang::date_name(date)),
            };
            div()
                .id(("slash-row", i))
                .h(row)
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
                .child(gpui::svg().path(icon_path).size(t.px(16.)).flex_none().text_color(t.soft))
                .child(div().flex_1().min_w_0().truncate().child(text))
                .child(
                    div()
                        .flex_none()
                        .font_family(crate::ui::assets::mono())
                        .text_size(t.px(12.))
                        .text_color(t.dim)
                        .child(token),
                )
                .into_any_element()
        });
        let mut panel = crate::ui::widgets::popover(t)
            .debug_selector(|| "slash-menu".into())
            .absolute()
            .left(at.x)
            .top(at.y)
            .w(width)
            .py(t.px(5.))
            .overflow_hidden()
            .text_size(t.px(13.));
        if bare && !basic.is_empty() {
            panel = panel
                .child(heading("Basic"))
                .child(div().flex().flex_wrap().px(t.px(5.)).pb(t.px(4.)).children(cells))
                .child(heading("More"));
        }
        Some(panel.children(rows))
    }

    /// Start of the query (after `[[` or `/`) and what matches it.
    pub(super) fn completion(&self) -> Option<(usize, Vec<Choice<'_>>)> {
        if !self.sel.is_empty() || !self.more.is_empty() {
            return None;
        }
        let c = self.cursor();
        let line = self.line_range(c).start;
        let before = &self.content[line..c];
        let found = self
            .names(before, c)
            .or_else(|| self.commands(before, c, line))
            .or_else(|| self.days(before, c, line));
        let (start, items) = found?;
        (!items.is_empty() && self.ac_dismissed != Some(start)).then_some((start, items))
    }

    /// After `[[`: the vault's notes; after `![[`: its images.
    pub(super) fn names(&self, before: &str, c: usize) -> Option<(usize, Vec<Choice<'_>>)> {
        let open = before.rfind("[[")?;
        let query = &before[open + 2..];
        if query.contains("]]") || query.contains('|') {
            return None;
        }
        let start = c - query.len();
        let query = query.to_lowercase();
        // `![[` displays a file: the vault's images are
        // offered, by file name; `[[` offers notes.
        let embed = before[..open].ends_with('!');
        let mut items: Vec<&str> = match embed {
            true => self
                .images
                .values()
                .filter_map(|p| p.file_name()?.to_str())
                .collect(),
            false => self.notes.iter().map(String::as_str).collect(),
        };
        items.retain(|name| name.to_lowercase().contains(&query));
        if embed {
            items.sort_unstable_by_key(|name| name.to_lowercase());
        }
        items.truncate(6);
        Some((start, items.into_iter().map(Choice::Name).collect()))
    }

    /// After a `/` at the start of a line or word, outside code: the commands whose
    /// name starts with the query, then those whose label contains it.
    pub(super) fn commands(
        &self,
        before: &str,
        c: usize,
        line: usize,
    ) -> Option<(usize, Vec<Choice<'_>>)> {
        let slash = before.rfind('/')?;
        let query = before[slash + 1..].to_lowercase();
        let starts_word = before[..slash]
            .chars()
            .next_back()
            .is_none_or(char::is_whitespace);
        if !starts_word || !query.chars().all(char::is_alphanumeric) || self.in_code(line) {
            return None;
        }
        let mut items: Vec<&'static Command> = COMMANDS
            .iter()
            .filter(|(name, en, fr, ..)| {
                (!query.is_empty() || !TYPED.contains(name))
                    && (name.starts_with(&query) || tr(en, fr).to_lowercase().contains(&query))
            })
            .collect();
        items.sort_by_key(|(name, ..)| !name.starts_with(&query));
        Some((
            c - query.len(),
            items.into_iter().map(Choice::Command).collect(),
        ))
    }

    /// After an `@` at the start of a line or word, outside code: the days the rest names
    /// (`@demain`, `@lundi`, `@2026-10-09`). An address `name@domain` opens nothing.
    pub(super) fn days(
        &self,
        before: &str,
        c: usize,
        line: usize,
    ) -> Option<(usize, Vec<Choice<'_>>)> {
        let at = before.rfind('@')?;
        let query = &before[at + 1..];
        let starts_word = before[..at]
            .chars()
            .next_back()
            .is_none_or(char::is_whitespace);
        if !starts_word
            || !query
                .chars()
                .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '\'' | '’'))
            || self.in_code(line)
        {
            return None;
        }
        let items = md::dates(query, crate::platform::clock::today())
            .into_iter()
            .map(|(en, fr, date)| Choice::Date(en, fr, date));
        Some((c - query.len(), items.collect()))
    }

    pub(super) fn accept_completion(&mut self, cx: &mut Context<Self>) -> bool {
        let Some((start, items)) = self.completion() else {
            return false;
        };
        let c = self.cursor();
        match items[self.ac_index.min(items.len() - 1)].clone() {
            // The note's name becomes a relative Markdown link (an image, `![name](path)`, after `![[`).
            Choice::Name(name) => {
                let open = start - 2;
                let bang = self.content[..open].ends_with('!');
                let from = if bang { open - 1 } else { open };
                // A `]]` the editor paired after the cursor goes with the link.
                let end = c + if self.content[c..].starts_with("]]") { 2 } else { 0 };
                let text = if bang {
                    let path = self.images.get(&name.to_lowercase()).cloned().unwrap_or_default();
                    format!("!{}", self.link_to(name, &path))
                } else {
                    let path = self.note_path(name);
                    self.link_to(name, &path)
                };
                if self.touches_pending(&(from..end), &text) {
                    self.refuse_pending(cx);
                    return true;
                }
                self.push_undo(true);
                self.edit(from..end, &text, cx);
            }
            // The day becomes a link to its daily note, created when first opened.
            Choice::Date(en, _, date) => {
                let name = crate::lang::date_name(date);
                let path = self.note_path(&name);
                let text = self.link_to(&date_label(en, date), &path);
                if self.touches_pending(&(start - 1..c), &text) {
                    self.refuse_pending(cx);
                    return true;
                }
                self.push_undo(true);
                self.edit(start - 1..c, &text, cx);
            }
            Choice::Command(&(name, _, _, before, after)) => {
                let slash = start - 1;
                if name == "table" {
                    if self.touches_pending(&(slash..c), "") {
                        self.refuse_pending(cx);
                        return true;
                    }
                    self.push_undo(true);
                    self.edit(slash..c, "", cx);
                    self.grid = Some((3, 3));
                    self.grid_typed.clear();
                    return true;
                }
                // A link goes in the sentence; everything else starts its own line.
                let line = self.line_range(c);
                let inline = before.ends_with('[')
                    || before == "@"
                    || self.content[line.start..slash].trim().is_empty();
                let lead = if inline { "" } else { "\n" };
                // A link goes in the sentence, so nothing starts a new line after it.
                let tail = if after.is_empty() || c == line.end || before.ends_with('[') {
                    ""
                } else {
                    "\n"
                };
                let text = format!("{lead}{before}{after}{tail}");
                if self.touches_pending(&(slash..c), &text) {
                    self.refuse_pending(cx);
                    return true;
                }
                self.push_undo(true);
                self.edit(slash..c, &text, cx);
                self.move_to(slash + lead.len() + before.len(), cx);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_slash_menu_offers_only_what_github_renders() {
        let bare: Vec<&str> = COMMANDS
            .iter()
            .map(|c| c.0)
            .filter(|name| !TYPED.contains(name))
            .collect();
        // No front matter, no date stamp, no wiki link or embed: each of these writes Markdown GitHub draws.
        for gone in ["date", "meta"] {
            assert!(!bare.contains(&gone), "{gone} is not GitHub Markdown");
        }
        // Only the two pickers write a `[[` or `![[` trigger, and their choices are Markdown links.
        for c in COMMANDS.iter().filter(|c| c.3.contains("[[")) {
            assert!(["linknote", "image"].contains(&c.0), "{} opens a wiki picker", c.0);
        }
        assert!(bare.starts_with(&BASIC));
        assert_eq!(BASIC.len(), 8);
        assert!(bare.iter().all(|name| !icon(name).is_empty() || ["h1", "h2", "h3"].contains(name)));
    }
}
