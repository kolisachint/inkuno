//! Keyboard navigation (plan task 3.3). Owner: task 3.3.
//!
//! Key bindings use the "MdEditor" key context that `Editor::render` sets. The pure motion
//! logic lives in [`move_offset`] and has no GPUI dependency.
//!
//! Bindings are platform-specific: macOS uses `cmd-` and `alt-` (the Mac conventions), Linux
//! uses `ctrl-`. `KeyBinding::new` cannot resolve GPUI's `secondary-` alias, so each platform
//! has its own binding list.

use gpui::{actions, prelude::*, Action, App, Context, Div, Entity, KeyBinding, Window};

use md_core::display_map::{Direction, DisplayMap};

use crate::clipboard::{Copy, Cut, Paste};
use crate::editor::Editor;
use crate::inline_render::document_hidden_ranges;
use crate::selection::Selection;

actions!(
    md_editor_keys,
    [
        MoveLeft,
        MoveRight,
        MoveUp,
        MoveDown,
        MoveWordLeft,
        MoveWordRight,
        MoveLineStart,
        MoveLineEnd,
        MoveDocStart,
        MoveDocEnd,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectWordLeft,
        SelectWordRight,
        SelectLineStart,
        SelectLineEnd,
        SelectDocStart,
        SelectDocEnd,
    ]
);

/// A caret motion. Each action above maps to one of these plus an `extend` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    Up,
    Down,
    WordLeft,
    WordRight,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
}

/// Result of a motion: the new offset, and the goal column to keep for the next vertical move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    pub offset: usize,
    pub goal_column: Option<usize>,
}

/// Where `motion` takes the caret from `offset` in `text`.
///
/// Pure function, unit-tested in 3.3. Left and Right move by one character (not one byte).
/// Up and Down keep `goal_column` (bytes from the line start) when it is given, and set it from
/// the current column otherwise. Word motion uses Unicode word boundaries. Line and document
/// motion stop at `\n` boundaries and the text ends.
pub fn move_offset(text: &str, offset: usize, motion: Motion, goal_column: Option<usize>) -> Moved {
    let offset = char_floor(text, offset.min(text.len()));
    let at = |offset: usize| Moved {
        offset,
        goal_column: None,
    };
    match motion {
        Motion::Left => at(prev_char(text, offset)),
        Motion::Right => at(next_char(text, offset)),
        Motion::Up => vertical(text, offset, false, goal_column),
        Motion::Down => vertical(text, offset, true, goal_column),
        Motion::WordLeft => at(word_left(text, offset)),
        Motion::WordRight => at(word_right(text, offset)),
        Motion::LineStart => at(line_start(text, offset)),
        Motion::LineEnd => at(line_end(text, offset)),
        Motion::DocStart => at(0),
        Motion::DocEnd => at(text.len()),
    }
}

/// Largest char boundary at or below `offset`.
fn char_floor(text: &str, mut offset: usize) -> usize {
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn prev_char(text: &str, offset: usize) -> usize {
    text[..offset]
        .chars()
        .next_back()
        .map_or(0, |c| offset - c.len_utf8())
}

fn next_char(text: &str, offset: usize) -> usize {
    text[offset..]
        .chars()
        .next()
        .map_or(offset, |c| offset + c.len_utf8())
}

/// Start of the line that contains `offset`.
fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |i| i + 1)
}

/// End of the line that contains `offset` (the position of its `\n`, or the text end).
fn line_end(text: &str, offset: usize) -> usize {
    text[offset..].find('\n').map_or(text.len(), |i| offset + i)
}

/// Coarse character class for word motion.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Space,
    Word,
    Other,
}

fn class(c: char) -> Class {
    if c.is_whitespace() {
        Class::Space
    } else if c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Other
    }
}

/// End of the next word after `offset`: skip whitespace, then one run of the same class.
fn word_right(text: &str, offset: usize) -> usize {
    let mut pos = offset;
    let mut chars = text[offset..].chars().peekable();
    while let Some(&c) = chars.peek() {
        if class(c) != Class::Space {
            break;
        }
        pos += c.len_utf8();
        chars.next();
    }
    if let Some(&first) = chars.peek() {
        let run = class(first);
        while let Some(&c) = chars.peek() {
            if class(c) != run {
                break;
            }
            pos += c.len_utf8();
            chars.next();
        }
    }
    pos
}

/// Start of the previous word before `offset`: skip whitespace backwards, then one run of the
/// same class.
fn word_left(text: &str, offset: usize) -> usize {
    let mut pos = offset;
    let mut chars = text[..offset].chars().rev().peekable();
    while let Some(&c) = chars.peek() {
        if class(c) != Class::Space {
            break;
        }
        pos -= c.len_utf8();
        chars.next();
    }
    if let Some(&first) = chars.peek() {
        let run = class(first);
        while let Some(&c) = chars.peek() {
            if class(c) != run {
                break;
            }
            pos -= c.len_utf8();
            chars.next();
        }
    }
    pos
}

/// Up (`down` false) or Down (`down` true) by one source line, keeping the goal column.
/// On the first line Up goes to the text start, and on the last line Down goes to the end.
fn vertical(text: &str, offset: usize, down: bool, goal_column: Option<usize>) -> Moved {
    let column = offset - line_start(text, offset);
    let goal = goal_column.unwrap_or(column);
    let offset = if down {
        let end = line_end(text, offset);
        if end == text.len() {
            text.len()
        } else {
            let next_start = end + 1;
            let next_end = line_end(text, next_start);
            char_floor(text, next_start + goal.min(next_end - next_start))
        }
    } else {
        let start = line_start(text, offset);
        if start == 0 {
            0
        } else {
            let prev_end = start - 1;
            let prev_start = line_start(text, prev_end);
            char_floor(text, prev_start + goal.min(prev_end - prev_start))
        }
    };
    Moved {
        offset,
        goal_column: Some(goal),
    }
}

impl Editor {
    /// Move the caret by `motion`, or move the selection head when `extend` is true.
    /// Called by the action handlers that [`attach`] wires (3.3).
    ///
    /// A plain Left, Right, WordLeft or WordRight from a non-empty selection collapses it to
    /// the edge in the direction of the motion, as text editors do. Every other motion moves
    /// from the selection head.
    pub fn move_caret(&mut self, motion: Motion, extend: bool, cx: &mut Context<Self>) {
        let current = self.selection.selection;
        let collapses = matches!(
            motion,
            Motion::Left | Motion::Right | Motion::WordLeft | Motion::WordRight
        );
        let text = self.text();
        // The hidden markers as drawn for the current caret (task 4.4). Left and Right skip them,
        // and the other motions snap onto a caret stop, so the caret never rests inside one.
        let map = DisplayMap::new(
            text.len(),
            document_hidden_ranges(&text, Some(current.head)),
        );
        let moved = if !extend && !current.is_empty() && collapses {
            let range = current.range();
            let offset = match motion {
                Motion::Left | Motion::WordLeft => range.start,
                _ => range.end,
            };
            Moved {
                offset: map.clip(offset),
                goal_column: None,
            }
        } else {
            match motion {
                Motion::Left => Moved {
                    offset: map.move_caret(&text, current.head, Direction::Left),
                    goal_column: None,
                },
                Motion::Right => Moved {
                    offset: map.move_caret(&text, current.head, Direction::Right),
                    goal_column: None,
                },
                Motion::WordLeft | Motion::WordRight => {
                    let word = move_offset(&text, current.head, motion, None);
                    Moved {
                        offset: map.clip(word.offset),
                        goal_column: None,
                    }
                }
                _ => move_offset(&text, current.head, motion, self.selection.goal_column),
            }
        };
        let anchor = if extend { current.anchor } else { moved.offset };
        // set_selection clears goal_column, so the goal is restored after it.
        self.set_selection(
            Selection {
                anchor,
                head: moved.offset,
            },
            cx,
        );
        self.selection.goal_column = moved.goal_column;
    }
}

/// Wire `f` to the root as the handler for action `A`. The handler receives the editor entity.
pub(crate) fn on<A: Action>(
    root: Div,
    editor: &Entity<Editor>,
    f: impl Fn(&mut Editor, &mut Context<Editor>) + 'static,
) -> Div {
    let editor = editor.clone();
    root.on_action(move |_: &A, _: &mut Window, cx: &mut App| {
        editor.update(cx, |editor, cx| f(editor, cx));
    })
}

/// Attach the movement, copy, cut and paste action handlers to the editor's root element.
///
/// Called from `Editor::render` (HOOK 3.3). Each action dispatches to `Editor::move_caret`
/// or to the clipboard methods.
pub fn attach(root: Div, editor: &Entity<Editor>) -> Div {
    let root = on::<MoveLeft>(root, editor, |e, cx| e.move_caret(Motion::Left, false, cx));
    let root = on::<MoveRight>(root, editor, |e, cx| e.move_caret(Motion::Right, false, cx));
    let root = on::<MoveUp>(root, editor, |e, cx| e.move_caret(Motion::Up, false, cx));
    let root = on::<MoveDown>(root, editor, |e, cx| e.move_caret(Motion::Down, false, cx));
    let root = on::<MoveWordLeft>(root, editor, |e, cx| {
        e.move_caret(Motion::WordLeft, false, cx)
    });
    let root = on::<MoveWordRight>(root, editor, |e, cx| {
        e.move_caret(Motion::WordRight, false, cx)
    });
    let root = on::<MoveLineStart>(root, editor, |e, cx| {
        e.move_caret(Motion::LineStart, false, cx)
    });
    let root = on::<MoveLineEnd>(root, editor, |e, cx| {
        e.move_caret(Motion::LineEnd, false, cx)
    });
    let root = on::<MoveDocStart>(root, editor, |e, cx| {
        e.move_caret(Motion::DocStart, false, cx)
    });
    let root = on::<MoveDocEnd>(root, editor, |e, cx| {
        e.move_caret(Motion::DocEnd, false, cx)
    });
    let root = on::<SelectLeft>(root, editor, |e, cx| e.move_caret(Motion::Left, true, cx));
    let root = on::<SelectRight>(root, editor, |e, cx| e.move_caret(Motion::Right, true, cx));
    let root = on::<SelectUp>(root, editor, |e, cx| e.move_caret(Motion::Up, true, cx));
    let root = on::<SelectDown>(root, editor, |e, cx| e.move_caret(Motion::Down, true, cx));
    let root = on::<SelectWordLeft>(root, editor, |e, cx| {
        e.move_caret(Motion::WordLeft, true, cx)
    });
    let root = on::<SelectWordRight>(root, editor, |e, cx| {
        e.move_caret(Motion::WordRight, true, cx)
    });
    let root = on::<SelectLineStart>(root, editor, |e, cx| {
        e.move_caret(Motion::LineStart, true, cx)
    });
    let root = on::<SelectLineEnd>(root, editor, |e, cx| {
        e.move_caret(Motion::LineEnd, true, cx)
    });
    let root = on::<SelectDocStart>(root, editor, |e, cx| {
        e.move_caret(Motion::DocStart, true, cx)
    });
    let root = on::<SelectDocEnd>(root, editor, |e, cx| e.move_caret(Motion::DocEnd, true, cx));
    let root = on::<Copy>(root, editor, |e, cx| e.copy_selection(cx));
    let root = on::<Cut>(root, editor, |e, cx| e.cut_selection(cx));
    on::<Paste>(root, editor, |e, cx| e.paste_clipboard(cx))
}

/// Register the default key bindings for the "MdEditor" key context.
///
/// Called from `editor::init` (HOOK 3.3). macOS uses the Cmd and Option modifiers; Linux uses
/// Ctrl. Home and End move to the line start and end on both platforms.
pub fn bind_keys(cx: &mut App) {
    let ctx = Some("MdEditor");
    let mut bindings = vec![
        KeyBinding::new("left", MoveLeft, ctx),
        KeyBinding::new("right", MoveRight, ctx),
        KeyBinding::new("up", MoveUp, ctx),
        KeyBinding::new("down", MoveDown, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("home", MoveLineStart, ctx),
        KeyBinding::new("end", MoveLineEnd, ctx),
        KeyBinding::new("shift-home", SelectLineStart, ctx),
        KeyBinding::new("shift-end", SelectLineEnd, ctx),
    ];
    #[cfg(target_os = "macos")]
    bindings.extend([
        KeyBinding::new("alt-left", MoveWordLeft, ctx),
        KeyBinding::new("alt-right", MoveWordRight, ctx),
        KeyBinding::new("shift-alt-left", SelectWordLeft, ctx),
        KeyBinding::new("shift-alt-right", SelectWordRight, ctx),
        KeyBinding::new("cmd-left", MoveLineStart, ctx),
        KeyBinding::new("cmd-right", MoveLineEnd, ctx),
        KeyBinding::new("shift-cmd-left", SelectLineStart, ctx),
        KeyBinding::new("shift-cmd-right", SelectLineEnd, ctx),
        KeyBinding::new("cmd-up", MoveDocStart, ctx),
        KeyBinding::new("cmd-down", MoveDocEnd, ctx),
        KeyBinding::new("shift-cmd-up", SelectDocStart, ctx),
        KeyBinding::new("shift-cmd-down", SelectDocEnd, ctx),
        KeyBinding::new("cmd-c", Copy, ctx),
        KeyBinding::new("cmd-x", Cut, ctx),
        KeyBinding::new("cmd-v", Paste, ctx),
    ]);
    #[cfg(not(target_os = "macos"))]
    bindings.extend([
        KeyBinding::new("ctrl-left", MoveWordLeft, ctx),
        KeyBinding::new("ctrl-right", MoveWordRight, ctx),
        KeyBinding::new("shift-ctrl-left", SelectWordLeft, ctx),
        KeyBinding::new("shift-ctrl-right", SelectWordRight, ctx),
        KeyBinding::new("ctrl-home", MoveDocStart, ctx),
        KeyBinding::new("ctrl-end", MoveDocEnd, ctx),
        KeyBinding::new("shift-ctrl-home", SelectDocStart, ctx),
        KeyBinding::new("shift-ctrl-end", SelectDocEnd, ctx),
        KeyBinding::new("ctrl-c", Copy, ctx),
        KeyBinding::new("ctrl-x", Cut, ctx),
        KeyBinding::new("ctrl-v", Paste, ctx),
    ]);
    cx.bind_keys(bindings);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str, offset: usize, motion: Motion) -> usize {
        move_offset(text, offset, motion, None).offset
    }

    #[test]
    fn left_and_right_move_by_one_character() {
        let text = "ab";
        assert_eq!(at(text, 0, Motion::Right), 1);
        assert_eq!(at(text, 1, Motion::Right), 2);
        assert_eq!(at(text, 2, Motion::Right), 2, "stops at the end");
        assert_eq!(at(text, 2, Motion::Left), 1);
        assert_eq!(at(text, 0, Motion::Left), 0, "stops at the start");
    }

    #[test]
    fn left_and_right_never_split_a_multibyte_character() {
        // "é" is 2 bytes, "日" is 3 bytes.
        let text = "é日x";
        assert_eq!(at(text, 0, Motion::Right), 2);
        assert_eq!(at(text, 2, Motion::Right), 5);
        assert_eq!(at(text, 5, Motion::Right), 6);
        assert_eq!(at(text, 6, Motion::Left), 5);
        assert_eq!(at(text, 5, Motion::Left), 2);
        assert_eq!(at(text, 2, Motion::Left), 0);
        // An offset inside a character snaps down to its start before moving.
        assert_eq!(at(text, 1, Motion::Right), 2);
    }

    #[test]
    fn left_and_right_step_over_an_emoji_as_one_character() {
        // The emoji is 4 bytes.
        let text = "a😀b";
        assert_eq!(at(text, 1, Motion::Right), 5);
        assert_eq!(at(text, 5, Motion::Left), 1);
        assert_eq!(at(text, 5, Motion::Right), 6);
    }

    #[test]
    fn left_and_right_cross_line_breaks_and_empty_lines() {
        // "a", empty line, "b": offsets 0 a, 1 \n, 2 \n, 3 b.
        let text = "a\n\nb";
        assert_eq!(at(text, 1, Motion::Right), 2);
        assert_eq!(at(text, 2, Motion::Right), 3);
        assert_eq!(at(text, 2, Motion::Left), 1);
        assert_eq!(at(text, 0, Motion::Right), 1);
        assert_eq!(at("", 0, Motion::Right), 0);
        assert_eq!(at("", 0, Motion::Left), 0);
    }

    #[test]
    fn word_right_moves_to_the_end_of_the_next_word() {
        let text = "hello, big world";
        assert_eq!(at(text, 0, Motion::WordRight), 5, "end of hello");
        assert_eq!(
            at(text, 5, Motion::WordRight),
            6,
            "the comma is its own run"
        );
        assert_eq!(at(text, 6, Motion::WordRight), 10, "skip space, end of big");
        assert_eq!(at(text, 10, Motion::WordRight), 16, "end of world");
        assert_eq!(at(text, 16, Motion::WordRight), 16, "stops at the end");
    }

    #[test]
    fn word_left_moves_to_the_start_of_the_previous_word() {
        let text = "hello, big world";
        assert_eq!(at(text, 16, Motion::WordLeft), 11, "start of world");
        assert_eq!(at(text, 11, Motion::WordLeft), 7, "start of big");
        assert_eq!(at(text, 7, Motion::WordLeft), 5, "the comma run");
        assert_eq!(at(text, 5, Motion::WordLeft), 0, "start of hello");
        assert_eq!(at(text, 0, Motion::WordLeft), 0);
    }

    #[test]
    fn word_motion_handles_multibyte_and_emoji_runs() {
        // "naïve" is 6 bytes, then a space, then "日本" (6 bytes).
        let text = "naïve 日本";
        assert_eq!(at(text, 0, Motion::WordRight), 6);
        assert_eq!(at(text, 6, Motion::WordRight), 13);
        assert_eq!(at(text, 13, Motion::WordLeft), 7);
        assert_eq!(at(text, 7, Motion::WordLeft), 0);
        // Emoji group as one run of symbols, then the word after it.
        let emoji = "😀😀 hi";
        assert_eq!(at(emoji, 0, Motion::WordRight), 8);
        assert_eq!(at(emoji, 8, Motion::WordRight), 11);
    }

    #[test]
    fn line_start_and_end_stop_at_line_breaks() {
        // "one" 0-2, \n at 3, "two words" 4-12, \n at 13, text ends at 14.
        let text = "one\ntwo words\n";
        assert_eq!(at(text, 6, Motion::LineStart), 4);
        assert_eq!(at(text, 6, Motion::LineEnd), 13);
        assert_eq!(at(text, 4, Motion::LineEnd), 13);
        assert_eq!(at(text, 0, Motion::LineEnd), 3);
        // The empty last line.
        assert_eq!(at(text, 14, Motion::LineStart), 14);
        assert_eq!(at(text, 14, Motion::LineEnd), 14);
    }

    #[test]
    fn document_start_and_end() {
        let text = "a\nbc";
        assert_eq!(at(text, 3, Motion::DocStart), 0);
        assert_eq!(at(text, 1, Motion::DocEnd), text.len());
        assert_eq!(at("", 0, Motion::DocEnd), 0);
    }

    #[test]
    fn up_and_down_keep_the_goal_column_across_a_short_line() {
        // "long line" 0-8, \n at 9, "x" at 10, \n at 11, "another long" 12-23, ends at 24.
        let text = "long line\nx\nanother long";
        // Column 7 on the first line. The short line ends at its only character (11).
        let down = move_offset(text, 7, Motion::Down, None);
        assert_eq!(down.offset, 11);
        assert_eq!(down.goal_column, Some(7));
        // Down again uses the goal column: column 7 of "another long" is 19.
        let down_again = move_offset(text, down.offset, Motion::Down, down.goal_column);
        assert_eq!(down_again.offset, 19);
        assert_eq!(down_again.goal_column, Some(7));
        // Up from 19 goes through the short line (11), keeping the goal.
        let up = move_offset(text, 19, Motion::Up, Some(7));
        assert_eq!(up.offset, 11);
        assert_eq!(up.goal_column, Some(7));
        // Up again returns to column 7 of the first line.
        let up_again = move_offset(text, up.offset, Motion::Up, up.goal_column);
        assert_eq!(up_again.offset, 7);
    }

    #[test]
    fn up_and_down_on_the_first_and_last_lines() {
        let text = "ab\ncd";
        // Up on the first line goes to the text start, Down on the last line to the end.
        assert_eq!(move_offset(text, 1, Motion::Up, None).offset, 0);
        assert_eq!(move_offset(text, 4, Motion::Down, None).offset, text.len());
        // Down from column 2 of the first line (the line break) lands at column 2 of "cd",
        // which is the text end.
        assert_eq!(move_offset(text, 2, Motion::Down, None).offset, 5);
    }

    #[test]
    fn up_and_down_never_split_a_multibyte_character() {
        // The goal column 2 lands inside "日" (bytes 5..8) on the second line, so it snaps to 5.
        let text = "abcd\n日本";
        assert_eq!(move_offset(text, 2, Motion::Down, None).offset, 5);
    }

    #[test]
    fn move_offset_clamps_an_offset_past_the_end() {
        assert_eq!(at("abc", 99, Motion::Left), 2);
        assert_eq!(at("é", 1, Motion::Right), 2);
    }
}
