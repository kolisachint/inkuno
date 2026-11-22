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
