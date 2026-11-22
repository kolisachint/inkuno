# Inkuno workflow run W1-W8 (Xvfb :120, debug binary, seeded temp vault)

Driver: `target/visual/wf-app.sh`
(subcommands `start`, `key`, `type`, `click`, `dclick`, `rclick`, `hover`, `shot`, `stop`). Exact action sequence: `wf-app/steps.txt`.
Screenshots: `wf-app/<step>.png` (full window crop 1300x940); 50% copies in `wf-small/`.
Coordinates are window-relative. Every shot was taken after a 1 s wait and a nudge. Clicks on the window use a hover first.
Window at (50,20). Welcome click is at absolute (731,505) as in visual-check.sh.

Notes on method: W3-04, W4 and W6-05 were rerun from a clean app after my own keystrokes corrupted the vault (W3) or the layout changed (W5). "Note" column says when a result is inferred.

## Per step

| Step | File | Keys / clicks | Observed | Flag |
|---|---|---|---|---|
| W1-01 start | W1-01.png | launch, welcome click | Empty "New note" with "Write here…" and Today/New note/Search cards; sidebar hidden | OK |
| W1-02 new note | W1-02.png | click "New note" card (not Ctrl+N, which is scratch) | Vault note opened; title becomes "Groceries" on first line (inferred from W1-03) | OK |
| W1-03 type | W1-03.png | type "# Groceries", Enter, "- [ ] milk", Enter, "- [ ] eggs" | Enter auto-continues "- [ ] ", so the literal second prefix gives "- [ ] - [ ] eggs"; fixed in W1-03c | OK (note: literal typing duplicates the marker) |
| W1-04 render | W1-04.png | click blank area below text | Lines render, checkboxes shown | OK |
| W1-05 toggle box | W1-05.png (+ W1-05b..i) | click box at (262,94), hover then click; click right edge; Ctrl+Enter | Mouse click on the box opens the block-type menu (grip overlays the box) or reveals raw source; never toggles. Ctrl+Enter toggles (`[x]`, struck text) | BUG |
| W1-06 sidebar tree | W1-06.png | Ctrl+E | Tree shows Groceries highlighted, plus Projects, ideas, reading-list, roadmap, week-plan | OK |
| W2-01 palette open | W2-01.png | Ctrl+P | Palette opens with search field | OK |
| W2-02 type | W2-02.png | type "road" | roadmap top, week-plan, "Create « road »" | OK |
| W2-03 Enter | W2-03.png | Enter | Roadmap opens in a new tab; tab strip appears (2 tabs) | OK |
| W2-04 single click | W2-04.png | click "ideas" in tree | Opens as a normal, non-italic tab. User guide says click = open, arrows = italic preview, so the scenario text is wrong | OK (scenario mismatch) |
| W2-05 double click | W2-05.png | double-click reading-list in tree | Previewed tab becomes solid. The app has no pinned-tab state | MISSING (pinned tab does not exist) |
| W2-06 Ctrl+W | W2-06.png | Ctrl+W | reading-list tab closed; ideas active | OK |
| W2-07 Ctrl+Shift+T | W2-07.png | Ctrl+Shift+T | reading-list reopened at the end of the strip | OK |
| W3-01 select word | W3-01.png | double-click "Language" | Word selected; selection toolbar (Text, B, I, S, code, link, comment) floats over the line above | OK |
| W3-02 bold | W3-02.png | click B on toolbar | Bold applied (raw shows `**Language**` when revealed). Ctrl+B is sidebar; bold is Ctrl+Alt+B per keybindings.md | OK (scenario mismatch) |
| W3-03 slash | W3-03.png | Enter, type "/" | Slash menu: Basic (H1-H3, lists, quote, code) and More (Table, Panel, Divider, ...) | OK |
| W3-04 pick Table | W3-04.png | type "tab", Enter, Enter (3x3) | Table opens a size grid ("3 × 3, or type 12×5"); Enter inserts 3x3 table, caret in first cell. Mouse pick in an earlier run dismissed the menu (not re-tested cleanly) | OK (keyboard) |
| W3-05 reveal | W3-05.png | click rendered line "A Pattern Language" | Line shows raw `- A Pattern **Language**` | OK |
| W4-01 Ctrl+\ | W4-01.png | Ctrl+\ | Right pane opens; palette shows immediately on it (footer "Ctrl+Enter in the other pane") | OK |
| W4-02 open other note | W4-02.png | type "ideas", Enter | Ideas opens in right pane | OK |
| W4-03 Ctrl+1 | W4-03.png | Ctrl+1 | Caret moves to left note. Inactive-pane tab dimming not visible: single-tab panes show no tab strip | OK (dimming not observable) |
| W4-04 close split | W4-04.png | Ctrl+2, Ctrl+W | Right pane closed, one pane left | OK |
| W5-01 outline | W5-01.png | Ctrl+Shift+O | Right panel shows Outline: Roadmap > Q1 goals, Later | OK |
| W5-02 outline click | W5-02.png | click "Later" | Entry highlighted, caret moves; note too short to scroll | OK |
| W5-03 comments view | W5-03.png | Ctrl+Alt+2 | "Comments 0 - This note has no comments" | OK |
| W5-04 comment | W5-04.png | select line, toolbar comment icon, type text | Inline `<!-- comment: … -->` markup inserted; card with Resolve appears in panel; text syncs | OK |
| W5-05 resolve | W5-05.png | click Resolve | Markup removed, text kept; count back to 0 | OK |
| W5-06 versions | W5-06.png | Ctrl+Alt+3 | "Save version" plus empty state "No version yet. Save one with Ctrl+Alt+S." | OK |
| W5-07 save named | W5-07.png | Ctrl+Alt+S, type "First draft of the roadmap", Enter | Toast "Revision 1 saved"; list shows "#1 · unknown · First draft…" (author "unknown" when none set) | OK |
| W5-08 compare | W5-08.png | edit note, click version #1 | Tab "roadmap · #1 ↔ now" with red/green line diff; CriticMarkup markers shown raw | OK |
| W5-09 assistant | W5-09.png | Ctrl+Alt+4 | Panel shows "`hoocode` is not installed or not on the PATH", plus Review/Improve buttons. Environment has no assistant CLI | OK (env-limited) |
| W6-01 context menu | W6-01.png | right-click reading-list | Menu: Open, Open to the side, New note here, New folder, copy link/path, Show in file manager, Duplicate, Rename F2, Move to, Icon, Move to the trash | OK |
| W6-02 rename | W6-02.png | click Rename | Dialog "Rename: new name", pre-filled | OK |
| W6-03 type name | W6-03.png | Ctrl+A, "reading-books", Enter | Renamed; tab and tree update | OK |
| W6-04 new folder | W6-04.png | Ctrl+Shift+N, "Archive", Enter | "Folder name" dialog; Archive folder created at top of tree | OK |
| W6-05 trash | W6-05.png (+ W6-05d..f) | context menu "Move to the trash" | Toast "Moved to the trash: « ideas.md »" with Undo. Undo restored reading-books within the toast window ("Put back" toast). Earlier Undo misses were my timing (toast about 4 s) | OK |
| W6-06 scratch | W6-06.png | Ctrl+N, type text | Scratch section lists "A quick scratch thought about…". Tab title truncated to "A quick scratch thought at" with no visible "· Scratch" marker | BUG (marker hidden) |
| W7-01 Ctrl+B | W7-01.png | Ctrl+B | Sidebar hidden | OK |
| W7-02 Ctrl+Shift+S | W7-02.png | Ctrl+Shift+S | Title band hidden; tab strip moves to top edge | OK |
| W7-03 hover top | W7-03.png | mouse to window top edge (y=1) | Band slides back over content with shadow | OK |
| W7-04 F9 | W7-04.png | F9 | Note alone, edge to edge: no sidebar, panel, tabs or band | OK |
| W7-05 Esc | W7-05.png | Escape | Panel and tabs return; sidebar stays hidden (as set before focus) | OK |
| W8-01 Settings | W8-01.png | Ctrl+, | Settings tab on Appearance (Theme, Text size, Page width, Animations) | OK |
| W8-02 Dark | W8-02.png | click Dark | Whole UI switches to dark; active segment contrast is faint | OK |
| W8-03 Editor page | W8-03.png | click Editor | Font, Code font, Wide tables, Formatting bar, Revision author, Assistant command. Dark-mode input boxes barely visible | OK |
| W8-04 Shortcuts page | W8-04.png | click Shortcuts (second try, after hover) | Page opens; description column narrow, text wraps over 3-5 lines. First click only highlighted the row | BUG (cramped layout) |
| W8-05 toggle | W8-05.png | Appearance > Animations | Toggle switches off | OK |
| W8-06 close | W8-06.png | Ctrl+W | Settings tab closes; back to roadmap note | OK |

Scenario count: 48 steps. OK 44, BUG 3 (W1-05, W6-06, W8-04), MISSING 1 (W2-05).

## Extra checks (not scenario steps)

| File | What | Result | Flag |
|---|---|---|---|
| W3-04a/b | Table via mouse | Not reproduced cleanly | - |
| W2-04b | Down arrow in tree | Previews reading-list in italics | OK |
| W4-r1..r5 | Split repro and palette | Palette opens on split; Escape closed it in E2; one earlier run kept palette after Escape (W4-r3), not reproduced | OK (low) |
| E2-split-escape.png | Shortcuts page in a split pane | Text column collapses to one word per line, unreadable | BUG (severe) |
| W6-05e-trashlist.png | palette `trash` | First item is "Create « trash »" (new note), restore command is second; docs say `trash` lists the trash | BUG (docs mismatch) |
| W8-01 zoom | Text size hint | "Also ⌘+ and ⌘−" on Linux (Mac key label) | BUG |
| W8-04b | F1 | Opens Settings Shortcuts page (not a help overlay) | OK |
| W1-06 | Window title | Shows vault name "Notes" while tree is shown, note title otherwise | BUG (minor) |
| W2-04, W2-07 | First list item of a note | Shows raw "-" until caret moves (reveal at load) | BUG (minor) |
| tabs (W1-01..W4-04) | Single tab per pane | No tab strip shown; docs (tabs-and-panes.md:9) say it shows even with one tab | BUG |

## app.log / crashes

- `wf-app/app.log`: empty for all runs (no stdout or stderr output). No panics, no errors.
- No crash file or log in the temp config folder.
- `wf-app/xvfb.log`: only xkbcomp keysym warnings (XF86SidevuSonar, XF86NavInfo). Harmless.

## Top problems

1. Clicking a task box never toggles; the hover grip covers it and opens the block menu (Ctrl+Enter works).
2. Shortcuts page is unreadable in a split pane: description text collapses to one word per line.
3. Shortcuts page is cramped at full width too (narrow description column, 3-5 line wraps).
4. Palette `trash` creates a note called "trash" first; the documented listing needs a second command.
5. Single-tab panes show no tab strip, contrary to docs; this hides pane-dimming in splits.
6. Tab title truncation hides the "· Scratch" marker on scratch notes.
7. Settings text-size hint shows Mac shortcuts (⌘+/⌘−) on Linux.
8. Pinned-tab behaviour (scenario W2-05) does not exist; the single click opens a normal tab, not an italic preview (docs agree on click).
9. Window title shows the vault name while the tree is shown, and the note title otherwise.
10. Dark mode: input fields and the active theme segment have very low contrast.
