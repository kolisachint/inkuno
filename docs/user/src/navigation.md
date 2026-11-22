# Navigation

The window has four regions and nothing else: the title band, the left sidebar, the panes and the right panel.

- **Title band**: the tab strip of each pane, the window controls and the right-panel switch. The strip is the band itself, even for one note.
- **Left sidebar**: files only. See below.
- **Panes**: one or two, side by side, each with its own tabs. See [Tabs and panes](tabs-and-panes.md).
- **Right panel**: one panel with four views, about the note that is open. See below.

The [palette](first-steps.md#find-anything-the-palette) (`Ctrl + P`) finds and creates notes, and runs commands. Recent notes and search live there, not in the sidebar.

## The left sidebar

The sidebar sits on the window background, beside the note, without a border. From top to bottom:

- **Search** (`Ctrl + P`) and **Today**, the day's note, as rows.
- **Notes**: the vault as a tree of folders and notes. The displayed file is highlighted. Its keys are in [Keys in the sidebar](#keys-in-the-sidebar) and [The tree](#the-tree).
- **Scratch**: a section at the bottom, folded or open, that lists the scratch notes not yet filed. See [Scratch notes](writing.md#scratch-notes).
- **Settings** at the foot, `Ctrl + ,`: the settings tab, with Appearance, Editor, Files, Shortcuts and About. See [Settings and appearance](settings.md).

Everything else is in the palette: the trash (`trash`), backup (`back up`), the theme (`theme`) and this help (`shortcuts`).

- **Hide**: `Ctrl + B` shows or hides the sidebar, from the note too. While it is hidden, its button sits at the left of the title band.
- **Resize**: drag the line between the sidebar and the note. Double-click the line for the default width.

## The right panel

The right panel holds what is about the open note. Its switcher, an icon for each view, sits in the title band:

| View | Key | Shows |
|---|---|---|
| Outline | `Ctrl + Shift + O` | the headings of the note, indented by level. Click one to go to it. |
| Comments | `Ctrl + Alt + 2` | the comments of the note, with their resolve marks. See [Comments](comments.md#the-comments-view). |
| Versions | `Ctrl + Alt + 3` | the saved versions of the note, newest first, and *Save version*. See [Versions](writing.md#versions). |
| Assistant | `Ctrl + Alt + 4` | the [Assistant](assistant.md): its actions and its prompt. |

One view shows at a time, at the same width. `Ctrl + Alt + 1` shows the outline. The button of the band shows or hides the panel, and the same key as the view you show hides it again.

- **Pending changes** from the Assistant or from an agent show in the note itself, with Accept and Reject on each change. They are kept beside the note, not in the file. See [Agents](agents.md#pending-changes).
- **Focus mode**: `F9`, or `› focus mode` in the palette, leaves the note alone, edge to edge: no sidebar, no right panel, no tab strip, no title band. `F9` again, or `Esc` in the note, brings back the layout as it was.
- **Title band**: `Ctrl + Shift + S` hides the band, with its window buttons; a band hidden shows again when the pointer touches its top edge. On macOS the key hides the system bar and its three window buttons, and shows them again; focus mode hides them too. On Windows the system draws the bar, so the key changes nothing there. The choice is kept at the next start.

## Keys in the sidebar

The same keys work in the tree and in the recent list.

| Key | Action |
|---|---|
| `↑` `↓` | select a note and preview it beside the panel, in a single tab in italics |
| `Enter`, or a click | open the note in its tab and start typing |
| `←` `→` | tree: fold or unfold a folder |
| `Tab`, `Shift + Tab` | tree and recent: go along the buttons of the sidebar. A ring shows which one has the keys; `Enter` or `Space` presses it. |
| `F2`, `Delete` | tree and recent: rename the selected note or folder, move it to the trash |
| `Ctrl + Shift + N` | new folder, next to the selected line of the tree |
| `Esc` | back to the note |

A previewed note only counts as opened once you click or type in it. Its tab then stops being italic, and stays.

## The tree

- **Current file**: the accent colour marks the name and icon of the file shown in the active tab and pane, for Markdown notes, diagrams, SVG, pictures and CSV/TSV tables — text, not a tinted background. A separately selected row or several selected rows keep their selection tint.
- **Order**: names are sorted as in a file manager: `Note 2` before `Note 10`.
- **Fold all**: the double-chevron button at the top of the tree folds every folder. When they are all folded, it unfolds them all.
- **New note**: with the tree on display, `Ctrl + Alt + N` creates the note in the folder of the selected line. The button left of the folder button, at the top of the tree, creates it at the root of the vault instead. `Ctrl + N` makes a scratch note instead, see [Scratch notes](writing.md#scratch-notes).
- **Scratch notes**: `scratch notes` in the palette lists the scratch notes that are not in the vault yet, newest first. See [Scratch notes](writing.md#scratch-notes).
- **Versions**: `Versions of the note` in the palette lists the saved versions of the note in the active tab. See [Versions](writing.md#versions).
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
- **Restore later**: type `trash` in the palette. It lists the trash, and `Enter` returns the chosen item to the top of the vault. The trash does not remember where an item came from. A name already taken gets another one.
- **Empty the trash** with your file manager.
