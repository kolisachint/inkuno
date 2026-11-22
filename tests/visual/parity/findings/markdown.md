# Markdown rendering audit (app vs v2)

Sources
- App shot: `/home/user/inkuno/target/visual/20261122-102724/week-plan.png` (1400x1000; the note window is 1300x940 at x 50, y 20; sidebar hidden, no right panel). This is the only app shot of the Week plan note.
- Prototype: `$P/proto/02-sidebar-hidden-light.png` (1268x868; note sheet x 0-982, panel open), `$P/proto/21-line-revealed-light.png`, `$P/proto/01-default-dark.png`.
- Design truth: `docs/design/prototypes/v2.html` CSS lines 149-230, 427-428. `docs/design/app-ui.md` §22 (Quiet block styles, line 120) and line 306 (MAX_WIDTH 720 to 800).
- Scale: both shots are 1:1 CSS px (same body glyphs: "Quiet theme" starts at the same x offset; H2 "Tasks" is 58 px wide in both). No rescale was needed. The note column in the app starts at x=300 (window 50 + (1300-800)/2), in the prototype at x=134.
- Not comparable, read first:
  - `app/light/01-default.png`, `app/light/21-line-revealed.png` and `app/dark/01-default.png` show the **Mixed** fixture ("Mixed: a week of project notes"), not Week plan. Only the target shot is the Week plan. Light 01 and 21 were used only to check rules the Mixed note shows (alert, table, H1 marker, reveal). Dark was compared at token level only.
  - The target window ends at y=959, so the quote, dotted rule and ordered list (below the fold) could not be measured in the app. Marked "not checked".

## Matches (measured, no action)

| item | app | v2 |
|---|---|---|
| Body text 15/22 | paragraph line pitch 22 | 15/22 |
| Code line pitch | 20 | 20 |
| List row pitch | 31 (22 line + 9 block gap) | 22 + 9 |
| Table cell text inset | 14 px from the frame (1 border + 12 pad + 1) | 9 12 pad |
| Table row pitch | 39 | 9+9+20+1 = 39 |
| Table font | 14 px ("Theme" 44 px wide in both) | 14 |
| Task box size / checked fill | 17 px, #3a85f7 | 17, --accent |
| Alert fill | 7 % of tint, #f0f4fc (v2 #f0f5fc) | 7 % |
| Link colour | #105592 | --link #105592 |
| Code bg / gutter / border | #fbfbfb / #f8f8f8 / #eaebeb | same |
| Table header bg, rounded frame, right-aligned State column | yes | yes |
| H1 raw `#` on the caret line | shown (caret is on H1) | `.reveal .live .mk` |

## Findings

| id | state/workflow | theme | difference (app vs v2) | v2 target | app location | sev | kind |
|---|---|---|---|---|---|---|---|
| D01 | Week plan, all H2s (Tasks, Code, Status) | light | Space above an H2 is much larger. Alert bottom to "Tasks" cap top = 55 px; "Settings page" to "Code" cap top about 69 px. App H2 pad is 24 + block gap 9 = 33 px. Below the H2 it is tighter: "Code" baseline to code box top = 10 px (v2 about 15); "Status" baseline to table top = 7 px (v2 about 16). | `.b.k-h{padding-top:22px}` (v2.html:165), H2 cap top about 28 px below the previous block | layout.rs:1879-1883 (heading pads 24 / 24 / 16 / 16), layout.rs:1900-1904 (`pad + block_gap`) | H | visual |
| D02 | Checkbox click on any task, with the hover handle | light | A click on the drawn box opens the block handle (drag or menu) and does not toggle the task. Full cause in the section below. | Click on the box flips `[ ]` / `[x]` (the v2 box is a plain button, v2.html:183-184) | blocks.rs:235-262; mouse.rs:484-497 (grip test before the box test); mouse.rs:568-576; layout.rs:304-322; layout.rs:2273-2293 | H | bug |
| D03 | Week plan, Note callout | light | Callout is 74 px tall (v2 66). Label-to-body gap is 30 px (v2 22). The fill starts above the first line by the `BLOCK_ROOM` room (9 px per mod.rs:203-204; the exact top offset is set in `shape_span`, not traced). | `.alert{padding:10px 14px}` (v2.html:191), `.al` 14px/semibold | paint.rs:294-316; mod.rs:204 (BLOCK_ROOM 9) | M | visual |
| D04 | Week plan, callout border | light | The 22 % border is drawn over the already tinted fill, so it reads darker: #c4d6f3 in app, #d1dff6 in v2. | 22 % of tint over the sheet (v2.html:191) | paint.rs:308-312 (`tint.opacity(0.22)` on the quad that has the 7 % fill) | L | visual |
| D05 | Week plan, callout, code block, rules | light | Callouts and code boxes extend 12 px beyond the text column on both sides (box 288-1111, text 300-1099). v2 boxes are flush with the text (134-853). | `.alert`, `.code` flush (v2.html:191, 196) | paint.rs:246, 288, 308 (`block(px(-12.), y, w + px(24.), h)`) | L | visual |
| D06 | Week plan, Tasks and nested tasks | light | List indent is 28.1 px per level (1.875 em gutter). v2 is 24 px. Measured: nested box at +28 from the top-level box (332 vs 304). | `.li{padding-left: var(--d)*24px}` (v2.html:174) | layout.rs:2226 (`gutter = em * 1.875`), layout.rs:2273 (`text_at`) | M | visual |
| D07 | Week plan, task boxes | light | Task box sits 3.6 px right of the text column (box 303.6-320.6, text col 300). v2 box is flush (x=134). The text is still 24 px from the box. | `.k-task .lead{padding-left:0}` (v2.html:187) | layout.rs:2275 (`right = text_at - gap`), 2293 (`from = right - side`) | L | visual |
| D08 | Week plan, task boxes | light | Box top is 1 px above the cap top and its centre is about 1.5 px below the text centre (box 365-381, text cap 366). | `.box` centred on the line (v2.html:183) | paint.rs:669-676 (non-Quiet mid is `lh/2 + 0.1em`; Quiet uses `lh/2`) | L | visual |
| D09 | Week plan, Code block | light | Code box is 152 px tall (v2 142). First code line is 28 px below the top border (v2 34). Last line to bottom border is 31 px (v2 15). Gutter is 27 px wide (v2 30). Code text starts 43 px from the box edge (v2 48). | `.nums{padding:4px 10px 10px 12px}` (v2.html:201), `.src{padding:4px 14px 10px}` (v2.html:204) | layout.rs:1359-1365 (gutter `20 + digit * n`), paint.rs:246-263 | M (height, bottom); L (gutter, offset) | visual |
| D10 | Week plan, every code block | light | Copy icon is always visible at the opening line (target at x 1082-1093, y about 566). Only the hover pill is gated. | `.code .copy{opacity:0}` until hover (v2.html:198-199) | paint.rs:839-848 (`if row.opens` paints the icon; `hover` only affects the bg) | M | visual |
| D11 | Week plan, Table | light | Table is sized to content: 302 px wide (block is 824). v2 stretches to the block. | `table{min-width:100%}` (v2.html:210) | grid.rs:994-998 (widths from content, capped at width/3) | M | visual |
| D12 | Week plan, commented text ("Tasks", "Settings page") | light | Comment passage is an accent-blue 10 % wash (#e6eefa in target). No amber underline, no margin dot. Dots only sit on the block handle. | `.cm{background:--cmw; border-bottom:1.5px --cmd}` (v2.html:427), `.cdot` right margin (v2.html:428) | widgets.rs:67-79 (`pill_tint` = accent 10 %), paint.rs:586-600 (wash), paint.rs:1090-1100 (handle dot) | M | visual (design intent to confirm) |
| D13 | Any note with an H1, not on the caret line (Mixed, light 01 and 21) | light | Raw `# ` on H1 stays visible while the caret is on another line. H2 "Summary" does not show `##`. v2 hides `.mk` unless the line is live. | `.reveal .live .mk{display:inline}` only (v2.html:155-156) | heading marker drawing not traced (layout.rs marker code near 1990-2030); verify the H1 `revealed` state | M | bug (unconfirmed) |
| D14 | Mixed, light 21 (line clicked) | light | The caret paragraph ("This note mixes...") does not show its inline markers (`**`, `~~`, backticks), so bold and strike render at once. v2 reveals the live line's markers. Prototype 21 shows its raw `- [ ]`. | `.reveal .live .mk` shows markers on the live line (v2.html:155-156) | mouse.rs:579-581 (`revealed = Some(lr.start)`); layout.rs live-row markers | M | behaviour (verify) |
| D15 | Week plan, callout | light | Duplicate of D05 (the icon sits at 303-304 px because the box starts at 288). | see D05 | see D05 | dup | visual |
| D16 | Week plan, comments | light | Duplicate of D12 (no amber underline; the blue wash reads like a selection). | see D12 | see D12 | dup | visual |
| D17 | Week plan, page width | light | Measure is 800 px (text 300-1099) vs v2 720 px. | `--measure:720px` (v2.html:97) | mod.rs:193 `MAX_WIDTH = 800`; app-ui.md:306 records 720 to 800 as a deliberate change | L | visual (decision made) |
| D18 | Week plan, title band | light | Chrome differs (window title only, no tab strip, H1 cap top 63 vs proto 68). Not a markdown item. | v2 tab band (v2.html `.head`, `.tabs`) | out of scope for this audit | L | visual (out of area) |
| D19 | Dark theme, Mixed note (`app/dark/01-default.png`) | dark | Tokens match v2 dark: sheet #1c1c1e, ground #141517, callout fill #22262e (7 % of --n), table header #232325. Layout and spacing could not be compared, because the shot is not Week plan. | v2.html:27-41 (dark tokens) | theme.rs (Quiet dark) | n/a | not comparable |
| D20 | Quote, dotted rule, ordered list, inline code chip | light | Not in the target shot (below the fold). Not checked. Proto values: quote bar 3 px #c9cacb; rule dots 2 px high. | v2.html:189, 216 | not checked | n/a | not checked |

## Checkbox click: cause (D02)

The click does not reach the checkbox code, because the block handle's grip sits exactly over the box. Four steps:

1. The task's lead has `len: 0` and no text (the box is painted). `pos(i)` only uses the lead when `i < lead.len` (layout.rs:308). For `i = 0` it falls to the `lead =>` arm (layout.rs:313-322) and returns the text start: `lead.width`. For a top-level task that is 28.1 px right of the column (`gutter = 1.875 em`, layout.rs:2226, text_at at 2273). It is not the line's left edge.
2. `block_left` uses `pos(indent)`, so the block's left edge becomes the text start (blocks.rs:235-246). `handle_bounds` then puts the grip at `block_left - 24`, 18 px wide (blocks.rs:248-262, grip at 256). For the target: grip x 304.1-322.1. The box is at 303.6-320.6 (`from = right - side`, layout.rs:2293). Nearly identical.
3. Pointer hover sets the handle (`aim_block`, blocks.rs:275-290, and it keeps the handle while the pointer is on it). The grip is painted over the box (paint.rs:1146-1160) with an open-hand cursor.
4. `mouse_down` tests the grip before the checkbox (mouse.rs:484-497 `grip.contains` returns early, then mouse.rs:568-576 `checkbox_at`). So the click starts a grip drag or the block menu, and `flip_box` never runs.

The existing test `a_drawn_box_checks_its_task` (layout.rs:3866) passes because it clicks without a prior hover (`aim_block` never runs), so no grip exists.

Same mechanism for bullets and ordered numbers (also `len: 0`, layout.rs:2304): the grip covers the bullet or number when hovered. Clicking them opens the menu rather than doing nothing. Not verified in the running app.

Fix direction (not applied, READ ONLY): make `block_left` use the lead's start for lead rows (`origin.x` for indent 0), or give the grip a fixed left margin that clears `lead.width`. Also make the grip test skip when `checkbox_at` hits.

## v3 candidates (app better than v2, or v2 looks wrong)

1. Keep the app's 800 px measure; update v2 `--measure` to 800 (D17). The page reads better at this width.
2. Keep natural-width tables (D11). Short tables look better unstretched; update v2 `table` to `width:auto`.
3. Headings: app's larger space above H2 (D01) makes sections read as breaks; v2's 22 px looks cramped against the 9 px body gap. Adopt the app's values after a visual review, and fix the space below (app is too tight there).
4. Callout: the 12 px outdent (D05) lines the box edge with the code-block edge. It is a small improvement; adopt it in v2.
5. Comment passage: v2's amber underline is clearer than the app's blue wash, which collides with the link and selection colours (D12). Keep v2's amber.
6. Copy icon always visible (D10): useful for discoverability; decide and align both.
7. Task box inset of 3.6 px (D07) and centring (D08) are minor. v2's flush box is cleaner; keep v2.

## Counts

- H: 2 (D01, D02)
- M: 8 (D03, D06, D09, D10, D11, D12, D13, D14)
- L: 5 (D04, D05, D07, D08, D17)
- Duplicates: D15 (of D05), D16 (of D12). Out of area: D18.
- Not checked or not comparable: D19, D20

Note on tooling: `/an.py` (empty file) was created by a mistaken command during this audit. A removal of it was blocked by the safety check, so it remains. It is safe to delete.
