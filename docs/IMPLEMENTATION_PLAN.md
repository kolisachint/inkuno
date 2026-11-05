# noteapp — Implementation Plan (Iteration 1)

Companion to [DESIGN.md](./DESIGN.md). Section references (§) point to that document.
Iteration 2 (P4 to P10) is planned in [IMPLEMENTATION_PLAN_ITERATION_2.md](./IMPLEMENTATION_PLAN_ITERATION_2.md).

## Contents

- [Scope of iteration 1](#scope-of-iteration-1)
- [How this plan is executed](#how-this-plan-is-executed)
- [Dependency graph](#dependency-graph)
- [Phase 0 — Scaffold](#phase-0--scaffold)
- [Phase 1 — Spikes S1–S4](#phase-1--spikes-s1s4)
- [Phase 2 — md_core](#phase-2--md_core)
- [Phase 3 — md_editor basic](#phase-3--md_editor-basic)
- [Deferred to iteration 2](#deferred-to-iteration-2)
- [Definition of Done](#definition-of-done)
- [Iteration 1 Definition of Done](#iteration-1-definition-of-done)
- [Subagent task template](#subagent-task-template)

---

## Scope of iteration 1

Iteration 1 ends at Phase 3. It delivers a plain-markdown editor and nothing beyond it:

- P0 Scaffold
- P1 Spikes S1 to S4
- P2 md_core
- P3 md_editor basic (tasks 3.1 to 3.7)

The bare minimum this iteration delivers: open, edit, select, navigate, IME, scroll and save a plain `.md` file, on macOS and Linux, released as nightly binaries by `.github/workflows/release.yml`.

Not in this iteration: live preview, code fences, math, Mermaid, modes, the workspace UI and theming. See [Deferred to iteration 2](#deferred-to-iteration-2).

Status values used below: **DONE** and **PARTIAL** (some acceptance is open, reason given).

## How this plan is executed

- This file covers iteration 1 only (P0 to P3). Phases 4 to 10 are in [IMPLEMENTATION_PLAN_ITERATION_2.md](./IMPLEMENTATION_PLAN_ITERATION_2.md).
- Work is dispatched to small-scoped Haiku subagents, one task each.
- Interfaces (types, traits, function signatures) for a phase are defined before that phase's implementation tasks start.
- Every task ends with `cargo build`, `cargo test` and `cargo clippy` passing.
- Risky UI tasks are verified with screenshots.
- The development target is macOS. `md_core` has no GPUI dependency, so it can be developed and tested on any OS and run in CI on Linux.
## Dependency graph

Iteration 1 only. Math and Mermaid are on hold (see [Deferred to iteration 2](#deferred-to-iteration-2)).

```
P0 Scaffold
 |
 +--> P1 Spikes S1, S2, S3, S4
 |      |  S2 -> pinned GPUI rev         (needed by P3)
 |      |  S3 -> display map go/no-go     (needed by 2.6 and 3.2)
 |      |  S4 -> editor base              (needed by P3)
 |      |  S1 -> split render host        (no iteration 1 consumer; on hold with P6/P7)
 |
 +--> P2 md_core  (2.1 .. 2.8; no GPUI; runs in parallel with P1)
        |
        v
      P3 md_editor basic  (needs P1 S2 + S4, and P2)
        3.1 Text element
         |--> 3.2 Caret, selection, click --> 3.3 Keys and clipboard
         |                               \--> 3.4 IME
         |--> 3.5 Virtualized block list
         |--> 3.6 File open and save
         |--> 3.7 Editing keys (after 3.2, 3.3)
```

---

## Phase 0 — Scaffold

### 0.1 Cargo workspace with five crates
- **Status:** DONE. Five crates under `crates/` (`md_core`, `md_render`, `md_editor`, `workspace`, `app`); toolchain pinned to 1.98.1 (`0b1f24c`).
- **Goal:** Create the workspace skeleton and toolchain pin.
- **Crate / files:** root `Cargo.toml`; `rust-toolchain.toml`; crates `md_core`, `md_render`, `md_editor`, `workspace`, `app`.
- **Depends on:** none.
- **Acceptance:** `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root; all five crates are members; `rust-toolchain.toml` pins a toolchain with `rustfmt` and `clippy` components.

### 0.2 Pinned GPUI git dependency
- **Status:** DONE. `gpui` and `gpui_platform` pinned to zed `c7bea343838f42b4c14151393e41f0bbe033c95e` in the root `Cargo.toml` (`34508a6`).
- **Goal:** Depend on GPUI from the `zed-industries/zed` git repository at one exact revision, not crates.io.
- **Crate / files:** root `Cargo.toml`; `Cargo.lock`.
- **Depends on:** 0.1.
- **Acceptance:** `cargo metadata` shows `gpui` sourced from git at the pinned revision; the revision is recorded in a comment next to the dependency; no crates.io `gpui` appears in the lock file.

### 0.3 License and ignore rules
- **Status:** DONE. `LICENSE` is GPL-3.0; all five manifests use `license.workspace = true`; `.gitignore` covers `/target`.
- **Goal:** Declare the licence and keep build artefacts out of git.
- **Crate / files:** `LICENSE` (GPL-3.0 text); `license = "GPL-3.0-or-later"` in each crate manifest; `.gitignore`.
- **Depends on:** 0.1.
- **Acceptance:** every crate manifest has the licence field; `LICENSE` contains the GPL-3.0 text; `.gitignore` excludes `target/`.

### 0.4 CI for md_core
- **Status:** DONE. `.github/workflows/md_core.yml` is enabled (`7a6d1e5`).
- **Goal:** Run formatting, lint and tests for `md_core` on every push.
- **Crate / files:** `.github/workflows/md_core.yml`.
- **Depends on:** 0.1.
- **Acceptance:** the workflow runs on Linux; it runs `cargo fmt --check`, `cargo clippy -p md_core` and `cargo test -p md_core`; it passes on the scaffold commit.

---

## Phase 1 — Spikes S1–S4

Each spike writes a findings note to `docs/spikes/` and ends with a go/no-go decision. Spikes S1 to S4 can run in parallel.

### 1.1 S1 — Render host
- **Status:** DONE (GO, split host). Measured on Linux only. The hidden WKWebView host for Mermaid is UNVERIFIED on macOS; this matters for iteration 2 (on hold), not iteration 1.
- **Goal:** Decide between a hidden WKWebView and an offscreen NSWindow. Confirm that Mermaid and MathJax produce SVG, and resolve the UNVERIFIED items in DESIGN §14.
- **Crate / files:** `docs/spikes/S1-render-host.md`; throwaway code kept outside the workspace.
- **Depends on:** 0.1.
- **Acceptance:** the note records measured round-trip latency; it states whether Mermaid output with `htmlLabels: false` contains any `<foreignObject>`; it states whether resvg renders the output; it contains a go/no-go line; it names the pinned mermaid.js and MathJax v3 versions.

### 1.2 S2 — GPUI on macOS
- **Status:** PARTIAL. Builds and opens a window on Linux under Xvfb (`docs/spikes/S2-linux-hello.png`). The macOS screenshot and go/no-go line are PENDING (AGENTS blocker 1).
- **Goal:** Prove the pinned GPUI git dependency builds and opens a window.
- **Crate / files:** `docs/spikes/S2-gpui-hello.md`; a hello-window binary in `app`.
- **Depends on:** 0.2.
- **Acceptance:** `cargo run -p app` opens a window on macOS; a screenshot is attached to the note; the note records the revision that worked and contains a go/no-go line.

### 1.3 S3 — Display map prototype
- **Status:** DONE (conditional GO). The caret criterion is met and property-tested in `md_core` (`cb61084`). The IME criterion across hidden ranges is NOT evaluated; it is macOS-only (AGENTS blocker 2b) and gates 3.4 on macOS.
- **Goal:** Prove caret movement and IME work across hidden `**` markers.
- **Crate / files:** `docs/spikes/S3-display-map.md`; prototype confined to a spike branch.
- **Depends on:** 0.2.
- **Acceptance:** the caret moves past a hidden `**` pair in one keystroke; IME composition works across the hidden range; the note contains a go/no-go line.

### 1.4 S4 — Editor base evaluation
- **Status:** DONE (provisional GO: Zed `editor` crate). Confirm on macOS together with S3.
- **Goal:** Choose between the `gpui-component` editor and the Zed `editor` crate as the base for `md_editor`.
- **Crate / files:** `docs/spikes/S4-editor-base.md`.
- **Depends on:** 0.2.
- **Acceptance:** the note compares both options on display map support, selections, IME and licence; it names the chosen base; it contains a go/no-go line.

---

## Phase 2 — md_core

All tasks in this phase are pure Rust with no GPUI dependency. They are unit-tested on Linux CI.

### 2.1 Buffer, transactions, undo and redo
- **Status:** DONE (`75fa05c`).
- **Goal:** Wrap Zed's `text` / `rope` for insert, delete and replace, with grouped transactions.
- **Crate / files:** `md_core/src/buffer.rs`.
- **Depends on:** 0.1, 0.4.
- **Acceptance:** insert, delete and replace produce the expected text; a transaction groups several edits into one undo step; undo followed by redo restores the exact prior text; tests cover empty buffers and multi-byte UTF-8 input.

### 2.2 Anchors and coordinate conversion
- **Status:** DONE (`75fa05c`).
- **Goal:** Provide anchors that survive edits, and convert between byte offset, line and column.
- **Crate / files:** `md_core/src/anchor.rs`, `md_core/src/coords.rs`.
- **Depends on:** 2.1.
- **Acceptance:** an anchor keeps pointing at the same text after an edit before it; offset to line/column and back round-trips on all test inputs; tests include CRLF-free and multi-byte lines.

### 2.3 Block splitter
- **Status:** DONE (`1627414`).
- **Goal:** Split the buffer into top-level blocks, each with a source range, and re-split only dirty ranges.
- **Crate / files:** `md_core/src/block.rs`.
- **Depends on:** 2.2.
- **Acceptance:** each block type in DESIGN §3.2 is recognized in tests; block ranges tile the document with no gaps or overlaps; an edit re-splits only blocks that overlap the dirty range (verified by a counter in tests).

### 2.4 Per-block parse with hash cache
- **Status:** DONE (`1627414`).
- **Goal:** Parse each block with the pulldown-cmark offset iterator and cache results by content hash.
- **Crate / files:** `md_core/src/parse.rs`.
- **Depends on:** 2.3.
- **Acceptance:** every event carries a source range inside its block; an unchanged block hits the cache (verified by a counter); a changed block misses the cache and is reparsed.

### 2.5 Inline span model
- **Status:** DONE (`38b96b9`).
- **Goal:** For each inline span kind in DESIGN §4.1, produce the marker ranges and the content range.
- **Crate / files:** `md_core/src/inline.rs`.
- **Depends on:** 2.4.
- **Acceptance:** strong, emphasis, inline code, link, image, inline math and strikethrough each produce correct marker and content ranges in table-driven tests; nested spans are reported with correct ranges.

### 2.6 Display map
- **Status:** DONE (`cb61084`).
- **Goal:** Map between source offsets and display positions, collapse hidden marker ranges, and let caret movement and hit-testing skip hidden ranges.
- **Crate / files:** `md_core/src/display_map.rs`.
- **Depends on:** 2.5, and the S3 go decision (1.3).
- **Acceptance:** a caret moving right across `**bold**` with the caret outside the span lands on the first visible content character in one step; a hit-test at a collapsed position resolves to the nearest visible offset; both behaviours are covered by tests.

### 2.7 Input rules engine
- **Status:** DONE (`79b4e82`).
- **Goal:** Implement each rule in DESIGN §4.3 as a pure function from (buffer, caret, typed input) to a list of edits.
- **Crate / files:** `md_core/src/input_rules.rs`.
- **Depends on:** 2.1, 2.3.
- **Acceptance:** each rule row in DESIGN §4.3 has at least one test that asserts the exact resulting text and caret; Enter on an empty list item exits the list in a test; no rule produces edits outside its block.

### 2.8 Property tests for the display map
- **Status:** DONE (`cb61084`).
- **Goal:** Check that the display map round-trips under random edits and random caret positions.
- **Crate / files:** `md_core/tests/display_map_props.rs`.
- **Depends on:** 2.6.
- **Acceptance:** for random documents and caret positions, source to display to source returns the original offset whenever the offset is visible; the property test runs in `cargo test` with a fixed seed.

---

## Phase 3 — md_editor basic

Requires P1 spikes S2 and S4, and Phase 2.

### 3.1 Text element
- **Status:** DONE (`dbf67cd`). Screenshots: `docs/screenshots/p3-3.1.png` and `docs/screenshots/p3-3.1-100lines.png`.
- **Goal:** Render plain text from the buffer in a GPUI custom element.
- **Crate / files:** `md_editor/src/element.rs`.
- **Depends on:** 2.1, 1.2, 1.4.
- **Acceptance:** a 100-line document is drawn with correct line breaks; screenshot of the window matches the buffer text.

### 3.2 Caret, selection and click placement
- **Status:** DONE.
- **Goal:** Draw the caret and selections, and place the caret on click through hit-testing.
- **Crate / files:** `md_editor/src/cursor.rs`, `md_editor/src/selection.rs`.
- **Depends on:** 3.1, 2.6.
- **Acceptance:** a click at a given character places the caret before that character; drag selection highlights the expected range; screenshot-verified.

### 3.3 Keyboard navigation and clipboard
- **Status:** DONE.
- **Goal:** Arrow keys, word and line movement, Home and End, Shift selection, and clipboard copy, cut and paste.
- **Crate / files:** `md_editor/src/keys.rs`, `md_editor/src/clipboard.rs`.
- **Depends on:** 3.2.
- **Acceptance:** each movement key moves the caret by the expected unit in tests; Cmd+C, Cmd+X and Cmd+V round-trip plain text; copy produces Markdown source.

### 3.4 IME via EntityInputHandler
- **Status:** DONE. The macOS IME check is still open (AGENTS blocker 2b).
- **Goal:** Support IME composition and marked text.
- **Crate / files:** `md_editor/src/ime.rs`.
- **Depends on:** 3.2.
- **Acceptance:** a Japanese IME composition shows marked text and commits the final text into the buffer as one edit; the marked range is cleared on commit.

### 3.5 Virtualized block list
- **Status:** DONE.
- **Goal:** Lay out blocks in a GPUI `list()`, so only visible blocks are laid out.
- **Crate / files:** `md_editor/src/block_list.rs`.
- **Depends on:** 3.1, 2.4.
- **Acceptance:** a 10,000-line document scrolls with only visible blocks laid out (verified by a counter); scrolling to the end shows the last line.

### 3.6 File open and save
- **Status:** DONE (`56537e9`). Save is bound to `cmd-s` on macOS and `ctrl-s` on Linux.
- **Goal:** Open a `.md` file into the buffer and save the buffer back to disk.
- **Crate / files:** `app/src/file_io.rs`.
- **Depends on:** 3.1.
- **Acceptance:** open then save without edits leaves the file byte-identical; Cmd+S writes edits to disk; the window shows a dirty marker after an edit and clears it after save.

### 3.7 Editing keys
- **Status:** DONE. Screenshots: `docs/screenshots/p3-3.7-*.png` (Linux, Xvfb display 87, xdotool). Undo is per keystroke: typing is not coalesced into one undo step.
- **Goal:** Backspace, Delete, Enter, Tab (four spaces), word delete, undo and redo, and select all, with the caret left at the edit position.
- **Crate / files:** `md_editor/src/edit_keys.rs`.
- **Depends on:** 3.2, 3.3.
- **Acceptance:** Backspace and Delete remove one character (or the selection) across ASCII, accented, emoji and CJK text; Enter and Tab replace the selection with a newline or four spaces, with no doubled newline from IME input; Cmd+Z / Cmd+Shift+Z (Ctrl+Z / Ctrl+Shift+Z, Ctrl+Y on Linux) undo and redo through the md_core buffer; Cmd+A (Ctrl+A) selects all; Option+Backspace (Ctrl+Backspace) deletes a word; typing over a selection replaces it.

### Integration note: per-block layout (after 3.5)

- A per-block layout fix landed after 3.5. `Editor.layout` became `Editor.layouts: Layouts`, with one `BlockLayout` per visible block. Hit testing, caret placement and IME bounds use the block under the point, or the nearest visible block.
- `cursor::register_mouse_handlers` now takes `(editor, bounds, window)`. It is called from `BlockHost::paint`, which also registers the IME input handler. The P3 interfaces doc said not to change these; they were needed to fix an integration bug.
- The Linux gate passed: 11 of 11 checks PASS. See [P3-gate-linux.md](./P3-gate-linux.md). The macOS gate run (screenshot, IME composition) is still pending.

---

## Deferred to iteration 2

Phases 4 to 10 are not part of iteration 1. Their task text, the starting point and the suggested order are in [IMPLEMENTATION_PLAN_ITERATION_2.md](./IMPLEMENTATION_PLAN_ITERATION_2.md).

**On hold: math and Mermaid.** Do NOT work on math (MathJax/QuickJS, P6 tasks 6.1, 6.3, 6.5 and 6.6) or Mermaid (P7), including the existing `md_render` math and diagram code, until the repo owner explicitly confirms. These are on hold. Tasks 6.2 and 6.4 are not on hold. The held task text is kept in the iteration 2 plan under "On hold (needs owner confirmation)".

Math (P6) and Mermaid (P7) were deferred from iteration 1 to iteration 2, and are now on hold within iteration 2.

---

## Definition of Done

A task is done when all of the following hold:

- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root with no new warnings.
- Every acceptance criterion in the task is met and checked.
- Risky UI tasks have a screenshot attached to the task record.
- No file outside the task's stated crate or files has changed, except for shared manifests that the task names.
- The GPUI revision is unchanged unless the task is the revision bump.
- If the task changes a decision recorded in DESIGN, the decision is updated in DESIGN or moved to Open Questions.

A phase is done when every task in it is done and the phase gate holds:

| Phase | Gate |
|-------|------|
| P0 | CI green on the scaffold commit |
| P1 | All four spike notes exist and each has a go/no-go line |
| P2 | `md_core` tests pass on Linux CI, including property tests |
| P3 | Plain-text file opens, edits, IME-composes and saves on macOS. Linux gate passed (`docs/P3-gate-linux.md`); macOS run pending |

---

## Iteration 1 Definition of Done

**Status: Iteration 1 complete on Linux (gate passed). The macOS gate run (screenshot, IME composition) is still pending and needs a Mac.**

Iteration 1 is complete when the P3 gate holds: a plain-text file opens, edits, IME-composes and saves on macOS (gate table row P3).

Also needed for the iteration to close:

- Every P0 to P3 task is DONE in the status lines above. Task 1.2 is PARTIAL until its macOS run is recorded.
- The two items that can keep the gate from holding both need a Mac: AGENTS blocker 1 (the S2 macOS run and screenshot) and blocker 2(b) (the macOS IME check for 3.4). Linux runs and screenshots cover everything else.
- Nightly macOS and Linux binaries are published from the iteration's final commit by `.github/workflows/release.yml`.
- Iteration 2 starts from [IMPLEMENTATION_PLAN_ITERATION_2.md](./IMPLEMENTATION_PLAN_ITERATION_2.md) once this holds.

---

## Subagent task template

Use this template for each task dispatched to a Haiku subagent in the implementation session. Fill every field. Do not leave a field empty; write "none" if it does not apply.

```markdown
## Task <ID>: <title>

**Context to read first**
- docs/DESIGN.md §<n>
- docs/IMPLEMENTATION_PLAN.md task <ID>
- <existing files this task touches or depends on>

**Scope**
- What to build, in one or two sentences.
- Crate and files this task may change.

**Interfaces** (defined before implementation; do not change)
- <type, trait or function signature>

**Acceptance tests**
- <each acceptance criterion, copied from the plan, as a test or screenshot>

**Out of scope**
- <work that belongs to other tasks>

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- Risky UI tasks include a screenshot.
```
