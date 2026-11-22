# Prototype workflow log (W1-W8)

File under test: /home/user/inkuno/docs/design/prototypes/v2.html (read only, nothing under /home/user/inkuno edited).
Driver: target/visual/wf-proto.js
Shots: `W#-NN.png` in this folder (`#app` element, 1300x940, DPR 1, light). Raw state per step: `results.json`.
Each scenario starts from a fresh page load. Every step waits 300 ms before its shot.
Flags: OK = works as scenario expects · MISSING = prototype lacks it (toast only or nothing) · ODD = prototype behaviour that looks wrong or inconsistent.

## W1 first-note
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W1-01.png | none | Opens with seeded "Week plan" tab active, not an empty/welcome pane | OK |
| 02 | W1-02.png | Control+Shift+N | "Untitled" tab and sidebar row added; toast "New note in ~/Documents/Notes" | OK |
| 03 | W1-03.png | keyboard.type "# Groceries", Enter, "- [ ] milk", Enter, "- [ ] eggs" | Keys go nowhere; note text unchanged, title stays "Untitled" | MISSING |
| 04 | W1-04.png | click .pscroll | Click is a no-op; live line cleared | OK |
| 05 | W1-05.png | #side [data-open=week], .box (first) | Week plan's "Pick the colours" box toggles (aria-pressed true to false). New note has no tasks to test with | OK |
| 06 | W1-06.png | sidebar screenshot | "Untitled" listed under Notes, between Ideas and Packing list | OK |

## W2 navigate
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W2-01.png | Control+p | Palette opens with search field and recent list | OK |
| 02 | W2-02.png | keyboard.type "road" | Typed into palette; list filters | OK |
| 03 | W2-03.png | Enter | Enter opens the first row, which is the already-open "Week plan" (its body contains "roadmap"; open tabs rank first). No Roadmap tab opened | ODD |
| 04 | W2-04.png | click #side [data-open=ideas] | Ideas opens as a preview tab (italic, "Ideas(prev)") | OK |
| 05 | W2-05.png | dblclick #side [data-open=ideas] | Preview promoted to a pinned tab | OK |
| 06 | W2-06.png | Control+w | Ideas tab closes; Week plan becomes active | OK |
| 07 | W2-07.png | Control+Shift+T | Nothing reopens; no reopen-closed-tab handler in the prototype | MISSING |

## W3 format
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W3-01.png | dblclick "quiet" | Selection toolbar appears above the word (Text, B, I, S, code, link, Comment) | OK |
| 02 | W3-02.png | .fbar button[title^=Bold] | Only toast "Bold: wraps the selection in **"; text not bolded. Tooltip reads "Ctrl B" | MISSING |
| 03 | W3-03.png | "/" | Slash menu opens with Basic grid and More list | OK |
| 04 | W3-04.png | .slash "Table" | Table block appended at the END of the note, not at the caret; the "/" is not in the text. Toast "Added table: \| Column \| Column \|" | ODD |
| 05 | W3-05.png | click first .b.k-h (heading) | Heading's "#" marker appears on the live line | OK |

## W4 split
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W4-01.png | Control+Backslash | Right pane opens with a copy of Week plan; focus moves right | OK |
| 02 | W4-02.png | #side [data-open=road] | Roadmap preview tab opens in the right pane | OK |
| 03 | W4-03.png | Control+1 | Left pane focused; right pane's tabs dimmed | OK |
| 04 | W4-04.png | Control+Backslash | Closes the focused (left) pane, leaving Roadmap in one pane | OK |

## W5 right panel
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W5-01.png | Control+Shift+O | Outline view with Week plan, Tasks, Code, Status | OK |
| 02 | W5-02.png | .ol "Tasks" | Note scrolls to Tasks; entry highlighted | OK |
| 03 | W5-03.png | Control+Alt+2 | Comments view, badge 2 | OK |
| 04 | W5-04.png | dblclick "quiet", .fbar Comment, #cdraft "Keep it short", Enter | Toast "Comment added"; card shows whole line as passage. Raw `<!-- comment: You, 10:10: Keep it short -->` shows as text on the live line; comment anchors to the line, not the selection | ODD |
| 05 | W5-05.png | [data-act=resolve] (last) | Toast "Comment resolved"; card removed | OK |
| 06 | W5-06.png | Control+Alt+3 | Versions view with Save version button and seeded list | OK |
| 07 | W5-07.png | Control+Alt+S, type "Before review", Enter | Name field; toast "Saved version "Before review"", listed first | OK |
| 08 | W5-08.png | .vrow (first) | Compare tab "Cmp:Before review" opens (diff with now) | OK |
| 09 | W5-09.png | Control+Alt+4 | Assistant view, Review / Tighten / ask field | OK |

## W6 files
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W6-01.png | right-click #side [data-ctx=road] | Context menu: Open, Open in new tab, Rename, Duplicate, Move to…, Copy path, Move to Trash | OK |
| 02 | W6-02.png | .pop "Rename" | Toast "Rename edits the file name in place"; no inline rename field | OK |
| 03 | W6-03.png | type "Roadmap v2", Enter | Nothing; sidebar still "Roadmap" | MISSING |
| 04 | W6-04.png | right-click [data-fold=Projects] | No menu; "New folder" appears nowhere (checked DOM text) | MISSING |
| 05 | W6-05.png | .pop "Move to Trash" | Roadmap removed from the tree; toast "Moved "Roadmap" to Trash" with Undo (Undo not exercised) | OK |
| 06 | W6-06.png | Control+n | Tab "Untitled scratch" opens; appears under Scratch; toast "New scratch note in ~/Documents/Scratch" | OK |

## W7 chrome
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W7-01.png | Control+b | Sidebar hidden (noside class) | OK |
| 02 | W7-02.png | Control+Shift+S | Title bar hidden (bandhide); 12 px strip stays | OK |
| 03 | W7-03.png | mouse over #strip | Title bar slides back (peek class). Leaving removes it | OK |
| 04 | W7-04.png | F9 | Focus mode (focus class); chrome hidden | OK |
| 05 | W7-05.png | Escape | Focus mode off | OK |

## W8 settings
| Step | File | Keys / selectors | Observed | Flag |
|---|---|---|---|---|
| 01 | W8-01.png | Control+k, ">settings", Enter | Settings tab opens on the Appearance page | OK |
| 02 | W8-02.png | [data-set=mode][data-v=Dark] | Theme set to Dark | OK |
| 03 | W8-03.png | [data-page=Editor] | Editor page shown | OK |
| 04 | W8-04.png | F1 | Shortcuts page opens | OK |
| 05 | W8-05.png | Editor > [data-toggle=reveal] | "Show Markdown on current line" switches off | OK |
| 06 | W8-06.png | Control+w | Settings tab closes; Week plan active | OK |

## Summary
- Steps: 48. OK: 40. MISSING: 5 (W1-03, W2-07, W3-02, W6-03, W6-04). ODD: 3 (W2-03, W3-04, W5-04).
- Driver notes: Playwright sends the key as typed, so shortcuts that check lowercase `e.key` need lowercase names (Control+b, Control+n, Control+w, Control+k, Control+p). Earlier uppercase sends failed for that reason and were rerun.
