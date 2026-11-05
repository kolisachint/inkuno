# S3: Display map prototype

Plan task 1.3. DESIGN §3.2, §4.1, §12, §13 (S3).

The display map is already built (`crates/md_core/src/display_map.rs`, tasks 2.6 and 2.8, commit `cb61084`). This spike evaluates that code against S3's acceptance criteria. No new prototype code was added. The one timing run used a throwaway example that was deleted afterwards.

Measured on Linux, Rust 1.97.0, release profile. Nothing here ran in GPUI, on macOS, or with a real IME.

## Acceptance criteria

| Criterion (plan 1.3) | Status | Evidence |
|----------------------|--------|----------|
| The caret moves past a hidden `**` pair in one keystroke | **Met in md_core.** | Unit tests `move_right_passes_one_visible_char` (`"a **bold** c"`, caret 2 to 5, so the whole `**` pair and `b` are passed in one step), `move_across_closing_markers` (8 to 11, and 11 back to 10), `move_left_passes_one_visible_char`, `move_handles_multibyte_text` (`"é**x**"`). The property test `move_caret_never_lands_inside_hidden` (`crates/md_core/tests/display_map_props.rs`) checks both directions on random documents. Not tested: keystroke routing in GPUI. |
| IME composition works across the hidden range | **Not evaluated. Not implemented.** | `md_core` has no marked-text API. A search of `crates/md_core/src` and `crates/md_editor/src` finds no IME or UTF-16 code. The display map works in byte offsets. macOS input (`NSTextInputClient`) works in UTF-16 units, so a conversion layer and a rule for marked ranges over hidden text are both missing. This needs `md_editor` on macOS. |
| The note contains a go/no-go line | Below. | |

## Behaviour checked

Unit tests in `display_map::tests` (all pass):

- Mapping and hit-testing: `for_text_hides_both_marker_runs`, `caret_inside_span_hides_nothing`, `caret_none_hides_all_markers`, `to_display_at_boundaries`, `to_source_at_collapse_points`, `to_source_between_collapse_points`, `to_source_at_document_ends`, `hidden_range_at_document_start_and_end`, `is_hidden_is_half_open`, `clip_snaps_inside_hidden_to_closer_end`, `clip_keeps_caret_stops_and_clamps`, `multibyte_round_trip`.
- Normalization: `new_normalizes_unsorted_overlapping_and_clipped_ranges`, `adjacent_ranges_merge`, `ranges_past_source_end_are_dropped`, `empty_hidden_set_is_identity`.
- Caret movement at the edges: `move_at_document_ends_is_unchanged`, `move_over_trailing_hidden_range_stops_at_visible_end`, `move_from_inside_hidden_range_starts_at_edge`, `caret_inside_span_hit_test_stays_visible_after_move`.

Property tests (`crates/md_core/tests/display_map_props.rs`, `proptest` 256 cases, `RngSeed::Fixed(42)`):

- `visible_offsets_round_trip`: every visible offset maps to display and back, with either bias.
- `to_display_is_monotonic`.
- `display_len_is_source_minus_hidden`.
- `move_caret_never_lands_inside_hidden`.
- `for_text_never_panics_and_hides_valid_ranges`.

Baseline: `cargo test -p md_core` passes 156 unit tests and 5 property tests.

## Performance (DESIGN §12 target: keystroke to paint under 16 ms on 10,000 lines)

Document: 10,000 lines, 958,890 bytes, each line with bold, emphasis, code, a link, strikethrough and inline math.

| Operation | Time |
|-----------|------|
| `DisplayMap::for_text` over the whole document (120,000 hidden ranges) | 30.7 ms, one-time |
| `DisplayMap::for_text` over one block (about 100 bytes, caret varies), average of 2,000 builds | 2.2 us |
| `move_caret` Right on a block, average of 200,000 | 0.03 us |
| `to_display` + `to_source` + `clip` on a block, average of 200,000 | 0.02 us |
| Whole-document sweep of `clip` and `to_display` (962 probes) | 0.23 ms |

The keystroke path works on one block, so the cost is in microseconds, well under the budget. A single whole-document build is 30.7 ms, which is over the 16 ms budget if it ran on every keystroke. The editor must keep the map per block, as DESIGN §3.2 says, and never rebuild the whole document for a keystroke.

## Findings

1. The caret criterion holds at the core level, with property tests that cover random documents and both directions.
2. IME is the gap. The map's byte-offset API needs a UTF-16 conversion for the platform input handler, and the rules for marked text over hidden markers are not designed yet. The check has to happen in `md_editor` on macOS, which cannot be done on Linux.
3. `for_text` parses the text it is given in full. The pipeline must call it per block (2.3 and 2.4 provide the block ranges), and the note above shows why a whole-document call is too slow for a keystroke.
4. S4 already leaves the display map ownership with md_core (`S4-editor-base.md`, finding 5). That is compatible with this result.

Not verified:

- Keystroke routing and caret movement inside GPUI.
- IME composition across hidden ranges, on macOS, with a real input method.
- Per-block timing inside the real pipeline (only the map itself was timed).

Go/no-go: GO (conditional). The caret criterion is met and property-tested in md_core, so task 2.6 can rely on the map. The IME criterion is not evaluated and has no implementation yet. It stays open and must be verified in `md_editor` on macOS before any input work is accepted.
