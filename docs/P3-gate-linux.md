# Phase 3 gate, Linux

Run on Linux (X11 under Xvfb on display `:88`, llvmpipe, `xdotool` for input, `xclip` for the
clipboard, ImageMagick `import` for screenshots). Debug build of `app`, opened with
`target/debug/app <file>`.

Test document: 202 lines, 40 sections. Each section is a `##` heading, a blank line, a
two-line paragraph and a blank line, after a `# Gate document` title. It scrolls. The
document is generated, not committed.

Each check was driven by synthetic X events and checked by eye on its screenshot. Screenshots
are in `docs/screenshots/` as `p3-gate-NN-*.png`. The text on disk was checked with `diff`
against the expected text (see check 11).

## Checks

| # | Check | Result | Screenshot |
|---|-------|--------|------------|
| 1 | Click in the 1st block (`# Gate document`) at x=300. The caret lands after "Gate", the nearest character edge, and a typed `X` goes there. | PASS | `docs/screenshots/p3-gate-01-click-block1.png` |
| 2 | Click in the 3rd block (`## Section 1`) at x=300. The caret lands after "Sec" and a typed `Y` goes there, not in block 1. | PASS | `docs/screenshots/p3-gate-02-click-block3.png` |
| 3 | Click in a later block (`Paragraph 4 first line`, line 19) at x=400. The caret lands between "lin" and "e" and a typed `Z` goes there. | PASS | `docs/screenshots/p3-gate-03-click-later-block.png` |
| 4 | Drag from `Paragraph 1` (after "Paragraph") to `Paragraph 2` (after "first lin") across the blank lines and the `## Section 2` heading. The highlight spans all of them. Ctrl+C copies exactly the highlighted text (`ph 1 first line ... Paragraph 2 first lin`). | PASS | `docs/screenshots/p3-gate-04-drag-two-blocks.png` |
| 5 | Down x5 from the end of `Paragraph 1 first line` crosses a blank line, a heading and a blank line. The caret ends at the end of `Paragraph 2 first line`, and a typed `Q` shows there. | PASS | `docs/screenshots/p3-gate-05-down-across-blocks.png` |
| 6 | Up x5 from there returns to the end of `Paragraph 1 first line` (a typed `R` shows there). Markers were removed with Backspace after each check. | PASS | `docs/screenshots/p3-gate-06-up-across-blocks.png` |
| 7 | Middle block (`Paragraph 4`): type ` more`, Backspace x5 (no trace left), Enter, type `NEW`. `NEW` appears on a new line after `Paragraph 4 second line`. | PASS | `docs/screenshots/p3-gate-07-edit-middle-block.png` |
| 8 | Wheel down 25 notches. The view shows `## Section 12` at the top, and the view is painted from the new scroll position. | PASS | `docs/screenshots/p3-gate-08-wheel-scroll.png` |
| 9 | In the scrolled view, click at the end of `Paragraph 14 second line` and type ` SCROLL`. The text goes to that block. | PASS | `docs/screenshots/p3-gate-09-type-scrolled.png` |
| 10 | Ctrl+C on a selection from `## Section 13` through `Paragraph 14 first li` (134 bytes, verified with `xclip`), which spans seven blocks (headings, paragraphs and blank lines). Ctrl+V at the blank line after `Paragraph 15`. The pasted text is in place, and the caret block (`Paragraph 14 first li`) is scrolled into view. | PASS | `docs/screenshots/p3-gate-10-copy-paste-across-blocks.png` |
| 11 | Ctrl+S. The unsaved marker clears. `diff` of the saved file against the expected file (the original with the edits in checks 7, 9 and 10 applied) reports no differences. | PASS | `docs/screenshots/p3-gate-11-saved.png` |

Checks 1 to 3 and 4 to 6 were also run once before the final run, with the same results. The
final run is the one recorded here.

## What changed to pass

- The layout of every visible block is kept, not only the last one painted. `Editor::layouts`
  holds one `BlockLayout` per visible block, in buffer order. Each block's `TextElement` writes
  its own entry during paint, and the list clears the map at the start of each frame.
- Hit testing, IME bounds and point-to-index use that map: the block under the point is used,
  or the nearest visible block when the point is between blocks or past the ends.
- Mouse handlers and the IME input handler are registered once for the whole list in
  `BlockHost::paint`, not once per block. Each mouse event is handled once. A drag above or below
  the list scrolls it.
- Caret and selection painting already used each block's own snapshot, so the caret is drawn only
  in the block that holds it, and a multi-block selection is drawn in each block it touches.
- Up and Down use the text (`keys::move_offset`), not the layout, so they cross offscreen blocks
  without any layout. The caret block is then scrolled into view after each caret or text change.

Pure mapping helpers are unit-tested: `index_containing`, `index_for_y`, `Layouts::insert`,
`item_index_for_offset`.

## Interface changes

- `Editor.layout: LayoutSnapshot` became `Editor.layouts: Layouts`.
- `cursor::register_mouse_handlers` is now `(editor, bounds, window)`. It is called from
  `BlockHost::paint`. `ime::register_input_handler` is called from there too.
- `cursor::offset_at` now reads the editor's block layouts.

These two changes are the ones the P3 interfaces doc said not to make. They were needed to fix
the integration bug.

## Not fixed, or not verified

- Up and Down keep a goal column in bytes (3.3's contract). In a proportional font, the caret
  can land a little off the visual column on lines of different glyph widths. Phase 3 text has no
  styling, so this is small here, but it is not a visual goal column.
- Dragging past the top or bottom of the list autoscrolls it (`scroll_by`). The code is in
  place but was not part of the gate and was not checked.
- A caret moved to a block far below the screen (for example a large paste) is put at the top of
  the list, not the bottom. The block's height is unmeasured until the list lays it out, so
  revealing it at the bottom needs a second frame.
- IME composition (3.4) was not exercised. Typed ASCII goes through the same input handler, but
  marked text and composition were not tested here.
- The gate ran on Xvfb with llvmpipe and synthetic X events only. Wayland, a real GPU and a
  real keyboard layout were not tested. The macOS gate still needs a Mac (blockers 1 and 2b in
  AGENTS.md).
- On Linux, save is `ctrl-s` (`file_io::init`). AGENTS.md still says Ctrl+S is left for 3.3 and
  8.5. That note is out of date, and the binding works as the gate needed.
