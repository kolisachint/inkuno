# Keyboard shortcuts

<!-- Written by the test `shortcuts_page_is_current` from `help_sections` (crates/inkuno/src/shell/keymap.rs).
     Do not edit by hand: run `make docs`. -->

Every line of the help panel (`F1`), with the default keys of Linux and Windows. On macOS, read `Cmd` for `Ctrl`, and `Alt` for `Ctrl` when moving by word. Moving between panes is `Cmd + Alt + arrows`, and back and forward are `Cmd + [` and `Cmd + ]`.

`Ctrl+P › word` means: open the palette and type that word. A line without keys has no default shortcut. Any line that names an action can be given keys in the help panel: see [Changing a shortcut](settings.md#changing-a-shortcut).

## Notes

| Keys | Action |
|---|---|
| `Ctrl+P` | Find by name or text, create, filter by \#tag |
| `F10` / `Ctrl+Shift+P` | Command palette: every command, found by what it does (also the first button of the rail, or \> in the palette) |
| `Ctrl+F` | Find in the note |
| `Ctrl+R` | Find and replace in the note |
| `F3` | Search bar: next match (Enter too) |
| `Shift+F3` / `Shift+Enter` | Search bar: previous match |
| `Alt+C` | Search bar: match case |
| `Alt+W` | Search bar: whole words |
| `Alt+R` | Search bar: regular expression ($1 in the replacement) |
| `Tab` | Search bar: replace field |
| `Ctrl+Enter` | Search bar: replace all (Enter: this match) |
|  | Scratch note: keep it in the vault, under its first line (the tab stays open) |
| `Ctrl+N` | New scratch note: a quick thought in a tab, kept in the app's own folder, outside the vault, until you file it |
| `Ctrl+Alt+N` | New note in the folder selected in the tree (in the vault) |
| `Ctrl+O` | Change vault |
| `Ctrl+P › terminal` | Open a terminal in the vault |
| `Ctrl+P › clone` | Clone a vault from a git address (also on the welcome screen) |
| A cloud folder | iCloud, Dropbox, Google Drive, Syncthing: put the vault there, nothing else; never a git repository inside one |
| `Ctrl+Shift+C` | Copy the code block, else the note |
| `Ctrl+click` | Open a \[\[link\]\], \#tag or URL |
| `F1` / `Ctrl+/` | This help |
| `Ctrl+Q` | Save and quit; stay open if saving fails |

## Navigation

| Keys | Action |
|---|---|
| `Ctrl+\` | Split the pane: a new one on its right, opened on the palette to choose what goes there |
| `Ctrl+Alt+\` | Split the pane: a new one below it |
| `Alt+Shift+Left` | Go to the pane on the left |
| `Alt+Shift+Right` | Go to the pane on the right |
| `Alt+Shift+Up` | Go to the pane above |
| `Alt+Shift+Down` | Go to the pane below |
| `Ctrl+1` | Go to the first pane |
| `Ctrl+2` | Go to the second pane |
|  | Move the tab to the pane on the left (split off if there is none) |
|  | Move the tab to the pane on the right (split off if there is none) |
|  | Move the tab to the pane above (split off if there is none) |
|  | Move the tab to the pane below (split off if there is none) |
|  | Give every pane the same room |
| `Alt+Shift+Enter` | The pane on the whole window, and back (the others keep what they hold) |
| `Shift+click on the rail, title of a view` | Shift+click stacks the view under those shown in its dock; drag its title to another place in a dock, or to the right edge to open the right dock. Escape lets go |
| Dragging a tab | Onto a tab bar: it moves there; onto the middle of a pane: it joins it; near an edge of a pane: a new pane opens there with it. A file from the tree does the same. Escape lets go without moving anything |
| Border between panes | Drag it to share the room; double click: even shares |
| Opening a file | Click in the tree, choose in the palette, follow a link: the file opens in a tab of its own, or its tab comes to the front if it is already open; the arrows in the tree preview notes in a single italic tab |
| `Ctrl+W` | Save and close the focused tab; keep it open if saving fails |
|  | Close the other tabs of the pane, each saved first; one that fails to save stays open |
|  | Close all the tabs of the pane; the other panes keep theirs |
|  | Close all files, in every pane, without deleting any; the window is left on a new empty note |
| Right click on a tab | Close it, close the other tabs, or all the tabs of its pane |
| `Ctrl+Shift+T` | Reopen the last closed tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+Shift+Page Up` | Move the tab one place to the left |
| `Ctrl+Shift+Page Down` | Move the tab one place to the right |
| Middle click | On a tab: close it |
| `Ctrl+Enter` | In the palette: open the chosen note in the other pane |
| `Ctrl+Shift+click` | Open a \[\[link\]\] in the other pane |
| Drag from the tree | Onto a pane: open the file there; onto the right half of a single pane: open a second one |
| `Ctrl+E` | Vault tree |
| Current file | The tree marks the file shown in the active tab and pane, including diagrams, pictures and tables; selection stays separate |
|  | Recent notes |
|  | Scratch notes: the panel lists them, newest first; Enter opens one, Delete discards it |
| A picture | Shown in place of the note |
| `Ctrl+M` | Panel on the whole window |
| `Ctrl+B` | Sidebar: show or hide the side panel (the rail stays) |
| `F9` | Focus mode: the note alone, edge to edge, no bars or panels; F9 or Esc goes back |
| `Ctrl+Shift+S` | Title bar: show or hide the bar the app draws (its buttons come back with it) |
| `Arrows` | Select and preview |
| `Enter` / `Esc` | Open the note / back to the note |
| `Ctrl+Shift+N` | New folder (also in the palette) |
| Point at an icon | Its name and its shortcut |
| `Alt + drag` | Move the window from anywhere (Linux) |
| `F2` | Panel: rename |
| `Ctrl+D` | Panel: duplicate |
| `Delete` | Panel: move to the trash |
| `Ctrl+click` / `Shift+click` | Select several rows: move, duplicate, trash them together |
| Right click | Copy the link or the path, set an icon, show a folder or a tag as a list… |
|  | Open the note in the other pane |
|  | Copy the \[\[link\]\] of the note |
|  | Copy the path of the note |
|  | Copy the relative path of the note |
|  | Show in the file manager |
|  | Move the note to a folder… |
|  | Icon of the note… |
|  | Show the note's folder as a list |
| `Tab` | From the panel: go from button to button (Shift+Tab: back); Enter or Space presses it |
| `Shift+Tab` | Back to the previous button |
| `Shift+F10` / `Menu` | The same menu from the keys, for the selected row or the cursor: arrows, Enter, Esc |
| In a list | Type to filter (key:text for one column), click a cell to rewrite it |

## Diagrams

| Keys | Action |
|---|---|
| `![](Schéma.svg)` | Show the diagram in a note |
| `.svg` | An SVG from elsewhere (draw.io, Mermaid.js…) opens in a viewer: drag or wheel to move, + - 0 to zoom |
| Picture, formula, diagram | Point at one in a note: a button at its corner shows it full size, to zoom and move around (Esc goes back) |

## Appearance

| Keys | Action |
|---|---|
| `Ctrl+P › theme` | Theme, previewed as you browse |
|  | Theme… |
| `Ctrl+P › font` | Font of the app / of the code |
| `Ctrl+P › page width` | Width of the text column: narrow, normal, wide, full |
| `Ctrl+P › wide tables` | A table wider than the text: in the column, or centered and wider |
| `Ctrl+P › animations` | Short fades (about 0.13 s) when the panel, the formatting bar or a folder opens; none on typing, scrolling or hover. Off: everything is instant (motion=off in settings) |
| `Ctrl+=` / `Ctrl++` | Bigger text; the panel, menus and help follow, up to 150 % |
| `Ctrl+-` | Smaller text; the interface follows, down to 85 % |
| `Ctrl+0` | Default text size |

## Tables in a note

| Keys | Action |
|---|---|
| `Grips` | Over a table: a small grip above the column and left of the row under the pointer, a round one at the corner for the whole table. Click one to take the column, the row or the table (copy, cut, erase, or any gesture below), drag it to move it, Esc lets go; the + between two grips inserts there |
|  | Insert a row above |
|  | Insert a row below |
|  | Duplicate the row |
|  | Move the row up |
|  | Move the row down |
|  | Delete the row |
|  | Insert a column on the left |
|  | Insert a column on the right |
|  | Duplicate the column |
|  | Move the column left |
|  | Move the column right |
|  | Delete the column |
|  | Sort the rows by this column, A to Z |
|  | Sort the rows by this column, Z to A |
|  | Swap rows and columns |
|  | Delete the table |
| `Ctrl+Shift+Backspace` | Remove the row, or the columns taken by their grip |

## Typing

| Keys | Action |
|---|---|
| `/` | Components: headings, lists, panel, table, code, formula… |
| `/table` | Table: pick its size on the grid (mouse, arrows), or type it: 12x5 |
| \> \[!NOTE\] | Colored panel: NOTE, TIP, IMPORTANT, WARNING, CAUTION |
| -  | Bullet list |
| 1.  | Numbered list |
| \[\]  | Task |
| \# \#\# \#\#\# | Headings; the first \# names the file |
| \>  | Quote |
| `` ```rust `` | Code block: colors, copy icon |
| `---` | Divider |
| `[[` | Link to a note |
| `@` | Link to the note of a day: @today, @monday, @2026-10-09 |
| `![](image.png)` | Picture, under its line |
| `![[` | Suggests the pictures and diagrams of the vault |
| Drop files | Drag files or folders from your file manager onto a note (a line shows where) or onto the tree (the folder under the pointer): they are copied, never overwritten, and a note gets a link per item |
| `Double` / triple click | Select a word / a line; drag to extend by words / lines |
| `` ```mermaid `` | Diagram, under its block; its magnifier shows it full size |
| $x^2$  $$…$$ | LaTeX formula, under its line |
| -\> != \<= =\> | Shown as → ≠ ≤ ⇒ |

## Editing

| Keys | Action |
|---|---|
| `Enter` | Continue the list, or leave it on an empty item |
| `Tab` / `Shift+Tab` | Indent / outdent |
| `Tab` / Enter in a table | Next cell / new row; columns stay aligned; + buttons add a row or a column |
| `Ctrl+Shift+L` | Align the column of the cursor to the left |
| `Ctrl+Shift+E` | Center the column of the cursor |
| `Ctrl+Shift+R` | Align the column of the cursor to the right |
| Pasting a table | Spreadsheet cells or Markdown fill the grid, or become a table |
| `Ctrl+Shift+B` | Turn the selected lines into a list, then the next kind: bullets, numbered, tasks, plain text |
|  | Selected lines: bulleted list (again: plain text) |
|  | Selected lines: numbered list (again: plain text) |
|  | Selected lines: task list (again: plain text) |
|  | Selected lines: plain text |
| `Ctrl+Enter` | Check / uncheck a task or a \[ \] box; in a table cell, add one |
| `Ctrl+Alt+B` | Bold |
| `Ctrl+I` | Italic |
| `Ctrl+U` | Underline |
| `Ctrl+Shift+X` | Strikethrough |
| `Ctrl+Shift+H` | Highlight |
| `Ctrl+Shift+K` | Inline code |
| `Ctrl+P › formatting bar` | Under the note, centered: bold, italic, underline, strikethrough, highlight (five colors), code, link, lists, tasks, table (in a table: add or delete a row or a column); a lit button means the selection has the style, click again to remove it; only the active pane shows it; in a narrow note it folds into a button at the bottom, in the middle, that opens it; hide it with the palette |
| `Ctrl+D` | Several cursors: select the word, then its next occurrence (Esc: back to one) |
| `Esc` | Close the find bar and the menus, back to one cursor |
| `Alt+click` | Add a cursor |
| `Ctrl+K` | Link: \[text\](address) from the selection; on a link, change its address |
| `Ctrl+Shift+M` | Comment the selection or the line (in a table: the cell, or the taken row, column or table); on a comment, resolve it |
| `Ctrl+P › comments` | Comments: show their panel, jump to one (a badge on the speech bubble counts them) |
| Right click | Menu of the note: link, comment, cut, copy, paste, bold, italic |
| `Ctrl+C` | Copy |
| `Ctrl+X` | Cut |
| `Ctrl+V` | Paste text or a picture; an address over a selection makes it a link |
| `Ctrl+A` | Select all |
| `Ctrl+Z` | Undo |
| `Ctrl+Shift+Z` / `Ctrl+Y` | Redo |
| `Ctrl+Left` / `Right` | Move by word |

## Blocks

| Keys | Action |
|---|---|
| `Grip` | Left of the line under the pointer: drag it to move the block (a line shows where it lands), click it to select the block and open its menu; the + left of it adds a block under it and opens the / list |
|  | Block: turn into text |
|  | Block: turn into heading 1 |
|  | Block: turn into heading 2 |
|  | Block: turn into heading 3 |
|  | Block: turn into a bulleted list |
|  | Block: turn into a numbered list |
|  | Block: turn into a task list |
|  | Block: turn into a quote |
|  | Block: turn into a code block |
|  | Block: duplicate |
| `Alt+Up` | Block: move up (in a table, the row) |
| `Alt+Down` | Block: move down (in a table, the row) |
|  | Block: delete |
