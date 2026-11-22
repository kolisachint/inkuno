# Popovers, menus, palette, toast, settings: app vs v2 prototype

Scope: states 08, 09, 10, 11, 12, 13 (app only), 14 settings pages, 20 toast; workflows W3 (format), W6 (files), W8 (settings). Light unless marked D (dark). Read-only audit; no repo files edited.

Target tokens and px come from `docs/design/prototypes/v2.html` (CSS lines 255-313, 395-402; tokens lines 7-90). App code is under `crates/inkuno/src/` and `crates/inkuno-ui/src/`.

Capture caveats (harness, not design):
- `app/light/20-toast.png` shows "Revision 1 saved" at bottom right, not the Move-to-Trash toast. The trash toast was checked in `wf-app/W6-05.png`.
- `app/light/14-settings.png` is Appearance, but `14a` shows a daily note, `14b`, `14c` and `14d` show Appearance. The Editor, Files, Shortcuts and About light pages were not captured. Editor and Shortcuts were checked in dark via `wf-app/W8-03.png` and `W8-04.png`. Files was checked by code only (SET-13). About was not compared.
- `app/dark/` has no `20-toast.png`.
- `app/light/08-palette.png` has the query "week" typed. The v2 `08` is empty, so compare with `wf-app/W2-01.png` for the empty state.
- `13-context-menu-note` is app only. It has no v2 counterpart, so it is listed for the record (C-06).

## Findings

| id | state/workflow | theme | difference (app vs v2) | v2 target (CSS/px/token) | app location | sev | kind |
|---|---|---|---|---|---|---|---|
| P-09 | W6-05e, palette query "trash" | L | Create row sits first, above the matching commands. Typing "trash" then Enter creates a note called "trash". The restore command is second. v2 puts Create last. | v2 order: matches, then `Create « q »` (v2.html palette JS; proto 38/W2-02) | `views/palette.rs:513-515` pushes `Item::Create`; commands are appended later at `:575-597` | H | bug (confirmed, known) |
| SET-10 | W8-04 (D), E2 split | D | Shortcut row puts the key column first (fixed 156 px, `flex_none`), then the description in `flex_1`. In the narrow pane the description collapses to one word per line, about 8 lines for "Find by name or text, create, filter by #tag". The Shortcuts section has no `max_w`; other pages cap at 640. v2 puts the description first and a right-aligned chip (min-width 72). | `.sr` flex with `.lbl` flex 1 (v2.html:303-307); `.kchip` right (v2.html:400) | `shell/help.rs:460` (key column `w(156)`), `:518` (`flex().gap_3().child(keys).child(effect)`); `shell/settings_tab.rs:476-485` (no `max_w`) vs `:496` (`max_w 640`) | H (split), M (full width) | visual (confirmed, known) |
| P-01 | 08, W2-01 | L, D | No magnifier icon in note search; the icon only appears in command mode. The placeholder starts at 18 px, v2 at 42 px. | `.pal .si` 18 px, `--faint`, always shown (v2.html:275, input padding-left 42 at :273) | `views/palette.rs:1066-1074` (`.when(self.commands_only)`) | M | visual |
| P-06 | 08, 09, 10, 12 | L, D | Selected row uses an accent tint (`selection = accent.opacity(share)`). v2 uses neutral `--hover` for `.pr.act`, `.mi.act`, `.gi.act`. | `.pr.act`, `.mi:hover,.mi.act` use `--hover` (v2.html:259, 260) | `views/palette.rs:896`; `views/editor/completion.rs:233`; `shell/menu.rs:783`; `inkuno-ui/src/theme.rs:691` | M | visual (design decision, see V3-3) |
| T-01 | 11, W3-01 | L, D | Comment button is icon-only. v2 shows the text "Comment" beside the icon. | `.fbar button` with label (proto 11 shows "Comment") | `views/editor/toolbar.rs:193` (`icon("comment.svg")` only) | M | visual |
| C-01 | 12, 12 D, W6-01 | L, D | No icons on any menu row. v2 has a 16 px `--icon` glyph per row (Open, Rename, Duplicate, Move to, Copy path, Move to Trash). | `.mi svg` 16 px, `--icon` (v2.html:257) | `shell/menu.rs:511-740` (groups hold label and `Do` only), row render `:773-800` (no icon element) | M | visual |
| TO-01 | 20 (W6-05), toast | L | Toast is bottom right and lifts over the right-panel composer. v2 is bottom centre. | `.toast` left 50%, translateX(-50%), bottom 18 (v2.html:294) | `shell/render.rs:201-211` (`bottom(54)`, `left_4 right_4`, `items_end`) | M | visual/behaviour |
| TO-02 | 20, W6-05 | L | Toast is a light floating card (`bg`, 1 px border, 12 radius) with a status icon and a close ×. v2 is a dark pill: `--toast` #1f2225 (dark: #e6e6e6), white text, 10 radius, no icon, no close. | `.toast` (v2.html:294); tokens `--toast` (:17, :36) | `shell/toasts.rs:100-108` (`status_card` = `floating`), `:113-170` | M | visual |
| SET-04 | 14 Appearance | L, D | Text-size hint reads "Also ⌘+ and ⌘−" on every platform. The English string hard-codes the Mac key; the French string says Ctrl. Linux shows the Mac label. | Platform key label (v2.html:~ text-size hint) | `shell/settings_tab.rs:611` | M | bug (confirmed, known) |
| SET-05 | W8-03 Editor (D) | D | v2 Editor page has four switches: Markdown on current line, Line numbers in code blocks, Continue lists on Enter, Check spelling. None of these strings exist in the app. The app Editor page has Font, Code font, Wide tables (segmented), Formatting bar (switch), Revision author and Assistant command. | v2 proto 14b | `shell/settings_tab.rs:636-700` | M | behaviour (content gap; confirm whether intended) |
| SET-06 | W8-03 Editor (D) | D | "Revision author" and "Assistant command" text fields are nearly invisible: no fill, border `text` at 8 % (≈ rgba 255,255,255,.08 on #1c1c1e). | No v2 field style. Nearest tokens `--chip` #262628 with `--chip-line` #353638 (v2.html:32) | `shell/settings_tab.rs:694-706` (fill none, border `text.opacity(0.08)`) | M | visual (contrast, known) |
| SET-07 | W8-02 (D) | D | Segmented control: track is `text` at 6 % (≈ #28282a), active segment is `t.bg` (#1c1c1e) with a shadow. The active chip is darker than its track and reads as a hole. v2 has the same inversion: active `--sheet` #1c1c1e on track `--seg` #2a2a2c (v2.html:33, 310), so the target is itself low contrast. | `.seg .on` (v2.html:310) | `shell/settings_tab.rs:139` (track), `:156` (active) | M | visual (contrast, known) |
| SET-13 | 14 Files (code only) | L | Scratch folder and Trash rows have no action button (empty control `div()`). v2 has "Change…" for Scratch and "Open Trash". "Vault folder" is labelled "Notes folder" in v2. | v2 proto 14c | `shell/settings_tab.rs:806-813` | M | behaviour (not captured, see caveats) |
| P-04 | W2-01 (empty palette) | L | Each row shows the folder tag beside the title ("beta Projects") and a right-aligned age ("today"). v2 right-aligns the folder tag only. | `.pr .t` with kbd right (v2.html:280-283) | `views/palette.rs` note row, age column `:957` (w 56, text right) | L | visual |
| P-02 | 08 | L | Input is 52 px tall, v2 46 px. Padding 18 px, v2 0 16 px (42 px left for the icon). | `.pal input` height 46 (v2.html:273) | `views/palette.rs:1054-1055` | L | visual |
| P-03 | W2-01 | L | No "Recent" label above the default list. v2 shows `.mlabel` "Recent". | `.mlabel` (v2.html:262, used at :1131) | `views/palette.rs` (no heading in code) | L | visual |
| P-05 | 08 | L | Placeholder copy: "Search or create a note, #tag…". v2: "Search notes, or type > for commands". | Copy | `views/palette.rs:1090-1094` | L | copy |
| P-07 | 08 | L | No scrim behind the palette. v2 draws `.scrim` (rgba(0,0,0,.12) light, .45 dark). | `.scrim` (v2.html:286; created at :1107) | none in `views/palette.rs` or `shell/palette_open.rs` | L | visual |
| P-08 | 08 | L | Panel top 72 px. v2 64 px. | `.pal top:64px` (v2.html:272) | `views/palette.rs:282` | L | visual |
| P-10 | 09 | L | In command mode the typed ">" is stripped and the field shows "Search a command…". v2 keeps ">" in the field so the mode is visible. | Field keeps `>` (proto 09) | `views/palette.rs:454-456` (strip), `:1090` (placeholder) | L | visual/behaviour |
| P-12 | 08, 09 | L | Footer top border uses `code_bg`. v2 uses `--line`. Footer chips match. | `.pal .ft` border `--line` (v2.html:284) | `views/palette.rs:999-1000` | L | visual |
| S-01 | 10 | L | No "/" query line at the top of the slash menu. v2 shows it in 12 px mono faint. | `.slash .q0` (v2.html:269) | `views/editor/completion.rs:162-200` (no header line) | L | visual |
| S-02 | 10 | L | "More" list differs. App: Panel: note, Panel: warning, Picture, Formula, Mermaid diagram, Link to note, Link to date. v2: Note, Warning, Image, Link, plus Table, Divider. v2 has no Formula or Mermaid. | v2 proto 10, 34, 35 | `views/editor/completion.rs` (More list, ~75-160) | L | copy/content |
| T-02 | 11 | L | Toolbar radius is 12 (the popover sheet token). v2 `.fbar` is 10. | `.fbar` border-radius 10 (v2.html:288) | `views/editor/toolbar.rs:173-180` (`widgets::popover`) | L | visual |
| C-02 | 12 | L | Separators run the full menu width. v2 insets them 10 px each side. | `.msep` margin 5px 10px (v2.html:263) | `shell/menu.rs:815-822` (`my(4)`, no `mx`) | L | visual |
| C-03 | 12 | L | Menu min width 230 px. v2 200 px. | `.pop` min-width 200 (v2.html:255) | `shell/menu.rs:868` | L | visual |
| C-04 | 12, W6-01 | L | Items differ. App: "Move to the trash" (Delete), New note here, New folder, Copy the [[link]], Copy relative path, Show in the file manager, Icon…. v2: Open in new tab (Ctrl click), Move to…, Move to Trash. | v2 proto 12 | `shell/menu.rs:560-600`, `:733` (trash label) | L | copy/content |
| C-06 | 13 (app only) | L | Note-text menu: Cut, Copy, Paste; Bold, Italic, Underline, Strikethrough, Highlight, Code, Make a link, Comment; Bulleted, Numbered and Task list; Plain text. Chords shown at right. No icons, same as C-01. v2 has no note context menu. | n/a | `shell/menu.rs` groups 2-4 (~580-720) | L | app only (record) |
| TO-03 | 20 | L | Undo is accent colour, regular weight, px 8, radius 6. v2 is weight 600 in the toast ink. | `.toast button` weight 600 (v2.html:295) | `shell/toasts.rs:147-158` | L | visual |
| TO-04 | 20 | L | Toast max width 440 px (v2 calc(100%-32)); bottom 54 px (v2 18). | `.toast` (v2.html:294) | `shell/toasts.rs:143`; `shell/render.rs:204` | L | visual |
| TO-05 | W6-05 | L | Trash toast copy "Moved to the trash: « ideas.md »". v2: "Moved “Roadmap” to Trash". | Copy | `shell/files.rs:151` | L | copy |
| TO-06 | 20 D | D | Dark app toast not captured (no `app/dark/20-toast.png`). v2 dark is a light pill. | v2.html:36 | `shell/toasts.rs` | - | not captured |
| SET-01 | 14 | L | Settings nav is 180 px wide with no icons. v2 is 160 px with 16 px icons. | `.snav` width 160 (v2.html:395) | `shell/settings_tab.rs:448` (w 180), `:455-475` (no icon) | L | visual |
| SET-02 | 14 | L | Selected nav row uses `t.hover`. v2 uses `--selected`, which is stronger. | `.snav .row.sel` `--selected` (v2.html:397) | `shell/settings_tab.rs:465` | L | visual |
| SET-03 | 14 Appearance | L | Appearance is one card. v2 has a second card under a "More themes" group heading with a Theme list row. The app puts a "More themes…" button in the Theme row. "Motion" is "Animations". Page width has four options (Narrow, Normal, Wide, Full width); v2 has three. | `.grp h2` (v2.html:306); proto 14a | `shell/settings_tab.rs:527-600` | L | content |
| SET-08 | 14 light | L | Switch off-track is `text` at 22 % (≈ #c7c8c9 light) versus v2 `--seg` #ebeced. The app's off state is more visible. Size 34×20 and knob 16 match. | `.sw` (v2.html:311-313) | `shell/settings_tab.rs:177` | L | visual (app better; see V3-7) |
| SET-11 | 14c (D) | D | Keycaps are 18 px high, no border, chip fill (`keycaps`). v2 `.kchip` is 24 px high with a 1 px border and min-width 72. | `.kchip` (v2.html:400) | `inkuno-ui/src/widgets.rs:66-90` | L | visual |
| SET-12 | 14c, awaiting key | L | "Press the keys…" in accent mono text, no chip border. v2 is "Press keys…" in accent chip border. | `.kchip.listen` (v2.html:402); proto 39 | `shell/help.rs` edit display (~line 418-420) | L | copy/visual |

Counts (table above): H 2 (P-09, SET-10), M 11 (P-01, P-06, T-01, C-01, TO-01, TO-02, SET-04, SET-05, SET-06, SET-07, SET-13), L 24.

## Confirmed matches (no action)

- Palette width 560 px, radius 12 (sheet token), popover shadow (`widgets.rs:146-156`). Rows 38 px with 8 px radius (`palette.rs:25`, `:896`).
- Menu rows: 32 px, 5 px side margin, 8 px padding, 6 px radius (`widgets.rs:160-172`). Keycaps right-aligned with a 16 px gap (`menu.rs:797`).
- Danger colour on "Move to the trash" and delete items uses the caution red (`menu.rs:742-758`). v2 `.mi.danger` is `--cau` (v2.html:260).
- Slash menu: width 260, 4-column grid, 38 px cells, 32 px rows, 18 px icons, Basic and More headings (`completion.rs:173-200`, `:233`).
- Settings: card radius 10 (`settings_tab.rs:109-125`), row min 52, padding 14 / 12, 12 px soft notes (`:74-80`). Switch 34×20 with 16 px knob (`:166-190`).
- Selection toolbar: 28 px buttons, 6 px radius, B / I / S / code / link order, Text dropdown (`toolbar.rs:114-156`).
- Palette footer hints use keycap chips (`palette.rs:966-1007`).
- Mid-word search snippets highlight the match in accent (`palette.rs:920`; v2 `.pr mark`). W2-02 and 08 show it.
- Fuzzy-matched palette note with snippet renders under the title (08 shows "Mixed: a week of project notes").

## v3 candidates (where the app is better, or v2 looks wrong)

- V3-1 Dark segmented control: v2 makes the active chip darker than its track in dark mode (v2.html:33, 310). Make the active segment lighter than the track (active `--pop` #2a2a2c on a darker track), in v2 and the app (SET-07).
- V3-2 Text fields: neither side has a dark-mode field style with enough edge. Give fields a fill (`--chip`/`--seg`) and a border at least 3:1 against the sheet (SET-06).
- V3-3 Selection tint: app's accent-tinted selection (P-06, S-02 grid, C-01 menus) is easier to read on the keyboard path than v2's neutral hover. Consider adopting it in v2 for `.pr.act`, `.mi.act`, `.gi.act`, or keep neutral and make the app match. Pick one.
- V3-4 Command mode grouping: the app's "This block" and "Formatting" headers with contextual commands (09) are better than v2's flat list. v2 already has the `.mlabel` style and could use it here.
- V3-5 Palette rows: the app's age column ("today") gives recency context. Consider adding it to v2 rows.
- V3-6 Keep the ">" in the palette field (v2) so command mode is visible. The app strips it (P-10).
- V3-7 Switch off-state: the app's off track (22 % ink) is more visible than v2's #ebeced on white (SET-08). Consider v2 `--seg` darker, or adopt the app's value.
- V3-8 Toast placement: v2's bottom-centre is clear of the right panel. The app's bottom-right toast covers the assistant composer (W6-05). Anchor toasts to the note sheet, not the window, or keep v2 centre.
- V3-9 Toast style: the app's light card with a status icon and a close × is quieter and works in dark mode without inversion. v2's dark pill inverts in dark theme (toast ink #1c1c1e on #e6e6e6). Decide which to keep.
- V3-10 Shortcut row: v2's description-left, chip-right layout (v2.html:303-307) is right. The app should adopt it and cap the Shortcuts page at 640 px like the other pages (SET-10).
- V3-11 Editor switches: v2 documents four switches the app lacks (SET-05). Decide whether "Continue lists on Enter" and "Markdown on current line" are wanted. The app has the behaviour in other forms, so the switches may be missing rather than unneeded.
- V3-12 Shortcut shortcut for the palette: v2 binds Ctrl+K and the sidebar says "Search Ctrl K". The app binds Ctrl+P (sidebar shows "Search Ctrl+P", as in `app/light/20-toast.png`), and docs/design/app-ui.md lists F10 / Ctrl+Shift+P. Pick one and align the prototype and the app.
- V3-13 Slash menu header: v2's "/" query line (v2.html:269) shows what was typed. The app has no such line (S-01). Consider adding it.
