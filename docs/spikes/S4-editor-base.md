# S4: Editor base for md_editor

Plan task 1.4. DESIGN §7, §11, §13.

Candidates:

- **A. Zed `editor` crate** (`crates/editor` in zed-industries/zed at `c7bea343838f42b4c14151393e41f0bbe033c95e`, the same rev as the GPUI pin).
- **B. `gpui-component` editor/input** (longbridge/gpui-component at `0ab9cf4ed6d237457e66efc56c8ffcf915e8ce05`, 2026-11-04, the default branch HEAD at the time of the check). Its input state lives in `crates/base/src/input/`, with the component wrapper in `crates/component/src/input/`.

Sources are read from those two checkouts. Nothing was compiled.

## Comparison

| Criterion | A. Zed `editor` | B. `gpui-component` |
|-----------|-----------------|---------------------|
| **Display map** | `crates/editor/src/display_map.rs`: a `DisplayMap` stacking fold, inlay, tab, wrap and block maps (`fold_map.rs`, `inlay_map.rs`, `tab_map.rs`, `wrap_map.rs`, `block_map.rs`). Folds and inlays are supported. | `crates/base/src/input/editor/display_map/`: `DisplayMap`, `WrapMap`, `FoldMap` (fold runs over wrap rows). Folds and soft-wrap are supported. No inlay map was found; the README describes only wrap and fold layers. |
| **Multi-selection** | `SelectionsCollection` (`crates/editor/src/selections_collection.rs`). | `Vec<CursorSelection>` in the input state, with `add_cursor_above`, `add_cursor_below` and `add_cursor_at` (`crates/base/src/input/base/state.rs`). |
| **IME** | `impl EntityInputHandler for Editor` (`crates/editor/src/input.rs`, around line 2798), including `replace_and_mark_text_in_range`. | `impl<M: InputModeKind> EntityInputHandler for InputBaseState<M>` (`crates/base/src/input/base/state.rs`, around line 4537). |
| **Licence** | `license = "GPL-3.0-or-later"` in `crates/editor/Cargo.toml`. Matches the project licence (DESIGN §11). | `license = "Apache-2.0"` in `crates/base/Cargo.toml` and `crates/component/Cargo.toml`. `LICENSE-APACHE` fetched from `raw.githubusercontent.com` (HTTP 200). A `LICENSE-MIT` was not found (HTTP 404). Apache-2.0 is compatible with the GPL-3.0-or-later project (DESIGN §11). |
| **GPUI** | The same zed git GPUI as our pin (`gpui.workspace = true`, resolving to the pinned rev). | Depends on crates.io `gpui-pre` `=0.3.11` (aliased as `gpui`) and `gpui-pre-sum-tree` `=0.3.11`, plus optional `gpui-fast`. This is a different GPUI line from our pin (zed git 0.2.2). |
| **Text storage** | Zed `text` / `rope` / `sum_tree` (GPL-3.0-or-later per DESIGN §11). | `ropey` `=2.0.0-beta.1` and `sum-tree`. |
| **Dependency weight** | 72 workspace-inherited entries in `[dependencies]` (11 optional). Transitive closure over zed workspace crates: **96 crates** (computed by a script over the checkout's manifests: non-optional normal deps only, target-gated deps not followed, so this is a lower bound and UNVERIFIED against a real `cargo tree`). Includes `project`, `language`, `workspace`, `ui`, `settings`, `multi_buffer`, `lsp`, `theme`, `db`. | `crates/base` has 27 non-optional `[dependencies]` entries, almost all external crates, plus two GPUI-line crates. No zed workspace crates. |
| **Fit with md_core DisplayMap** | Zed's `DisplayMap` is coupled to `MultiBuffer`, `Editor` and language/project state. md_core's map (DESIGN §3.2, task 2.6) would have to sit beside it, or replace it. | Its `DisplayMap` lives in the `gpui-component` base crate, which uses GPUI, and is coupled to its input state. md_core's map would also have to sit beside it or replace it. Note that md_core's `display_map.rs` is currently a one-line stub (task 2.6 not started), so no md_core behaviour can be compared yet. |

## Findings that decide it

1. **GPUI lineage.** Option B depends on a different GPUI release line (`gpui-pre` 0.3.11 from crates.io) from the one DESIGN §10 pins (zed git). Using B would mean changing the GPUI pin, or building against two GPUI copies. Neither is allowed by the current plan. This is not verified to fail at compile time; it is a mismatch in the manifests at the revisions read.
2. **Licence.** Both are acceptable under DESIGN §11. A is GPL-3.0-or-later, the same as the project. B is Apache-2.0, which is compatible.
3. **Weight.** A pulls about 96 zed workspace crates through `editor`. B's base crate has about 27 direct dependencies. This is the main cost of A and is the reason to avoid it.
4. **Feature coverage.** Both cover folds, multi-selection and IME at the source level. A covers inlays too; B's README does not describe inlays.
5. **Ownership of the display map.** DESIGN §3.2 puts the display map in md_core. Neither candidate's display map should be the source of truth for hidden `**` ranges. Whichever base is chosen, md_editor should route hit-testing and caret movement through md_core's map and use the base only for input, selection and paint plumbing. This is a design constraint, not a result of the comparison.

## Recommendation

**Choose A, the Zed `editor` crate, provisionally.**

Reasons:

- It uses the same GPUI pin as the rest of the project (finding 1), which is the only hard blocker found for B.
- Its licence is the project's licence (finding 2).
- It has the richest source-level feature set (finding 4), so it is the safer base if md_core's map needs a reference for edge cases such as inlays and wrapping.

Cost accepted: A's weight (finding 3) is high. The mitigation is to depend on `editor` only where md_editor needs its input and selection types, and to re-measure the dependency closure once the crate is actually added. If the weight blocks the macOS build or the render budget in DESIGN §12, B becomes the fallback, but only after its GPUI line is reconciled with the pin.

Not verified:

- Whether the 96-crate closure matches a real `cargo tree` (see table).
- Whether gpui-component's `gpui-pre` 0.3.11 can build against zed git GPUI at all. UNVERIFIED.
- Behaviour of either IME implementation with hidden ranges. This needs S3 on macOS.
- Whether either editor's display map handles a hidden `**` range correctly. Not tested.

Go/no-go: GO (provisional) — Zed `editor` crate; confirm on macOS with S3.
