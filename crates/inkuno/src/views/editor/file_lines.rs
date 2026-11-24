//! The file line numbers of the gutter (`app-ui.md` §23.7): the line of each row of the note, drawn in a
//! column left of the block and list handles. Which rows carry one is decided by `file_line_of`.

use super::*;

/// The file line numbers: this share of the body size, and their right edge this far left of the page (past the
/// block and list handles, which take 44 px).
const LINE_NUMBER_SIZE: f32 = 0.85;
const LINE_NUMBER_RIGHT: f32 = 50.;

impl Editor {
    /// The x where the file line numbers end, when they show and their column fits in the view; else `None`.
    pub(super) fn line_numbers_right(&self, window: &mut Window) -> Option<Pixels> {
        if !self.theme.line_numbers {
            return None;
        }
        let size = px(self.theme.size * LINE_NUMBER_SIZE);
        let text_system = window.text_system();
        let digit = text_system
            .advance(text_system.resolve_font(&font(sans())), size, '0')
            .map_or(size * 0.6, |s| s.width);
        let right = self.origin.x - px(LINE_NUMBER_RIGHT);
        let column = digit * crate::logic::lines::digits(self.lines.1.count()) as f32;
        (right - column >= self.viewport.left()).then_some(right)
    }

    /// The file line a row carries, if it has a number: a shown row that begins a line of the file. The rest of a
    /// wrapped line, a hidden row, a mark or added line of a pending change, and a collapsed blank line (9 px,
    /// nothing drawn) carry none (`app-ui.md` §23.7).
    pub(super) fn file_line_of(&self, row: &Row) -> Option<usize> {
        (!row.hidden() && !row.collapsed)
            .then(|| self.lines.1.number_of_line(row.start))
            .flatten()
    }

    /// The file line numbers the gutter shows, top to bottom, for the rows laid out: none while the setting is off.
    #[cfg(test)]
    pub fn file_line_numbers(&self) -> Vec<usize> {
        if !self.theme.line_numbers {
            return Vec::new();
        }
        self.rows
            .iter()
            .filter_map(|row| self.file_line_of(row))
            .collect()
    }

    /// The file line number of a row, right-aligned at `right` on the row's first line (`app-ui.md` §23.7): the
    /// faint colour, and the cursor's line one step darker.
    pub(super) fn paint_line_number(
        &self,
        row: &Row,
        number: usize,
        right: Pixels,
        cursor_line: usize,
        window: &mut Window,
        cx: &mut App,
    ) {
        let t = self.theme;
        let text_top = self.origin.y + row.y + row.pad;
        let text = number.to_string();
        let run = TextRun {
            len: text.len(),
            font: font(sans()),
            color: if number == cursor_line { t.soft } else { t.dim },
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = window.text_system().shape_line(
            SharedString::from(text),
            px(t.size * LINE_NUMBER_SIZE),
            &[run],
            None,
        );
        shaped
            .paint(
                point(right - shaped.width, text_top),
                row.lh,
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .ok();
    }
}
