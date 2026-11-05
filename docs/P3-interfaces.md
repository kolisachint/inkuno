# Phase 3 interfaces and task briefs (md_editor basic)

Plan tasks 3.1 to 3.6 ([IMPLEMENTATION_PLAN.md](./IMPLEMENTATION_PLAN.md#phase-3--md_editor-basic)).
The interfaces are defined in the skeleton under `crates/md_editor/src` and `crates/app/src`.
Each task brief below is ready to paste into a Haiku agent prompt. The skeleton compiles, and
every behaviour body is `todo!("3.x")`. Do not change a signature. If a signature is wrong,
stop and report it instead of changing it.

## Module layout

| File | Task | Owner | Public interface (skeleton) |
|------|------|-------|-----------------------------|
| `md_editor/src/element.rs` | 3.1 | 3.1 | `line_ranges`, `LineLayout`, `LayoutSnapshot`, `TextElement::new(editor, source)` |
| `md_editor/src/selection.rs` | 3.2 | 3.2 | `Selection`, `SelectionState`, `map_through_edit`, `Editor::replace_range`, `replace_selection`, `set_selection`, `paint_selection` (no-op) |
| `md_editor/src/cursor.rs` | 3.2 | 3.2 | `hit_test`, `paint_caret` (no-op), `register_mouse_handlers` (no-op) |
| `md_editor/src/keys.rs` | 3.3 | 3.3 | `actions!` (movement), `Motion`, `Moved`, `move_offset`, `Editor::move_caret`, `attach` (identity), `bind_keys` (no-op) |
| `md_editor/src/clipboard.rs` | 3.3 | 3.3 | `actions!` (`Copy`, `Cut`, `Paste`), `copy_text`, `normalize_paste`, `Editor::copy_selection`, `cut_selection`, `paste_clipboard` |
| `md_editor/src/ime.rs` | 3.4 | 3.4 | `ImeState`, `utf16_to_utf8_range`, `utf8_to_utf16_range`, `decorate_runs` (identity), `register_input_handler` (no-op), `impl EntityInputHandler for Editor` (todo) |
| `md_editor/src/block_list.rs` | 3.5 | 3.5 | `BlockListState` (`list`, `layouts` counter), `blocks_for`, `render_blocks` |
| `md_editor/src/editor.rs` | shared | phase owner | `Editor`, `SAMPLE_TEXT`, `Editor::new`, `buffer`, `text`, `Render`, `Focusable`, `init`, HOOK lines |
| `md_editor/src/lib.rs` | shared | phase owner | module declarations, gated on macOS and Linux |
| `app/src/file_io.rs` | 3.6 | 3.6 | `read_file`, `write_file`, `Document`, `Save` action, `init`, `open_initial` |
| `app/src/main.rs` | shared | phase owner | opens the window; calls `md_editor::editor::init`, `file_io::init`, `file_io::open_initial`. No task edits it. |

### Ownership rules

- **Each task owns its own file or files.** Its state type (`SelectionState`, `ImeState`,
  `BlockListState`) and its `impl Editor` blocks live in its own file. A task adds fields
  to its own state struct. It never adds a field to `Editor`.
- **`editor.rs` and `lib.rs` are shared.** The only allowed edit is the one HOOK line for the
  task (see the table below). The skeleton compiles with every HOOK line commented out.
  Uncommenting the HOOK line is the registration.
- **Cross-task calls are pre-wired.** `TextElement::paint` (3.1) calls the no-op stubs
  `paint_selection` (3.2), `paint_caret` (3.2), `register_mouse_handlers` (3.2),
  `decorate_runs` (3.4) and `register_input_handler` (3.4), in the order in the doc comment
  on `TextElement`. So 3.2 and 3.4 do not edit `element.rs`.
- **HOOK lines:**

| HOOK | Location in `editor.rs` | Task | What the task does |
|------|--------------------------|------|--------------------|
| HOOK 3.3 (render) | `Render for Editor`, `keys::attach` line | 3.3 | Uncomment |
| HOOK 3.1 | `Render for Editor`, `TextElement::new` line | 3.1 | Uncomment |
| HOOK 3.5 | `Render for Editor`, `render_blocks` line | 3.5 | Uncomment and delete the HOOK 3.1 line |
| HOOK 3.3 (init) | `init`, `keys::bind_keys` line | 3.3 | Uncomment |

- **Overlap.** The only files more than one task may change are `editor.rs` (HOOK lines
  for 3.1, 3.3 and 3.5) and `lib.rs` (none needed). Merge conflicts there are on adjacent
  HOOK lines. Keep both sides.

### Dependency order

- 3.1 first. It is the only task that puts text on screen.
- Then 3.2, 3.5 and 3.6 in parallel.
- 3.3 and 3.4 after 3.2. Both call `Editor::replace_range` and `replace_selection`, which
  are 3.2's. Their pure functions (`move_offset`, `copy_text`, `utf16_to_utf8_range`) do not
  depend on 3.2, so these can be written and unit-tested in parallel.
- The plan lists 3.6 as depending on 3.1. It has no interface dependency on 3.1 (it uses
  only `Editor::new`, `buffer().version()` and `text()`), so it can start with the skeleton.

## Linux verification

The app runs on Linux with GPUI at the pinned revision. Screenshot command (run from the
worktree root, after `cargo build -p app`):

```bash
OUT="$SCRATCH/p3-<task>.png" xvfb-run -n 99 -s "-screen 0 1280x800x24" sh -c \
  'cargo run -q -p app & sleep 20; DISPLAY=:99 import -window root "$OUT"; pkill -x app'
```

Replace `<task>` with the task ID and `$SCRATCH` with the scratchpad directory. The file must
be under the scratchpad, not `/tmp`. The screenshot is attached to the task record.

If display `:99` is already taken (check for `/tmp/.X99-lock` and a live `Xvfb :99`), pick a
free display number with `-n <N>` and use `DISPLAY=:<N>` in the same command. Do not delete
another process's lock file. The `cargo run` in the command starts the app, so `pkill -x app`
must run after the screenshot.

Tested result: with `-n 91` the skeleton opens an 800x600 white window (the `Editor` root
background). Under `xvfb-run` the `app` binary links the X11 and Wayland backends only because
`crates/app/Cargo.toml` enables `gpui_platform` features `wayland` and `x11` on Linux. Without
them GPUI starts headless and opens no window.

GPU-free logic is tested with `cargo test -p md_editor` and runs on any platform where the
crate compiles. Keep pure logic in the task's own file, in `#[cfg(test)] mod tests`.

Whether GPUI opens and paints a window under Xvfb in this environment is not yet verified.
See the report that accompanies this skeleton.

---

## Task 3.1: Text element

**Context to read first**
- docs/DESIGN.md §7 (Editor element), §3.1 and §3.2 (pipeline and stages)
- docs/IMPLEMENTATION_PLAN.md task 3.1
- docs/P3-interfaces.md (this file), section "Module layout" and the paint order in `element.rs`
- `crates/md_editor/src/element.rs`, `editor.rs`
- `crates/md_core/src/buffer.rs` (`Buffer::text`, `slice`, `len`)
- GPUI at the pinned rev: `crates/gpui/examples/input.rs` (`TextElement`, `request_layout`,
  `paint`, `shape_line`), `crates/gpui/src/element.rs` (`Element` trait)

**Scope**
- Draw the buffer's plain text in `TextElement`, one `ShapedLine` per source line, in the
  `source` byte range. Write the `LayoutSnapshot` into the editor during paint. Call the
  stubs in the paint order given in the `TextElement` doc comment.
- Crate and files this task may change:
  - `crates/md_editor/src/element.rs` (all of it)
  - `crates/md_editor/src/editor.rs`: the HOOK 3.1 line only, uncommented. Do not change
    the line text.

**Interfaces** (defined before implementation; do not change)
- `pub fn line_ranges(text: &str) -> Vec<Range<usize>>`
- `pub struct LineLayout { pub source: Range<usize>, pub shaped: ShapedLine, pub origin: Point<Pixels> }`
- `pub struct LayoutSnapshot { pub lines: Vec<LineLayout>, pub bounds: Bounds<Pixels>, pub line_height: Pixels }` (`Default`)
- `pub struct TextElement` with `pub fn new(editor: Entity<Editor>, source: Range<usize>) -> Self`
- `impl IntoElement for TextElement`, `impl gpui::Element for TextElement`
- Calls into other tasks, which are no-ops in the skeleton, in this order:
  `selection::paint_selection(&Editor, &LayoutSnapshot, &mut Window)`,
  `ime::decorate_runs(Vec<TextRun>, Range<usize>, Option<Range<usize>>)`,
  `cursor::paint_caret(&Editor, &LayoutSnapshot, bool, &mut Window)`,
  `cursor::register_mouse_handlers(&Entity<Editor>, &LayoutSnapshot, Bounds<Pixels>, &mut Window)`,
  `ime::register_input_handler(&Entity<Editor>, &FocusHandle, Bounds<Pixels>, &mut Window, &App)`

**Acceptance tests**
- Unit test: `line_ranges` on `""`, `"a"`, `"a\n"`, `"a\n\nb"` and a multi-byte input gives
  the expected ranges. Ranges exclude `\n`, and offsets are byte offsets.
- Unit test: `line_ranges` on a 100-line text gives 100 ranges, or 101 when the text ends in
  `\n`, and each range slices back to its line.
- Screenshot: open the app (`cargo run -p app`) with `SAMPLE_TEXT`, which has 3 lines. Replace
  the sample with a 100-line file for the test (temporarily, in your worktree only) and check
  that the screenshot matches the buffer text, with correct line breaks.
- Acceptance from the plan: "a 100-line document is drawn with correct line breaks;
  screenshot of the window matches the buffer text."

**Out of scope**
- Caret, selection and mouse (3.2). Keys (3.3). IME, including marked-text underline (3.4).
  Virtualized list (3.5). File IO (3.6). Hidden syntax and live preview (Phase 4).
- Soft wrapping. Each source line is one shaped line. `shape_line` asserts that the text has no `\n`.

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- The screenshot is attached.

---

## Task 3.2: Caret, selection and click placement

**Context to read first**
- docs/DESIGN.md §7, §3.2 (display map owns caret movement and hit-testing)
- docs/IMPLEMENTATION_PLAN.md task 3.2
- docs/spikes/S4-editor-base.md (display map owned by md_core, not by the base editor)
- `crates/md_editor/src/selection.rs`, `cursor.rs`, `element.rs` (`LayoutSnapshot`)
- `crates/md_core/src/display_map.rs` (`DisplayMap::for_text`, `to_source`, `to_display`, `clip`)
- `crates/md_core/src/buffer.rs` (`edit`, `transact`, `version`)
- GPUI: `crates/gpui/src/window.rs` (`on_mouse_event`), `crates/gpui/src/elements/div.rs`
  (mouse listeners), `crates/gpui/examples/input.rs` (`is_selecting`, `index_for_mouse_position`)

**Scope**
- Implement `Selection`, `map_through_edit`, the `Editor` edit methods and the caret,
  selection and mouse code. Hit-testing goes through `md_core::display_map::DisplayMap`.
  Phase 3 has no hidden ranges, so use `DisplayMap::new(len, vec![])`.
- Crate and files this task may change:
  - `crates/md_editor/src/selection.rs` (all of it)
  - `crates/md_editor/src/cursor.rs` (all of it)
  - Nothing else. `editor.rs` needs no change for this task.

**Interfaces** (defined before implementation; do not change)
- `pub struct Selection { pub anchor: usize, pub head: usize }` with `caret`, `is_empty`, `range`, `is_reversed`
- `pub struct SelectionState { pub selection: Selection, pub goal_column: Option<usize>, pub dragging: bool }` (`Default`).
  Keep `goal_column` with the meaning in the doc comment; 3.3 uses it.
- `pub fn map_through_edit(selection: Selection, old: Range<usize>, new_len: usize) -> Selection`
- `impl Editor`: `replace_range(&mut self, Range<usize>, &str, &mut Context<Self>)`,
  `replace_selection(&mut self, &str, &mut Context<Self>)`, `set_selection(&mut self, Selection, &mut Context<Self>)`
- `pub fn paint_selection(&Editor, &LayoutSnapshot, &mut Window)` (filled in by 3.2)
- `pub fn hit_test(&LayoutSnapshot, &DisplayMap, Point<Pixels>) -> usize`
- `pub fn paint_caret(&Editor, &LayoutSnapshot, bool, &mut Window)`
- `pub fn register_mouse_handlers(&Entity<Editor>, &LayoutSnapshot, Bounds<Pixels>, &mut Window)`

`replace_range` must update the buffer through `Buffer::edit` (or `transact`) and then
call `map_through_edit` for the selection, so the selection stays valid after any edit.
This is the only edit path that tasks 3.3 and 3.4 use.

**Acceptance tests**
- Unit test: `map_through_edit` for an edit before, after, at and inside the selection.
- Unit test: `Selection::range` on forward and reversed selections.
- Unit test (through the `Buffer`, no GPUI): `replace_range` logic, if factored into a
  pure function, keeps the caret after the inserted text.
- Screenshot: a click at a given character places the caret before that character; a drag
  highlights the expected range. Attach a screenshot for the click and for the drag.
- Acceptance from the plan: "a click at a given character places the caret before that
  character; drag selection highlights the expected range; screenshot-verified."

**Out of scope**
- Arrow keys, word and line motion, and clipboard (3.3). IME (3.4). Element layout
  and text paint (3.1). Hidden-range hit-testing (Phase 4, where the display map gets hidden ranges).

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- Risky UI tasks include a screenshot.

---

## Task 3.3: Keyboard navigation and clipboard

**Context to read first**
- docs/DESIGN.md §7 (keyboard navigation, clipboard, copy produces Markdown)
- docs/IMPLEMENTATION_PLAN.md task 3.3
- `crates/md_editor/src/keys.rs`, `clipboard.rs`, `editor.rs` (HOOK 3.3, two lines)
- `crates/md_editor/src/selection.rs` (`Selection`, `SelectionState.goal_column`, `replace_range`, `replace_selection`)
- GPUI: `crates/gpui/examples/input.rs` (`actions!`, `KeyBinding::new`, `on_action`, `cx.listener`,
  `write_to_clipboard`, `read_from_clipboard`, `ClipboardItem::new_string`)

**Scope**
- Movement by character, word, line and document, with and without Shift. Up and Down keep
  the goal column. Copy, cut and paste use the system clipboard, and copy gives Markdown source.
- Crate and files this task may change:
  - `crates/md_editor/src/keys.rs` (all of it)
  - `crates/md_editor/src/clipboard.rs` (all of it)
  - `crates/md_editor/src/editor.rs`: the HOOK 3.3 render line and the HOOK 3.3 init line,
    each uncommented. No other line.

**Interfaces** (defined before implementation; do not change)
- `actions!` in `keys.rs`: `MoveLeft`, `MoveRight`, `MoveUp`, `MoveDown`, `MoveWordLeft`,
  `MoveWordRight`, `MoveLineStart`, `MoveLineEnd`, `MoveDocStart`, `MoveDocEnd` and the
  `Select*` equivalents. Keep the names as given.
- `pub enum Motion { Left, Right, Up, Down, WordLeft, WordRight, LineStart, LineEnd, DocStart, DocEnd }`
- `pub struct Moved { pub offset: usize, pub goal_column: Option<usize> }`
- `pub fn move_offset(text: &str, offset: usize, motion: Motion, goal_column: Option<usize>) -> Moved`
- `impl Editor`: `move_caret(&mut self, Motion, bool, &mut Context<Self>)`
- `pub fn attach(root: Div, editor: &Entity<Editor>) -> Div`: adds one `on_action` per action. The
  skeleton returns `root` unchanged.
- `pub fn bind_keys(cx: &mut App)`: binds the keys in the "MdEditor" context.
  Suggested bindings: `left`, `right`, `up`, `down`, `shift-*` for the Select actions, `home`, `end`,
  `alt-left`, `alt-right` for word motion, `cmd-up` and `cmd-down` for document start and end,
  `cmd-left` and `cmd-right` for line start and end (macOS convention).
- `actions!` in `clipboard.rs`: `Copy`, `Cut`, `Paste`. Bind `cmd-c`, `cmd-x`, `cmd-v`.
- `pub fn copy_text(text: &str, range: Range<usize>) -> Option<String>`
- `pub fn normalize_paste(text: &str) -> String`
- `impl Editor`: `copy_selection`, `cut_selection`, `paste_clipboard` (all `&mut self, &mut Context<Self>`)

**Acceptance tests**
- Unit tests for `move_offset`: each motion on ASCII, on multi-byte characters (move by one
  character, never inside a UTF-8 sequence), on emoji, on empty lines and at the text ends.
  Up and Down keep the goal column across a short line.
- Unit tests for `copy_text` (returns the source bytes, `None` for an empty range) and
  `normalize_paste` (`\r\n` and `\r` to `\n`).
- Unit test: Cmd+C, Cmd+X and Cmd+V round-trip plain text. Test the pure part through
  `copy_text` and `normalize_paste`. The GPUI clipboard calls are checked in the screenshot run.
- Screenshot run: press each movement key and check the caret. Copy produces Markdown source
  (for example `**bold**` copies with its asterisks).
- Acceptance from the plan: "each movement key moves the caret by the expected unit in tests;
  Cmd+C, Cmd+X and Cmd+V round-trip plain text; copy produces Markdown source."

**Out of scope**
- The selection math and `replace_range` (3.2). Mouse input (3.2). IME (3.4). Undo and redo (later phase).
- Vertical motion over soft-wrapped lines. Phase 3 has no soft wrap, so Up and Down move by source line.

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- Risky UI tasks include a screenshot.

---

## Task 3.4: IME via EntityInputHandler

**Context to read first**
- docs/DESIGN.md §7 (`EntityInputHandler`, for IME composition and marked text)
- docs/IMPLEMENTATION_PLAN.md task 3.4
- `crates/md_editor/src/ime.rs`, `selection.rs` (`replace_range`, `replace_selection`), `element.rs`
- GPUI: `crates/gpui/src/input.rs` (the `EntityInputHandler` trait and its docs, `ElementInputHandler`),
  `crates/gpui/src/platform.rs` (`UTF16Selection`), `crates/gpui/examples/input.rs`
  (`replace_and_mark_text_in_range`, `utf16` helpers, `marked_range`)

**Scope**
- Implement the UTF-16 and UTF-8 range conversion, the `EntityInputHandler` methods for
  `Editor`, the marked-text state, and the commit path. A commit is one `replace_range`.
- Crate and files this task may change:
  - `crates/md_editor/src/ime.rs` (all of it)
  - Nothing else. Registration is already in `element.rs` (`register_input_handler`) and in
    the trait impl, which lives in `ime.rs`.

**Interfaces** (defined before implementation; do not change)
- `pub struct ImeState { pub marked: Option<Range<usize>> }` (`Default`)
- `pub fn utf16_to_utf8_range(text: &str, range: Range<usize>) -> Range<usize>`
- `pub fn utf8_to_utf16_range(text: &str, range: Range<usize>) -> Range<usize>`
- `pub fn decorate_runs(Vec<TextRun>, Range<usize>, Option<Range<usize>>) -> Vec<TextRun>`
  (identity in the skeleton). 3.4 underlines the marked part.
- `pub fn register_input_handler(&Entity<Editor>, &FocusHandle, Bounds<Pixels>, &mut Window, &App)`
  (no-op in the skeleton). 3.4 calls `window.handle_input(focus, ElementInputHandler::new(bounds, editor.clone()), cx)`.
- `impl EntityInputHandler for Editor`: the eight required methods, with the signatures in the file.
  The optional methods (`selection_bounds_for_range`, `text_length_utf16`, `accepts_text_input`,
  `text_input_configuration`, `text_input_editable_range`) use the trait defaults unless a test
  needs them.

**Acceptance tests**
- Unit tests for the conversions: ASCII, 2-byte (`é`), 3-byte (`日`), 4-byte (`🎉`, which is a
  surrogate pair in UTF-16), empty text, and out-of-range input clamped to the text end.
  `utf16_to_utf8_range(utf8_to_utf16_range(r)) == r` for all char-boundary `r`.
- Unit test (through `Buffer` and the pure conversions): a composition sets `marked`, a second
  update replaces the marked text, and a commit clears `marked` and writes one edit.
- Screenshot run (macOS input method, or a documented Linux IBus test, which is not required
  here): a Japanese composition shows marked text and commits the final text as one edit.
  The marked range clears on commit.
- Acceptance from the plan: "a Japanese IME composition shows marked text and commits the final
  text into the buffer as one edit; the marked range is cleared on commit."

**Out of scope**
- The `replace_range` edit path (3.2). Caret painting (3.2). Hidden-range IME across `**`
  markers (Phase 4, verified with S3). Keys (3.3).

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- Risky UI tasks include a screenshot, or a note that the IME check ran on macOS only.

---

## Task 3.5: Virtualized block list

**Context to read first**
- docs/DESIGN.md §3.1 and §3.2 (GPUI list, virtualized, variable height), §12 (layout target)
- docs/IMPLEMENTATION_PLAN.md task 3.5
- `crates/md_editor/src/block_list.rs`, `editor.rs` (HOOK 3.5 replaces the HOOK 3.1 line)
- `crates/md_core/src/block.rs` (`split_blocks`, `BlockMap::new`, `apply_edit`) and `types.rs` (`Block`)
- GPUI: `crates/gpui/src/elements/list.rs` (`list`, `ListState::new`, `ListAlignment`,
  `splice`, `reset`, `scroll_to_end`, `scroll_to_reveal_item`, `item_count`). Note that the
  `list` render closure is `FnMut(usize, &mut Window, &mut App) -> AnyElement`, so it cannot
  borrow the editor directly. Clone the `Entity<Editor>` into it.

**Scope**
- Keep the block list in step with the buffer: one list item per top-level block, each item
  a `TextElement` for that block's range. Count laid-out items in `layouts`.
- Crate and files this task may change:
  - `crates/md_editor/src/block_list.rs` (all of it)
  - `crates/md_editor/src/editor.rs`: the HOOK 3.5 line, uncommented, and the HOOK 3.1 line
    deleted. No other change.

**Interfaces** (defined before implementation; do not change)
- `pub struct BlockListState { pub list: ListState, pub layouts: Rc<Cell<usize>> }`, `Default` (kept as in the skeleton)
- `pub fn blocks_for(text: &str) -> Vec<md_core::Block>`
- `pub fn render_blocks(editor: &Entity<Editor>, cx: &mut Context<Editor>) -> AnyElement`
- Uses `crate::element::TextElement::new(editor, block.range)` from 3.1 without changing it.

**Acceptance tests**
- Unit test: `blocks_for` on a sample document gives the same blocks as `md_core::split_blocks`.
- Unit test: `blocks_for` on a document is the same after a no-op re-split (cache hit, if cached).
- Counter test: a 10,000-line document is built. Scroll to the middle and then to the end. The
  `layouts` counter stays near the number of visible blocks (print it under a debug env var
  or in a test that uses the counter). It must not approach 10,000.
- Screenshot: scrolling to the end shows the last line.
- Acceptance from the plan: "a 10,000-line document scrolls with only visible blocks laid out
  (verified by a counter); scrolling to the end shows the last line."

**Out of scope**
- Rendering each block's rich form (Phase 4). Block-level edits and incremental re-split
  wiring to the buffer (3.2 owns edits; wire them only if the list needs it, in your own file).
- Mouse and caret across blocks (3.2 handles hit-testing within the element. Cross-block
  caret is a 3.2 follow-up, not 3.5 scope).

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- Risky UI tasks include a screenshot.

---

## Task 3.6: File open and save

**Context to read first**
- docs/DESIGN.md §2 (the file on disk is the buffer's text), §9 and §10 (app owns file IO)
- docs/IMPLEMENTATION_PLAN.md task 3.6
- `crates/app/src/file_io.rs`, `crates/app/src/main.rs`
- `crates/md_editor/src/editor.rs` (`Editor::new`, `text`, `buffer().version()`)
- GPUI: `crates/gpui/examples/input.rs` (`actions!`, key binding), `Entity::read`, `Context::new`

**Scope**
- Read the file named on the command line into `Document`, or show the sample when none is
  given. Save with Cmd+S. Track the dirty state and show a marker in the title or the view.
- Crate and files this task may change:
  - `crates/app/src/file_io.rs` (all of it)
  - `crates/app/Cargo.toml`: adding a `[dev-dependencies]` entry (for example `tempfile`) is
    allowed for tests. No other manifest change.

**Interfaces** (defined before implementation; do not change)
- `pub fn read_file(path: &Path) -> io::Result<String>`
- `pub fn write_file(path: &Path, text: &str) -> io::Result<()>`
- `pub struct Document { pub editor: Entity<Editor>, pub path: Option<PathBuf>, pub saved_version: u64 }`
- `Document::sample(cx)` (real, keep), `Document::is_dirty(&self, &App) -> bool`,
  `Document::save(&mut self, Option<PathBuf>, &mut Context<Self>) -> io::Result<()>`
- `impl Render for Document` (renders the editor now; add the dirty marker and Save handler)
- `actions!`: `Save`. `pub fn init(cx: &mut App)` binds `cmd-s` to `Save`.
- `pub fn open_initial(cx: &mut Context<Document>) -> Document`: replace the sample with the
  file from `std::env::args().nth(1)` when given.

**Acceptance tests**
- Unit test (no GPUI): `read_file` then `write_file` on a temp file gives a byte-identical
  file, including a trailing newline, CRLF line endings and multi-byte UTF-8. Use a unique
  name under `std::env::temp_dir()` or `tempfile` (dev-dependency), and clean up.
- Unit test: `Document` dirty logic, if it is factored so it can run without GPUI (for
  example, a function on the version numbers).
- Screenshot run: open a file, edit it, check the dirty marker shows. Save with Cmd+S (or call
  `save` from a test) and check the marker clears and the file on disk has the edit.
- Acceptance from the plan: "open then save without edits leaves the file byte-identical;
  Cmd+S writes edits to disk; the window shows a dirty marker after an edit and clears it after save."

**Out of scope**
- Creating new files and a file dialog. Save-as UI. Vault and sidebar (Phase 9). File watching
  (Phase 9). Autosave.
- Editing through the buffer directly: edits happen through `Editor::replace_range` (3.2).

**Done when**
- `cargo build`, `cargo test` and `cargo clippy` pass at the workspace root.
- Every acceptance criterion above is met.
- Risky UI tasks include a screenshot.

---

## GPUI API uncertainty

Read from the pinned checkout at `c7bea343838f42b4c14151393e41f0bbe033c95e`:

- `Element` (`request_layout`, `prepaint`, `paint`), `IntoElement`, `Focusable`, `Render`:
  signatures used as written. Verified in `src/element.rs` and `src/window.rs`.
- `EntityInputHandler`: the eight required methods are listed in `ime.rs`. Verified in `src/input.rs`.
  `ElementInputHandler::new(bounds, entity)` and `Window::handle_input(&focus, handler, &App)`
  are verified.
- `ListState::new(count, ListAlignment, overdraw)`, `list(state, FnMut(usize, &mut Window, &mut App) -> AnyElement)`:
  verified. Each item needs its own clone of the `Entity<Editor>`.
- `ClipboardItem::new_string`, `App::read_from_clipboard` and `write_to_clipboard`: verified.
- `actions!`, `KeyBinding::new`, `track_focus`, `key_context`: used as in `examples/input.rs`.
- Not verified: whether `key_context("MdEditor")` takes `&'static str` without a conversion. The
  build will show it.
- Not verified: whether GPUI paints a window under Xvfb in this environment (the screenshot
  command above). The Linux build of GPUI itself is verified by building `app`.
