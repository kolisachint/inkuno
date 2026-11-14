# The palette and the panel

The [palette](first-steps.md#find-anything-the-palette) (`Ctrl + P`) finds and creates notes. The panel shows the vault in four views: the tree, recent notes, vault search, and the outline.

## The rail and its views

A rail of icons stays on the left of the window. It opens a panel beside the note, in one of these views:

| View | Key | Shows |
|---|---|---|
| Tree | `Ctrl + E` | the vault and its folders. The displayed file is highlighted. |
| Recent | — | every note, from the most to the least recently opened (`› recent notes`; `Ctrl + P` opens on recent) |
| Search | — | the matches of a vault search, note by note |
| Outline | — | the headings of the open note, in the right dock |

The rail also holds, at the top: the command palette, the vault tree, search and new note. Everything else left the rail for the palette: the trash (`trash`), backup (`back up`), the theme (`theme`) and this help (`shortcuts`).

- **Fold**: press the key of the view on display, or click its icon. The panel folds back to the rail.
- **Resize**: drag the line between the panel and the note. Dragged all the way left, the panel folds to its rail. All the way right, it takes the whole window. Double-click the line for the default width.
- **Whole window**: `Ctrl + M`, or the arrows button at the top of the panel, puts the panel on the whole window, and back.
- **Sidebar**: `Ctrl + B` shows or hides the panel, and the rail stays. It works from the note too. The same switch is at the foot of the rail, and the choice is kept at the next start.
- **Title bar**: `Ctrl + Shift + S` hides the bar Inkuno draws on Linux, with its window buttons. The switch at the foot of the rail brings it back, and the choice is kept at the next start. On macOS and Windows the system draws the bar, so the key changes nothing there.
- **Focus mode**: `F9`, or `› focus mode` in the palette, leaves the note alone, edge to edge: no rail, no panel, no tab bar, no title bar. `F9` again, or `Esc` in the note, brings back the layout as it was.


## Keys in the panel

The same keys work in every view.

| Key | Action |
|---|---|
| `↑` `↓` | select a note and preview it beside the panel, in a single tab in italics |
| `Enter`, or a click | open the note in its tab and start typing |
| `←` `→` | tree: fold or unfold a folder |
| `Tab`, `Shift + Tab` | tree and recent: go along the buttons of the panel and of the rail. A ring shows which one has the keys; `Enter` or `Space` presses it. |
| `F2`, `Delete` | tree and recent: rename the selected note or folder, move it to the trash |
| `Ctrl + Shift + N` | new folder, next to the selected line of the tree |
| `Esc` | back to the note |

A previewed note only counts as opened once you click or type in it. It then moves to the top of the recent list, and its tab stays.

## The tree

- **Current file**: the accent colour marks the name and icon of the file shown in the active tab and pane, for Markdown notes, diagrams, SVG, pictures and CSV/TSV tables — text, not a tinted background. A separately selected row or several selected rows keep their selection tint.
- **Order**: names are sorted as in a file manager: `Note 2` before `Note 10`.
- **Fold all**: the double-chevron button at the top of the tree folds every folder. When they are all folded, it unfolds them all.
- **New note**: with the tree on display, `Ctrl + Alt + N` creates the note in the folder of the selected line. The button left of the folder button, at the top of the tree, creates it at the root of the vault instead. `Ctrl + N` makes a scratch note instead, see [Scratch notes](writing.md#scratch-notes).
- **New folder**: the folder button, `Ctrl + Shift + N`, or `new folder` in the palette. The field says where the folder goes.
- **Several lines**: `Ctrl + click` adds a line to the selection or takes it out. `Shift + click` selects everything from the last line chosen. Dragging, duplicating, copying links or paths, and moving to the trash then apply to all of them.
- **Drag and drop**: drag a note or a folder onto a folder to move it there, or below the last line to move it to the top of the vault. Nothing is ever overwritten: if the name is taken, the move is refused.
- **Rename**: `F2`. Renaming a note whose first line is its name rewrites that line too. The links to it follow, see [Names and renaming](vault.md#names-and-renaming).

**The menu.** Right click a line, or press `Shift + F10` (the `Menu` key) on the selected line:

- open, or open to the side;
- new note here, new folder;
- copy the `[[link]]`, the path, or the path relative to the vault;
- show in the file manager (Finder, Explorer);
- duplicate (`Ctrl + D`), rename, move to a folder chosen in a list (what dragging does);
- set an icon;
- move to the trash.

A right click below the last line acts on the vault itself. For the open note, the same entries are in the command palette: **Open in the other pane**, **Copy the [[link]] of the note**, **Copy the path of the note**, **Copy the relative path of the note**, **Move the note to a folder…**, **Icon of the note…**, **Show the note's folder as a list**.

## Icons

Right click a note, a file or a folder in the tree, then **Icon…**. Pick one of forty in the grid (arrows and `Enter` work), or the cross to remove it.

- Without one, every row shows a small icon of its kind (folder, note, table, diagram, picture, other file), so the names line up.
- A file that is not a note shows its extension next to its name, lighter (`contacts.csv`). A long name is cut before its extension.
- The icon you give replaces the default one, before the name. It follows a rename or a move.
- Icons are kept in `.inkuno-icons` at the root of the vault, so they travel with it.

## The trash

The trash is the hidden folder `.trash` at the top of the vault. Deleting moves a note or a folder there; nothing is ever destroyed.

- **Undo**: moving something to the trash shows a message with **Undo**. It puts the item back exactly where it was. Its folder is made again if it is gone, and nothing is overwritten.
- **Restore later**: click the bin at the bottom of the rail, or type `trash` in the palette. It lists the trash, and `Enter` returns the chosen item to the top of the vault. The trash does not remember where an item came from. A name already taken gets another one.
- **Empty the trash** with your file manager.
