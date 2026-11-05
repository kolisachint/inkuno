//! Performance benchmarks for md_core on a generated 10,000-line mixed document (DESIGN §12).
//!
//! Run with `cargo bench -p md_core -- --warm-up-time 1 --measurement-time 3`.
//! Before timing, the dirty-block benchmark asserts how many blocks a single-character insert
//! re-splits, so the "only dirty blocks are reparsed" target is checked by the run itself.

use std::hint::black_box;
use std::ops::Range;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use md_core::block::{split_blocks, BlockMap};
use md_core::buffer::Buffer;
use md_core::display_map::DisplayMap;
use md_core::inline::{hidden_ranges, inline_spans};
use md_core::input_rules::{apply_input_rule, Input};
use md_core::{BlockKind, Edit};

/// Target size of the benchmark document, in lines.
const LINES: usize = 10_000;

/// A mixed document of headings, paragraphs with inline spans, lists, code fences, tables,
/// block quotes and blank lines, cut once it has at least `lines` lines.
fn generate(lines: usize) -> String {
    let mut out = String::new();
    let mut n = 0usize;
    while out.lines().count() < lines {
        n += 1;
        out.push_str(&format!("## Section {n}\n\n"));
        out.push_str(&format!(
            "Paragraph {n} has **bold**, *emphasis*, `code`, a [link](https://example.com/{n}) and ~~gone~~ text.\n"
        ));
        out.push_str("It continues on a second line with _more_ words and $x$ inline math.\n\n");
        out.push_str("- first item with **strong**\n- second item\n  - nested item\n- third [ref](https://a.b/c)\n\n");
        out.push_str("1. ordered one\n2. ordered two\n\n");
        out.push_str(&format!(
            "```rust\nfn main() {{\n    println!(\"hi {n}\");\n}}\n```\n\n"
        ));
        out.push_str("| name | value |\n| ---- | ----- |\n| alpha | 1 |\n| beta | 2 |\n\n");
        out.push_str("> quoted line one\n> quoted line two\n\n");
    }
    out
}

/// A byte offset in the middle of the document: the start of the middle line plus a few bytes.
fn middle_offset(text: &str) -> usize {
    let mid_line = text.lines().count() / 2;
    let start: usize = text
        .split_inclusive('\n')
        .take(mid_line)
        .map(str::len)
        .sum();
    start + text[start..].find(' ').unwrap_or(0)
}

/// Hidden ranges of the whole document for one caret, computed the way the editor does it:
/// the inline spans of every non-blank block, then the hidden ranges for the caret.
/// `md_editor::inline_render::document_hidden_ranges` is benched in md_editor.
fn document_hidden_ranges(text: &str, caret: Option<usize>) -> Vec<Range<usize>> {
    let mut hidden = Vec::new();
    for block in split_blocks(text) {
        if block.kind == BlockKind::Blank {
            continue;
        }
        let spans = inline_spans(&text[block.range.clone()], block.range.start);
        hidden.extend(hidden_ranges(&spans, caret));
    }
    hidden
}

fn bench_reparse(c: &mut Criterion) {
    let text = generate(LINES);
    let at = middle_offset(&text);
    let edit = Edit::insert(at, "x");
    let mut new_text = text.clone();
    new_text.insert(at, 'x');

    // Dirty-block check: one character re-splits a few blocks, not the document.
    let total_blocks = split_blocks(&text).len();
    let mut map = BlockMap::new(&text);
    let before = map.resplit_count();
    map.apply_edit(&edit, &new_text);
    let resplit = map.resplit_count() - before;
    println!(
        "document: {} bytes, {} lines, {total_blocks} blocks",
        text.len(),
        text.lines().count()
    );
    println!("single-char insert re-split {resplit} of {total_blocks} blocks");
    assert!(
        resplit <= 3 && resplit * 100 < total_blocks,
        "insert re-split {resplit} of {total_blocks} blocks"
    );

    let mut group = c.benchmark_group("reparse");
    group.bench_function("insert_char_buffer_and_blockmap", |b| {
        b.iter_batched(
            || (Buffer::new(&text), BlockMap::new(&text)),
            |(mut buffer, mut map)| {
                buffer.edit(edit.clone());
                map.apply_edit(&edit, &new_text);
                black_box((buffer.version(), map.resplit_count()))
            },
            BatchSize::LargeInput,
        )
    });
    group.finish();
}

/// The pure work of one keystroke besides layout and paint.
fn bench_keystroke(c: &mut Criterion) {
    let text = generate(LINES);
    let buffer = Buffer::new(&text);
    let at = middle_offset(&text);
    let map = BlockMap::new(&text);
    let mut group = c.benchmark_group("keystroke");

    // The editor copies the whole buffer through `Editor::text()`.
    group.bench_function("buffer_text_copy", |b| b.iter(|| black_box(buffer.text())));
    group.bench_function("document_hidden_ranges_caret", |b| {
        b.iter(|| black_box(document_hidden_ranges(&text, Some(at))))
    });
    // `input_rules::try_input_rule` passes the whole text to the engine.
    group.bench_function("apply_input_rule_char", |b| {
        b.iter(|| black_box(apply_input_rule(&text, at, &Input::Char('x'))))
    });
    group.bench_function("display_map_for_text_caret", |b| {
        b.iter(|| black_box(DisplayMap::for_text(&text, Some(at))))
    });
    group.bench_function("block_at_caret", |b| b.iter(|| black_box(map.block_at(at))));
    group.finish();
}

fn bench_open(c: &mut Criterion) {
    let text = generate(LINES);
    let mut group = c.benchmark_group("open");
    group.sample_size(20);
    group.bench_function("split_blocks", |b| {
        b.iter(|| black_box(split_blocks(&text)))
    });
    group.bench_function("block_map_new", |b| {
        b.iter(|| black_box(BlockMap::new(&text)))
    });
    group.bench_function("split_and_inline_spans", |b| {
        b.iter(|| black_box(document_hidden_ranges(&text, None)))
    });
    group.finish();
}

criterion_group!(benches, bench_reparse, bench_keystroke, bench_open);
criterion_main!(benches);
