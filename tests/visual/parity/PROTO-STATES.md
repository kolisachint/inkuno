# v2.html screenshot states

Each state has a `-light.png` and a `-dark.png` file in this folder. Theme is set with `document.documentElement.dataset.theme`. Each shot is a fresh page load. Window shots are `#app`; `30-design-log` is the full viewport.

- `01-default`: fresh load, no actions. Reached by a plain load. Sidebar, the Week plan note, and the Outline panel are all visible.
- `02-sidebar-hidden`: Ctrl+B (`.app.noside`). Reached by pressing Control+b. The sidebar is gone and the note sheet fills the window with the right panel still open.
- `03-panel-outline`: Ctrl+Alt+1. Reached by pressing Control+Alt+1 (`.rsw .ib[data-view=outline]`). The right panel shows the heading outline of the note.
- `04-panel-comments`: Ctrl+Alt+2. Reached by pressing Control+Alt+2. The right panel lists the note's comment cards with their quoted passages.
- `05-panel-versions`: Ctrl+Alt+3. Reached by pressing Control+Alt+3. The right panel lists the three saved versions under a Save version button.
- `06-panel-assistant`: Ctrl+Alt+4. Reached by pressing Control+Alt+4. The right panel shows the Review, Tighten and Ask controls.
- `07-panel-closed`: band right-panel button (`#band [data-act=rtog]`). Reached by clicking the title-bar panel icon. The right panel is hidden and the note widens.
- `08-palette`: Ctrl+K (`.pal`). Reached by pressing Control+k. A search box with the note list (Recent heading) floats over the window.
- `09-palette-commands`: Ctrl+K then typing `>`. Reached by pressing Control+k and typing `>`. The palette lists commands such as New note, Split pane right and Settings.
- `10-slash-menu`: click `[data-act=slash]` at the end of the note. Reached by clicking the "Type / to add a block" placeholder. A block-insert popover shows the Basic icon grid and the More list.
- `11-selection-toolbar`: text selected in the note. Reached by selecting "Everything in this note is GitHub Markdown". A floating format bar (Text, Bold, Italic, Strike, Code, Link, Comment) sits above the selection.
- `12-context-menu-file`: right-click a sidebar note (Roadmap). Reached by right-clicking `#side [data-ctx=road]`. A menu shows Open, Rename, Duplicate, Move to Trash and similar items.
- `13-context-menu-note`: SKIPPED. The prototype has no custom context menu for note text; the contextmenu handler only matches sidebar rows.
- `14a-settings-appearance`: Settings tab (`#side [data-act=settings]`) with Appearance selected. Theme, Text size, Page width, Motion and the More themes row are shown.
- `14b-settings-editor`: Settings tab on the Editor page (`.snav [data-page=Editor]`). Four switches: Markdown on current line, line numbers, continue lists, spell check.
- `14c-settings-files`: Settings tab on the Files page. Notes folder, Scratch folder and Trash rows with their buttons.
- `14d-settings-shortcuts`: Settings tab on the Shortcuts page. The list of shortcut keys as keycap chips.
- `14e-settings-about`: Settings tab on the About page. Two cards: the Inkuno description and the design line.
- `15-focus-mode`: F9. Reached by pressing F9 (`.app.focus`). Sidebar, title bar and right panel are hidden and only the note sheet remains.
- `16-split`: Ctrl+Backslash. Reached by pressing Control+Backslash (two `.pane` elements). The window shows two panes side by side, both on the Week plan note.
- `17-titlebar-hidden`: Ctrl+Shift+S (`.app.bandhide`). Reached by pressing Control+Shift+S. The tab band collapses to a thin strip above the note.
- `18-tabs-multiple`: Roadmap opened with a double-click, Reading list opened with a single click. Reached by dblclick on `[data-open=road]` then click on `[data-open=read]`. The tab strip holds three tabs and the Reading list tab is italic as the preview tab.
- `19-empty-pane`: all tabs closed. Reached by opening Roadmap, then Control+w twice. The pane shows the empty-state Today, New note and Search cards.
- `20-toast`: Move to Trash on Ideas. Reached by right-clicking `[data-ctx=ideas]` and choosing Move to Trash. A dark toast reads "Moved Ideas to Trash" with an Undo button.
- `21-line-revealed`: a line clicked. Reached by clicking the "Spacing between blocks" task line. That line shows its raw `- [ ]` Markdown while the rest of the note stays rendered.
- `22-compare-version`: a saved version opened for comparison. Reached by Control+Alt+3 then clicking the first version. A compare bar reads "Comparing ... with now" with Restore and Close, and the diff is shown.
- `23-scratch-section-expanded`: Scratch section toggled closed then open. Reached by clicking the Scratch heading twice. The Packing list and Sprint notes rows are visible under Scratch.
- `30-design-log`: Design log dialog (`button[data-act=dlog]`, full viewport). A right-side sheet with the design decisions table and open questions.
- `31-try-guide`: "What to try" guide (`button[data-act=whattry]`). A small card at the bottom right with usage tips and a close button.
- `32-theme-picker`: Appearance "Choose…" (`[data-act=themes]`). Opens the command palette pre-filled with `>Theme`, listing the Theme System, Light and Dark commands.
- `33-trash-info-toast`: Settings Files, Open Trash. A toast reads "Trash keeps notes in .trash inside your notes folder".
- `34-link-to-note-picker`: `/` then Link to note. A popover titled "Link to note" lists the other notes with their folders.
- `35-link-to-date-picker`: `/` then Link to date. A popover titled "Link to date" lists seven recent days with ISO dates.
- `36-review-pending-pill`: Assistant Review this note, waits for the 1s delay. The note shows pending inserted and struck text with Accept and Reject buttons, and a bottom pill reads "2 changes pending" with Accept all and Reject all.
- `37-comment-draft`: Comment chosen in the selection toolbar. The right panel shows a new comment card with the quoted passage and an empty comment field.
- `38-palette-query`: Ctrl+K then typing "road". The palette shows the Roadmap note with a matching text snippet.
- `39-shortcuts-listening`: Settings Shortcuts, first key chip clicked. The chip reads "Press keys…" in the accent colour, waiting for a new key.
- `40-titlebar-peek`: Ctrl+Shift+S then hovering the strip. The title band slides back over the top of the note as a floating bar with the tabs. The DOM check for `#band.peek` was flaky in the harness (the band covers the pointer after it slides in), but the 50% view of the light shot confirms the revealed band.

Totals: 74 PNGs (37 states x 2 themes). Skipped: 13-context-menu-note.
