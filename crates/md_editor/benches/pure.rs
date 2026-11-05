//! Per-keystroke pure work of the editor (DESIGN §12, task 10.4), without GPUI calls.
//!
//! Run with `cargo bench -p md_editor --bench pure -- --warm-up-time 1 --measurement-time 3`.
//! The core edit, reparse and open costs are in `crates/md_core/benches/core.rs`. The layout
//! and paint cost of a keystroke needs a window and is measured by the Xvfb run recorded in
//! `docs/spikes/perf-pass.md`.

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use md_core::buffer::Buffer;
use md_core::input_rules::{apply_input_rule, Input};
use md_core::Edit;
use md_editor::block_list::{sync_hashes, ItemIndex};
use md_editor::inline_render::document_hidden_ranges;

/// Target size of the benchmark document, in lines.
const LINES: usize = 10_000;

/// Same mixed document as `md_core/benches/core.rs`: headings, paragraphs with inline spans,
/// lists, code fences, tables, block quotes and blank lines.
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

/// A byte offset in the middle of the document.
fn middle_offset(text: &str) -> usize {
    let mid_line = text.lines().count() / 2;
    let start: usize = text
        .split_inclusive('\n')
        .take(mid_line)
        .map(str::len)
        .sum();
    start + text[start..].find(' ').unwrap_or(0)
}

fn bench_pure_keystroke(c: &mut Criterion) {
    let text = generate(LINES);
    let buffer = Buffer::new(&text);
    let at = middle_offset(&text);
    let mut new_text = text.clone();
    new_text.insert(at, 'x');
    let mut group = c.benchmark_group("editor_pure");

    // `Editor::text()`, called on every keystroke by the input and list paths.
    group.bench_function("editor_text_copy", |b| b.iter(|| black_box(buffer.text())));

    // `inline_render::document_hidden_ranges` for the caret: run on each caret move and edit.
    group.bench_function("document_hidden_ranges", |b| {
        b.iter(|| black_box(document_hidden_ranges(&text, Some(at))))
    });

    // `input_rules::try_input_rule` on the full text.
    group.bench_function("input_rule_check_full_text", |b| {
        b.iter(|| black_box(apply_input_rule(&text, at, &Input::Char('x'))))
    });

    // `block_list::sync_items` after an edit: re-split and re-hash every item (full rebuild, used
    // after undo and redo).
    group.bench_function("block_list_sync_hashes", |b| {
        b.iter(|| black_box(sync_hashes(&new_text)))
    });

    // The per-edit path of the block list (10.4): insert a character and delete it again, both
    // through `ItemIndex::apply_edit`. Each iteration is two edits.
    let mut buffer = Buffer::new(&text);
    let mut index = ItemIndex::default();
    index.build(&text, buffer.version());
    group.bench_function("block_list_incremental_edit_pair", |b| {
        b.iter(|| {
            let before = buffer.version();
            buffer.edit(Edit::insert(at, "x"));
            black_box(
                index
                    .apply_edit(before, &buffer, at..at, "x")
                    .expect("incremental path"),
            );
            let before = buffer.version();
            buffer.edit(Edit::delete(at..at + 1));
            black_box(
                index
                    .apply_edit(before, &buffer, at..at + 1, "")
                    .expect("incremental path"),
            );
        })
    });

    // Caret movement with the list's cached hidden ranges (10.4), warm: the blocks the caret is
    // not in reuse their cached ranges.
    let mut cached = ItemIndex::default();
    cached.build(&text, 0);
    black_box(cached.hidden_ranges(&text, Some(at)));
    group.bench_function("hidden_ranges_cached_caret", |b| {
        b.iter(|| black_box(cached.hidden_ranges(&text, Some(at))))
    });

    group.finish();

    // Estimate of the pure per-keystroke cost: the sum of the parts above that run per edit.
    println!(
        "pure per-keystroke estimate = editor_text_copy + document_hidden_ranges + \
         input_rule_check_full_text + block_list_sync_hashes (see criterion means)"
    );
}

criterion_group!(benches, bench_pure_keystroke);
criterion_main!(benches);
