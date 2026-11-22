# Window chrome audit: app vs v2 prototype

Scope: states 01-default, 02-sidebar-hidden, 07-panel-closed, 15-focus-mode, 16-split, 17-titlebar-hidden, 18-tabs-multiple, 19-empty-pane, 23-scratch-section; workflows W2, W4, W7.
Target: `docs/design/prototypes/v2.html` CSS (top of file) and decisions in `docs/design/prototypes/README.md` (rows 3, 4, 5, 8).
Read-only: nothing in the repo was edited.

Coverage notes:
- Light theme checked for all states. Dark checked for 01 (band) only; app dark shots exist for 01-16 and not for 17, 18, 19, 23.
- The app capture harness drew server-side decorations (no window buttons, no title-bar pill). That hides the right-panel toggle in the shots; see C02 for the code gate and what it means on each platform.
- Fixture differences, not chrome bugs: the app seed vault has no scratch notes (01 shows "No scratch note") and no notes older than a month (19 shows no "From a month ago" row). Items that depend on this are marked (fixture).

## Findings

| id | state/workflow | theme | difference (app vs v2) | v2 target (CSS/px/token) | app location | sev | kind |
|--|--|--|--|--|--|--|--|
| C01 | 01, 07, 19; W1 | light, dark | Known. One note has no tab strip; the band shows the vault name centred instead. v2 always shows the strip, even for one tab (README #3). | `.band` always rendered; `.tab` 28px in 38px band (v2 L136, L348) | `shell/tabs.rs:226` (`render_tabs` returns None when `tabs.len() < 2`); `shell/render.rs:1138` (`strip = band_shown && !barred`) | H | visual |
| C02 | 01, 07, 17; W1 | light | Known. Right-panel toggle is absent from the band. Code draws it only when `title_bar && band_shown()`, and `title_bar` needs `client` decorations (`render.rs:92`, `:102`). So it is missing on macOS/Windows (server decorations), when the title bar is hidden (17), and in this capture harness. On a client-decorated Linux window it should draw. Also, when drawn it is 28px (not v2's 30px) and sits 2px low: top = (38-24)/2 = 7 for a 28px button (centre at 21 vs band centre 19). | `.rsw`/band `.ib` 30px, radius 8, `.on` = selected (v2 L100-L119, L1 `.ib`); README #8 "the band button toggles the panel" | `shell/sidebar.rs:2090-2112` (top at `:2097`); `shell/render.rs:494-495` (gate); `crates/inkuno-ui/src/widgets.rs:33` (28px box) | H | visual / behaviour |
| C03 | 01 (and 07, identical in app) | light, dark | App's stock state has the right panel closed (`nav.rs` stock `right_shown` empty; asserted in `nav.rs:865`). v2 opens with the Outline panel shown. So the app's 01 and 07 are the same picture. | v2 `.rpanel` 280px open in 01 | `views/nav.rs:556-578`, `:865` | M | behaviour (decide) |
| C04 | 01, 18, 19 (band) | light, dark | Band over the note is drawn in the ground colour (grey, same as the sidebar) in Quiet. v2 band is the sheet colour (white / dark sheet). Visible as a grey top bar over a white sheet. The code comment says this is deliberate ("Quiet: the band is the ground"). | `.band{background:var(--sheet)}` (v2 L348); `--ground` vs `--sheet` tokens | `shell/tabs.rs:449` (`when(quiet \|\| peek, bg(ground))`) | M | visual (decide) |
| C05 | 18 | light | Tab inner gap is 4px; v2 is 6px (`.tab{gap:6px}`). The shown tab's cross is always drawn at full `soft` colour; v2 shows it at 0.7 opacity on the shown tab and on hover only otherwise. Pill height 28, radius 8, 13px, pad 10/6, max 200 all match v2. | `.tab{gap:6px}` (v2 L136); `.tab.on .x{opacity:.7}` (v2 L141) | `shell/tabs.rs:335` (gap 4); `:296` (cross on `over \|\| (quiet && on)`) | L | visual |
| C06 | 18; W2-04 | light | Single click in the tree opens a normal (solid) tab. v2 makes a single click a preview (italic, replaced by the next single click), and double-click or editing pins it (README #3). The app keeps previews for arrow keys only (keymap says so). Italic styling itself matches v2. | `.tab.prev .tt{font-style:italic}` (v2 L361); README #3 "replaced by the next single click" | `shell/notes.rs:249` (`preview_note`, reached from `sidebar.rs:479` on arrows); `shell/keymap.rs:90` (help text: click opens a tab of its own) | M | behaviour (decide) |
| C07 | 23, 18; W6-06 | light | Known. A long tab title is truncated with an ellipsis after the " · Scratch" suffix is appended, so the scratch marker disappears (23: "Quick idea: try the outline ×", no marker). The preview dot and the folder suffix are cut the same way. Fix: truncate the title part, keep the suffix. | v2 has no scratch marker; app-ui intends one | `shell/tabs.rs:82-90` (`caption` appends the marker); `:346-353` (label truncated at `max_w(200)`) | M | bug |
| C08 | 18 | light | No tab tooltip. v2 sets `title=` to the tab's path (`~/Documents/Notes/...`). README #4 says the folder shows as the tab tooltip and in the window title. App has no tooltip on tabs at all, so the folder is only in the window title. | `tabHTML` `title="${tabPath(t)}"` (v2 L689-L690, L733) | `shell/tabs.rs` `render_tabs` cells (~L320-L330, no `.tooltip(`) | M | behaviour |
| C09 | 16; W4-03 | light, dark | No visible focus cue between two panes in Quiet. The active-pane ring is drawn only when `!t.quiet`. Quiet's dim of the unfocused pane's tabs (`tab_cue`, 0.55) only applies inside a tab strip, and a single-tab pane has none. So in the Quiet split both panes look alike. v2: focused pane's active tab solid, other pane's tabs dimmed (README #5, `.ptabs.dim{opacity:.55}` v2 L359). | `.ptabs.dim{opacity:.55}` (v2 L359); README #5 | `shell/render.rs:1379` (ring gated `!t.quiet`); `shell/tabs.rs:100-105` (`tab_cue`) | H | visual / behaviour |
| C10 | 16 | light | Panes: app is 1px divider in `t.border` between panes, flat, no corners, no shadow. v2 is 1px `--line` (`.pane+.pane{border-left}`, v2 L371). Same geometry; colour token differs slightly (`border` vs rgba .08) — not measured. | `.pane+.pane{border-left:1px solid var(--line)}` (v2 L371) | `shell/render.rs:1426-1428` | L | visual (verify colour) |
| C11 | 16; W4-03 | light | Each pane shows a different centred label. A pane with the sidebar hidden (leftmost) shows its note's title; the corner pane shows the vault name ("Notes"). Same band, two meanings, both centred. v2 shows the note name only as the tab. | README #4 (no breadcrumb; folder in window title / tooltip) | `shell/render.rs:1316-1330` (vault name, `window-title`, corner pane only); `:1317-1322` (note title, `strip-title`, at-left pane) | M | visual |
| C12 | 17, 02, 07, 01; W7 | light | Known. Window title is `Note — Vault — Inkuno` (Zed-style), and the band's centred label is the vault name. v2 has no window title (README #4 puts the folder in the window title and tooltip). | README #4 | `shell/render.rs:60-72` (window title); `:1326` (band `window-title`) | M | visual / decide |
| C13 | 17; W7-02 | light | With the title bar hidden, the sidebar toggle leaves the sidebar's top row and moves to the sidebar foot next to Settings (v2 keeps it in the top row beside the lights). The top row becomes 12px, so Search moves up. Verified in crop: foot shows toggle (accent), Settings, title-bar button. Lights are absent in the capture. | `.side .top{height:38px}` with lights + toggle (v2 L342); `.app.bandhide .strip{display:block}` (v2 L365) | `shell/sidebar.rs:1403` (12px row); `:1650` (toggle in foot when band hidden) | M | visual / behaviour |
| C14 | 17 (peek); W7-03 | light | Peek overlay works (band slides over note, shadow, strip hover). But for a single note the peeked band centres the note's title, while the shown band (C12) centres the vault name. Two labels for one band. | `.app.bandhide .band.peek` (v2 L366-L367) | `shell/tabs.rs:550-575` (`render_peek`, one-note branch uses `tab_name`) vs `render.rs:1326` | L | bug |
| C15 | 17 with tabs (W7-02, code only) | light | With several tabs and the title bar hidden, the app keeps a 28px tab row under the 12px strip (always visible). v2 hides the whole band (tabs included) off-screen until the peek; no tabs are visible in hidden mode. Not captured as a screenshot (only single-note 17 exists). | `.app.bandhide .band{position:absolute; transform:translateY(-100%)}` (v2 L366) | `shell/tabs.rs:439-445` (`BAND_HIDDEN + TAB_ROW_HIDDEN`), `:445` (`pt(BAND_HIDDEN)`) | M | behaviour (verify visually) |
| C16 | 15 | light | Focus mode matches v2: no sidebar, band, strip or panel; note alone on the sheet, centred; ground takes the sheet colour in Quiet. Differences only in fixture content. | `.app.focus .side,.band,.strip,.rpanel{display:none}` (v2 L374) | `shell/render.rs:230` (ground), `shell/tabs.rs:226` (no bar in focus) | - | matches |
| C17 | 23 | light | The Scratch section header and rows match v2 (30px header, 12px semibold soft, chevron; rows 30px). Empty state text "No scratch note: Ctrl+N makes one" wraps to two lines in the 240px sidebar; v2 has no empty text. | `.sect` 30px, 12px/600 (v2 L118); `.row` 30px (v2 L110) | `shell/sidebar.rs` `render_scratch` (empty message) | L | visual |
| C18 | 23, 01 | light | Sidebar footer is 38px (`h(BAND)`); v2 footer is 6px top pad + 30px row = 36px. Footer holds Settings with `Ctrl ,` and the title-bar button, as in v2. | `.foot{padding-top:6px}` + `.row{height:30px}` (v2 L122, L110) | `shell/sidebar.rs:1644` | L | visual |
| C19 | 01, 02, 16 (note edge) | light, dark | No sheet edge shadow. v2 puts `--shadow-note` (-1px 1px 0 .035, -2px 0 10px .045; dark .3/.35) on the sheet's left edge. App panes are flat with no shadow, and the sidebar's divider comment ("the note's shadow is the edge", `sidebar.rs:1489`) assumes a shadow that is not drawn. The sidebar/note boundary is a bare colour step. | `--shadow-note` on `.sheet` / `.main` (v2 L125, L347) | `shell/render.rs:1263` (pane "no shadow"); `shell/sidebar.rs:1489` (comment) | M | visual |
| C20 | 01, 17, 02 | light | Band toggles: sidebar toggle and right-panel toggle are 28px, radius 6 (`small()`). v2 `.ib` is 30px, radius 8. Icons match. | `.ib{width:30px;height:30px;border-radius:8px}` (v2 `.ib` rule, L106) | `crates/inkuno-ui/src/widgets.rs:33`; `crates/inkuno-ui/src/theme.rs:718` (`small()`=6) | L | visual |
| C21 | 19 | light | Empty pane is a live, empty editor with a "Write here…" placeholder (caret visible), plus the cards Today / New note / Search in a 2+1 grid. v2 shows only the cards (no editor), plus a "From a month ago" row with two note cards (fixture: app shows none, see coverage note). Card width ~200px vs v2 190px; min-height and 12px radius not measured. | `.empty`, `.ecards`, `.ecard{width:190px;min-height:84px;radius 12}`, `.emlab` (v2 L403-L409, L414) | `shell/render.rs:755` (`render_cards`), `:938` (month-ago row), `views/editor/paint.rs:885` (placeholder) | M | visual / behaviour (decide) |
| C22 | 19 | light | Shortcut text in the empty cards wraps mid-token: "Ctrl+Alt" / "+N", and the search card wraps "Ctrl+P" text. v2 keeps each card's sub-copy to two clean lines ("open the daily note", "run a command (Ctrl K)"). | `.ecard span` 2-line clamp (v2 L412) | `shell/render.rs:755+` (card lines) | L | visual |
| C23 | 18 (cross-pane) | light | App draws a dirty dot (accent 6px) on unsaved tabs, which turns into the cross on hover. v2 has no unsaved state. | none in v2 | `shell/tabs.rs` (`slot` match on `dirty`, ~L300-L318) | - | v3 candidate (app better) |

## Matches (no action)

- Title band height 38px (v2 L348 `.band{height:38px}`; app `theme.rs:11` `BAND = 38`).
- Tab pill: 28px high, radius 8, 13px, padding 10/6, max width 200, shown tab on a 7.2% wash (`tabs.rs:330-335`, `:328`, `theme.rs` quiet wash). Close cross box 18px, icon 12px.
- Preview tab italic in Quiet (`tabs.rs:352`, `tab_italic`; v2 L361).
- Sidebar width 240px in Quiet (`sidebar.rs:380`, `default_list_w`); rows 30px (`sidebar.rs:42`, v2 L110).
- Section header: 30px, 12px semibold, soft, pad 8/4 (`sidebar.rs:1693-1703` vs v2 L118).
- Hidden title-bar strip 12px (`theme.rs:14` `BAND_HIDDEN`; v2 L345 `.strip`). Peek slides the band over the note with a floating shadow (`tabs.rs:598`).
- Right panel width 280px (`views/nav.rs:60` `PANEL_W`; v2 `.rpanel{width:280px}`).
- Focus mode (F9) hides sidebar, band, strip and panel (`render.rs:102-106`; v2 L374).
- Window controls: the app's `Ctrl+Shift+S` and `F9` workflows (W7-01..05) behave as v2 (W7 log: all OK).

## v3 candidates (app better, or v2 looks wrong)

1. Reopen closed tab (Ctrl+Shift+T): app has it (W2-07 OK), v2 has none (W2-07 MISSING in the prototype). Keep in v3.
2. Dirty dot on unsaved tabs (C23): app has it, v2 does not. Keep.
3. Hidden title band with several tabs keeps a tab row under the 12px strip (C15). v2 hides every tab until peek. The app is more discoverable; worth a v3 decision.
4. Empty pane as a writable page with a "Write here…" placeholder (C21). Typing starts a note; v2 has only cards. Decide whether v3 adopts it.
5. Stock right panel: app starts closed (C03), v2 starts open on Outline. The app's quieter default may be the better one; v2 may look busy at 1300px.
6. Window title `Note — Vault — Inkuno` (C12): more informative than v2 (no window title). Keep; v2 should show it.
7. Scratch empty message (C17) helps a first-time user; v2 has none. Keep, but shorten so it does not wrap.
8. Band colour (C04): the app's grey band joins the grey sidebar into one top row. v2's white band beside a grey sidebar may simply look wrong. Decide once for both themes.
9. Sidebar toggle in the foot when the title bar is hidden (C13): v2 keeps it at the top. The foot placement is reachable; v2's top-row placement is cleaner while lights exist. Pick one.
10. Sheet edge shadow (C19): v2 is right; the app's comment already assumes it. Restore it, not a v3 change.

## Known items confirmed

- No tab strip for a single note: confirmed (C01; code `tabs.rs:226`).
- Band lacks the right-panel toggle: confirmed in the capture (no toggle in 01, 07, 17) and in code (C02). It is gated on client decorations, so it is missing on macOS/Windows and with the title bar hidden. Check on a client-decorated Linux window before filing it as a bug on Linux.
- Truncated tab titles hide the "· Scratch" marker: confirmed (C07; 23 and W6-06).
- Window title shows the vault name: confirmed (C12; `render.rs:60-72`, band label `render.rs:1326`).

## Severity count

H: 3 (C01, C02, C09). M: 11 (C03, C04, C06, C07, C08, C11, C12, C13, C15, C19, C21). L: 7 (C05, C10, C14, C17, C18, C20, C22). Matches / v3-only rows: C16, C23. Total 23.
