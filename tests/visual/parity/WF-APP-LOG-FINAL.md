# Inkuno workflow run W1-W8, final build

Driver: `tests/visual/parity/wf-app.sh` (debug binary, never rebuilt). Display :120, seeded temp vault (removed on stop).
Screenshots: `target/visual/wf-final/wf-app/<step>.png` (full window crop 1300x940); 50% copies in `.../wf-final/wf-small/`.
Coordinates are window-relative. The W1-W8 run was one continuous session; the palette `trash` check (W6-05p/q) was a separate short restart after the run.
`app.log` is empty for both launches (no panics or errors). Vault files checked on disk (see W1-05, W6-05).

| Step | File | Action | Result | Flag |
|---|---|---|---|---|
| W1-01 start | W1-01.png | launch, welcome click | Empty "New note" with Today / New note / Search cards; sidebar hidden | OK |
| W1-02 new note | W1-02.png | click "New note" card | Vault note opened (title becomes "Groceries") | OK |
| W1-03 type | W1-03.png | type "# Groceries", Enter, "- [ ] milk", Enter, "- [ ] eggs" (literal) | Enter auto-continues "- [ ] ", so the literal prefix gives "- [ ] - [ ] eggs" on disk. Same as the previous run; input artifact of the literal script, not a new fault | OK (note) |
| W1-04 render | W1-04.png | click blank area below text | "milk" renders as a checkbox; the last line stays raw because the caret stays on it. Same as before | OK (note) |
| W1-05 toggle box | W1-05.png | single click on milk checkbox at (256,120) | Box checked, file now `- [x] milk` | OK (FIXED) |
| W1-06 sidebar tree | W1-06.png | Ctrl+B to show sidebar | Tree lists Projects, Groceries (active), ideas, reading-list, roadmap, week-plan | OK |
| W2-01 palette open | W2-01.png | Ctrl+P | Palette with search field | OK |
| W2-02 type | W2-02.png | type "road" | roadmap first, week-plan, "Create « road »" last | OK |
| W2-03 Enter | W2-03.png | Enter | Roadmap opens in a tab | OK |
| W2-04 single click | W2-04.png | single click "ideas" in tree | Tab "ideas" in italic (preview) | OK |
| W2-05 double click | W2-05.png | double click "reading-list" | Preview replaced; "reading-list" tab now solid (kept) | OK (FIXED) |
| W2-06 Ctrl+W | W2-06.png | Ctrl+W | reading-list tab closed; roadmap active | OK |
| W2-07 Ctrl+Shift+T | W2-07.png | Ctrl+Shift+T | reading-list reopened | OK |
| W3-01 select word | W3-01.png | double click "Designing" | Word selected; selection toolbar (Text, B, I, S, code, link, comment) shown | OK |
| W3-02 bold | W3-02.png | click B on toolbar | "Designing" bold; raw `**Designing**` shown while the caret is on the line | OK |
| W3-03 slash | W3-03.png | End, Enter, "/" | Slash menu: Basic (H1-H3, lists, quote, code) and More (Table, Panel, Divider, Link, ...) | OK |
| W3-04 pick Table | W3-04a/W3-04.png | type "tab", Enter, Enter | Size grid ("4 × 6"); Enter inserts a 4-column table, caret in first cell | OK |
| W3-05 reveal | W3-05.png | click rendered "A Pattern Language" | Line shows raw `- A Pattern Language` with caret | OK |
| W4-01 Ctrl+\ | W4-01.png | Ctrl+\ | Right pane opens with a New note tab and the palette | OK |
| W4-02 open other note | W4-02.png | type "ideas", Enter | ideas opens in right pane | OK |
| W4-03 Ctrl+1 | W4-03.png | Ctrl+1 | Focus to left pane; right pane tab "ideas" dimmed | OK |
| W4-04 close split | W4-04.png | Ctrl+2, Ctrl+W | Right pane closed; one pane | OK |
| W5-01 outline | W5-01.png | Ctrl+Shift+O | Outline: Roadmap > Q1 goals, Later | OK |
| W5-02 outline click | W5-02.png | click "Later" | Caret jumps to Later; entry highlighted | OK |
| W5-03 comments view | W5-03.png | Ctrl+Alt+2 | "Comments 0 - This note has no comments" | OK |
| W5-04 comment | W5-04a/b/.png | select "Stabilise", toolbar comment button, type text, Enter | Draft card in Comments panel ("Add a comment, then press Enter"); after Enter card shows the text with Resolve, inline markup updates | OK (FIXED) |
| W5-05 resolve | W5-05.png | click Resolve | Count 0; markup removed, text kept | OK |
| W5-06 versions | W5-06.png | Ctrl+Alt+3 | Save version button and "No version yet. Save one with Ctrl+Alt+S." | OK |
| W5-07 save named | W5-07.png | Ctrl+Alt+S, "First draft of the roadmap", Enter | Toast "Revision 1 saved"; list "#1 · unknown · First draft of the road…" | OK |
| W5-08 compare | W5-08.png | type " (v2)" in note, click version #1 | Tab "roadmap · #1 ↔ now"; red/green line diff | OK |
| W5-09 assistant | W5-09.png | Ctrl+Alt+4 | "hoocode is not installed…" with Review/Improve buttons (environment has no assistant CLI) | OK (env) |
| W6-01 context menu | W6-01.png | right click reading-list | Menu: Open, Open to the side, New note here, New folder, copy items, Show in file manager, Duplicate, Rename F2, Move to, Icon, Move to the trash | OK |
| W6-02 rename | W6-02.png | click Rename | Dialog "Rename: new name", pre-filled | OK |
| W6-03 type name | W6-03.png | Ctrl+A, "reading-books", Enter | Renamed in tree and tab | OK |
| W6-04 new folder | W6-04a/.png | Ctrl+Shift+N, "Archive", Enter | Archive folder at top of tree | OK |
| W6-05 trash | W6-05.png | right click ideas, "Move to the trash" | Toast pill bottom-centre "Moved to the trash: « ideas.md »" with Undo; ideas gone from tree and from the vault on disk | OK |
| W6-05p/q palette trash | W6-05p.png, W6-05q.png | Ctrl+P, type "trash", Enter (separate restart) | First row "Restore from the trash…", then "Create « trash »". Enter runs the trash command (toast "The trash is empty"); no note named trash created | OK (FIXED) |
| W6-06 scratch | W6-06.png | Ctrl+N, type text | Scratch section lists the note; tab "A quick scratch thought … · Scratch" with the marker visible | OK (FIXED) |
| W7-01 Ctrl+B | W7-01.png | Ctrl+B | Sidebar hidden | OK |
| W7-02 Ctrl+Shift+S | W7-02.png | Ctrl+Shift+S | Title band hidden with its tab strip; no tabs visible | OK (note) |
| W7-03 hover top | W7-03.png | mouse to top edge | Band slides back over content with shadow, tabs visible | OK |
| W7-04 F9 | W7-04.png | F9 | Note centred alone; no sidebar, panel or band | OK |
| W7-05 Esc | W7-05.png | Escape | Panel returns; sidebar and band stay hidden; tabs do not return until the band is shown again (see notes) | OK (note) |
| W8-01 Settings | W8-01.png | Ctrl+, | Settings tab on Appearance | OK |
| W8-02 Dark | W8-02.png | click Dark | Whole UI switches to dark | OK |
| W8-03 Editor page | W8-03.png | click Editor | Editor settings; inputs readable in dark mode. Revision author input takes focus | OK |
| W8-04 Shortcuts (F1) | W8-04.png | F1 while Editor field focused | No navigation: F1 is not handled while the Editor page's text field has focus | BUG (minor) |
| W8-04c | W8-04c.png | click Appearance, then F1 | Shortcuts page opens | OK |
| W8-04b | W8-04b.png | click Shortcuts | Descriptions on one line, keys right-aligned, readable | OK (FIXED) |
| W8-05 toggle | W8-05.png, W8-05c.png | click Animations toggle | Toggle off (first two clicks missed the switch: coordinate error, not app) | OK |
| W8-06 close | W8-06.png | Ctrl+W | Settings tab closed; back to reading-books note | OK |

Scenario count: 48 steps (W1 6, W2 7, W3 5, W4 4, W5 9, W6 6, W7 5, W8 6). Plus 2 extra palette shots (W6-05p/q) and extra retries noted above.
Counts: OK 47, BUG 1 (W8-04, minor), MISSING 0.

## Changes since the first run

- W1-05 task box click now toggles (fixed).
- W2-05 preview and kept tabs exist: single click previews in italic, double click keeps (fixed; was MISSING).
- W5-04 comment shows a draft card in the Comments panel (fixed).
- W6-05 palette `trash` runs the trash command, not Create (fixed).
- W6-06 scratch tab shows the "· Scratch" marker (fixed).
- W8-04 Shortcuts page readable: one-line descriptions, keys at right (fixed; severe split-pane case not rechecked).
- New: F1 ignored while the Editor settings text field has focus (W8-04). Minor.
- Changed: hiding the title band (Ctrl+Shift+S) also hides the tab strip, and tabs do not come back on Esc. Check if intended.
- Unchanged, not re-fixed: literal "- [ ] " typing duplicates the marker; the caret-line raw reveal; the Settings text-size hint shows Mac shortcuts.
