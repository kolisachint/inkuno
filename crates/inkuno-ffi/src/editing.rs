//! What the keys do to the text: Enter on a list, Tab and Shift-Tab on lines.

use inkuno_logic::markdown::{self as md, Enter, Fence, Kind};

use crate::text::{byte_offset, offset_utf16};

/// What Enter does on a line (see `inkuno_logic::markdown::on_enter`).
#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq)]
pub enum EnterOutcome {
    /// Insert `text` at the cursor. It starts with the line break, then the next item's marker
    /// or the indent the new line keeps.
    Insert { text: String },
    /// An empty list item: the line's text is replaced by nothing (the line break stays), which
    /// leaves the list.
    Clear,
}

/// What Enter does with the cursor at `col_utf16` of `line`. `in_fence`: the line is inside a
/// ``` code block (see `document_layout`'s `in_code_block`); a block with another fence (`~~~`)
/// is read as code by this call, so it is not an exact answer for those.
#[uniffi::export]
pub fn on_enter(line: String, col_utf16: u64, in_fence: bool) -> EnterOutcome {
    let open = in_fence.then_some(Fence { ch: b'`', len: 3 });
    let col = byte_offset(&line, col_utf16);
    match md::on_enter(&line, col, open) {
        Enter::Insert(text) => EnterOutcome::Insert { text },
        Enter::Clear => EnterOutcome::Clear,
    }
}

/// A replacement of the text: replace the UTF-16 range `start_utf16..end_utf16` of the old text
/// with `replacement`, then select `selection_start_utf16..selection_end_utf16` of the new text.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start_utf16: u64,
    pub end_utf16: u64,
    pub replacement: String,
    pub selection_start_utf16: u64,
    pub selection_end_utf16: u64,
}

/// Tab (`outdent` false) or Shift-Tab (`outdent` true) on the selection `start_utf16..end_utf16`.
///
/// A list line, or a selection, moves each line of the selected lines by one indent (4 spaces).
/// Shift-Tab removes one tab, or up to 4 spaces, at the start of each line. A Tab with a cursor on
/// any other line inserts an indent at the cursor. Same rules as the desktop editor.
#[uniffi::export]
pub fn indent_lines(text: String, start_utf16: u64, end_utf16: u64, outdent: bool) -> TextEdit {
    let (a, b) = (
        byte_offset(&text, start_utf16),
        byte_offset(&text, end_utf16),
    );
    let (sel_start, sel_end) = (a.min(b), a.max(b));

    if !outdent && sel_start == sel_end {
        let (line_start, line_end) = line_bounds(&text, sel_start);
        let open = md::open_at(&md::blocks(&text), line_start);
        let (kind, _) = md::classify_in(&text[line_start..line_end], open);
        if !matches!(kind, Kind::Bullet | Kind::Ordered | Kind::Task(_)) {
            let at = offset_utf16(&text, sel_start);
            return TextEdit {
                start_utf16: at,
                end_utf16: at,
                replacement: md::INDENT.to_string(),
                selection_start_utf16: at + md::INDENT.len() as u64,
                selection_end_utf16: at + md::INDENT.len() as u64,
            };
        }
    }

    // The whole lines the selection touches.
    let (block_start, _) = line_bounds(&text, sel_start);
    let (_, block_end) = line_bounds(&text, sel_end);
    let old = &text[block_start..block_end];

    let mut deltas: Vec<isize> = Vec::new();
    let shifted: Vec<String> = old
        .split('\n')
        .map(|line| {
            let new = if !outdent {
                format!("{}{line}", md::INDENT)
            } else if let Some(rest) = line.strip_prefix('\t') {
                rest.to_string()
            } else {
                let spaces = line
                    .bytes()
                    .take(md::INDENT.len())
                    .take_while(|&b| b == b' ')
                    .count();
                line[spaces..].to_string()
            };
            deltas.push(new.len() as isize - line.len() as isize);
            new
        })
        .collect();
    let replacement = shifted.join("\n");
    let total: isize = deltas.iter().sum();

    // The selection keeps its place in its lines; a start inside a removed indent goes to the
    // line's start, and the end moves by the whole change.
    let start = (sel_start as isize + deltas[0]).max(block_start as isize) as usize;
    let end = (sel_end as isize + total).max(start as isize) as usize;

    let mut new_text = String::with_capacity(text.len() + replacement.len());
    new_text.push_str(&text[..block_start]);
    new_text.push_str(&replacement);
    new_text.push_str(&text[block_end..]);

    TextEdit {
        start_utf16: offset_utf16(&text, block_start),
        end_utf16: offset_utf16(&text, block_end),
        replacement,
        selection_start_utf16: offset_utf16(&new_text, start),
        selection_end_utf16: offset_utf16(&new_text, end),
    }
}

/// The byte range of the line that holds byte `at`, without its line break.
fn line_bounds(text: &str, at: usize) -> (usize, usize) {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies an edit (UTF-16 in, UTF-16 out) to `text`, as the Swift side does.
    fn apply(text: &str, edit: &TextEdit) -> String {
        let start = byte_offset(text, edit.start_utf16);
        let end = byte_offset(text, edit.end_utf16);
        format!("{}{}{}", &text[..start], edit.replacement, &text[end..])
    }

    #[test]
    fn enter_on_a_bullet_continues_the_list() {
        assert_eq!(
            on_enter("- milk".into(), 6, false),
            EnterOutcome::Insert {
                text: "\n- ".into()
            }
        );
    }

    #[test]
    fn enter_on_an_empty_item_clears_it() {
        assert_eq!(on_enter("- ".into(), 2, false), EnterOutcome::Clear);
    }

    #[test]
    fn enter_renumbers_an_ordered_item_and_keeps_the_indent() {
        assert_eq!(
            on_enter("  9. nine".into(), 9, false),
            EnterOutcome::Insert {
                text: "\n  10. ".into()
            }
        );
    }

    #[test]
    fn enter_on_an_unchecked_task_goes_on_unchecked() {
        assert_eq!(
            on_enter("- [x] done".into(), 10, false),
            EnterOutcome::Insert {
                text: "\n- [ ] ".into()
            }
        );
    }

    #[test]
    fn enter_in_a_code_block_only_keeps_the_indent() {
        assert_eq!(
            on_enter("  - not a list".into(), 13, true),
            EnterOutcome::Insert {
                text: "\n  ".into()
            }
        );
    }

    #[test]
    fn enter_counts_the_cursor_in_utf16() {
        // After the marker (unit 2) Enter continues the list; before it (unit 0) it only breaks the line.
        let line = "- 😀 é".to_string();
        assert_eq!(
            on_enter(line.clone(), 2, false),
            EnterOutcome::Insert {
                text: "\n- ".into()
            }
        );
        // Before the marker, Enter only breaks the line.
        assert_eq!(
            on_enter(line, 0, false),
            EnterOutcome::Insert { text: "\n".into() }
        );
    }

    #[test]
    fn tab_on_a_plain_line_inserts_an_indent_at_the_cursor() {
        let text = "hello";
        let edit = indent_lines(text.into(), 2, 2, false);
        assert_eq!(apply(text, &edit), "he    llo");
        assert_eq!(edit.selection_start_utf16, 6);
        assert_eq!(edit.selection_end_utf16, 6);
    }

    #[test]
    fn tab_on_a_list_line_indents_it() {
        let text = "one\n- two\nthree";
        let edit = indent_lines(text.into(), 6, 6, false);
        assert_eq!(apply(text, &edit), "one\n    - two\nthree");
        assert_eq!((edit.start_utf16, edit.end_utf16), (4, 9));
        // The cursor keeps its place in the text: after "- " it was 2 bytes further than the line's start.
        assert_eq!(edit.selection_start_utf16, 10);
    }

    #[test]
    fn tab_on_a_selection_indents_every_selected_line() {
        let text = "a\nb\nc";
        let edit = indent_lines(text.into(), 0, 3, false);
        assert_eq!(apply(text, &edit), "    a\n    b\nc");
        assert_eq!(edit.selection_start_utf16, 4);
        assert_eq!(edit.selection_end_utf16, 11);
    }

    #[test]
    fn tab_in_a_code_block_on_a_plain_line_inserts_at_the_cursor() {
        let text = "```\nlet x;\n```";
        // A cursor on `let x;`, which is code and not a list item: an indent at the cursor.
        let edit = indent_lines(text.into(), 4, 4, false);
        assert_eq!(apply(text, &edit), "```\n    let x;\n```");
    }

    #[test]
    fn shift_tab_removes_a_tab_or_up_to_four_spaces() {
        let text = "\tone\n  two\n      three";
        let all = text.encode_utf16().count() as u64;
        let edit = indent_lines(text.into(), 0, all, true);
        assert_eq!(apply(text, &edit), "one\ntwo\n  three");
    }

    #[test]
    fn shift_tab_with_no_indent_changes_nothing() {
        let text = "flush";
        let edit = indent_lines(text.into(), 3, 3, true);
        assert_eq!(apply(text, &edit), text);
        assert_eq!(edit.selection_start_utf16, 3);
    }

    #[test]
    fn selection_in_utf16_survives_a_shift() {
        // "é😀" then a line break: the selection covers "😀" (units 1..3), and the line gets indented.
        let text = "é😀\nx";
        let edit = indent_lines(text.into(), 1, 3, false);
        assert_eq!(apply(text, &edit), "    é😀\nx");
        assert_eq!(edit.selection_start_utf16, 5);
        assert_eq!(edit.selection_end_utf16, 7);
    }
}
