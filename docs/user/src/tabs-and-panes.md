# Tabs and panes

## Tabs

A pane holds as many files as you like, as in a code editor.

- **Opening**: a click on a file in the tree, a note chosen in the palette, a link followed, a new note: each opens in a tab of its own, right of the one shown. If the file is already open, its tab comes to the front instead. A note is open only once.
- **Previewing**: a single click in the tree, or the arrows, previews a note in a single tab, in italics. The next single click or arrow replaces it. A double click, or Enter on the row, opens the note for good and keeps its tab.
- **The tab strip** is in the title band, above each pane (see [Navigation](navigation.md)), even when it holds one tab. Each tab is a pill; a preview tab is in *italics*. A click shows a tab. Its cross, or the middle button, closes it.

| Key | Action |
|---|---|
| `Ctrl + Tab`, `Ctrl + Shift + Tab` | next, previous tab |
| `Ctrl + Shift + Page Up`, `Page Down` | move the tab along the bar |
| `Ctrl + W` | close the tab, saving what it holds first. The pane goes with its last tab. |
| `Ctrl + Shift + T` | reopen the last tab closed |

A right click on a tab opens its menu, which the arrows, `Enter` and `Escape` also work:

- **Close**: that tab, as `Ctrl + W` does.
- **Close other tabs**: every tab of its pane but the one clicked, even if it was not the one shown. It stays, shown, with the focus.
- **Close all tabs in this pane**: the pane goes, the other panes keep their tabs.

**Close all files**, in the palette, closes every tab of every pane. The window is left with one pane on a new empty note, and comes back that way at the next start. These three also have a line in the palette and can be given a shortcut. Closing never deletes a file: each tab is saved first, a new note with text is kept as a note, and only a new note left empty leaves nothing behind. A tab that cannot be saved stays open with its edits while the others close; the first one gets the focus. `Ctrl + Shift + T` reopens the closed files one by one, the last first.

A tab can be dragged along the bar, into another pane, or to an edge to split one. See [Drag a tab](#drag-a-tab).

- **If saving fails**, the tab stays open and comes to the front with its text and undo history. The message names the file and explains the error.
- **Retry**: restore the destination's writability, then close again.
- **Closing the window**, using **Quit**, or switching vaults also keeps the current documents open if one cannot be saved.

Tabs come back at the next start. A tab out of sight keeps its text, cursor and undo, but lets go of what it drew, so a hundred tabs cost little memory.

## HTML pages

An HTML file of the vault opens as a page, not as text. Where it opens depends on the system:

- **macOS**: in a tab of its own, beside the notes.
- **Linux**: in a separate window, opened by `inkuno view`.
- **Windows**: in the system's default app for HTML. Inkuno has no viewer there.

In Inkuno's page (macOS, Linux):

- The page sees only the files in its own folder and the folders inside it. A file outside them does not load.
- A link to `http`, `https` or `mailto` opens in the system's browser or mail program.

## Panes

`Ctrl + \` splits the pane to the right, `Ctrl + Alt + \` below, as many times as the window has room for. Write a note while reading two others, or keep a table under a note.

- A new pane opens on the palette, to choose what goes there. Each pane shows anything Inkuno can show.
- A pane is split only if both halves keep room to write: 300 px wide, 200 px high. Otherwise a message says so. The very first split hides the sidebar when that is enough.
- The pane with the focus shows its active tab solid, and the other pane's tabs are dimmed. A hairline divides the panes. The palette, the tree and the shortcuts act on it.
- A note already shown in one pane is not opened twice: the focus moves to it.
- The layout comes back at the next launch.

| Key | Action |
|---|---|
| `Alt + Shift + ←` `→` `↑` `↓` | move to the pane on that side (`Cmd + Alt + arrows` on macOS) |
| `Ctrl + 1`, `Ctrl + 2` | move to the first, second pane |
| `Ctrl + W` | close the tab that has the focus, and the pane with its last tab. Its neighbour takes its room. |
| `Alt + Shift + Enter` | put the pane on the whole window, and back. The others keep what they hold. |
| `Ctrl + Enter` in the palette | open the chosen note in the pane beside |
| `Ctrl + Shift + click` on a `[[link]]` | open it in the pane beside |

**Open to the side** in the menu of the tree does the same. The palette also has **Move the tab to the pane on the left** (right, above, below; a pane is split off when there is none on that side) and **Even out the panes**.

**Sizes**: every border between panes can be dragged, down to the smallest size of a pane. A double click shares that split evenly. A window too small for every pane shows the one with the focus alone, and all of them again once it is big enough.

### Drag a tab

Drag a tab to arrange the window:

- dropped on a tab bar, it moves there (a line shows where);
- dropped on the middle of a pane, it joins it;
- dropped near one of its edges, the pane splits on that side and the tab takes the new half.

The part that would receive it is tinted while you drag. `Esc` lets go without moving anything.

A file dragged from the tree does the same. Dropped on the right half of a window that has a single pane, it opens a second pane.

## The right panel

The panel holds the views about the open note: outline, comments, versions and the Assistant. There is one panel, not one per tool, and it is not a dock: the switcher in the title band picks the view, and a view's key does the same. See [Navigation](navigation.md#the-right-panel).

- **Hiding**: the band's switch, or the key of the view on display.
