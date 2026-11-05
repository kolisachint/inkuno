//! Block splitter for top-level blocks (task 2.3).
//!
//! The scanner is forward-only: the block that starts at a line boundary is
//! decided by the text from that boundary onwards and nothing before it. That
//! is what lets [`BlockMap::apply_edit`] re-scan only a window around an edit
//! and resume with the old blocks as soon as a freshly scanned boundary lines
//! up with an old one. The result is always identical to [`split_blocks`] on
//! the new text.

use std::ops::Range;

use crate::{Block, BlockKind, Edit};

/// A physical line. `text` excludes the terminating `\n`; `end` is the offset just past it.
#[derive(Clone, Copy)]
struct Line<'a> {
    text: &'a str,
    end: usize,
}

fn lines_from<'a>(src: &'a str, pos: usize) -> impl Iterator<Item = Line<'a>> + Clone + 'a {
    let mut offset = pos;
    src[pos..].split_inclusive('\n').map(move |raw| {
        offset += raw.len();
        Line {
            text: raw.strip_suffix('\n').unwrap_or(raw),
            end: offset,
        }
    })
}

/// Forward scanner over blocks, starting at a line boundary `pos`.
struct Scanner<'a> {
    src: &'a str,
    pos: usize,
}

impl Iterator for Scanner<'_> {
    type Item = Block;

    fn next(&mut self) -> Option<Block> {
        if self.pos >= self.src.len() {
            return None;
        }
        let (kind, end) = scan_block(self.src, self.pos);
        let range = self.pos..end;
        self.pos = end;
        Some(Block { kind, range })
    }
}

/// Split whole text into top-level blocks. Ranges tile `0..text.len()` exactly;
/// a block's range includes its trailing newline. Runs of blank lines are
/// `BlockKind::Blank` blocks.
pub fn split_blocks(text: &str) -> Vec<Block> {
    Scanner { src: text, pos: 0 }.collect()
}

/// Decide the block starting at line boundary `pos` (which must be `< src.len()`).
/// Returns its kind and the end offset (exclusive, after the last line's newline).
fn scan_block(src: &str, pos: usize) -> (BlockKind, usize) {
    let lines = lines_from(src, pos);
    let mut it = lines.clone();
    let first = it.next().expect("scan_block requires pos < src.len()");
    let t = first.text;

    if is_blank(t) {
        let mut end = first.end;
        for l in it {
            if !is_blank(l.text) {
                break;
            }
            end = l.end;
        }
        return (BlockKind::Blank, end);
    }
    if let Some(level) = atx_heading(t) {
        return (BlockKind::Heading(level), first.end);
    }
    if let Some((fence, len)) = fence_open(t) {
        let mut end = first.end;
        for l in it {
            end = l.end;
            if fence_close(l.text, fence, len) {
                break;
            }
        }
        return (BlockKind::CodeFence, end);
    }
    if let Some(rest) = math_open(t) {
        let mut end = first.end;
        if !rest.contains("$$") {
            for l in it {
                end = l.end;
                if l.text.trim_end().ends_with("$$") {
                    break;
                }
            }
        }
        return (BlockKind::MathBlock, end);
    }
    if thematic_break(t) {
        return (BlockKind::ThematicBreak, first.end);
    }
    if quote_line(t) {
        let mut end = first.end;
        for l in it {
            // `>` lines, or a non-blank lazy continuation of the quoted paragraph.
            if quote_line(l.text) || (!is_blank(l.text) && !interrupts_paragraph(l.text)) {
                end = l.end;
            } else {
                break;
            }
        }
        return (BlockKind::BlockQuote, end);
    }
    if list_marker(t).is_some() {
        return (BlockKind::List, list_end(first, it));
    }
    if table_at(lines.clone()) {
        // The delimiter row is the second line; rows continue until a blank or another block start.
        let delim = it.next().expect("table_at checked the delimiter row");
        let mut end = delim.end;
        for l in it {
            if is_blank(l.text) || interrupts_paragraph(l.text) {
                break;
            }
            end = l.end;
        }
        return (BlockKind::Table, end);
    }
    if html_open(t) {
        let mut end = first.end;
        if t.trim_start().starts_with("<!--") {
            if !t.contains("-->") {
                for l in it {
                    end = l.end;
                    if l.text.contains("-->") {
                        break;
                    }
                }
            }
        } else {
            for l in it {
                if is_blank(l.text) {
                    break;
                }
                end = l.end;
            }
        }
        return (BlockKind::HtmlBlock, end);
    }

    // Paragraph, possibly turned into a setext heading by an underline.
    let mut end = first.end;
    let mut kind = BlockKind::Paragraph;
    loop {
        let Some(l) = it.clone().next() else { break };
        if is_blank(l.text) {
            break;
        }
        if let Some(level) = setext_underline(l.text) {
            kind = BlockKind::Heading(level);
            end = l.end;
            break;
        }
        if interrupts_paragraph(l.text) || table_at(it.clone()) {
            break;
        }
        end = l.end;
        it.next();
    }
    (kind, end)
}

/// End of a list block that starts with `first`. Items, indented continuation
/// lines and lazy paragraph lines belong to the list; blank lines belong to it
/// only when another item or an indented line follows them.
fn list_end<'a>(first: Line<'a>, mut it: impl Iterator<Item = Line<'a>> + Clone) -> usize {
    let mut end = first.end;
    loop {
        let Some(l) = it.clone().next() else { break };
        if is_blank(l.text) {
            match it.clone().find(|n| !is_blank(n.text)) {
                Some(n) if list_marker(n.text).is_some() || indent(n.text) >= 1 => {
                    it.next();
                }
                _ => break,
            }
        } else if list_continues(l.text) {
            end = l.end;
            it.next();
        } else {
            break;
        }
    }
    end
}

fn list_continues(t: &str) -> bool {
    if thematic_break(t) {
        return false;
    }
    if list_marker(t).is_some() || indent(t) >= 1 {
        return true;
    }
    // Lazy continuation of an item's paragraph; anything that starts a block ends the list.
    !(atx_heading(t).is_some()
        || fence_open(t).is_some()
        || quote_line(t)
        || html_open(t)
        || math_open(t).is_some())
}

/// True when the lines starting at the iterator's head form a table header row
/// followed by a delimiter row.
fn table_at<'a>(mut la: impl Iterator<Item = Line<'a>>) -> bool {
    match (la.next(), la.next()) {
        (Some(header), Some(delim)) => table_header(header.text) && table_delim(delim.text),
        _ => false,
    }
}

fn table_header(t: &str) -> bool {
    !is_blank(t) && t.contains('|')
}

fn table_delim(t: &str) -> bool {
    let r = t.trim();
    if !r.contains('|') || !r.contains('-') {
        return false;
    }
    if r.chars()
        .any(|c| !matches!(c, '|' | '-' | ':' | ' ' | '\t'))
    {
        return false;
    }
    let cells: Vec<&str> = r.split('|').collect();
    let mut count = 0;
    for (i, cell) in cells.iter().enumerate() {
        let c = cell.trim();
        if c.is_empty() {
            // Only the outer cells may be empty (leading or trailing pipe).
            if i == 0 || i == cells.len() - 1 {
                continue;
            }
            return false;
        }
        let inner = c.trim_start_matches(':').trim_end_matches(':');
        if inner.is_empty() || !inner.chars().all(|ch| ch == '-') {
            return false;
        }
        count += 1;
    }
    count > 0
}

/// Lines that end a paragraph (CommonMark interruption rules, simplified).
fn interrupts_paragraph(t: &str) -> bool {
    atx_heading(t).is_some()
        || fence_open(t).is_some()
        || thematic_break(t)
        || quote_line(t)
        || html_open(t)
        || math_open(t).is_some()
        || list_marker(t).is_some_and(|m| m.has_content && (!m.ordered || m.number == 1))
}

fn indent(s: &str) -> usize {
    let mut width = 0;
    for c in s.chars() {
        match c {
            ' ' => width += 1,
            '\t' => width += 4 - width % 4,
            _ => break,
        }
    }
    width
}

fn strip_indent(s: &str) -> &str {
    s.trim_start_matches([' ', '\t'])
}

fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

fn atx_heading(t: &str) -> Option<u8> {
    if indent(t) > 3 {
        return None;
    }
    let r = strip_indent(t);
    let hashes = r.bytes().take_while(|&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    let after = &r[hashes..];
    (after.is_empty() || after.starts_with([' ', '\t', '\r'])).then_some(hashes as u8)
}

fn fence_open(t: &str) -> Option<(char, usize)> {
    if indent(t) > 3 {
        return None;
    }
    let r = strip_indent(t);
    let fence = r.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let len = r.chars().take_while(|&c| c == fence).count();
    if len < 3 {
        return None;
    }
    if fence == '`' && r[len..].contains('`') {
        return None;
    }
    Some((fence, len))
}

fn fence_close(t: &str, fence: char, len: usize) -> bool {
    if indent(t) > 3 {
        return false;
    }
    let r = strip_indent(t);
    let run = r.chars().take_while(|&c| c == fence).count();
    run >= len && r[run..].trim().is_empty()
}

/// `$$` opener; returns the text after it on the same line.
fn math_open(t: &str) -> Option<&str> {
    if indent(t) > 3 {
        return None;
    }
    strip_indent(t).strip_prefix("$$")
}

fn thematic_break(t: &str) -> bool {
    if indent(t) > 3 {
        return false;
    }
    let mut marker: Option<char> = None;
    let mut count = 0;
    for c in t.trim().chars() {
        match c {
            ' ' | '\t' => {}
            '-' | '*' | '_' if marker.is_none() || marker == Some(c) => {
                marker = Some(c);
                count += 1;
            }
            _ => return false,
        }
    }
    count >= 3
}

/// Setext underline: a run of `=` (level 1) or `-` (level 2).
fn setext_underline(t: &str) -> Option<u8> {
    if indent(t) > 3 {
        return None;
    }
    let r = t.trim();
    if r.is_empty() {
        None
    } else if r.chars().all(|c| c == '=') {
        Some(1)
    } else if r.chars().all(|c| c == '-') {
        Some(2)
    } else {
        None
    }
}

fn quote_line(t: &str) -> bool {
    indent(t) <= 3 && strip_indent(t).starts_with('>')
}

fn html_open(t: &str) -> bool {
    if indent(t) > 3 {
        return false;
    }
    let r = strip_indent(t);
    let mut chars = r.chars();
    if chars.next() != Some('<') {
        return false;
    }
    match chars.next() {
        Some('!') => r.starts_with("<!--"),
        Some('/') => chars.next().is_some_and(|c| c.is_ascii_alphabetic()),
        Some(c) => c.is_ascii_alphabetic(),
        None => false,
    }
}

#[derive(Clone, Copy)]
struct ListMarker {
    ordered: bool,
    number: u64,
    has_content: bool,
}

fn list_marker(t: &str) -> Option<ListMarker> {
    let r = strip_indent(t);
    let bytes = r.as_bytes();
    let first = *bytes.first()?;
    let (ordered, number, marker_len): (bool, u64, usize) = if matches!(first, b'-' | b'+' | b'*') {
        (false, 0, 1)
    } else if first.is_ascii_digit() {
        let digits = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
        if digits > 9 || !matches!(bytes.get(digits).copied(), Some(b'.' | b')')) {
            return None;
        }
        (true, r[..digits].parse().ok()?, digits + 1)
    } else {
        return None;
    };
    let after = &r[marker_len..];
    if !(after.is_empty() || after.starts_with([' ', '\t'])) {
        return None;
    }
    Some(ListMarker {
        ordered,
        number,
        has_content: !after.trim().is_empty(),
    })
}

fn shift(r: &Range<usize>, delta: isize) -> Range<usize> {
    let f = |x: usize| (x as isize + delta) as usize;
    f(r.start)..f(r.end)
}

/// Block list for a document that is kept up to date across edits.
pub struct BlockMap {
    blocks: Vec<Block>,
    resplit_count: usize,
}

impl BlockMap {
    pub fn new(text: &str) -> Self {
        Self {
            blocks: split_blocks(text),
            resplit_count: 0,
        }
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// Number of blocks produced by re-splitting in [`BlockMap::apply_edit`], cumulative.
    pub fn resplit_count(&self) -> usize {
        self.resplit_count
    }

    /// Index of the block containing `offset`, or `None` at or past the end.
    pub fn block_at(&self, offset: usize) -> Option<usize> {
        let idx = self.blocks.partition_point(|b| b.range.end <= offset);
        (idx < self.blocks.len() && self.blocks[idx].range.start <= offset).then_some(idx)
    }

    /// `new_text` is the text after applying `edit` to the previous text.
    ///
    /// Re-scans from the nearest non-blank block before the edit, then keeps
    /// scanning until a fresh block boundary past the edit coincides with an old
    /// boundary. Blocks before the start are kept as they are, and blocks after
    /// the realigned boundary are kept with their ranges shifted.
    ///
    /// Returns the span of blocks that changed: the old block indices that were
    /// replaced, and how many new blocks replace them. Blocks outside the span
    /// keep their kind and text, so their index moves only by the span's size change.
    pub fn apply_edit(&mut self, edit: &Edit, new_text: &str) -> (Range<usize>, usize) {
        let (s, e) = (edit.range.start, edit.range.end);
        let delta = edit.text.len() as isize - (e - s) as isize;
        let old_len = self.blocks.last().map_or(0, |b| b.range.end);
        debug_assert_eq!(new_text.len() as isize, old_len as isize + delta);

        if self.blocks.is_empty() {
            self.blocks = split_blocks(new_text);
            self.resplit_count += self.blocks.len();
            return (0..0, self.blocks.len());
        }

        let new_edit_end = s + edit.text.len();
        let touched = self.block_at(s).unwrap_or(self.blocks.len() - 1);
        // A block's extent can depend on the lines after it, through blank runs
        // (a list continues across blank lines). Start at the nearest non-blank
        // block before the touched one, which is a stable boundary.
        let mut lo = touched.saturating_sub(1);
        while lo > 0 && self.blocks[lo].kind == BlockKind::Blank {
            lo -= 1;
        }
        let region_start = self.blocks[lo].range.start;

        let mut fresh = Vec::new();
        let mut resume = self.blocks.len();
        let scanner = Scanner {
            src: new_text,
            pos: region_start,
        };
        for block in scanner {
            let end = block.range.end;
            fresh.push(block);
            if end < new_edit_end {
                continue;
            }
            let old_pos = end as isize - delta;
            if old_pos < e as isize {
                continue;
            }
            let old_pos = old_pos as usize;
            if old_pos == old_len {
                resume = self.blocks.len();
                break;
            }
            if let Ok(idx) = self
                .blocks
                .binary_search_by_key(&old_pos, |b| b.range.start)
            {
                resume = idx;
                break;
            }
        }

        self.resplit_count += fresh.len();
        let fresh_count = fresh.len();
        let mut out = Vec::with_capacity(lo + fresh_count + self.blocks.len() - resume);
        out.extend_from_slice(&self.blocks[..lo]);
        out.extend(fresh);
        out.extend(self.blocks[resume..].iter().map(|b| Block {
            kind: b.kind,
            range: shift(&b.range, delta),
        }));
        let old_span = lo..resume;
        self.blocks = out;
        (old_span, fresh_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply_edits;

    /// Mixed document covering every block kind, with blank runs and no trailing text issues.
    const SAMPLE: &str = "# Title\n\
Setext head\n===\n\n\
Sub head\n---\n\n\
Paragraph one\ncontinues here\n\n\
- item a\n- item b\n  more text\n\n  para in item\n- item c\n\n\
1. first\n2) second\n\n\
> quoted\n> more quote\nlazy quote line\n\n\
| a | b |\n|---|:-:|\n| 1 | 2 |\n\n\
```rust\nfn main() {}\n\n```\n\n\
~~~\ntilde fence\n~~~\n\n\
$$\nx^2\n$$\n\n\
---\n\n\
<div>\nhtml\n</div>\n\n\
<!-- comment\nstill -->\n\n\
tail paragraph";

    fn kinds(text: &str) -> Vec<BlockKind> {
        split_blocks(text).iter().map(|b| b.kind).collect()
    }

    /// Asserts that ranges tile the text exactly and returns the blocks.
    fn assert_tiles(text: &str) -> Vec<Block> {
        let blocks = split_blocks(text);
        let mut pos = 0;
        for b in &blocks {
            assert_eq!(b.range.start, pos, "gap or overlap in {text:?}");
            assert!(b.range.end > b.range.start, "empty block in {text:?}");
            pos = b.range.end;
        }
        assert_eq!(pos, text.len(), "blocks do not cover {text:?}");
        let joined: String = blocks.iter().map(|b| &text[b.range.clone()]).collect();
        assert_eq!(joined, text);
        blocks
    }

    /// Applies `edit`, updates a BlockMap incrementally, and compares with a full split.
    fn check_edit(old: &str, edit: Edit) {
        let new = apply_edits(old, std::slice::from_ref(&edit));
        let mut map = BlockMap::new(old);
        map.apply_edit(&edit, &new);
        assert_eq!(
            map.blocks(),
            &split_blocks(&new)[..],
            "old={old:?} edit={edit:?} new={new:?}"
        );
        assert_tiles(&new);
    }

    #[test]
    fn recognises_atx_headings() {
        assert_eq!(kinds("# Title\n"), vec![BlockKind::Heading(1)]);
        assert_eq!(kinds("###### six\n"), vec![BlockKind::Heading(6)]);
        assert_eq!(kinds("####### seven\n"), vec![BlockKind::Paragraph]);
        assert_eq!(kinds("#nospace\n"), vec![BlockKind::Paragraph]);
    }

    #[test]
    fn recognises_setext_headings() {
        assert_eq!(kinds("Title\n===\n"), vec![BlockKind::Heading(1)]);
        assert_eq!(kinds("Sub\n---\n"), vec![BlockKind::Heading(2)]);
    }

    #[test]
    fn recognises_paragraphs() {
        assert_eq!(kinds("one\ntwo\n"), vec![BlockKind::Paragraph]);
        assert_eq!(kinds("no newline"), vec![BlockKind::Paragraph]);
    }

    #[test]
    fn recognises_lists() {
        assert_eq!(kinds("- a\n- b\n"), vec![BlockKind::List]);
        assert_eq!(kinds("1. a\n2) b\n"), vec![BlockKind::List]);
        assert_eq!(kinds("- a\n  - b\n    - c\n"), vec![BlockKind::List]);
        // Indented continuation and a blank line followed by an indented line stay in the list.
        assert_eq!(
            kinds("- a\n  more\n\n  para2\n- c\n"),
            vec![BlockKind::List]
        );
        // A blank line followed by a non-indented paragraph ends the list.
        let text = "- x\n\nafter\n";
        assert_eq!(
            kinds(text),
            vec![BlockKind::List, BlockKind::Blank, BlockKind::Paragraph]
        );
    }

    #[test]
    fn recognises_blockquotes_with_lazy_lines() {
        assert_eq!(
            kinds("> q1\n> q2\nlazy\n\nafter\n"),
            vec![
                BlockKind::BlockQuote,
                BlockKind::Blank,
                BlockKind::Paragraph
            ]
        );
    }

    #[test]
    fn recognises_tables() {
        let text = "| a | b |\n|---|:-:|\n| 1 | 2 |\n\nafter\n";
        assert_eq!(
            kinds(text),
            vec![BlockKind::Table, BlockKind::Blank, BlockKind::Paragraph]
        );
        // A pipe line without a delimiter row is a paragraph.
        assert_eq!(kinds("| a | b |\nplain\n"), vec![BlockKind::Paragraph]);
    }

    #[test]
    fn recognises_code_fences() {
        assert_eq!(
            kinds("```rust\nfn main() {}\n\n```\nafter\n"),
            vec![BlockKind::CodeFence, BlockKind::Paragraph]
        );
        assert_eq!(kinds("~~~\nx\n~~~\n"), vec![BlockKind::CodeFence]);
        // Unclosed fence runs to end of file, including blank lines.
        let text = "```\nopen\n\nmore\n";
        let blocks = assert_tiles(text);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].kind, BlockKind::CodeFence);
        assert_eq!(blocks[0].range, 0..text.len());
    }

    #[test]
    fn recognises_math_blocks() {
        assert_eq!(kinds("$$\nx^2\n$$\n"), vec![BlockKind::MathBlock]);
        assert_eq!(
            kinds("$$ y $$\nafter\n"),
            vec![BlockKind::MathBlock, BlockKind::Paragraph]
        );
    }

    #[test]
    fn recognises_thematic_breaks() {
        for text in ["---\n", "***\n", "___\n", "- - -\n"] {
            assert_eq!(kinds(text), vec![BlockKind::ThematicBreak], "{text:?}");
        }
    }

    #[test]
    fn recognises_html_blocks() {
        assert_eq!(
            kinds("<div>\nhi\n</div>\n\nafter\n"),
            vec![BlockKind::HtmlBlock, BlockKind::Blank, BlockKind::Paragraph]
        );
        assert_eq!(kinds("</span>x\n"), vec![BlockKind::HtmlBlock]);
        assert_eq!(
            kinds("<!-- hidden\nstill -->\n"),
            vec![BlockKind::HtmlBlock]
        );
        // A bare `<` followed by a space is not HTML.
        assert_eq!(kinds("< not html\n"), vec![BlockKind::Paragraph]);
    }

    #[test]
    fn recognises_blank_runs() {
        assert_eq!(kinds("\n  \n\t\n"), vec![BlockKind::Blank]);
        assert_eq!(
            kinds("a\n\n\nb\n"),
            vec![BlockKind::Paragraph, BlockKind::Blank, BlockKind::Paragraph]
        );
    }

    #[test]
    fn mixed_sample_has_every_kind() {
        let kinds = kinds(SAMPLE);
        for kind in [
            BlockKind::Heading(1),
            BlockKind::Heading(2),
            BlockKind::Paragraph,
            BlockKind::List,
            BlockKind::BlockQuote,
            BlockKind::Table,
            BlockKind::CodeFence,
            BlockKind::MathBlock,
            BlockKind::ThematicBreak,
            BlockKind::HtmlBlock,
            BlockKind::Blank,
        ] {
            assert!(kinds.contains(&kind), "missing {kind:?} in {kinds:?}");
        }
    }

    #[test]
    fn tiling_on_edge_cases() {
        for text in [
            "",
            "a",
            "a\n",
            "\n",
            "\n\n",
            "   ",
            "x\n\n\n",
            "```\nopen",
            "| a |\n|-|",
            "- ",
            "#",
            "$$",
            "<",
            "> ",
            "a\r\nb\r\n\r\nc",
        ] {
            assert_tiles(text);
        }
        assert!(split_blocks("").is_empty());
    }

    #[test]
    fn tiling_on_big_mixed_sample() {
        let big = SAMPLE.repeat(30);
        let blocks = assert_tiles(&big);
        assert!(blocks.len() > 30 * 10);
    }

    #[test]
    fn edit_insert_char_in_paragraph() {
        let doc = "para one\n\npara two\n\npara three\n";
        check_edit(doc, Edit::insert(12, "x"));
    }

    #[test]
    fn edit_insert_blank_line_splits_paragraph() {
        let doc = "line one\nline two\nline three\n";
        check_edit(doc, Edit::insert(9, "\n"));
        check_edit(doc, Edit::insert(9, "\n\n"));
    }

    #[test]
    fn edit_delete_blank_line_merges_paragraphs() {
        let doc = "para one\n\npara two\n";
        // Delete the blank line at offset 9.
        check_edit(doc, Edit::delete(9..10));
        let mut map = BlockMap::new(doc);
        let new = apply_edits(doc, &[Edit::delete(9..10)]);
        map.apply_edit(&Edit::delete(9..10), &new);
        assert_eq!(map.blocks().len(), 1);
    }

    #[test]
    fn edit_typing_fence_opener_swallows_following_blocks() {
        let doc = "para a\n\npara b\n\npara c\n\n- list\n";
        let start = doc.find("para b").unwrap();
        check_edit(doc, Edit::insert(start, "```\n"));
        // And typing the closing fence later restores the original structure.
        let fenced = apply_edits(doc, &[Edit::insert(start, "```\n")]);
        let close_at = fenced.find("para c").unwrap();
        check_edit(&fenced, Edit::insert(close_at, "```\n"));
    }

    #[test]
    fn edit_removing_fence_opener_reflows_document() {
        let doc = "```\ncode\n```\n\npara\n";
        check_edit(doc, Edit::delete(0..4));
    }

    #[test]
    fn edit_inside_one_paragraph_resplits_few_blocks() {
        let doc: String = (0..25)
            .map(|i| format!("paragraph number {i} has words\n\n"))
            .collect();
        let mut map = BlockMap::new(&doc);
        assert_eq!(map.blocks().len(), 50);
        let target = map.blocks()[24].range.clone();
        let edit = Edit::insert(target.start + 3, "inserted ");
        let new = apply_edits(&doc, std::slice::from_ref(&edit));
        map.apply_edit(&edit, &new);
        assert!(
            map.resplit_count() <= 3,
            "resplit {} blocks",
            map.resplit_count()
        );
        assert_eq!(map.blocks(), &split_blocks(&new)[..]);
    }

    #[test]
    fn every_edit_matches_full_split() {
        let doc = SAMPLE;
        let inserts = [
            "x",
            "\n",
            "\n\n",
            "```\n",
            "- ",
            "> ",
            "# ",
            "| a |\n|---|\n",
            "$$\n",
            "---\n",
            "1. ",
            "<div>\n",
        ];
        let boundaries: Vec<usize> = (0..=doc.len())
            .filter(|&i| doc.is_char_boundary(i))
            .collect();
        for &i in &boundaries {
            for ins in inserts {
                check_edit(doc, Edit::insert(i, ins));
            }
            for &j in boundaries.iter().skip_while(|&&j| j <= i).take(2) {
                check_edit(doc, Edit::delete(i..j));
                check_edit(doc, Edit::replace(i..j, "```\n"));
            }
        }
    }

    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    const FRAGMENTS: [&str; 22] = [
        "# H\n",
        "para text\n",
        "\n",
        "- item\n",
        "  cont\n",
        "1. one\n",
        "> quote\n",
        "> q2\n",
        "```\n",
        "code line\n",
        "~~~\n",
        "| a | b |\n",
        "|---|---|\n",
        "$$\n",
        "x^2\n",
        "---\n",
        "<div>\n",
        "</div>\n",
        "<!-- c -->\n",
        "    indented\n",
        "tail without newline",
        "Title\n===\n",
    ];

    #[test]
    fn random_edits_match_full_split() {
        for seed in 1..=150u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            let mut doc: String = (0..12)
                .map(|_| FRAGMENTS[rng.below(FRAGMENTS.len())])
                .collect();
            let mut map = BlockMap::new(&doc);
            for _ in 0..25 {
                let len = doc.len();
                let s = rng.below(len + 1);
                let edit = if rng.below(2) == 0 {
                    let text = FRAGMENTS[rng.below(FRAGMENTS.len())].to_string();
                    Edit::insert(s, text)
                } else {
                    let e = (s + 1 + rng.below(6)).min(len);
                    Edit::delete(s..e)
                };
                let new = apply_edits(&doc, std::slice::from_ref(&edit));
                map.apply_edit(&edit, &new);
                assert_eq!(
                    map.blocks(),
                    &split_blocks(&new)[..],
                    "seed {seed}, old={doc:?}, edit={edit:?}"
                );
                assert_tiles(&new);
                doc = new;
            }
        }
    }

    #[test]
    fn block_at_finds_containing_block() {
        let doc = "# H\n\npara\n";
        let map = BlockMap::new(doc);
        assert_eq!(map.block_at(0), Some(0));
        assert_eq!(map.block_at(4), Some(1));
        assert_eq!(map.block_at(5), Some(2));
        assert_eq!(map.block_at(doc.len()), None);
    }
}
