# Performance pass (task 10.4, performance part)

Measured against DESIGN §12 on a 10,000-line mixed document. The math and Mermaid part of 10.4 is on hold and is not covered here.

## Machine

- CPU: Intel(R) Xeon(R) Processor @ 2.10GHz, 4 vCPU (`/proc/cpuinfo`), 15 GiB RAM.
- Kernel: Linux 6.18.44-fc-v236 (a VM).
- Display: Xvfb 1280x800x24 under `xvfb-run`, software GL. AGENTS.md records llvmpipe; the renderer string was not re-checked for this run.
- Not measured on a GPU or on macOS.
- Builds: release profile for every number (`cargo bench` uses the bench profile, which inherits release; the app was built with `cargo build --release -p app`).
- Criterion 0.8.2, `default-features = false`, `--warm-up-time 1 --measurement-time 3`.

## Test documents

- `crates/md_core/benches/core.rs` and `crates/md_editor/benches/pure.rs` generate the same mixed document: headings, paragraphs with inline spans, lists, fences, tables, block quotes and blank lines. It is 169 KB, 10,017 lines, 4,452 blocks.
- The Xvfb run used a separate 10,010-line file with the same kinds of blocks (455 sections). Its middle is Section 232 at line 5,083. The four runs typed at different places, listed below.

## Results against DESIGN §12

| Target | Measured | Verdict |
|---|---|---|
| Keystroke to paint under 16 ms on 10,000 lines | Xvfb, release app, 58 characters typed at line 5,083 (mid-document). `NOTEAPP_PERF` samples: median 10.1 ms, p90 17.1 ms, max 20.0 ms, 47 of 58 under 16 ms. | Not met at p90 on this rig. The median passes. The miss is in layout, paint and software raster, since the pure per-keystroke work is about 1 ms (see below). Re-measure on a GPU and on macOS before deciding. |
| Reparse: only dirty blocks | `BlockMap::apply_edit` after a one-character insert in the middle: 3 of 4,452 blocks re-split (asserted in the bench). | Pass in md_core. The editor does not use `apply_edit` (see hot spots), so the editor path re-splits the whole document on every edit. |
| Layout: only visible blocks | During the Xvfb run (526 frames, `MD_EDITOR_LAYOUT_LOG=1`), a mean of 19 of 5,461 list items were laid out per frame, and the maximum was 21. | Pass. |
| Rich rendering async and cached | Math and Mermaid are on hold. | Not measured. |
| Search | DESIGN §12 has no search target. | Not measured. Not added to the bench. |

### Keystroke-to-paint runs

The Xvfb run was repeated four times, with the caret at a different place each time. The wheel-scroll count sets the place.

| Run | Caret place | Median ms | p90 ms | Max ms | Under 16 ms |
|---|---|---|---|---|---|
| 1 | Section 34, line 727 (7%) | 10.3 | 17.4 | 19.3 | 47 of 58 |
| 2 | Section 34, layout log on | 9.6 | 16.5 | 21.3 | 49 of 58 |
| 3 | Section 158, line 3,455 (35%) | 11.6 | 17.6 | 34.3 | 47 of 58 |
| 4 | Section 232, line 5,083 (middle) | 10.1 | 17.1 | 20.0 | 47 of 58 |

Run 4 is the one that counts for the mid-document target. Its screenshot is `docs/screenshots/10.4-perf.png`. It shows the 58 typed characters in the middle of the document, with the vault sidebar.

Method: `NOTEAPP_PERF=1` prints `keystroke_to_paint_ms` for each typed character. The clock starts in the IME text-commit handler (`ime.rs`, `replace_text_in_range`) and stops when the block list finishes painting (`block_list.rs`, `paint`). The hook is in `crates/md_editor/src/perf.rs`. Without the variable, each call costs one cached boolean read. xdotool drove the input.

Caveat: the samples rise in steps of about 3 ms within bursts of five or six characters. The cause is not explained. It may come from the hook (one pending timestamp per paint) or from how xdotool delivers events. Read the per-key numbers as indicative. The median and the spread across the four runs are the more reliable figures.

## Pure per-keystroke work (criterion, medians)

Measured in `crates/md_core/benches/core.rs` and `crates/md_editor/benches/pure.rs`.

| Work | Median | Runs per |
|---|---|---|
| `Buffer::edit` + `BlockMap::apply_edit`, one character mid-document | about 54 µs | typed character |
| `block_list` `sync_hashes`: re-split and re-hash all items | 0.80 ms | typed character (the editor's `sync_items`) |
| `Editor::text()` / `Buffer::text()`: copy of the whole 169 KB | 6.6 µs | several times per keystroke |
| `apply_input_rule` on the full text (one char) | 27 ns | typed character |
| `block_at` | 28 ns | caret move |
| `document_hidden_ranges` for the caret | 3.8 ms | caret move (`keys.rs` `move_caret`), not typed character |
| `DisplayMap::for_text` | 2.5 ms | tests and benches only, not on the keystroke path |

So a typed character costs about 0.9 ms of pure work. A caret move adds about 3.8 ms, from `document_hidden_ranges`. The rest of the 10 ms median is layout, paint and raster.

## Open and reparse

| Work | Median |
|---|---|
| `md_core::block::split_blocks`, whole document | 0.82 ms |
| `BlockMap::new`, whole document | 0.71 ms |
| Split, plus inline spans of every block | 3.6 ms |

The open cost is well under any target, and DESIGN §12 sets none for it.

## Hot spots

1. **The editor re-splits the whole document on every edit.** `block_list.rs` `sync_items` calls `item_state` (`blocks_for`, then hashing every item) each time the buffer version changes. That is 0.8 ms per edit and grows with the document. `md_core::BlockMap::apply_edit` already does the incremental work (3 blocks re-split), but nothing in `md_editor` calls it. Fix: keep a `BlockMap` in `BlockListState` and apply the edits from `Buffer::edits_since`. Not done in this pass. Fixed in the follow-up fixes below.
2. **Caret moves re-parse the inline spans of every block.** `document_hidden_ranges` (called from `keys.rs` `move_caret`) costs 3.8 ms per arrow key, which is the largest pure cost on the keystroke path. Fix: cache spans per block, keyed by block text hash (similar to `md_core::parse::ParseCache`), or compute hidden ranges only for blocks near the caret. Not small, so not done in this pass. Fixed in the follow-up fixes below.
3. **Whole-text copies per keystroke are not a hot spot.** `Editor::text()` costs 6.6 µs at 169 KB. No change.
4. **Double layout per block (measure and prepaint).** The task brief says each block is laid out twice per frame. This run did not measure it. Not changed.

No optimization was made in this task. Hot spots 1 and 2 are the candidates if a GPU or macOS run still misses 16 ms at p90.

## Reproduce

- `cargo bench -p md_core --bench core -- --warm-up-time 1 --measurement-time 3`
- `cargo bench -p md_editor --bench pure -- --warm-up-time 1 --measurement-time 3`
- App run: build with `cargo build --release -p app`, then, inside `xvfb-run -n 97 -s "-screen 0 1280x800x24"`, start `NOTEAPP_PERF=1 target/release/app <10k-line file.md>`, scroll to the middle with `xdotool click --repeat 3100 --delay 2 5`, and type with `xdotool type --delay 120 "<58 chars>"`. Read the `NOTEAPP_PERF` lines from stderr.

## Follow-up fixes

Hot spots 1 and 2 above are fixed. Both paths keep the old output: `ItemIndex::apply_edit` and `ItemIndex::build` produce the same items as a full rebuild, and `ItemIndex::hidden_ranges` the same hidden ranges as `document_hidden_ranges`. Tests check both after 200 pseudo-random edits on a mixed document, and at every caret position.

### What changed

1. **Block list (hot spot 1).** `BlockMap` (md_core) is kept in the editor's block list state. `Editor::replace_range` passes each edit to `BlockListState::note_edit`, which calls `ItemIndex::apply_edit`. That re-splits only the blocks the edit touched, and the list gets one splice for those items. `BlockMap::apply_edit` now returns the changed block span, which is what makes the splice possible. The full rebuild (`ItemIndex::build`) still runs for undo, redo, the first sync, and any edit the incremental path declines: a whole-text replacement (load and reload), an emptied text, or a change to the trailing newline.
2. **Hidden ranges (hot spot 2).** `Editor::hidden_ranges_for_caret` (used by `keys.rs` `move_caret`) asks the index. The index keeps, for each item, its hidden ranges with the caret elsewhere, relative to the item start. It fills them on demand and drops them for the items an edit changed. The caret's own item is computed each time. When the list is out of step with the buffer, the old full computation runs.

### Criterion, before and after (medians, `--warm-up-time 1 --measurement-time 3`)

| Work | Before | After | Note |
|---|---|---|---|
| Per-edit block list, full rebuild (`block_list_sync_hashes`) | 0.84 ms | 0.79 ms | Unchanged code path, now used only for undo, redo and rebuilds |
| Per-edit block list, incremental (`block_list_incremental_edit_pair`) | not measured | 44.7 µs for two edits, so about 22 µs each | New bench: insert one character and delete it, through `apply_edit` |
| Caret move, full (`document_hidden_ranges`) | 3.45 ms | 4.05 ms | Same function. The change is run-to-run noise in this run (the core bench gave 3.86 ms after and 3.56 ms before) |
| Caret move, cached (`hidden_ranges_cached_caret`) | not measured | 52 µs | New bench: warm cache, caret in the middle |
| `Editor::text()` copy | 6.2 µs | 6.7 µs | Noise. Not changed |
| md_core reparse, one character (`reparse/insert_char_buffer_and_blockmap`) | 51.0 µs | 48.6 µs | `BlockMap::apply_edit` path unchanged apart from its return value. Noise |

The two hot spots drop from about 0.8 ms per typed character and about 3.5 ms per caret move to about 22 µs and about 52 µs. Those were the two largest pure costs on the keystroke path.

### Xvfb keystroke_to_paint (after)

Release build of `-p app` with this change. Document: `perf10k.md`, generated with the bench generator (`crates/md_editor/benches/pure.rs` `generate`) to 10,017 lines and 371 sections. The file is not kept in the repo. The caret was in the middle (Section 181, roughly line 4,900, about 49%). Typing was done with `xdotool type --delay 120`. The log holds 61 characters, not the 58 planned.

| Run | Median ms | p90 ms | Max ms | Under 16 ms |
|---|---|---|---|---|
| Follow-up, this change (one run) | 9.42 | 16.73 | 32.46 | 52 of 61 |
| Previous pass, run 4 (`perf-pass.md` above, different file) | 10.1 | 17.1 | 20.0 | 47 of 58 |

The screenshot is `docs/screenshots/10.4-followup-typed.png`. It shows the typed text in the middle of the paragraph, with the caret placed.

Verdict: the per-character pure work is now about 0.8 ms smaller, but that is about 8% of the median and is within the run-to-run spread. The keystroke target (p90 under 16 ms) is still missed on this rig, and the cost is still in layout, paint and software raster. Caret moves were not timed in Xvfb. Their pure cost is about 3.5 ms smaller per arrow key. The before and after Xvfb numbers come from different files and single runs, so treat them as indicative.

The remaining item on the list is the double layout per block (measure and prepaint), listed in the iteration 2 open items. It is not changed here.
