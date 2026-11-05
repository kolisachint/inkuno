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
- With no argument, the app reopens the last vault if it still exists (see Settings below).
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
| Source Mode (markers shown, highlighted as source) | Ctrl+/ | Cmd+/ |
| Focus Mode (blocks other than the caret's block are dimmed) | F8 | F8 |
| Typewriter Mode (the caret line stays at the middle of the window) | F9 | F9 |
| Cycle the theme (Auto, Light, Dark) | Ctrl+K, then Ctrl+T | Cmd+K, then Cmd+T |

Notes:

- On Linux, Ctrl+Backspace deletes a word in the editor. It deletes a note only when the sidebar
  has keyboard focus.
- Deleting a note is permanent. There is no trash.
- Mouse: click a folder to open or close it, click a note to open it, double-click a note to rename it,
  and right-click a note to delete it.

## Display modes

Modes change how the note is drawn. Each one has a key (see the table above). Focus and
Typewriter can be on together, and both work in Source Mode.

## Theme

The theme is Light, Dark or Auto. Auto is the default and follows the system appearance, and
the window changes at once when the system appearance changes. The Cycle the theme key steps
Auto, Light, Dark, Auto. Light and Dark are a manual override: they ignore the system
appearance. The choice is saved, so the app starts with it again.

## Settings

The app saves these settings in a file named `settings.json`:

- the theme (Auto, Light or Dark),
- the last vault (reopened when you start the app with no argument),
- Focus Mode and Typewriter Mode (on or off). Source Mode is not saved and starts off.

The file is in `~/.config/noteapp/` on Linux (or `$XDG_CONFIG_HOME/noteapp/`), and in
`~/Library/Application Support/noteapp/` on macOS. Set `NOTEAPP_CONFIG_DIR` to use another
folder. If the file is missing or damaged, the app starts with the defaults and leaves the
damaged file alone until you change a setting.
