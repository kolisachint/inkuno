# noteapp — Implementation Plan (Iteration 2)

Companion to [DESIGN.md](./DESIGN.md). Section references (§) point to that document.
Iteration 1 (P0 to P3) is in [IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md) and must hold its gate first.

> **On hold: math and Mermaid.** Do NOT work on math (MathJax/QuickJS, P6 tasks 6.1, 6.3, 6.5 and 6.6) or Mermaid (P7), including the existing `md_render` math and diagram code, until the repo owner explicitly confirms. Their task text is in the section "On hold (needs owner confirmation)" at the end of this file.

## Contents

- [Starting point](#starting-point)
- [Suggested order](#suggested-order)
- [Open items carried over](#open-items-carried-over)
- [Dependency graph](#dependency-graph)
- [How this plan is executed](#how-this-plan-is-executed)
- [Phase 4 — Live preview](#phase-4--live-preview)
- [Phase 5 — Code fences](#phase-5--code-fences)
- [Phase 6 — Render host and Math (tasks not on hold)](#phase-6--render-host-and-math-tasks-not-on-hold)
- [Phase 8 — Modes](#phase-8--modes)
- [Phase 9 — Workspace](#phase-9--workspace)
- [Phase 10 — Theming and polish](#phase-10--theming-and-polish)
- [On hold (needs owner confirmation)](#on-hold-needs-owner-confirmation)
- [Definition of Done](#definition-of-done)
- [Subagent task template](#subagent-task-template)

---

## Starting point

What iteration 1 leaves in the repo, and what iteration 2 can reuse. Task IDs refer to this plan's numbering, which matches iteration 1.

### Reusable code already in the repo

| Area | Files | What exists | Used by |
|------|-------|-------------|---------|
| Highlighting | `crates/md_render/src/highlight.rs` | `Highlighter` trait (no GPUI types) and tree-sitter implementation (5.1) | 5.3, 8.1 |
| Fence language | `crates/md_render/src/lang.rs` | Info string to language, plain text fallback (5.2) | 5.3 |
| Render traits | `crates/md_render/src/traits.rs` | Render traits from DESIGN §6.7, as implemented in 6.2 | 6.5 (on hold), 7.1 (on hold) |
| Cache | `crates/md_render/src/cache.rs` | Result cache keyed by `hash(kind, source, theme)` (6.2) | 10.4 |
| Queue | `crates/md_render/src/queue.rs` | Async request queue; `DEFAULT_MERMAID_DEBOUNCE` = 300 ms (6.2, 7.3 core) | 7.3 UI (on hold) |
| Rasterization | `crates/md_render/src/raster.rs` | resvg SVG to pixels (6.4). The screenshot with bundled fonts waits on 6.3 (on hold) | 6.6 (on hold) |
| Fallback | `crates/md_render/src/fallback.rs` | Last-good fallback for diagram renders (7.3 core) | 7.3 UI (on hold) |
| Vault tree | `crates/workspace/src/tree.rs` | Tree model, `Expansion`, `visible_rows` (9.1 core) | 9.1 sidebar UI |
| Search | `crates/workspace/src/search.rs` | Full-text search core, `MatchSpan`; skips unreadable files (9.4 core) | 9.4 search panel UI |
| File ops | `crates/workspace/src/ops.rs` | Create, rename, delete (9.3, DONE) | 9.5 |
| Watcher | `crates/workspace/src/watcher.rs` | `VaultWatcher` and `plan_buffer_updates` (9.2 core); gitignored notes hidden | 9.2 app side |
| Editor | `crates/md_editor/src/` | Iteration 1 editor: `element.rs`, `selection.rs`, `cursor.rs`, `keys.rs`, `clipboard.rs`, `ime.rs`, `block_list.rs`, `editor.rs` | 4.x, 8.x |
| Display map and parsing | `crates/md_core/src/` | `display_map.rs` (2.6), `block.rs` and `parse.rs` (2.3, 2.4), `inline.rs` (2.5), `input_rules.rs` (2.7) | 4.1 to 4.5, 5.3, 8.x |

### Decisions already made

- **Render host split** ([S1](./spikes/S1-render-host.md#decision), DESIGN §6.1). Math is meant to run in process (MathJax v4 in embedded QuickJS, no WebView). Mermaid is meant to run in a hidden WebView because it needs a DOM. Both are on hold with P6 and P7.
- **Editor base:** Zed `editor` crate (S4, provisional GO; confirm on macOS with S3).
- **Display map:** conditional GO (S3). The caret criterion is met; IME is open.
- **Mermaid debounce:** 300 ms (DESIGN §16 item 8).

### S1 prototype example

- File: `crates/md_render/examples/spike_s1_math.rs`. It sits behind the features `spike-quickjs` and `spike-resvg`, is off by default, and is not wired into `RenderQueue`.
- Run: `cargo run -p md_render --example spike_s1_math --features "spike-quickjs spike-resvg" --release -- <mathjax-bundle.js> <out-dir> [mermaid.svg]`
- It ran MathJax v3.2.2 only. Re-verify with MathJax v4 before reusing it. It is a reference, not a production path. Touching it is part of the on-hold math work.

---

## Suggested order

1. **P4 Live preview** (4.1 to 4.5). Needs P3. Nothing else in iteration 2 shows rendered output without it.
2. **P5.3 CodeFence element and fence reveal.** 5.1 and 5.2 are done. Needs 4.4.
3. **P8 Modes** (8.1 to 8.5). Needs 4 and 5. 8.5 key bindings wait on DESIGN §16 item 1.
4. **P9 Workspace UI.** 9.1 sidebar, 9.4 search panel, 9.2 applying watcher updates in the app, 9.5 image paste. 9.3 is done. Needs P3 and P4.
5. **P10 Theming and polish.** Needs 8 and 9. Also needs 6 and 7 for their parts, which are on hold, so the Mermaid and math render parts of 10.4 wait too.

Held until the owner confirms: P6 math (6.1, 6.3, 6.5, 6.6) and P7 Mermaid. See [On hold](#on-hold-needs-owner-confirmation). Tasks 6.2 and 6.4 are not held, but 6.4's screenshot waits on 6.3.

---

## Open items carried over

From AGENTS.md, the spike notes and the Linux P3 gate (`docs/P3-gate-linux.md`). Items that belong to held tasks are kept here so they are not lost.

- **Undo is per keystroke.** Typing is not grouped into transactions. Group typing runs into one transaction each (2.1, 3.7).
- **Backspace, Delete and word boundaries are char-based, not grapheme-based** (3.7).
- **Up and Down use a byte goal column.** It drifts visually in proportional fonts. Use an x-position goal instead (3.3).
- **A caret revealed far below the viewport is scrolled to the top, not the bottom.** The block's height is unmeasured until the list lays it out, so this needs a second frame.
- **Drag autoscroll is implemented but unverified** (`scroll_by` in the block list).
- **IME `bounds_for_range`, `character_index_for_point` and `text_length_utf16` are unverified** (macOS only; AGENTS blocker 2b).
- **Each IME call copies the full document text (O(n)).**
- **Virtualization is at block granularity**, so a giant single paragraph is laid out whole.
- **macOS run of the WebView host** (AGENTS blocker 2a; DESIGN §14 item 1). Gates the WebView half of 6.1 and 7.1. Held with them. Unblock: a macOS run.
- **Mermaid output without `<foreignObject>` on every diagram type** (7.1). S1 checked flowcharts only. Held with 7.1.
- **MathJax v4 version and re-verification** (DESIGN §16 item 5; 6.3). The exact v4 version is still to be chosen. The S1 prototype ran v3.2.2 only. The v4 call that replaces `tex2svgPromise` is confirmed in 6.3. Held.
- **Mermaid host approach** is still open: hidden WKWebView or offscreen NSWindow (DESIGN §16 item 4). Held.
- **Screenshot with bundled fonts** (6.4) waits on 6.3. Held.
- **Mode key bindings** (DESIGN §16 item 1). Proposals are Cmd+/, F8, F9; undecided. Task 8.5 keymap. Save is `cmd-s` on macOS and `ctrl-s` on Linux (see 3.6); keep both in the 8.5 keymap.
- **Sidebar and search panel UI** (9.1, 9.4) are not started. Watcher updates must be applied in the app (9.2); the 1 s latency is not asserted in tests.
- **Image paste** (9.5) is not started.
- **Settings storage** location and format (DESIGN §16 item 9; 10.3) is undecided.
- **Performance targets** (DESIGN §12) need measuring in 10.4; the results go to `docs/spikes/perf-pass.md`, which does not exist yet.
- **Rope backend** (DESIGN §16 item 10). md_core uses `ropey` behind `Buffer`. Undecided; not phase-specific.
- **Linux scope conflict.** DESIGN §1.2 lists Linux as a v1 non-goal, but iteration 1 and AGENTS treat Linux as a target. The owner decides and updates DESIGN §1.2.
- **macOS signing and notarization** (AGENTS blocker 3). Release item, not phase-specific. Stays in AGENTS.md.

---

## Dependency graph

```
P0 Scaffold
 |
 +--> P1 Spikes S1, S2, S3, S4
 |      |  S2 -> pinned GPUI rev     (needed by P3)
 |      |  S3 -> display map go/no-go (needed by 2.6)
 |      |  S4 -> editor base          (needed by P3)
 |      |  S1 -> split render host: QuickJS math, WebView for Mermaid (needed by P6, P7)
 |
 +--> P2 md_core  (2.1 .. 2.8; no GPUI; runs in parallel with P1)
        |
        v
      P3 md_editor basic  (needs P1 S2 + S4, and P2)
        |
        v
      P4 Live preview  (needs P3)
        |         \
        v          \
      P5 Code fences  \
        |              \
        +------+        \
               v         v
      P6 Render host and Math  (needs P1 S1, P4)
        |
        v
      P7 Mermaid  (needs P6)

      P8 Modes  (needs P4 and P5)
      P9 Workspace  (needs P3 and P4)

      P10 Theming and polish  (needs P6, P7, P8, P9)
```

---

## How this plan is executed

- This file covers iteration 1 only (P0 to P3). Phases 4 to 10 are in [IMPLEMENTATION_PLAN_ITERATION_2.md](./IMPLEMENTATION_PLAN_ITERATION_2.md).
- Work is dispatched to small-scoped Haiku subagents, one task each.
- Interfaces (types, traits, function signatures) for a phase are defined before that phase's implementation tasks start.
- Every task ends with `cargo build`, `cargo test` and `cargo clippy` passing.
- Risky UI tasks are verified with screenshots.
- The development target is macOS. `md_core` has no GPUI dependency, so it can be developed and tested on any OS and run in CI on Linux.

---

## Phase 4 — Live preview

### 4.1 Basic block elements
- **Goal:** Render headings, paragraphs with styled runs, blockquotes and thematic breaks.
- **Crate / files:** `md_editor/src/blocks/basic.rs`.
- **Depends on:** 3.5, 2.4.
- **Acceptance:** each element type renders its sample in a screenshot test; heading levels 1 to 6 have distinct sizes.

### 4.2 Lists and task items
- **Goal:** Render bullet, ordered and task list items, including nesting.
- **Crate / files:** `md_editor/src/blocks/list.rs`.
- **Depends on:** 4.1.
- **Acceptance:** nested lists indent by one level per nesting; task items show checked and unchecked states; screenshot-verified.

### 4.3 Read-only tables and image blocks
- **Goal:** Render tables read-only, and render images from relative paths.
- **Crate / files:** `md_editor/src/blocks/table.rs`, `md_editor/src/blocks/image.rs`.
- **Depends on:** 4.1.
- **Acceptance:** a three-column table with a header row renders with aligned columns; an image with a relative path resolves against the note's folder; a missing image shows a visible placeholder.

### 4.4 Inline reveal
- **Goal:** Hide inline markers outside the caret's span and reveal them inside it (DESIGN §4.1).
- **Crate / files:** `md_editor/src/inline_render.rs`.
- **Depends on:** 2.6, 2.5, 4.1.
- **Acceptance:** moving the caret into `**bold**` shows the `**` markers; moving it out hides them; the same holds for each span type in DESIGN §4.1; screenshot-verified.

### 4.5 Input rules wired to keys
- **Goal:** Route key input through the input rules engine.
- **Crate / files:** `md_editor/src/keys.rs` (extended).
- **Depends on:** 2.7, 3.3.
- **Acceptance:** typing `# ` at the start of a line produces a heading in a screenshot test; Enter continues a list item; Backspace at block start applies the demotion in DESIGN §4.3.

---

## Phase 5 — Code fences

### 5.1 Highlighter trait and tree-sitter implementation
- **Goal:** Define the `Highlighter` trait and implement it with tree-sitter.
- **Crate / files:** `md_render/src/highlight.rs`.
- **Depends on:** 0.1.
- **Acceptance:** the trait is defined before implementation (interface first); a Rust snippet produces keyword and string token ranges in a test; the trait has no GPUI types.

### 5.2 Language detection from info string
- **Goal:** Map a fence's info string to a highlighter language, falling back to plain text.
- **Crate / files:** `md_render/src/lang.rs`.
- **Depends on:** 5.1.
- **Acceptance:** a table test maps each supported info string to its language; an unknown info string yields plain text; an empty info string yields plain text.

### 5.3 CodeFence element and fence reveal
- **Goal:** Render code fences as highlighted text, with fence lines revealed only when the caret is inside the block (DESIGN §4.2).
- **Crate / files:** `md_editor/src/blocks/code_fence.rs`.
- **Depends on:** 5.2, 4.4.
- **Acceptance:** a fenced block shows highlighted text with fence lines hidden when the caret is outside; moving the caret inside reveals the fence lines; screenshot-verified.

---

## Phase 6 — Render host and Math (tasks not on hold)


Requires P1 spike S1 and Phase 4.

### 6.2 Request queue and cache
- **Goal:** Queue requests and cache results by `hash(kind, source, theme)`.
- **Crate / files:** `md_render/src/cache.rs`, `md_render/src/queue.rs`.
- **Depends on:** 6.1.
- **Acceptance:** an identical second request is served from cache (verified by a counter); the same source under a different theme is a separate cache entry; a placeholder is returned while a render is pending.

### 6.4 resvg rasterization
- **Goal:** Rasterize SVG to pixels and hand the result to GPUI as an image.
- **Crate / files:** `md_render/src/raster.rs`.
- **Depends on:** 6.3.
- **Acceptance:** the rasterized output of a known formula has the expected dimensions in a test; a screenshot shows the formula with bundled fonts.

---

## Phase 8 — Modes

Requires Phases 4 and 5.

### 8.1 Source Code Mode
- **Goal:** Toggle a mode with no hidden ranges and tree-sitter-markdown highlighting.
- **Crate / files:** `md_editor/src/modes/source.rs`.
- **Depends on:** 4.4, 5.1.
- **Acceptance:** in Source mode every marker is visible (verified by a counter of hidden ranges, which must be zero); highlighting uses the markdown grammar; screenshot-verified.

### 8.2 Caret and scroll preservation across toggles
- **Goal:** Preserve the caret's source offset and the scroll anchor when toggling modes.
- **Crate / files:** `md_editor/src/modes/mod.rs`.
- **Depends on:** 8.1.
- **Acceptance:** toggling Source mode on and off leaves the caret at the same source offset; the top visible block stays the same after a toggle, in a test with a 1,000-line document.

### 8.3 Focus Mode
- **Goal:** Draw blocks other than the caret's block at reduced opacity.
- **Crate / files:** `md_editor/src/modes/focus.rs`.
- **Depends on:** 4.1.
- **Acceptance:** only the caret's block is at full opacity; moving the caret changes which block is at full opacity; screenshot-verified.

### 8.4 Typewriter Mode
- **Goal:** After each caret move, scroll so the caret line sits at a fixed viewport fraction (default 0.5), with half-viewport overscroll padding and smooth scrolling.
- **Crate / files:** `md_editor/src/modes/typewriter.rs`.
- **Depends on:** 3.5.
- **Acceptance:** after moving the caret, the caret line is at the 0.5 fraction within one pixel; the first and last lines can each reach that position; scrolling is animated.

### 8.5 Mode key bindings and combinations
- **Goal:** Bind mode toggles to keys, and make Focus and Typewriter work together and in Source mode.
- **Crate / files:** `app/src/keymap.rs`.
- **Depends on:** 8.1, 8.3, 8.4.
- **Acceptance:** each mode is toggled by its bound key; Focus and Typewriter can be on at the same time; both work in Source mode. The bound keys follow DESIGN §16 item 1, and until that is decided they are defined in one keymap table so they can be changed in a single place.

---

## Phase 9 — Workspace

Requires Phases 3 and 4.

### 9.1 Vault model and folder-tree sidebar
- **Goal:** Treat a folder as a vault, list its `.md` files as a tree, and show the tree in a sidebar.
- **Crate / files:** `workspace/src/tree.rs`, `workspace/src/sidebar.rs`.
- **Depends on:** 3.6.
- **Acceptance:** a test vault with nested folders produces the expected tree; non-`.md` files are excluded; screenshot-verified.

### 9.2 File watcher
- **Goal:** Watch the vault with `notify` and update the tree and open buffers on external changes.
- **Crate / files:** `workspace/src/watcher.rs`.
- **Depends on:** 9.1.
- **Acceptance:** creating a file on disk adds it to the tree within one second; an externally modified open file reloads when it has no unsaved edits.

### 9.3 Create, rename and delete
- **Goal:** Create, rename and delete notes and folders from the sidebar.
- **Crate / files:** `workspace/src/ops.rs`.
- **Depends on:** 9.1.
- **Acceptance:** each operation is reflected on disk and in the tree; renaming an open note keeps its buffer open under the new path.

### 9.4 Full-text search
- **Goal:** Search all notes with `ignore` and `grep-searcher`, with no index, and show results in a panel that jumps to matches.
- **Crate / files:** `workspace/src/search.rs`, `workspace/src/search_panel.rs`.
- **Depends on:** 9.1, 3.6.
- **Acceptance:** a query returns each matching line with file and line number in a test vault; selecting a result opens the note with the caret on the match; no index file is written to disk.

### 9.5 Image paste into assets/
- **Goal:** Implement the paste rule in DESIGN §7.
- **Crate / files:** `md_editor/src/clipboard.rs` (extended), `workspace/src/assets.rs`.
- **Depends on:** 3.3, 9.1.
- **Acceptance:** pasting an image creates `assets/<note-stem>-<timestamp>.png` next to the note, creating `assets/` if needed; the inserted text is a relative `![](assets/...)` link; the image renders in the live preview.

---

## Phase 10 — Theming and polish

Requires Phases 6, 7, 8 and 9.

### 10.1 Theme tokens with Light and Dark
- **Goal:** Define colours as tokens and provide Light and Dark themes.
- **Crate / files:** `app/src/theme.rs`.
- **Depends on:** 3.1.
- **Acceptance:** every UI colour is read from a token; switching the manual theme changes all tokens; screenshot-verified in both themes.

### 10.2 Auto theme from macOS appearance
- **Goal:** Make Auto the default, following the macOS appearance through GPUI window appearance-change events.
- **Crate / files:** `app/src/theme.rs`.
- **Depends on:** 10.1.
- **Acceptance:** changing the macOS appearance updates the window within one frame; Auto is the default on first launch.

### 10.3 Settings persistence and manual override
- **Goal:** Persist the theme choice and other settings, with a manual override in the settings UI.
- **Crate / files:** `app/src/settings.rs`.
- **Depends on:** 10.2.
- **Acceptance:** a chosen theme survives an app restart; the override takes precedence over Auto; a missing or corrupt settings file falls back to defaults without crashing. The storage location follows DESIGN §16 item 9.

### 10.4 Theme-aware render cache and performance pass
- **Goal:** Confirm that renders re-run on theme change, and check every target in DESIGN §12 with benchmarks.
- **Crate / files:** `md_render/src/cache.rs`, `md_core/benches/`, `app/benches/`.
- **Depends on:** 6.2, 7.3, 8.4, 9.4.
- **Acceptance:** switching theme re-renders visible diagrams and math under the new theme and keeps the old cache entries; a benchmark on a 10,000-line document reports keystroke-to-paint under 16 ms; reparse is measured to touch only dirty blocks; the benchmark results are recorded in `docs/spikes/perf-pass.md`.

---

## On hold (needs owner confirmation)

Do NOT start these tasks until the repo owner explicitly confirms. This includes the existing `md_render` math and diagram code. The task text is kept verbatim for when they are confirmed.

### Held: Phase 6 math tasks (6.1, 6.3, 6.5, 6.6)

### 6.1 Mermaid WebView host plus an in-process QuickJS math host
- **Goal:** Run two services behind one request interface (split host, S1 Decision): the hidden WKWebView (or the offscreen NSWindow, UNVERIFIED on macOS) for Mermaid, and an in-process rquickjs host for MathJax v4 with no WebView.
- **Crate / files:** `md_render/src/host.rs` (WebView host), `md_render/src/math_host.rs` (QuickJS host).
- **Depends on:** 1.1, 4.1.
- **Acceptance:** a fixed Mermaid test string returns an SVG string from the WebView host, and a fixed TeX string returns an SVG string from the QuickJS host; the WebView call runs off the UI thread (verified by thread assertion); both services start and stop cleanly.

### 6.3 MathRenderer with MathJax v4 in QuickJS
- **Goal:** Implement `MathRenderer` on MathJax v4 (`@mathjax/src`) running in the in-process QuickJS host from 6.1, with the liteAdaptor and `fontCache: 'none'`. No WebView. The v3 `tex2svgPromise` name does not apply; the v4 equivalent of the synchronous conversion is confirmed in this task.
- **Crate / files:** `md_render/src/math.rs`, `md_render/assets/mathjax/` (pinned v4 bundle built from `@mathjax/src`).
- **Depends on:** 6.1 (QuickJS host), 6.2.
- **Acceptance:** the SVG for `\frac{a}{b}` is produced in a test, on v4 (the S1 prototype ran v3 only, so it is re-verified); the output has no `<foreignObject>` and no shared font-cache references; the fill colour follows the requested theme; invalid input gives an inline error.

### 6.5 Math elements and reveal
- **Goal:** Render inline and block math, showing the render until the caret enters the block (DESIGN §4.2).
- **Crate / files:** `md_editor/src/blocks/math.rs`.
- **Depends on:** 6.4, 4.4.
- **Acceptance:** `$x$` renders inline and shows its `$` markers only when the caret is inside; a `$$` block shows the render, and entering it shows source with the preview below; screenshot-verified.

### 6.6 Error display and scale-factor re-rasterization
- **Goal:** Show render errors inline, keep the last good render, and re-rasterize when the scale factor changes.
- **Crate / files:** `md_render/src/raster.rs`, `md_editor/src/blocks/math.rs`.
- **Depends on:** 6.5.
- **Acceptance:** invalid LaTeX shows an inline error while the last good render stays visible; changing the display scale factor re-rasterizes the image at the new scale (verified by a counter and screenshot).

### Held: Phase 7 — Mermaid


Requires Phase 6.

### 7.1 DiagramRenderer with Mermaid
- **Goal:** Implement `DiagramRenderer` on Mermaid, with top-level `htmlLabels: false` and the theme mapping in DESIGN §6.3.
- **Crate / files:** `md_render/src/diagram.rs`, `md_render/assets/mermaid/` (pinned bundle).
- **Depends on:** 6.1 (WebView host), 6.2.
- **Acceptance:** a flowchart source produces SVG in a test, on the WebView host; the SVG contains no `<foreignObject>` element, checked on every diagram type and not only flowcharts (S1 checked flowcharts only); `default` and `dark` themes produce different output.

### 7.2 Mermaid block element and enter-to-edit
- **Goal:** Render Mermaid blocks, and on caret entry show source with the live preview below (DESIGN §4.2).
- **Crate / files:** `md_editor/src/blocks/mermaid.rs`.
- **Depends on:** 7.1, 4.4.
- **Acceptance:** a Mermaid fence shows the diagram when the caret is outside; entering it shows source plus preview; screenshot-verified.

### 7.3 Debounced re-render and last-good fallback
- **Goal:** Re-render after edits with a debounce, keep the last good diagram on invalid source, and show errors inline.
- **Crate / files:** `md_editor/src/blocks/mermaid.rs`, `md_render/src/queue.rs`.
- **Depends on:** 7.2.
- **Acceptance:** typing a burst of characters triggers one render after the debounce interval; invalid source keeps the previous diagram visible and shows an inline error; the debounce interval matches the value chosen in DESIGN §16 item 8.

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
| P4 | Live preview matches DESIGN §4 in screenshot tests |
| P5 | Code fences highlight and reveal correctly |
| P6 | Math renders inline and in blocks, with error handling and scale-factor re-rasterization |
| P7 | Mermaid renders, edits and falls back correctly |
| P8 | All three modes work alone and in combination |
| P9 | Vault tree, watcher, file operations, search and image paste work |
| P10 | Themes work, and the DESIGN §12 targets are measured |

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
