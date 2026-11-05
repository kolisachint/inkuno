# noteapp — Implementation Plan

Companion to [DESIGN.md](./DESIGN.md). Section references (§) point to that document.

## Contents

- [How this plan is executed](#how-this-plan-is-executed)
- [Dependency graph](#dependency-graph)
- [Phase 0 — Scaffold](#phase-0--scaffold)
- [Phase 1 — Spikes S1–S4](#phase-1--spikes-s1s4)
- [Phase 2 — md_core](#phase-2--md_core)
- [Phase 3 — md_editor basic](#phase-3--md_editor-basic)
- [Phase 4 — Live preview](#phase-4--live-preview)
- [Phase 5 — Code fences](#phase-5--code-fences)
- [Phase 6 — Render host and Math](#phase-6--render-host-and-math)
- [Phase 7 — Mermaid](#phase-7--mermaid)
- [Phase 8 — Modes](#phase-8--modes)
- [Phase 9 — Workspace](#phase-9--workspace)
- [Phase 10 — Theming and polish](#phase-10--theming-and-polish)
- [Definition of Done](#definition-of-done)
- [Subagent task template](#subagent-task-template)

---

## How this plan is executed

- Implementation happens in a later session.
- Work is dispatched to small-scoped Haiku subagents, one task each.
- Interfaces (types, traits, function signatures) for a phase are defined before that phase's implementation tasks start.
- Every task ends with `cargo build`, `cargo test` and `cargo clippy` passing.
- Risky UI tasks are verified with screenshots.
- The development target is macOS. `md_core` has no GPUI dependency, so it can be developed and tested on any OS and run in CI on Linux.

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

## Phase 0 — Scaffold

### 0.1 Cargo workspace with five crates
- **Goal:** Create the workspace skeleton and toolchain pin.
- **Crate / files:** root `Cargo.toml`; `rust-toolchain.toml`; crates `md_core`, `md_render`, `md_editor`, `workspace`, `app`.
- **Depends on:** none.
- **Acceptance:** `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root; all five crates are members; `rust-toolchain.toml` pins a toolchain with `rustfmt` and `clippy` components.

### 0.2 Pinned GPUI git dependency
- **Goal:** Depend on GPUI from the `zed-industries/zed` git repository at one exact revision, not crates.io.
- **Crate / files:** root `Cargo.toml`; `Cargo.lock`.
- **Depends on:** 0.1.
- **Acceptance:** `cargo metadata` shows `gpui` sourced from git at the pinned revision; the revision is recorded in a comment next to the dependency; no crates.io `gpui` appears in the lock file.

### 0.3 License and ignore rules
- **Goal:** Declare the licence and keep build artefacts out of git.
- **Crate / files:** `LICENSE` (GPL-3.0 text); `license = "GPL-3.0-or-later"` in each crate manifest; `.gitignore`.
- **Depends on:** 0.1.
- **Acceptance:** every crate manifest has the licence field; `LICENSE` contains the GPL-3.0 text; `.gitignore` excludes `target/`.

### 0.4 CI for md_core
- **Goal:** Run formatting, lint and tests for `md_core` on every push.
- **Crate / files:** `.github/workflows/md_core.yml`.
- **Depends on:** 0.1.
- **Acceptance:** the workflow runs on Linux; it runs `cargo fmt --check`, `cargo clippy -p md_core` and `cargo test -p md_core`; it passes on the scaffold commit.

---

## Phase 1 — Spikes S1–S4

Each spike writes a findings note to `docs/spikes/` and ends with a go/no-go decision. Spikes S1 to S4 can run in parallel.

### 1.1 S1 — Render host
- **Goal:** Decide between a hidden WKWebView and an offscreen NSWindow. Confirm that Mermaid and MathJax produce SVG, and resolve the UNVERIFIED items in DESIGN §14.
- **Crate / files:** `docs/spikes/S1-render-host.md`; throwaway code kept outside the workspace.
- **Depends on:** 0.1.
- **Acceptance:** the note records measured round-trip latency; it states whether Mermaid output with `htmlLabels: false` contains any `<foreignObject>`; it states whether resvg renders the output; it contains a go/no-go line; it names the pinned mermaid.js and MathJax v3 versions.

### 1.2 S2 — GPUI on macOS
- **Goal:** Prove the pinned GPUI git dependency builds and opens a window.
- **Crate / files:** `docs/spikes/S2-gpui-hello.md`; a hello-window binary in `app`.
- **Depends on:** 0.2.
- **Acceptance:** `cargo run -p app` opens a window on macOS; a screenshot is attached to the note; the note records the revision that worked and contains a go/no-go line.

### 1.3 S3 — Display map prototype
- **Goal:** Prove caret movement and IME work across hidden `**` markers.
- **Crate / files:** `docs/spikes/S3-display-map.md`; prototype confined to a spike branch.
- **Depends on:** 0.2.
- **Acceptance:** the caret moves past a hidden `**` pair in one keystroke; IME composition works across the hidden range; the note contains a go/no-go line.

### 1.4 S4 — Editor base evaluation
- **Goal:** Choose between the `gpui-component` editor and the Zed `editor` crate as the base for `md_editor`.
- **Crate / files:** `docs/spikes/S4-editor-base.md`.
- **Depends on:** 0.2.
- **Acceptance:** the note compares both options on display map support, selections, IME and licence; it names the chosen base; it contains a go/no-go line.

---

## Phase 2 — md_core

All tasks in this phase are pure Rust with no GPUI dependency. They are unit-tested on Linux CI.

### 2.1 Buffer, transactions, undo and redo
- **Goal:** Wrap Zed's `text` / `rope` for insert, delete and replace, with grouped transactions.
- **Crate / files:** `md_core/src/buffer.rs`.
- **Depends on:** 0.1, 0.4.
- **Acceptance:** insert, delete and replace produce the expected text; a transaction groups several edits into one undo step; undo followed by redo restores the exact prior text; tests cover empty buffers and multi-byte UTF-8 input.

### 2.2 Anchors and coordinate conversion
- **Goal:** Provide anchors that survive edits, and convert between byte offset, line and column.
- **Crate / files:** `md_core/src/anchor.rs`, `md_core/src/coords.rs`.
- **Depends on:** 2.1.
- **Acceptance:** an anchor keeps pointing at the same text after an edit before it; offset to line/column and back round-trips on all test inputs; tests include CRLF-free and multi-byte lines.

### 2.3 Block splitter
- **Goal:** Split the buffer into top-level blocks, each with a source range, and re-split only dirty ranges.
- **Crate / files:** `md_core/src/block.rs`.
- **Depends on:** 2.2.
- **Acceptance:** each block type in DESIGN §3.2 is recognized in tests; block ranges tile the document with no gaps or overlaps; an edit re-splits only blocks that overlap the dirty range (verified by a counter in tests).

### 2.4 Per-block parse with hash cache
- **Goal:** Parse each block with the pulldown-cmark offset iterator and cache results by content hash.
- **Crate / files:** `md_core/src/parse.rs`.
- **Depends on:** 2.3.
- **Acceptance:** every event carries a source range inside its block; an unchanged block hits the cache (verified by a counter); a changed block misses the cache and is reparsed.

### 2.5 Inline span model
- **Goal:** For each inline span kind in DESIGN §4.1, produce the marker ranges and the content range.
- **Crate / files:** `md_core/src/inline.rs`.
- **Depends on:** 2.4.
- **Acceptance:** strong, emphasis, inline code, link, image, inline math and strikethrough each produce correct marker and content ranges in table-driven tests; nested spans are reported with correct ranges.

### 2.6 Display map
- **Goal:** Map between source offsets and display positions, collapse hidden marker ranges, and let caret movement and hit-testing skip hidden ranges.
- **Crate / files:** `md_core/src/display_map.rs`.
- **Depends on:** 2.5, and the S3 go decision (1.3).
- **Acceptance:** a caret moving right across `**bold**` with the caret outside the span lands on the first visible content character in one step; a hit-test at a collapsed position resolves to the nearest visible offset; both behaviours are covered by tests.

### 2.7 Input rules engine
- **Goal:** Implement each rule in DESIGN §4.3 as a pure function from (buffer, caret, typed input) to a list of edits.
- **Crate / files:** `md_core/src/input_rules.rs`.
- **Depends on:** 2.1, 2.3.
- **Acceptance:** each rule row in DESIGN §4.3 has at least one test that asserts the exact resulting text and caret; Enter on an empty list item exits the list in a test; no rule produces edits outside its block.

### 2.8 Property tests for the display map
- **Goal:** Check that the display map round-trips under random edits and random caret positions.
- **Crate / files:** `md_core/tests/display_map_props.rs`.
- **Depends on:** 2.6.
- **Acceptance:** for random documents and caret positions, source to display to source returns the original offset whenever the offset is visible; the property test runs in `cargo test` with a fixed seed.

---

## Phase 3 — md_editor basic

Requires P1 spikes S2 and S4, and Phase 2.

### 3.1 Text element
- **Goal:** Render plain text from the buffer in a GPUI custom element.
- **Crate / files:** `md_editor/src/element.rs`.
- **Depends on:** 2.1, 1.2, 1.4.
- **Acceptance:** a 100-line document is drawn with correct line breaks; screenshot of the window matches the buffer text.

### 3.2 Caret, selection and click placement
- **Goal:** Draw the caret and selections, and place the caret on click through hit-testing.
- **Crate / files:** `md_editor/src/cursor.rs`, `md_editor/src/selection.rs`.
- **Depends on:** 3.1, 2.6.
- **Acceptance:** a click at a given character places the caret before that character; drag selection highlights the expected range; screenshot-verified.

### 3.3 Keyboard navigation and clipboard
- **Goal:** Arrow keys, word and line movement, Home and End, Shift selection, and clipboard copy, cut and paste.
- **Crate / files:** `md_editor/src/keys.rs`, `md_editor/src/clipboard.rs`.
- **Depends on:** 3.2.
- **Acceptance:** each movement key moves the caret by the expected unit in tests; Cmd+C, Cmd+X and Cmd+V round-trip plain text; copy produces Markdown source.

### 3.4 IME via EntityInputHandler
- **Goal:** Support IME composition and marked text.
- **Crate / files:** `md_editor/src/ime.rs`.
- **Depends on:** 3.2.
- **Acceptance:** a Japanese IME composition shows marked text and commits the final text into the buffer as one edit; the marked range is cleared on commit.

### 3.5 Virtualized block list
- **Goal:** Lay out blocks in a GPUI `list()`, so only visible blocks are laid out.
- **Crate / files:** `md_editor/src/block_list.rs`.
- **Depends on:** 3.1, 2.4.
- **Acceptance:** a 10,000-line document scrolls with only visible blocks laid out (verified by a counter); scrolling to the end shows the last line.

### 3.6 File open and save
- **Goal:** Open a `.md` file into the buffer and save the buffer back to disk.
- **Crate / files:** `app/src/file_io.rs`.
- **Depends on:** 3.1.
- **Acceptance:** open then save without edits leaves the file byte-identical; Cmd+S writes edits to disk; the window shows a dirty marker after an edit and clears it after save.

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

## Phase 6 — Render host and Math

Requires P1 spike S1 and Phase 4.

### 6.1 Mermaid WebView host plus an in-process QuickJS math host
- **Goal:** Run two services behind one request interface (split host, S1 Decision): the hidden WKWebView (or the offscreen NSWindow, UNVERIFIED on macOS) for Mermaid, and an in-process rquickjs host for MathJax v4 with no WebView.
- **Crate / files:** `md_render/src/host.rs` (WebView host), `md_render/src/math_host.rs` (QuickJS host).
- **Depends on:** 1.1, 4.1.
- **Acceptance:** a fixed Mermaid test string returns an SVG string from the WebView host, and a fixed TeX string returns an SVG string from the QuickJS host; the WebView call runs off the UI thread (verified by thread assertion); both services start and stop cleanly.

### 6.2 Request queue and cache
- **Goal:** Queue requests and cache results by `hash(kind, source, theme)`.
- **Crate / files:** `md_render/src/cache.rs`, `md_render/src/queue.rs`.
- **Depends on:** 6.1.
- **Acceptance:** an identical second request is served from cache (verified by a counter); the same source under a different theme is a separate cache entry; a placeholder is returned while a render is pending.

### 6.3 MathRenderer with MathJax v4 in QuickJS
- **Goal:** Implement `MathRenderer` on MathJax v4 (`@mathjax/src`) running in the in-process QuickJS host from 6.1, with the liteAdaptor and `fontCache: 'none'`. No WebView. The v3 `tex2svgPromise` name does not apply; the v4 equivalent of the synchronous conversion is confirmed in this task.
- **Crate / files:** `md_render/src/math.rs`, `md_render/assets/mathjax/` (pinned v4 bundle built from `@mathjax/src`).
- **Depends on:** 6.1 (QuickJS host), 6.2.
- **Acceptance:** the SVG for `\frac{a}{b}` is produced in a test, on v4 (the S1 prototype ran v3 only, so it is re-verified); the output has no `<foreignObject>` and no shared font-cache references; the fill colour follows the requested theme; invalid input gives an inline error.

### 6.4 resvg rasterization
- **Goal:** Rasterize SVG to pixels and hand the result to GPUI as an image.
- **Crate / files:** `md_render/src/raster.rs`.
- **Depends on:** 6.3.
- **Acceptance:** the rasterized output of a known formula has the expected dimensions in a test; a screenshot shows the formula with bundled fonts.

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

---

## Phase 7 — Mermaid

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
| P3 | Plain-text file opens, edits, IME-composes and saves on macOS |
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
