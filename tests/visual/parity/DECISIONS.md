# Parity decisions (orchestrator, 2026-11-22)

For each finding: **fix** (the app moves to v2), **keep** (the app stays; v3 records it), or **later**.
IDs refer to `findings/<area>.md`.

## Right panel
| id | decision | note |
|--|--|--|
| RP-01 | fix | selected switcher button: neutral `selected` pill, as sidebar rows |
| RP-03/04 | fix | the switcher row is the panel's own 38 px band, aligned with the title band; no empty band above it |
| RP-07 | fix | active outline row: ink, weight 500, no accent |
| RP-12 | fix | comment card quote bar: 3 px, comment gold (`--cmd`) |
| RP-17 | fix | comment mark in the note: yellow wash + 1.5 px gold underline + 6 px gold dot in the right margin |
| RP-11 | fix (read) | a comment whose text starts `Name, HH:MM: ` shows name and time on its card; writing format unchanged |
| RP-13 | later | replies need a format decision: v3 |
| RP-19/20 | fix (draft card) | the comment draft is a card in the Comments view; anchoring stays the app's (passage rule from P7) |
| RP-23 | keep | compare stays a read-only tab (app-ui §19); v3 shows it as a tab with the compare bar on top |
| RP-25 | keep | assistant copy follows app-ui §16/§18 |
| RP-26 | fix | literal backticks in the missing-assistant message |
| RP-27/28 | keep | pending changes keep the app-ui §17 flow; v3 draws it |
| other L | fix when the file is open anyway | |

## Popovers, menus, settings
| id | decision | note |
|--|--|--|
| P-09 | fix | palette: matching notes and commands first, the "Create «…»" row always last |
| P-01 | fix | palette input carries the search icon in every mode |
| P-06 | fix | the active palette/menu row is the neutral hover wash, not accent |
| SET-10 | fix | shortcuts: description left (flex), keys right as chips (v2 `.keys`: 1fr auto); page max width 640 px; in a narrow pane the keys wrap under the description, never the words one per line |
| SET-04 | fix | modifier names follow the platform (Ctrl on Linux/Windows, ⌘ on macOS) everywhere a key is shown |
| SET-06 | fix | text fields: ground fill + `line` border + 2 px accent focus ring, both themes |
| SET-07 | fix (v3 too) | dark segmented: the active chip is lighter than the track (elevated surface), v2 had it inverted |
| C-01 | fix | context menu items carry their 16 px icon, as v2 `.mi` |
| TO-01/02 | fix | toast: dark pill, bottom centre, one action button (Undo…); a sticky error keeps its × |
| T-01 | fix | selection toolbar Comment: icon with tooltip, like the other buttons |
| SET-05 | keep | the Editor page keeps the app's real settings (no new settings without an issue, CONTRIBUTING); v3 draws the real set |
| SET-13 | later | Files page buttons |
| other L | fix when the file is open anyway | |

## Window chrome
| id | decision | note |
|--|--|--|
| C01 | fix | the band always shows the pane's tab strip, even for one note (v2 #3); the centred label goes |
| C02 | fix | the right-panel toggle is in the band on every platform and decoration mode |
| C09 | fix | with two panes, the other pane's tabs are dimmed (opacity .55); no ring |
| C03 | keep | the right panel starts closed on a first run (quiet default); v3 starts the same |
| C06 | fix | a single click in the tree opens a preview tab (italic) that the next single click replaces; double-click or an edit keeps it (v2 #3) |
| C07 | fix | a truncated tab truncates its title, never the "· Scratch" marker |
| C08 | fix | tab tooltip: the note's folder path |
| C04 | fix | the band is the sheet colour, not the ground |
| C05 | fix | tab: gap 6 px; the cross at .7 on the shown tab, on hover otherwise |
| C10 | fix | pane divider uses `line` |
| C11/C12/C14 | fix | follows C01: no centred label in the band; window title stays "Note — Folder — Inkuno" |
| C13 | fix | with the title bar hidden, the sidebar toggle stays in the sidebar's top row |
| C15 | fix | hidden title bar hides the tab row too; the strip peeks the whole band |
| C17 | fix | empty Scratch text fits one line ("Ctrl+N: new scratch note") |
| C18 | fix | sidebar footer 36 px (6 + 30) |
| C19 | fix | the sheet keeps its faint left shadow (`--shadow-note`) |
| C20 | fix | band icon buttons 30 px, radius 8 (in `components::icon_button`) |
| C21 | keep | an empty pane stays a live editor with the cards; v3 draws that |
| C22 | fix | empty-pane cards never break a key chord across lines (chips) |
| C23 | keep | unsaved dot on tabs; v3 adds it |
