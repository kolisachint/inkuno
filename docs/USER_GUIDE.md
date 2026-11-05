# noteapp user guide

noteapp is a Markdown editor. It shows your notes as they will look, and the Markdown markers
appear only on the line where the caret is. It runs on macOS and Linux.

## Running the app

From the repository root:

```sh
cargo run -p app                 # no argument: opens the sample document, no sidebar
cargo run -p app -- path/to/note.md   # opens the note; its folder is the vault (sidebar shown)
cargo run -p app -- path/to/folder    # the folder is the vault; the sidebar lists its notes
```

- A file argument opens that note, reveals it in the sidebar, and uses its folder as the vault.
- A folder argument makes it the vault. Folders in the sidebar start collapsed.
- On Linux, install the packages listed in `.github/workflows/release.yml` and `fonts-liberation`
  (the body text font). Without `fonts-liberation`, bold and italic text may look wrong.

## Editing and saving

- Click to place the caret, drag to select, and type as in any editor.
- Typing `# ` at the start of a line makes a heading. Enter on a list item starts the next item.
- Click a task checkbox (`[ ]` or `[x]`) to toggle it.
- Images: an image line `![alt](path)` shows the picture when the caret is off the line.
  Pasting an image (with an image on the clipboard) saves it to `assets/` next to the note and
  inserts a link. In an untitled note, a pasted image inserts nothing, so save the note first.
- A note changed on disk while you have unsaved edits shows a banner with Reload and Keep mine.

## Key bindings

Linux uses Ctrl. macOS uses Cmd, and Option for word moves and word delete.

| Action | Linux | macOS |
|--------|-------|-------|
| Save | Ctrl+S | Cmd+S |
| Undo | Ctrl+Z | Cmd+Z |
| Redo | Ctrl+Shift+Z or Ctrl+Y | Cmd+Shift+Z |
| Select all | Ctrl+A | Cmd+A |
| Copy, cut, paste | Ctrl+C, Ctrl+X, Ctrl+V | Cmd+C, Cmd+X, Cmd+V |
| Move by character | Left, Right, Up, Down | Left, Right, Up, Down |
| Move by word | Ctrl+Left, Ctrl+Right | Option+Left, Option+Right |
| Line start and end | Home, End | Home, End; Cmd+Left, Cmd+Right |
| Document start and end | Ctrl+Home, Ctrl+End | Cmd+Up, Cmd+Down |
| Extend selection | Shift with any move key | Shift with any move key |
| Delete word backward | Ctrl+Backspace | Option+Backspace |
| Insert new line or continue list | Enter | Enter |
| Insert tab | Tab | Tab |
| Search the vault | Ctrl+Shift+F | Cmd+Shift+F |
| Close search | Escape | Escape |
| Open the first search result | Enter (in search) | Enter (in search) |
| New note (sidebar) | Ctrl+N | Cmd+N |
| Rename the open note | F2 | F2 |
| Delete a note (sidebar focused, then Enter to confirm) | Ctrl+Backspace | Cmd+Backspace |
| Cancel a rename or delete | Escape | Escape |

Notes:

- On Linux, Ctrl+Backspace deletes a word in the editor. It deletes a note only when the sidebar
  has keyboard focus.
- Deleting a note is permanent. There is no trash.
- Mouse: click a folder to open or close it, click a note to open it, double-click a note to rename it,
  and right-click a note to delete it.

## Display modes

Modes change how the note is drawn. They are not bound to keys yet (task 8.5 is in progress).
For now, start the app with one of these set to `1`:

| Mode | Environment variable |
|------|----------------------|
| Source (all Markdown markers visible, highlighted as source) | `NOTEAPP_SOURCE=1` |
| Focus (blocks other than the caret's block are dimmed) | `NOTEAPP_FOCUS=1` |
| Typewriter (the caret line stays at the middle of the window) | `NOTEAPP_TYPEWRITER=1` |

Planned keys, not yet bound: Source on Ctrl+/ (Linux) or Cmd+/ (macOS), Focus on F8,
Typewriter on F9.
