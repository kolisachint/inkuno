//! Lists and templates: what Enter does on a line, list kinds, renumbering, template fields.

use super::*;

#[derive(PartialEq, Debug)]
pub enum Enter {
    /// Text to insert at the cursor.
    Insert(String),
    /// Empty item: the line is emptied to leave the list.
    Clear,
}

/// Behaviour of the Enter key on `line`, cursor at byte `col`.
pub fn on_enter(line: &str, col: usize, in_code: bool) -> Enter {
    let (kind, marker) = classify(line, in_code);
    let indent = &line[..indent_len(line)];
    let list = matches!(
        kind,
        Kind::Bullet | Kind::Ordered | Kind::Task(_) | Kind::Quote
    );
    if !list || col < marker {
        let keep = if col >= indent.len() { indent } else { "" };
        return Enter::Insert(format!("\n{keep}"));
    }
    if line[item_prefix(line)..].trim().is_empty() {
        return Enter::Clear;
    }
    let m = &line[indent.len()..marker];
    let next = match kind {
        Kind::Task(_) => format!("{}[ ] ", &m[..2]),
        Kind::Ordered => {
            let digits = m.len() - 2;
            let n: u64 = m[..digits].parse().unwrap_or(0);
            // An ordered task goes on as an unchecked one.
            let boxed = if task_box(line).is_some() { "[ ] " } else { "" };
            format!("{}{}{boxed}", n + 1, &m[digits..])
        }
        _ => m.to_string(),
    };
    Enter::Insert(format!("\n{indent}{next}"))
}

/// A filled template: `{{title}}`, `{{date}}` and `{{time}}` take their value (ignoring
/// case and spaces inside the braces), `{{cursor}}` marks where the cursor will go and disappears. Any other
/// `{{word}}`, or a brace never closed, stays as is; what is inserted is not reread. Returns the text
/// and the cursor's place (the end of the text without `{{cursor}}`).
pub fn fill(template: &str, title: &str, date: &str, time: &str) -> (String, usize) {
    let (mut out, mut cursor, mut rest) = (String::with_capacity(template.len()), None, template);
    while let Some(open) = rest.find("{{") {
        out.push_str(&rest[..open]);
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            rest = &rest[open..];
            break;
        };
        match after[..close].trim().to_lowercase().as_str() {
            "title" => out.push_str(title),
            "date" => out.push_str(date),
            "time" => out.push_str(time),
            "cursor" => {
                cursor.get_or_insert(out.len());
            }
            _ => out.push_str(&rest[open..open + 2 + close + 2]),
        }
        rest = &after[close + 2..];
    }
    out.push_str(rest);
    let at = cursor.unwrap_or(out.len());
    (out, at)
}

/// The note written as a template: its first `#` heading becomes `{{title}}` (and one is added at the top if there
/// is none), so that the note born from the template carries the name it is given.
pub fn templatize(text: &str) -> String {
    let start = front_matter(text);
    let (mut at, mut fenced) = (start, false);
    for line in text[start..].split_inclusive('\n') {
        let bare = line.trim_end_matches('\n');
        if is_fence(bare) {
            fenced = !fenced;
        }
        if !fenced
            && classify(bare, false).0 == Kind::Heading(1)
            && line.trim_start().starts_with("# ")
        {
            let end = at + line.trim_end_matches('\n').len();
            return format!("{}# {{{{title}}}}{}", &text[..at], &text[end..]);
        }
        at += line.len();
    }
    format!("{}# {{{{title}}}}\n\n{}", &text[..start], &text[start..])
}

/// What lines become: a list kind, or text.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ListStyle {
    Text,
    Bullet,
    Ordered,
    Task,
}

/// The lines of `block` (without a final line break) written in style `to`. Returns the new
/// text and, for each line, where its content starts before and after (in bytes from the start
/// of the line), to restore the selection. Only paragraphs and list items change:
/// not headings, tables, quotes, code blocks nor empty lines. If all touched lines
/// are already in this style, they become text again. Items keep their indent, a numbered
/// list restarts from 1 at each level, and a checked box stays checked from one task to the other.
pub fn restyle(block: &str, to: ListStyle) -> (String, Vec<(usize, usize)>) {
    let mut fenced = false;
    let lines: Vec<(&str, Option<Kind>)> = block
        .split('\n')
        .map(|line| {
            if is_fence(line) {
                fenced = !fenced;
                return (line, None);
            }
            if fenced || line.trim().is_empty() {
                return (line, None);
            }
            let kind = classify(line, false).0;
            (
                line,
                matches!(
                    kind,
                    Kind::Para | Kind::Bullet | Kind::Ordered | Kind::Task(_)
                )
                .then_some(kind),
            )
        })
        .collect();
    let same = |kind: Kind| match to {
        ListStyle::Text => kind == Kind::Para,
        ListStyle::Bullet => kind == Kind::Bullet,
        ListStyle::Ordered => kind == Kind::Ordered,
        ListStyle::Task => matches!(kind, Kind::Task(_)),
    };
    let touched = lines.iter().filter_map(|(_, kind)| *kind);
    let target = if touched.clone().next().is_some() && touched.clone().all(same) {
        ListStyle::Text
    } else {
        to
    };
    let (mut out, mut spans) = (
        Vec::with_capacity(lines.len()),
        Vec::with_capacity(lines.len()),
    );
    // Rank of the last item at each indent, for numbering.
    let mut counts: Vec<(usize, usize)> = Vec::new();
    for (line, kind) in lines {
        let Some(kind) = kind else {
            // An empty line lets numbering run on; any other block interrupts it.
            if !line.trim().is_empty() {
                counts.clear();
            }
            out.push(line.to_string());
            spans.push((0, 0));
            continue;
        };
        let indent = indent_len(line);
        let start = if kind == Kind::Para {
            indent
        } else {
            classify(line, false).1
        };
        let pad = if target == ListStyle::Text {
            ""
        } else {
            &line[..indent]
        };
        let marker = match target {
            ListStyle::Text => String::new(),
            ListStyle::Bullet => "- ".to_string(),
            ListStyle::Task => format!("- [{}] ", if kind == Kind::Task(true) { 'x' } else { ' ' }),
            ListStyle::Ordered => {
                while counts.last().is_some_and(|&(level, _)| level > indent) {
                    counts.pop();
                }
                match counts.last_mut() {
                    Some((level, n)) if *level == indent => *n += 1,
                    _ => counts.push((indent, 1)),
                }
                format!("{}. ", counts.last().map_or(1, |c| c.1))
            }
        };
        spans.push((start, pad.len() + marker.len()));
        out.push(format!("{pad}{marker}{}", &line[start..]));
    }
    (out.join("\n"), spans)
}

/// Renumbers the ordered list around the cursor. Returns the new text and
/// the adjusted cursor, or `None` if nothing changes.
pub fn renumber(text: &str, cursor: usize) -> Option<(String, usize)> {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut starts = Vec::with_capacity(lines.len());
    let mut off = 0;
    for l in &lines {
        starts.push(off);
        off += l.len() + 1;
    }
    let cur = starts.partition_point(|&s| s <= cursor).saturating_sub(1);
    let ordered = |i: usize| classify(lines[i], false).0 == Kind::Ordered;
    // After an item is deleted, the list to fix starts below the cursor.
    let anchor = [cur, cur + 1]
        .into_iter()
        .find(|&i| i < lines.len() && ordered(i))?;
    let level = indent_len(lines[anchor]);
    let member = |i: usize| {
        let l = lines[i];
        (ordered(i) && indent_len(l) == level) || (!l.trim().is_empty() && indent_len(l) > level)
    };
    let mut first = anchor;
    while first > 0 && member(first - 1) {
        first -= 1;
    }
    let mut last = anchor;
    while last + 1 < lines.len() && member(last + 1) {
        last += 1;
    }

    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let mut new_cursor = cursor as isize;
    let mut number = None;
    let mut changed = false;
    for i in first..=last {
        let l = lines[i];
        if !(ordered(i) && indent_len(l) == level) {
            continue;
        }
        let digits = l[level..].bytes().take_while(u8::is_ascii_digit).count();
        let n = match number {
            None => l[level..level + digits].parse::<u64>().unwrap_or(1),
            Some(prev) => prev + 1,
        };
        number = Some(n);
        let fixed = format!("{}{}{}", &l[..level], n, &l[level + digits..]);
        if fixed != l {
            if cursor > starts[i] + level {
                new_cursor += fixed.len() as isize - l.len() as isize;
            }
            out[i] = fixed;
            changed = true;
        }
    }
    changed.then(|| (out.join("\n"), new_cursor.max(0) as usize))
}

/// Width of the leading whitespace of `line`, in columns: a tab is four, as in the usual editors.
pub fn indent_columns(line: &str) -> usize {
    line.bytes()
        .take_while(|&b| b == b' ' || b == b'\t')
        .map(|b| if b == b'\t' { 4 } else { 1 })
        .sum()
}

/// Nesting level of a list item indented by `indent` columns, from the lines above it (nearest
/// first): the number of distinct smaller indents among the items it follows. Blank lines and
/// indented lines (the continuations of an item) are passed over; any other unindented line ends
/// the list, so that the items above it do not count.
pub fn list_depth<'a>(indent: usize, above: impl IntoIterator<Item = &'a str>) -> usize {
    // Nothing is less indented than column 0: a top-level item need not look above.
    if indent == 0 {
        return 0;
    }
    let mut levels: Vec<usize> = Vec::new();
    for line in above {
        if line.trim().is_empty() {
            continue;
        }
        let columns = indent_columns(line);
        if matches!(
            classify(line, false).0,
            Kind::Bullet | Kind::Ordered | Kind::Task(_)
        ) {
            if columns < indent && !levels.contains(&columns) {
                levels.push(columns);
            }
        } else if columns == 0 {
            break;
        }
    }
    levels.len()
}

/// The checkbox of a task line: the byte where its `[` is, and whether it is checked. A bullet
/// task's box ends its marker (`- [ ] `); an ordered task (`1. [ ] `, `1) [x] `) has its box after
/// the number, which its marker does not cover. Other lines: `None`.
pub fn task_box(line: &str) -> Option<(usize, bool)> {
    let (kind, marker) = classify(line, false);
    match kind {
        Kind::Task(done) => Some((marker - 4, done)),
        Kind::Ordered => match line.get(marker..marker + 4)? {
            "[ ] " => Some((marker, false)),
            "[x] " | "[X] " => Some((marker, true)),
            _ => None,
        },
        _ => None,
    }
}

/// Where the source prefix of a list item ends: its marker and, for an ordered task, its box.
/// Other lines: their marker.
pub fn item_prefix(line: &str) -> usize {
    match task_box(line) {
        Some((at, _)) => at + 4,
        None => classify(line, false).1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_follows_the_indents_above() {
        // Two spaces under a bullet, then four.
        assert_eq!(list_depth(2, ["- a"]), 1);
        assert_eq!(list_depth(4, ["  - b", "- a"]), 2);
        // Three spaces under `1. `: the number's width, not a level of its own.
        assert_eq!(list_depth(3, ["1. a"]), 1);
        assert_eq!(list_depth(6, ["   - b", "1. a"]), 2);
        // Four spaces, and a tab (four columns) for a level.
        assert_eq!(list_depth(4, ["- a"]), 1);
        assert_eq!(indent_columns("\t\t- c"), 8);
        assert_eq!(list_depth(indent_columns("\t- b"), ["- a"]), 1);
        assert_eq!(list_depth(indent_columns("\t\t- c"), ["\t- b", "- a"]), 2);
        // Siblings do not add a level, and a top-level item has none.
        assert_eq!(list_depth(3, ["   - c", "   - b", "1. a"]), 1);
        assert_eq!(list_depth(0, ["- a"]), 0);
    }

    #[test]
    fn depth_stops_at_a_line_that_ends_the_list() {
        // Blank lines and the continuations of an item belong to the list.
        assert_eq!(list_depth(4, ["", "  continued", "- a"]), 1);
        // An unindented paragraph ends it: the items above do not count.
        assert_eq!(list_depth(2, ["Intro", "- a"]), 0);
        assert_eq!(list_depth(2, ["# Title", "- a"]), 0);
    }

    #[test]
    fn task_boxes_and_prefixes() {
        assert_eq!(task_box("- [ ] a"), Some((2, false)));
        assert_eq!(task_box("  - [x] a"), Some((4, true)));
        assert_eq!(task_box("1. [ ] a"), Some((3, false)));
        assert_eq!(task_box("10) [X] a"), Some((4, true)));
        assert_eq!(task_box("1. a"), None);
        assert_eq!(task_box("- a"), None);
        // An ordered task's prefix runs over its box; the others end at their marker.
        assert_eq!(item_prefix("- [ ] a"), 6);
        assert_eq!(item_prefix("1. [ ] a"), 7);
        assert_eq!(item_prefix("1. a"), 3);
        assert_eq!(item_prefix("  - a"), 4);
        assert_eq!(item_prefix("para"), 0);
    }

    #[test]
    fn enter_carries_an_ordered_task_on() {
        assert_eq!(
            on_enter("1. [ ] a", 8, false),
            Enter::Insert("\n2. [ ] ".to_string())
        );
        assert_eq!(
            on_enter("1. [x] a", 8, false),
            Enter::Insert("\n2. [ ] ".to_string())
        );
        assert_eq!(on_enter("1. [ ] ", 7, false), Enter::Clear);
        assert_eq!(
            on_enter("1. a", 4, false),
            Enter::Insert("\n2. ".to_string())
        );
    }
}
