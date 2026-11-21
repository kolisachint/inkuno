//! CommonMark 0.31.2 and GFM 0.29-gfm conformance (docs/design/markdown_renderer.md §11.4).
//!
//! The oracle is comrak, a dev-dependency only. Per example:
//! - blocks: the kind `classify_in` gives each source line equals the kind comrak's AST gives it
//!   (`expected_blocks`);
//! - inline: the bytes Inkuno styles as Strong, Emph, Code, Strikethrough and Link (`flags`) equal
//!   the content bytes comrak puts in those nodes (`comrak_spans`).
//!
//! `tests/spec/expected-failures.txt` lists the examples that fail today: a failure not listed is a
//! regression, a listed example that passes is stale. `INKUNO_SPEC_WRITE=1` rewrites the list.
//! Run with `--nocapture` to see the summary per spec and per section.

use std::collections::{BTreeMap, BTreeSet};

use comrak::nodes::{AstNode, ListType, NodeValue};
use comrak::{Arena, Options, parse_document};
use inkuno_logic::markdown::{self as md, BOLD, CODE, DIM, ITALIC, Kind, LINK, STRIKE};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/spec");
const STYLES: [&str; 5] = ["strong", "emph", "code", "strike", "link"];

struct Spec {
    label: &'static str,
    file: &'static str,
    gfm: bool,
}

const SPECS: [Spec; 2] = [
    Spec { label: "cm", file: "commonmark-0.31.2.json", gfm: false },
    Spec { label: "gfm", file: "gfm-0.29-extensions.json", gfm: true },
];

/// comrak with the five GFM extensions, or with none (CommonMark).
fn options(gfm: bool) -> Options<'static> {
    let mut o = Options::default();
    o.extension.table = gfm;
    o.extension.tasklist = gfm;
    o.extension.strikethrough = gfm;
    o.extension.autolink = gfm;
    o
}

struct Case {
    number: u32,
    section: String,
    markdown: String,
}

fn load(spec: &Spec) -> Vec<Case> {
    let text = std::fs::read_to_string(format!("{DIR}/{}", spec.file)).unwrap();
    let json: Vec<serde_json::Value> = serde_json::from_str(&text).unwrap();
    json.iter()
        .map(|v| Case {
            number: v["example"].as_u64().unwrap() as u32,
            section: v["section"].as_str().unwrap().to_string(),
            markdown: v["markdown"].as_str().unwrap().to_string(),
        })
        .collect()
}

/// The kind of each line (index 0 is line 1) that comrak's AST gives it. A line that opens a quote
/// or an item takes the outermost such marker, since `classify` reads the first marker only; a line
/// with a `>` under a quote is a quote line. Other lines take their innermost leaf block; blank
/// lines take `Para`. Ambiguous: HTML blocks map to `Para` (Inkuno has no HTML kind: its text is
/// read as paragraph text); indented code maps to `Code`; list depth is not compared (`classify`
/// has none).
fn expected_blocks<'a>(root: &'a AstNode<'a>, lines: &[&str]) -> Vec<Kind> {
    // (depth, first line, last line, kind, needs a `>` on the line): the blocks with a marker.
    let mut marks = Vec::new();
    // (first line, last line, kind) of the leaf blocks, outer before inner.
    let mut leaves = Vec::new();
    // (first line, last line, fence character) of the fenced code blocks.
    let mut fences = Vec::new();
    for node in root.descendants().skip(1) {
        let ast = node.data.borrow();
        let (first, last) = (ast.sourcepos.start.line, ast.sourcepos.end.line);
        let depth = node.ancestors().count();
        match &ast.value {
            NodeValue::BlockQuote => marks.push((depth, first, last, Kind::Quote, true)),
            NodeValue::Item(list) => marks.push((depth, first, first, bullet(list.list_type), false)),
            NodeValue::TaskItem(task) => {
                let done = task.symbol.is_some_and(|c| c != ' ');
                marks.push((depth, first, first, Kind::Task(done), false))
            }
            NodeValue::Paragraph | NodeValue::HtmlBlock(_) => leaves.push((first, last, Kind::Para)),
            NodeValue::Heading(h) => leaves.push((first, last, Kind::Heading(h.level))),
            NodeValue::ThematicBreak(_) => leaves.push((first, last, Kind::Rule)),
            NodeValue::Table(_) => leaves.push((first, last, Kind::Table)),
            NodeValue::CodeBlock(code) => {
                leaves.push((first, last, Kind::Code));
                if code.fenced {
                    fences.push((first, last, code.fence_char as char));
                }
            }
            _ => {}
        }
    }
    let mut kinds = vec![Kind::Para; lines.len()];
    for &(first, last, kind) in &leaves {
        kinds[first - 1..last].fill(kind);
    }
    for &(first, last, fence) in &fences {
        kinds[first - 1] = Kind::Fence;
        if last > first && lines[last - 1].trim_start().starts_with(fence) {
            kinds[last - 1] = Kind::Fence;
        }
    }
    (0..lines.len())
        .map(|i| {
            let l = i + 1;
            let quoted = lines[i].trim_start().starts_with('>');
            marks
                .iter()
                .filter(|m| m.1 <= l && l <= m.2 && (!m.4 || quoted))
                .min_by_key(|m| m.0)
                .map_or(kinds[i], |m| m.3)
        })
        .collect()
}

fn bullet(list: ListType) -> Kind {
    match list {
        ListType::Bullet => Kind::Bullet,
        ListType::Ordered => Kind::Ordered,
    }
}

/// The bytes comrak puts in the content of each style of `STYLES`: the text below a Strong, Emph,
/// Strikethrough or Link node (code spans below it included), and the content between the backticks
/// of a code span. Link covers autolinks too, as Inkuno has one flag for both.
fn comrak_spans<'a>(root: &'a AstNode<'a>, starts: &[usize], len: usize) -> [Vec<bool>; 5] {
    let pos = |line: usize, column: usize| starts[line - 1] + column - 1;
    let mut out = [(); 5].map(|_| vec![false; len]);
    for node in root.descendants() {
        let k = match node.data.borrow().value {
            NodeValue::Strong(_) => 0,
            NodeValue::Emph(_) => 1,
            NodeValue::Code(_) => 2,
            NodeValue::Strikethrough(_) => 3,
            NodeValue::Link(_) => 4,
            _ => continue,
        };
        for d in node.descendants() {
            let ast = d.data.borrow();
            let sp = ast.sourcepos;
            let (s, e) = (pos(sp.start.line, sp.start.column), pos(sp.end.line, sp.end.column) + 1);
            match &ast.value {
                NodeValue::Text(_) => out[k][s..e].fill(true),
                NodeValue::Code(code) if e >= s + 2 * code.num_backticks => {
                    let n = code.num_backticks;
                    out[k][s + n..e - n].fill(true)
                }
                _ => {}
            }
        }
    }
    out
}

/// Inkuno's view of the same bytes, by line: the block kind and the styled bytes. Lines Inkuno reads
/// as code or as a rule have no inline styles.
fn ours_spans(lines: &[&str], starts: &[usize], blocks: &[md::Block], len: usize) -> [Vec<bool>; 5] {
    let mut out = [(); 5].map(|_| vec![false; len]);
    for (i, line) in lines.iter().enumerate() {
        let base = starts[i];
        let (kind, from) = md::classify_in(line, md::open_at(blocks, base));
        if matches!(kind, Kind::Fence | Kind::Code | Kind::Rule) {
            continue;
        }
        for (j, &f) in md::flags(line, from).iter().enumerate() {
            let at = base + j;
            out[0][at] = f & BOLD != 0 && f & DIM == 0;
            out[1][at] = f & ITALIC != 0 && f & DIM == 0;
            out[2][at] = f & CODE != 0;
            out[3][at] = f & STRIKE != 0 && f & DIM == 0;
            out[4][at] = f & LINK != 0;
        }
    }
    out
}

/// Ok if Inkuno and comrak agree on every line's block kind and on every styled byte; otherwise the
/// first few disagreements.
fn check(markdown: &str, gfm: bool) -> Result<(), String> {
    let arena = Arena::new();
    let root = parse_document(&arena, markdown, &options(gfm));
    let lines: Vec<&str> = markdown.split('\n').collect();
    let starts: Vec<usize> = lines
        .iter()
        .scan(0, |at, l| {
            let s = *at;
            *at += l.len() + 1;
            Some(s)
        })
        .collect();
    let blocks = md::blocks(markdown);
    let mut errors = Vec::new();
    let expected = expected_blocks(root, &lines);
    for (i, line) in lines.iter().enumerate() {
        let (ours, theirs) = (md::classify_in(line, md::open_at(&blocks, starts[i])).0, expected[i]);
        if ours != theirs {
            errors.push(format!("line {}: ours {ours:?}, oracle {theirs:?}", i + 1));
        }
    }
    let (ours, theirs) = (ours_spans(&lines, &starts, &blocks, markdown.len()), comrak_spans(root, &starts, markdown.len()));
    for k in 0..5 {
        for at in (0..markdown.len()).filter(|&at| ours[k][at] != theirs[k][at]) {
            let line = starts.partition_point(|&s| s <= at);
            let side = if ours[k][at] { "styled, oracle plain" } else { "oracle content, not styled" };
            errors.push(format!("line {line} {}: {side}", STYLES[k]));
        }
    }
    errors.truncate(3);
    if errors.is_empty() { Ok(()) } else { Err(errors.join("; ")) }
}

#[test]
fn conformance() {
    // Failing examples: (spec label, example number) to the first disagreements.
    let mut failing: BTreeMap<(&str, u32), String> = BTreeMap::new();
    for spec in &SPECS {
        let cases = load(spec);
        let mut sections: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for case in &cases {
            let score = sections.entry(&case.section).or_default();
            score.1 += 1;
            match check(&case.markdown, spec.gfm) {
                Ok(()) => score.0 += 1,
                Err(why) => {
                    if std::env::var("INKUNO_SPEC_VERBOSE").is_ok() {
                        println!("{} {}: {why}\n  {:?}", spec.label, case.number, case.markdown);
                    }
                    failing.insert((spec.label, case.number), why);
                }
            }
        }
        let passed: usize = sections.values().map(|s| s.0).sum();
        let total: usize = sections.values().map(|s| s.1).sum();
        println!("\n{} {}: {passed}/{total} passed", spec.label, spec.file);
        let mut rows: Vec<_> = sections.into_iter().collect();
        rows.sort_by_key(|(_, (ok, all))| (all - ok, *all));
        for (name, (ok, all)) in rows.iter().rev() {
            println!("  {ok:>4}/{all:<4} {name}");
        }
    }
    let path = format!("{DIR}/expected-failures.txt");
    let now: BTreeSet<(&str, u32)> = failing.keys().copied().collect();
    if std::env::var("INKUNO_SPEC_WRITE").as_deref() == Ok("1") {
        let body: String = now.iter().map(|(label, n)| format!("{label}:{n}\n")).collect();
        std::fs::write(&path, body).unwrap();
        return;
    }
    let listed: BTreeSet<(String, u32)> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let (label, n) = l.split_once(':').unwrap();
            (label.to_string(), n.parse().unwrap())
        })
        .collect();
    let regressions: Vec<String> = failing
        .iter()
        .filter(|((label, n), _)| !listed.contains(&(label.to_string(), *n)))
        .map(|((label, n), why)| format!("{label}:{n}: {why}"))
        .collect();
    let stale: Vec<String> = listed
        .iter()
        .filter(|(label, n)| !failing.keys().any(|(l, k)| *l == label.as_str() && k == n))
        .map(|(label, n)| format!("{label}:{n}"))
        .collect();
    assert!(
        regressions.is_empty() && stale.is_empty(),
        "regressions (fix, or list in expected-failures.txt):\n{}\nstale entries (remove from expected-failures.txt):\n{}",
        regressions.join("\n"),
        stale.join("\n")
    );
}
