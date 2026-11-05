//! Input rules for markdown auto-formatting (task 2.7, DESIGN §4.3).
//!
//! Every rule is a pure function of `(text, caret, input)` that returns text edits
//! on `text`. Rules only touch the caret's line, which is always inside the current
//! block (the run of non-blank lines around the caret). The engine works on `&str`
//! only and does not depend on the buffer type.

use crate::Edit;

/// A typed or structural input the rules react to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Text(String),
    Enter,
    Backspace,
}

/// The outcome of a matching rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleResult {
    pub edits: Vec<Edit>,
    pub caret: usize,
    pub rule: &'static str,
}

/// Apply the first matching input rule. `caret` is a byte offset into `text`.
/// Returns None if no rule matches (the caller then performs the default insertion/deletion).
/// Edits are relative to the original `text`; `caret` is the offset in the text after applying the edits.
pub fn apply_input_rule(text: &str, caret: usize, input: &Input) -> Option<RuleResult> {
    if !text.is_char_boundary(caret) {
        return None;
    }
    let line = Line::at(text, caret);
    match input {
        Input::Char(c) => typed_char_rule(line, *c),
        Input::Text(s) => {
            // Only a single character (for example an IME commit of a space) can trigger a rule.
            let mut chars = s.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => typed_char_rule(line, c),
                _ => None,
            }
        }
        Input::Enter => enter_rule(line),
        Input::Backspace => backspace_rule(line),
    }
}

/// The line under the caret. Offsets are absolute byte offsets into `text`.
#[derive(Clone, Copy)]
struct Line<'a> {
    text: &'a str,
    caret: usize,
    /// Offset of the first byte of the line.
    start: usize,
    /// Offset of the newline ending the line (or `text.len()`).
    end: usize,
}

impl<'a> Line<'a> {
    fn at(text: &'a str, caret: usize) -> Self {
        let start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
        let end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
        Self {
            text,
            caret,
            start,
            end,
        }
    }

    /// The line without its newline.
    fn content(&self) -> &'a str {
        &self.text[self.start..self.end]
    }

    /// The line from its start up to the caret.
    fn before_caret(&self) -> &'a str {
        &self.text[self.start..self.caret]
    }

    fn caret_at_end(&self) -> bool {
        self.caret == self.end
    }

    /// Everything above the line.
    fn above(&self) -> &'a str {
        &self.text[..self.start]
    }

    /// Everything below the line.
    fn below(&self) -> &'a str {
        self.text.get(self.end + 1..).unwrap_or("")
    }
}

/// Offset of the first line of the current block. The block is the run of non-blank
/// lines that contains the line starting at `line_start`.
fn block_start(text: &str, line_start: usize) -> usize {
    let mut start = line_start;
    while start > 0 {
        let prev_end = start - 1; // the newline ending the previous line
        let prev_start = text[..prev_end].rfind('\n').map_or(0, |i| i + 1);
        if text[prev_start..prev_end].trim().is_empty() {
            break;
        }
        start = prev_start;
    }
    start
}

fn insert_at(line: Line<'_>, text: &str, rule: &'static str) -> RuleResult {
    RuleResult {
        edits: vec![Edit::insert(line.caret, text)],
        caret: line.caret + text.len(),
        rule,
    }
}

// ---------------------------------------------------------------------------
// Markers and prefixes
// ---------------------------------------------------------------------------

fn is_heading_marker(s: &str) -> bool {
    (1..=6).contains(&s.len()) && s.bytes().all(|b| b == b'#')
}

fn is_ordered_marker(s: &str) -> bool {
    s.strip_suffix('.')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// Bullet or ordered marker, without indentation and without the trailing space.
fn is_list_marker(s: &str) -> bool {
    matches!(s, "-" | "*") || is_ordered_marker(s)
}

fn is_task_marker(s: &str) -> bool {
    matches!(s, "- [ ]" | "- [x]" | "- [X]")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// `-` or `*`.
    Bullet(char),
    Ordered(u64),
    /// `- [ ]`, `- [x]` or `- [X]`; the char is the bullet.
    Task(char),
    Quote,
}

/// The list or quote prefix of a line.
#[derive(Debug, Clone, Copy)]
struct Prefix<'a> {
    indent: &'a str,
    kind: Kind,
    /// Byte length of the prefix, indentation included, marker and following space(s) included.
    marker_end: usize,
    /// The text after the prefix.
    body: &'a str,
}

fn parse_prefix(line: &str) -> Option<Prefix<'_>> {
    let indent_len = line.len() - line.trim_start_matches([' ', '\t']).len();
    let indent = &line[..indent_len];
    let rest = &line[indent_len..];

    if let Some(body) = rest.strip_prefix("> ") {
        return Some(Prefix {
            indent,
            kind: Kind::Quote,
            marker_end: indent_len + 2,
            body,
        });
    }

    let bullet = match rest.as_bytes() {
        [b @ (b'-' | b'*'), b' ', ..] => Some(char::from(*b)),
        _ => None,
    };
    if let Some(bullet) = bullet {
        let after = &rest[2..];
        for boxed in ["[ ] ", "[x] ", "[X] "] {
            if let Some(body) = after.strip_prefix(boxed) {
                return Some(Prefix {
                    indent,
                    kind: Kind::Task(bullet),
                    marker_end: indent_len + 2 + boxed.len(),
                    body,
                });
            }
        }
        if matches!(after, "[ ]" | "[x]" | "[X]") {
            return Some(Prefix {
                indent,
                kind: Kind::Task(bullet),
                marker_end: line.len(),
                body: "",
            });
        }
        return Some(Prefix {
            indent,
            kind: Kind::Bullet(bullet),
            marker_end: indent_len + 2,
            body: after,
        });
    }

    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        if let Some(body) = rest[digits..].strip_prefix(". ") {
            let number = rest[..digits].parse::<u64>().ok()?;
            return Some(Prefix {
                indent,
                kind: Kind::Ordered(number),
                marker_end: indent_len + digits + 2,
                body,
            });
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Typed characters
// ---------------------------------------------------------------------------

/// Typing a space right after a block marker at the start of a line.
fn typed_char_rule(line: Line<'_>, c: char) -> Option<RuleResult> {
    if c != ' ' {
        return None;
    }
    let before = line.before_caret();
    let unindented = before.trim_start_matches([' ', '\t']);
    let rule = if is_heading_marker(before) {
        "heading"
    } else if before == ">" {
        "blockquote"
    } else if is_task_marker(unindented) {
        "task"
    } else if is_list_marker(unindented) {
        "list"
    } else {
        return None;
    };
    Some(insert_at(line, " ", rule))
}

// ---------------------------------------------------------------------------
// Enter
// ---------------------------------------------------------------------------

fn enter_rule(line: Line<'_>) -> Option<RuleResult> {
    if !line.caret_at_end() {
        return None;
    }
    code_fence(line)
        .or_else(|| math_block(line))
        .or_else(|| thematic_break(line))
        .or_else(|| list_exit(line))
        .or_else(|| list_continue(line))
}

fn is_code_fence_line(l: &str) -> bool {
    l.trim_start().starts_with("```")
}

fn is_math_delimiter_line(l: &str) -> bool {
    l.trim() == "$$"
}

/// True if a block delimited by `is_delim` lines must not be opened on this line:
/// either the line closes an open block (odd number of delimiters above), or the block
/// is already closed further down.
fn delimiter_already_used(line: Line<'_>, is_delim: fn(&str) -> bool) -> bool {
    let above = line.above().lines().filter(|l| is_delim(l)).count();
    above % 2 == 1 || line.below().lines().any(is_delim)
}

/// Inserts an empty middle line and a closing delimiter below the line, caret on the middle line.
fn wrap_block(line: Line<'_>, delim: &str, rule: &'static str) -> RuleResult {
    let content = line.content();
    let indent = &content[..content.len() - content.trim_start().len()];
    let inserted = format!("\n{indent}\n{indent}{delim}");
    RuleResult {
        edits: vec![Edit::insert(line.end, inserted)],
        caret: line.end + 1 + indent.len(),
        rule,
    }
}

/// "```" with an optional info string, then Enter.
fn code_fence(line: Line<'_>) -> Option<RuleResult> {
    let info = line.content().trim_start().strip_prefix("```")?;
    if info.contains('`') {
        return None;
    }
    if delimiter_already_used(line, is_code_fence_line) {
        return None;
    }
    Some(wrap_block(line, "```", "code_fence"))
}

/// "$$" on its own line, then Enter.
fn math_block(line: Line<'_>) -> Option<RuleResult> {
    if !is_math_delimiter_line(line.content()) {
        return None;
    }
    if delimiter_already_used(line, is_math_delimiter_line) {
        return None;
    }
    Some(wrap_block(line, "$$", "math_block"))
}

/// "---" on its own, starting its block (a `---` under a text line is a setext heading), then Enter.
fn thematic_break(line: Line<'_>) -> Option<RuleResult> {
    if line.content().trim() != "---" {
        return None;
    }
    if block_start(line.text, line.start) != line.start {
        return None;
    }
    Some(RuleResult {
        edits: vec![Edit::insert(line.end, "\n\n")],
        caret: line.end + 2,
        rule: "thematic_break",
    })
}

/// Enter on an empty list item: remove the marker, or outdent one level when indented.
fn list_exit(line: Line<'_>) -> Option<RuleResult> {
    let prefix = parse_prefix(line.content())?;
    if !prefix.body.trim().is_empty() || prefix.kind == Kind::Quote {
        return None;
    }
    if prefix.indent.is_empty() {
        return Some(RuleResult {
            edits: vec![Edit::delete(line.start..line.end)],
            caret: line.start,
            rule: "list_exit",
        });
    }
    // Nested empty item: remove one tab, or up to two spaces, from the end of the indentation.
    let indent_end = line.start + prefix.indent.len();
    let trailing_spaces = prefix.indent.len() - prefix.indent.trim_end_matches(' ').len();
    let outdent = if prefix.indent.ends_with('\t') {
        1
    } else {
        trailing_spaces.min(2)
    };
    Some(RuleResult {
        edits: vec![Edit::delete(indent_end - outdent..indent_end)],
        caret: line.end - outdent,
        rule: "list_exit",
    })
}

/// Enter at the end of a non-empty list item or quote line: start the next item on a new line.
fn list_continue(line: Line<'_>) -> Option<RuleResult> {
    let prefix = parse_prefix(line.content())?;
    if prefix.body.trim().is_empty() {
        return None;
    }
    let next = match prefix.kind {
        Kind::Bullet(c) => format!("{c} "),
        Kind::Task(c) => format!("{c} [ ] "),
        Kind::Ordered(n) => format!("{}. ", n.checked_add(1)?),
        Kind::Quote => "> ".to_string(),
    };
    let inserted = format!("\n{}{next}", prefix.indent);
    Some(RuleResult {
        caret: line.end + inserted.len(),
        edits: vec![Edit::insert(line.end, inserted)],
        rule: "list_continue",
    })
}

// ---------------------------------------------------------------------------
// Backspace
// ---------------------------------------------------------------------------

/// Backspace right after a block marker removes the marker. A task becomes a plain list item.
fn backspace_rule(line: Line<'_>) -> Option<RuleResult> {
    let before = line.before_caret();
    let before_space = before.strip_suffix(' ')?;

    if is_heading_marker(before_space) {
        return Some(RuleResult {
            edits: vec![Edit::delete(line.start..line.caret)],
            caret: line.start,
            rule: "demote",
        });
    }

    let prefix = parse_prefix(before)?;
    if prefix.marker_end != before.len() {
        return None;
    }
    let marker_start = line.start + prefix.indent.len();
    let (edit, caret) = match prefix.kind {
        Kind::Task(_) => {
            // Keep the bullet, drop the "[ ] " box.
            let box_start = marker_start + 2;
            (Edit::delete(box_start..line.caret), box_start)
        }
        _ => (Edit::delete(marker_start..line.caret), marker_start),
    };
    Some(RuleResult {
        edits: vec![edit],
        caret,
        rule: "demote",
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply_edits;

    /// Splits text with one `|` caret marker into the plain text and the caret offset.
    fn split(marked: &str) -> (String, usize) {
        assert_eq!(
            marked.matches('|').count(),
            1,
            "need exactly one caret marker in {marked:?}"
        );
        let caret = marked.find('|').unwrap();
        (
            format!("{}{}", &marked[..caret], &marked[caret + 1..]),
            caret,
        )
    }

    /// Runs the rules on `before` and returns (resulting text, caret, rule name).
    fn run(before: &str, input: Input) -> Option<(String, usize, &'static str)> {
        let (text, caret) = split(before);
        let result = apply_input_rule(&text, caret, &input)?;
        // Edits must stay on the caret's line, which lies inside the current block.
        let line_start = text[..caret].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[caret..].find('\n').map_or(text.len(), |i| caret + i);
        for edit in &result.edits {
            assert!(
                line_start <= edit.range.start && edit.range.end <= line_end,
                "edit {edit:?} is outside the caret line of {before:?}"
            );
        }
        Some((apply_edits(&text, &result.edits), result.caret, result.rule))
    }

    /// Asserts that `input` at the caret of `before` yields `after` (with its caret marker) and `rule`.
    fn expect(before: &str, input: Input, after: &str, rule: &str) {
        let (want_text, want_caret) = split(after);
        let (text, caret, got_rule) =
            run(before, input).unwrap_or_else(|| panic!("no rule matched {before:?}"));
        assert_eq!(
            (text.as_str(), caret, got_rule),
            (want_text.as_str(), want_caret, rule),
            "input on {before:?}"
        );
    }

    /// Asserts that no rule matches `input` at the caret of `before`.
    fn none(before: &str, input: Input) {
        assert!(
            run(before, input.clone()).is_none(),
            "expected no rule for {before:?} with {input:?}"
        );
    }

    fn space() -> Input {
        Input::Char(' ')
    }

    #[test]
    fn heading_marker_then_space() {
        expect("#|", space(), "# |", "heading");
        expect("###|", space(), "### |", "heading");
        expect("######|", space(), "###### |", "heading");
        expect("intro\n\n##|", space(), "intro\n\n## |", "heading");
    }

    #[test]
    fn heading_negative_cases() {
        none("#######|", space()); // seven hashes is not a heading
        none("foo #|", space()); // not at the start of the line
    }

    #[test]
    fn list_marker_then_space() {
        expect("-|", space(), "- |", "list");
        expect("*|", space(), "* |", "list");
        expect("1.|", space(), "1. |", "list");
        expect("  -|", space(), "  - |", "list");
        none("foo -|", space());
    }

    #[test]
    fn blockquote_marker_then_space() {
        expect(">|", space(), "> |", "blockquote");
        none("a >|", space());
    }

    #[test]
    fn task_marker_then_space() {
        expect("- [ ]|", space(), "- [ ] |", "task");
        expect("- [x]|", space(), "- [x] |", "task");
        none("- foo [ ]|", space());
    }

    #[test]
    fn single_character_text_acts_like_a_char() {
        expect("#|", Input::Text(" ".into()), "# |", "heading");
        none("#|", Input::Text("ab".into()));
        none("#|", Input::Text(String::new()));
    }

    #[test]
    fn code_fence_enter_opens_block() {
        expect("```rust|", Input::Enter, "```rust\n|\n```", "code_fence");
        expect("```|", Input::Enter, "```\n|\n```", "code_fence");
        expect(
            "intro\n\n```rust|\n",
            Input::Enter,
            "intro\n\n```rust\n|\n```\n",
            "code_fence",
        );
    }

    #[test]
    fn code_fence_negative_cases() {
        none("```\ncode\n```|", Input::Enter); // closing fence
        none("```|\ncode\n```", Input::Enter); // fence already closed below
        none("```ru|st", Input::Enter); // caret not at line end
        none("```a`b|", Input::Enter); // backticks in the info string
    }

    #[test]
    fn math_block_enter_opens_block() {
        expect("$$|", Input::Enter, "$$\n|\n$$", "math_block");
    }

    #[test]
    fn math_block_negative_cases() {
        none("$$\nx\n$$|", Input::Enter); // closing delimiter
        none("$$x|", Input::Enter);
    }

    #[test]
    fn thematic_break_enter() {
        expect("---|", Input::Enter, "---\n\n|", "thematic_break");
        expect(
            "intro\n\n---|",
            Input::Enter,
            "intro\n\n---\n\n|",
            "thematic_break",
        );
    }

    #[test]
    fn thematic_break_under_text_is_setext_not_a_rule() {
        none("intro\n---|", Input::Enter);
    }

    #[test]
    fn list_continue_bullet_and_ordered() {
        expect("- foo|", Input::Enter, "- foo\n- |", "list_continue");
        expect("* foo|", Input::Enter, "* foo\n* |", "list_continue");
        expect("3. foo|", Input::Enter, "3. foo\n4. |", "list_continue");
        expect(
            "intro\n- foo|\nafter",
            Input::Enter,
            "intro\n- foo\n- |\nafter",
            "list_continue",
        );
    }

    #[test]
    fn list_continue_task_quote_and_indent() {
        expect(
            "- [x] foo|",
            Input::Enter,
            "- [x] foo\n- [ ] |",
            "list_continue",
        );
        expect(
            "- [ ] foo|",
            Input::Enter,
            "- [ ] foo\n- [ ] |",
            "list_continue",
        );
        expect("  - foo|", Input::Enter, "  - foo\n  - |", "list_continue");
        expect("> foo|", Input::Enter, "> foo\n> |", "list_continue");
    }

    #[test]
    fn list_continue_uses_byte_offsets() {
        expect("- 日本|", Input::Enter, "- 日本\n- |", "list_continue");
    }

    #[test]
    fn enter_on_empty_list_item_exits_the_list() {
        expect("- foo\n- |", Input::Enter, "- foo\n|", "list_exit");
        expect(
            "- foo\n- |\nafter",
            Input::Enter,
            "- foo\n|\nafter",
            "list_exit",
        );
        expect("1. |", Input::Enter, "|", "list_exit");
        expect("- [ ] |", Input::Enter, "|", "list_exit");
    }

    #[test]
    fn enter_on_indented_empty_item_outdents_one_level() {
        expect("- foo\n  - |", Input::Enter, "- foo\n- |", "list_exit");
        expect("- a\n    - |", Input::Enter, "- a\n  - |", "list_exit");
        expect("- a\n\t- |", Input::Enter, "- a\n- |", "list_exit");
    }

    #[test]
    fn enter_negative_cases() {
        none("plain paragraph|", Input::Enter);
        none("- fo|o", Input::Enter); // caret not at line end
        none("> |", Input::Enter); // empty quote: default newline
    }

    #[test]
    fn backspace_demotes_heading_and_quote() {
        expect("## |x", Input::Backspace, "|x", "demote");
        expect("> |x", Input::Backspace, "|x", "demote");
    }

    #[test]
    fn backspace_demotes_list_items() {
        expect("- |x", Input::Backspace, "|x", "demote");
        expect("1. |x", Input::Backspace, "|x", "demote");
        expect("- |", Input::Backspace, "|", "demote");
    }

    #[test]
    fn backspace_turns_task_into_plain_item() {
        expect("- [ ] |x", Input::Backspace, "- |x", "demote");
        expect("- [x] |x", Input::Backspace, "- |x", "demote");
    }

    #[test]
    fn backspace_keeps_indentation_when_demoting() {
        expect("intro\n  - |x", Input::Backspace, "intro\n  |x", "demote");
    }

    #[test]
    fn backspace_negative_cases() {
        none("## x|", Input::Backspace); // caret not right after the marker
        none("- x|", Input::Backspace);
        none("foo |", Input::Backspace); // not a marker
        none("|x", Input::Backspace);
    }

    /// The acceptance cases of task 4.5, written out in full.
    #[test]
    fn acceptance_cases_for_task_4_5() {
        expect("#|", space(), "# |", "heading");
        expect("- item|", Input::Enter, "- item\n- |", "list_continue");
        expect("- |", Input::Enter, "|", "list_exit");
        expect("1. a|", Input::Enter, "1. a\n2. |", "list_continue");
        expect("## |x", Input::Backspace, "|x", "demote");
    }

    #[test]
    fn caret_inside_multibyte_char_is_rejected() {
        assert!(apply_input_rule("é", 1, &Input::Enter).is_none());
    }
}
