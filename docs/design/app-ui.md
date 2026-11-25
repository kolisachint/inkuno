# UI Design Standards — noteapp

> The single unified UI doc. The user checks/updates this file; agents implement from it.
> Nothing here changes code by itself — see §1 for the flow.
> Primary visual reference: §9 (Markscribe mockup). Locked scope: §10.

## 1. Purpose + how this doc is used

This file is the source of truth for what the app should look and feel like.
The user owns it; agents read it before touching any UI code and update it
when a UI decision changes.

Flow for any UI change (the check-box flow):

- [ ] **Propose** — the agent (or user) writes the proposal *in this doc first*
      (new section or edit, marked `PROPOSED` with a date).
- [ ] **User approves** — the user edits the doc: approves, amends, or rejects.
      Only the user's edit counts as approval; agent edits never self-approve.
- [ ] **Implement** — the agent implements exactly what the approved doc says,
      citing the doc section in the change, and checks off the boxes.

Rules of the flow:

1. Doc first, code second. No UI code changes without a matching approved doc line.
2. Small steps. One proposal = one component or one interaction, never a redesign PR.
3. Every claim about *what Inkuno/the app does today* cites a `file:line`.
   Anything unverified is marked **TBD** — never invented.
4. The user may edit any section at any time, especially §6 (preferences).
5. Every editor or Markdown rendering change runs `scripts/visual-check.sh`, and its
   screenshots are looked at before the change is called done. Unit tests do not show
   how text is drawn (`docs/workflow.md`, "Visual check"; `CONTRIBUTING.md:48`).
   Owner instruction, 2026-11-21.

## 2. Product vision

A **Typora-inspired, document-first writing app**:

- **Content-first, chrome-quiet.** The note is the interface. Panels, bars, and
  badges step back; the text column is the visual center of the window.
- **Unified edit + preview.** One surface: what you type is what the document
  is (live rendering, no edit/preview mode split).
- **Source preserved.** The Markdown file on disk stays clean and portable;
  rendering never corrupts or rewrites the source.
- **Keyboard-first.** Every command reachable from the keyboard via the command
  palette; mouse is optional, never required.
- **Local-first, fast, offline.** A folder of Markdown files is the vault;
  the app opens instantly, works with no network, and every keystroke stays
  cheap (damaged-line-only layout relayout).
- **Calm aesthetics.** Quiet neutrals, one accent, small radii, no gradients,
  no big buttons, no permanent toolbars.

## 3. Design tokens + typography system

> Target system. Inkuno's current values are noted where they differ; deltas are
> decided in §6 by the user.

### Typefaces (target)

- **UI / body / headings:** Inter (variable if available, else regular/medium/
  semibold cuts).
- **Code:** JetBrains Mono.
- **Current state (Inkuno): done.** The bundled faces are **Inter + JetBrains Mono**
  (`FONT_FILES`, `crates/inkuno-ui/src/assets.rs:26`), exposed as `sans()` (`crates/inkuno-ui/src/assets.rs:39`) and
  `mono()` (`crates/inkuno-ui/src/assets.rs:43`), overridable from prefs
  (`apply_fonts`, `crates/inkuno-ui/src/assets.rs:67`). IBM Plex Sans and Plex Mono are no
  longer bundled; they stay last in the `apply_fonts` fallback for people who have them
  installed. Plex Serif ships but nothing draws with it (see `typography.md` §2, §5).
- Fallback chain must cover CJK: the app's faces have no CJK glyphs, so
  `needs_system_font` (`crates/inkuno-ui/src/assets.rs:49`) sends those characters to a system font.

### Sizes (target, px at scale 1)

| Role      | Size | Line-height | Current state |
|-----------|------|-------------|---------------|
| Body      | 15   | 22/15 (≈1.47) | Default body size is 15, `Prefs::SIZE` (`crates/inkuno-logic/src/prefs.rs`); the sizes below hold for it (`crates/inkuno/src/views/editor/layout.rs`) |
| H1        | 28   | 1.3         | Bold (`layout.rs`, heading table) |
| H2        | 21   | 1.3         | Bold |
| H3        | 18   | 1.33        | Semibold |
| H4        | 16   | 1.4         | Semibold |
| H5–H6     | 15   | 1.47        | Semibold; H6 is `soft` |
| UI chrome | 13–14| —           | UI scale follows body size: `ui = size/15` clamped 0.85–1.5 (`crates/inkuno-ui/src/theme.rs`) |
| Code      | 13   | 20/13 (≈1.54) | Code blocks and inline code (`layout.rs`) |
| Table cells | 14 | 20/14       | `crates/inkuno/src/views/editor/grid.rs` |
| Block gap | 9    | —           | Above each body block's line: paragraph, list item, quote or callout line; scales with size (`BLOCK_GAP`, `crates/inkuno/src/views/editor/mod.rs`) |
| Metadata (counts, hints) | 12 | — | Secondary text uses `soft` color token, WCAG AA 4.5:1 (`crates/inkuno-ui/src/theme.rs`) |

### Measure + spacing (target)

- **Doc max-width: 800 px** at body 15.
- **Block gap: 9 px** above each body block's line (paragraph, list item, quote or callout line); headings keep their own top pads, and code, table rows, formula lines and the front matter get none.
- **Current state (Inkuno):** page widths are Narrow 560 / Normal 800 / Wide 960 /
  Full (`crates/inkuno-ui/src/theme.rs:49-51`), width follows zoom
  (`Page::measure`, `crates/inkuno-ui/src/theme.rs:44-54`), default Normal
  (`crates/inkuno-logic/src/prefs.rs:28`, `Page::of` fallback `crates/inkuno-ui/src/theme.rs:34`). 800 px
  landed 2026-11-08 as the Normal measure (option (a), see §14).
- Interface spacing unit is the rem at UI scale (`crates/inkuno/src/shell/render.rs:27-28`
  `window.set_rem_size(t.px(16.))`).

### Color + shape (target: quiet neutrals, one accent)

- Three themes: **light, dark, sepia**. Warm paper for sepia (new).
- **Current state (Inkuno):** 13 built-in themes in `THEMES`
  (`crates/inkuno-ui/src/theme.rs:129`), picked from prefs or system appearance
  (`Theme::of`, `crates/inkuno-ui/src/theme.rs:134`), with WCAG-AA enforcement for text,
  accent, and callouts (`crates/inkuno-ui/src/theme.rs:300-330` approx; tests
  `every_theme_reads`, `every_pair_reads` in `crates/inkuno-ui/src/theme.rs:290+`).
  → **RESKIN**: keep the `Theme` engine + contrast tests; curate the default
  offering down to light/dark/sepia (whether the other 10 stay as extras: **TBD**).
- Radii: small 6, tab pill 9, large 8, sheet 12 (`crates/inkuno-ui/src/theme.rs:133-155`
  approx: `small()`, `tab()`, `large()`, `sheet()`). Keep.
- Shadows: `lifted` for floating bars, `floating` for palette/menus
  (`crates/inkuno-ui/src/theme.rs:150-162` approx). Keep; no new elevation levels without §1 flow.
- All tokens centralize in `crates/inkuno-ui/src/theme.rs` (`Theme` struct,
  `crates/inkuno-ui/src/theme.rs:100-124`); views must not hard-code colors/sizes.
  (Inkuno rule: layers depend downwards only, `docs/design/architecture.md:19-27`.)
- **2026-11-23: one layout for every theme.** The Quiet layout (named Dark and Light now) is the only layout; the `quiet` flag and the non-Quiet branches are gone. A theme changes colours, fonts, sizes and spacing, never button placement, row heights, sidebar width, tab style or menus.
- **2026-11-21: Quiet theme is the default; Inkuno Light/Dark stay selectable.** `THEMES[0]` and `THEMES[1]`
  are Dark and Light, picked by the system appearance when no theme is chosen. The Quiet extras
  (link, ground, six code colours) are set in `quiet_extras` (`crates/inkuno-ui/src/theme.rs`); other themes derive them.

- **2026-11-21: Quiet block styles** (every theme). A code block is a box with a 1 px `code_line` border and radius 8, its line numbers in a gutter (mono 13, `gutter_num` on `gutter`, 10 px each side, a 1 px divider), the fences showing their language alone off the live block. A table has a 1 px `table_line` frame and grid, radius 8, a `th` header in semibold, cells padded 12 × 9. An alert (`> [!NOTE]`, TIP, IMPORTANT, WARNING, CAUTION) is a radius-8 box of its tint at 7 % with a 22 % line, its first line an info icon and the kind's name in semibold 14, its `[!TYPE]` cut off the live line. An image has radius 8 and a 1 px ring. An unchecked to-do box is drawn at text 40 % on whole pixels. Tokens: `Theme::code_line`, `gutter`, `gutter_num`, `table_line`, `th`, `ring`, `alerts`, `block_radius()`.

## 4. Layout standard: three regions

```
┌──────────┬────────────────────────────┬───────────┐
│  left    │         center             │   right   │
│  explorer│         canvas             │   outline │
│  (dock)  │  (panes: tabs + note)      │   (dock)  │
└──────────┴────────────────────────────┴───────────┘
```

- **Left region — explorer.** The file tree, recent notes, calendar, tags, links,
  graph (Inkuno: `Mode::Tree/Recent/Calendar/Tags/Links/Graph` shown via
  `Nav::left_shown`, `crates/inkuno/src/shell/docks.rs:70-75`; tree state `crates/inkuno/src/views/nav.rs`,
  drawing `crates/inkuno/src/shell/sidebar.rs`). Default: visible. The unified sidebar (§20) holds the band, the panel and the foot; there is no separate rail.
- **Center region — canvas.** The note(s): tab bar + editor, arranged by the
  pane tree (`render_layout`, `crates/inkuno/src/shell/render.rs:990`; placed by
  `logic/layout.rs`, `docs/design/architecture.md:48`). Always present; it is the focus.
- **Right region — outline.** Document outline (headings of the active note)
  plus comments panel. Current: outline is a right-dock tool, shown by `Ctrl+Shift+O` or the
  palette's **Outline of the note** (`toggle_outline`, `crates/inkuno/src/shell/sidebar.rs`); the comments panel sits
  right of everything (`crates/inkuno/src/shell/render.rs` approx); right-dock stacking
  via `Nav::right_shown` (`crates/inkuno/src/views/nav.rs`). Persistent right-dock view: built 2026-11-08,
  placement decided 2026-11-15 (§12.4).
- **Independently toggleable.** Panel states Rail / Split / Full
  (`Panel`, `crates/inkuno/src/views/nav.rs:95-99`); full-panel toggle
  (`toggle_full`, `crates/inkuno/src/shell/sidebar.rs:64-70`); closing the last left tool
  collapses to rail (`crates/inkuno/src/shell/docks.rs:118-120`); right dock renders only
  when non-empty (`crates/inkuno/src/shell/docks.rs:47-51`). Every region must be hideable
  so the canvas can own the window (zen: both docks hidden).
- **Small-screen rules.** When panes crowd, the window promotes one group to
  alone/full (`alone()`, `crowded_in()`, `crates/inkuno/src/shell/pane.rs:467-474`;
  `crowded()`, `crates/inkuno/src/shell/pane.rs:30`); the side panel keeps `NOTE_MIN`
  (`crates/inkuno/src/views/nav.rs:35`) room for the note and floors the panel at 120 px
  (170 with calendar) (`crates/inkuno/src/shell/sidebar.rs:88-95`). Keep this behavior;
  exact narrow breakpoints (< 700 px etc.): **TBD** — measure, don't invent.

## 5. Component + interaction standards

Verdict per component: **KEEP** (as-is) · **RESKIN** (same behavior, new look) ·
**HIDE** (exists but off/quiet by default). Reason required in each row.

| Component | Today (with citation) | Verdict | Reason |
|-----------|----------------------|---------|--------|
| Tabs + tab bar | Tab bar 34 px high (`crates/inkuno/src/shell/tabs.rs:6`); tab pills radius 9 (`crates/inkuno-ui/src/theme.rs:tab()`); cycle/reorder/close actions (`crates/inkuno/src/actions.rs:6`: `NextTab/PrevTab/MoveTabLeft/...`) | KEEP | Multi-note work needs tabs; restyle only via tokens |
| Panes / splits | Split right/down, move tabs between panes, even/maximize/zoom (`split`, `crates/inkuno/src/shell/pane.rs:604`; `toggle_zoom`, `crates/inkuno/src/shell/pane.rs:457`; actions `SplitPane/SplitDown/MaximizePane`, `crates/inkuno/src/actions.rs:6`) | KEEP | Power layout; quiet by default (single pane) |
| Command palette | Views `crates/inkuno/src/views/palette.rs`; opens for notes/commands/outline/questions (`docs/design/architecture.md:53`, `crates/inkuno/src/shell/palette_open.rs`); ranked note search (`docs/design/architecture.md:108-111` approx, Palette section) | KEEP | The keyboard-first core; central to vision |
| File tree / explorer | `views/nav.rs` state + `shell/sidebar.rs` drawing (`docs/design/architecture.md:90`); left-dock stacking (`crates/inkuno/src/shell/docks.rs:42-51`) | KEEP | Vault navigation; collapses to rail |
| Outline nav | Right-dock list of headings (`crates/inkuno/src/views/nav.rs`); action `ToggleOutline` (`crates/inkuno/src/actions.rs`), chord `Ctrl+Shift+O` and palette `Outline of the note` (2026-11-15) | KEEP | Jump-to-heading is vision (§4 right region); the right-dock view is built (§12.4) |
| Graph view | Force-directed, off-UI-thread (`crates/inkuno/src/views/graph.rs:261-263`); a `Mode` in nav | HIDE (tool, not default) | Useful but anti-chrome-quiet; open on demand only |
| Calendar / Tags / Recent / Links | Nav modes (`crates/inkuno/src/views/nav.rs`; toggles `ShowRecent/ShowTags/ShowCalendar/ShowLinks`, `crates/inkuno/src/shell/render.rs:150-170` approx) | HIDE (tools, not default) | Same as graph: available, never forced |
| Format bar | Bottom-of-note bar (`crates/inkuno/src/views/editor/format_bar.rs`, `docs/design/architecture.md:90`); pref `bar`, `off` hides it (`crates/inkuno-logic/src/prefs.rs:17-18`; `Theme::bar`, `crates/inkuno-ui/src/theme.rs:122`) | HIDE by default | Vision: no permanent toolbar; keep pref, default `off` (**TBD** user confirms default) |
| Focus / typewriter modes | **Not in codebase** (no matches for typewriter/focus-mode) | Vision-only, TBD | Typora parity wants them; needs full §1 proposal |
| Word / char counts | A counter at the bottom right of the note: words and characters of the note, or of the selection (`crates/inkuno/src/views/editor/mod.rs:653`, "Footer counter") | REMOVED 2026-11-14 | Owner decision: removed completely; the round Assistant button takes its corner (§16) |
| Auto-save | `EditorEvent::Changed` → `dirty=true` → 400 ms debounce → `flush` (the handler in `crates/inkuno/src/shell/notes.rs`); the debounced save runs off the UI thread for an existing note (`Shell::flush_debounced`); close and switch save synchronously; atomic writes through `vault::write` (`crates/inkuno-platform/src/vault.rs`), crash never truncates | KEEP | Already local-first; surface a subtle saved indicator (proposal) |
| Session restore | Layout persisted on change (`panes_state`, `crates/inkuno/src/shell/render.rs:137-140`); vault backup/restore archives (`crates/inkuno-platform/src/vault.rs:590`, `:703`) | KEEP + TBD | Layout restore exists; unsaved-text-after-crash recovery: **TBD** — verify `dirty` tabs reload path |
| Zoom (text) | `ZoomIn/Out/Reset` (`crates/inkuno/src/actions.rs:6`); `resize_text` (`crates/inkuno/src/shell/settings.rs:96`); page width follows size (`crates/inkuno-ui/src/theme.rs:44-54`) | KEEP | Accessibility + vision zoom |
| Themes UI | `ChooseTheme` (`crates/inkuno/src/actions.rs:6`); 13 themes (`crates/inkuno-ui/src/theme.rs:129`) | RESKIN | Curate to light/dark/sepia per §3 |
| Motion | `motion` pref, instant when off (`Theme::motion`, `crates/inkuno-ui/src/theme.rs:124`) | KEEP | Respects reduced-motion spirit |
| Help / shortcuts | Help panel + generated shortcuts page (`docs/design/architecture.md:95`, `docs/design/architecture.md:99`) | KEEP | Keyboard-first needs discoverability |
| Toasts / update banner | `render_toasts`, `render_update_banner` (`crates/inkuno/src/shell/render.rs`) | KEEP, quiet | Transient only; never modal for routine info |

## 6. User preferences (EVOLVING — the user edits this section directly)

This section is explicitly unfinished. The user changes values here; agents
implement what it says. Current preference keys (all in `crates/inkuno-logic/src/prefs.rs`):

| Key | Today | User's choice |
|-----|-------|---------------|
| `theme` | theme name, else system (`crates/inkuno-ui/src/theme.rs:134`) | TBD — pick default (sepia? light?) |
| `font` / `mono` | family names, default Inter / JetBrains Mono (`crates/inkuno-logic/src/prefs.rs:7-8`, `crates/inkuno-ui/src/assets.rs:67`) | Inter / JetBrains Mono, landed (see §3) |
| `size` | body px, default 16 (`crates/inkuno-logic/src/prefs.rs:24`) | 16 (confirm) |
| `page` | narrow/normal/wide/full (`crates/inkuno-logic/src/prefs.rs:53`) | Normal = 800 px (landed 2026-11-08, see §3) |
| `wide` | wide-table behavior (`crates/inkuno-logic/src/prefs.rs:53`) | TBD |
| `bar` | `off` hides format bar (`crates/inkuno-logic/src/prefs.rs:17-18`) | default `off` (confirm) |
| `motion` | `off` = instant (`crates/inkuno-logic/src/prefs.rs:39`) | TBD |
| `updates` | update checks on/off (`crates/inkuno-logic/src/prefs.rs:50-51`) | TBD |

Wanted (not yet prefs — proposals via §1): sepia theme definition; default
docks open/closed; outline pinned or not; focus/typewriter toggles; keybinding
overrides file/format (**TBD** — check `crates/inkuno/src/shell/keymap.rs:326` defaults and
whether user overrides exist).

## 7. Acceptance criteria

From the vision (§2); each must hold before calling the UI work done:

1. [ ] **The document is the focus.** With both docks hidden and the bar off,
      the window is the note and nothing else (Panel Rail/Full paths,
      `crates/inkuno/src/views/nav.rs:95-99`).
2. [ ] **Unified editing.** No edit/preview mode split; typing renders in place
      (custom editor element, `docs/design/architecture.md:103-111` approx, Editor section).
3. [ ] **Source preserved.** Round-trip: file on disk unchanged except the
      user's keystrokes (atomic write `vault::write` in `crates/inkuno-platform/src/vault.rs`; merge rule
      `docs/design/architecture.md:100`).
4. [ ] **Offline.** No network needed for any editing/viewing path (vault is a
      local folder; update check is opt-out background, `crates/inkuno-logic/src/prefs.rs:50-51`).
5. [ ] **Consistent fonts.** Inter for UI/body/headings, JetBrains Mono for
      code, everywhere (replacing `crates/inkuno-ui/src/assets.rs:16` defaults).
6. [ ] **Hideable sidebars.** Left and right regions toggle independently and
      the choice persists (`settle_nav` saves layout, `crates/inkuno/src/shell/sidebar.rs:73-82`).
7. [ ] **Three themes.** Light, dark, sepia; all text ≥ 4.5:1 (extend the
      `every_theme_reads` test family, `crates/inkuno-ui/src/theme.rs:290+`).
8. [ ] **Large-doc responsive.** Typing cost stays per-damaged-line
      (`views/editor/layout.rs` + `paint.rs`, `docs/design/architecture.md:91`); tables >
      1500 rows use light layout (`docs/design/architecture.md:105-107` approx).
9. [ ] **Auto-save + recovery.** 400 ms debounce flush (`EditorEvent::Changed` in `crates/inkuno/src/shell/notes.rs`)
      plus verified post-crash restore (see §5 TBD).
10. [ ] **Feels like a writing app.** No IDE chrome by default: no permanent
      toolbar, no forced panels, quietCounts — judged by the user, not by tests.

## 8. Non-rules (what NOT to do)

1. **No IDE clutter.** No file-tree-always-open, no status-bar full of widgets,
   no multi-tool-window layouts as defaults. Tools exist; they stay hidden.
2. **No gradients, shadows-as-decoration, or big colorful buttons.** One accent,
   flat surfaces, radii from §3 only.
3. **No permanent toolbars.** The format bar defaults off; no ribbon, ever.
4. **No mass rewrites.** One component per change (§1). The layer rule holds:
   shell → views → ui → logic (`docs/design/architecture.md:19-27`).
5. **No new dependencies without asking** (`docs/design/architecture.md:133`). Fonts ship
   in the binary like today (`crates/inkuno-ui/src/assets.rs:26-28`).
6. **No untranslated interface text.** All UI strings go through `tr()`
   (`docs/design/architecture.md:131`).
7. **No invented behavior.** If the code doesn't show it and this doc doesn't
   approve it, it's TBD — not implemented.
8. **No drive-by refactors.** Touch only what the approved proposal names.

## 9. Visual reference — Markscribe mockup (PRIMARY)

File: `Downloads/a_clean_minimal_modern_markdown_editor_ui_mockup.png` (+ in-repo build: `docs/design/mockups/editor-current-v1.html`)
(the generated UI mockup; design guidance, not pixel-perfect spec).
What it shows, region by region:

- **Window bar:** native traffic lights, vault dropdown (`Markscribe ⌄`),
  centered document title (`Markdown Guide.md`), split-view + `⋮` icons right.
- **Left sidebar:** icon row (folder / search / collapse), `Documents` tree
  (`notes`, `writing`, `docs` expanded with files, `projects` collapsed),
  lavender selected-file highlight, `Tags` section with counts, `Settings`
  pinned at the bottom.
- **Center canvas:** large H1 + gray subtitle, H2 sections, lavender info
  callout (left accent border), inline-code pills, code block with named
  header row (`markdown` + copy icon) and line numbers.
- **Right sidebar:** `Outline | Statistics` tabs, expandable heading tree
  (`Basic Syntax ⌄` → Headings/Lists/Code/Quotes/Tables/Links).
- **Status bar:** format name left (`Markdown`); `832 words · Ln 24, Col 18 ·
  100% ⌄` right.
- **Theme cards:** Light / Dark / Sepia swatches; type-scale card
  (H1/H2/H3/body + code sample). Confirms the §3 token approach.

Every Phase 1 proposal must name which mockup element it implements.

## 10. Locked decisions (user, 2026-11-08)

1. **Phase 1 = document canvas + typography first** (not full shell).
   Touch `crates/inkuno-ui/src/theme.rs`, `crates/inkuno-ui/src/assets.rs`, `views/editor/style.rs`,
   `views/editor/render.rs`. Shell keeps working as-is until Phase 2.
2. **Inkuno power features are KEEP + reskin, none hidden:** tabs, panes,
   palette, tree, graph, docks, calendar/tags all stay, dressed in §3 tokens.
   > **SUPERSEDED by §13 (user-approved 2026-11-08).** The KEEP-all stance
   > above is replaced by the Strip-down v1 direction: graph / calendar / tags /
   > recent / links / search leave the main UI (rail shows the tree only) while
   > staying palette- and chord-reachable with their code untouched. The item-4
   > geometry locks below stay in force.
3. **Mockup (§9) wins ties** against the written brief on look/feel questions.
4. **Geometry after the 800 landing** (2026-11-08): format-bar default stays
   shown — flipping it moves the bottom-right corner button that
   `boards.rs`/`cursors_comments.rs` click at `height-31`; paragraph spacing
   stays as laid out (a blanket Text pad would also gap list items).
   > **Gap lock LIFTED (owner decision 2026-11-14).** The 8 px ground between the sheets and the
   > window's edges is gone in every mode. Panes take the window's edge to edge; neighbours (panes,
   > the panel, the right dock) are separated by a 1 px hairline (`nav::DIVIDER`, the divider colour)
   > whose handle overlaps both sides (`nav::DIVIDER_HIT`). A pane has no outer margin, no corners and
   > no shadow. The window keeps its own frame (rounding, shadow and resize margin where Inkuno draws
   > it). The comments column keeps its 8 px from the note: it is a column of cards, not a pane. The
   > rest of item 4 (page widths, pill clearance) stands.
   > 2026-11-21: lifted for the Quiet reskin: body blocks get a 9 px gap, list items included. The gap sits above the line box, in the row's pad: the caret, selection, clicks, the block grip and the markers stay with the text.
   Normal page measure is now 800 (option (a) landed, item 5 below);
   pill clearance went PILL_CLEAR 48→56 with it. Inset crew owns
   `render.rs` + `layout.rs` + `shell/mod.rs`: any traffic math depending on
   the width there is their follow-up, verified against `window.rs:497` and
   `md_tables.rs:176` on a green tree.
5. **Phase 2 plan: Normal 800px (LANDED 2026-11-08, option (a)):**
   - `crates/inkuno-ui/src/theme.rs:50` Normal base 720→800; theme.rs width test to px(800)/px(1600). ✅
   - `crates/inkuno/src/views/editor/mod.rs:180` PILL_CLEAR px(48)→px(56) (pill bottom is 53px in 860 windows; check render.rs:434-444 at ui>1). ✅
   - `crates/inkuno/src/views/editor/mod.rs:181,489-490` MAX_WIDTH px(720)→px(800). ✅ (`mod.rs:795-805` table test: 952 stays, lead 116→76. ✅)
   - `crates/inkuno/src/shell/tests/window.rs:480` narrow gate re-derives (threshold 848); asserts keep logic. (verify on green tree)
   - `crates/inkuno/src/shell/tests/end_to_end/md_tables.rs:176` wrap threshold recalibrate (~15-25 or longer filler) + dependent offsets :178-185. (verify on green tree)
   - Leave shell-chrome click tests (boards, cursors_comments) untouched, re-verify only.
   - Re-verify order: window test, page-width test, md_tables/boards/cursors_comments/tags, then full suite.

---

## 11. Keybinding philosophy (PROPOSED 2026-11-08)

> Status: proposal. Ground truth for WHAT the keys are lives in code + generated
> docs, never here. This section is only WHY they are shaped this way.
>
> - Source of truth: `defaults()` in `crates/inkuno/src/shell/keymap.rs` (order matters: for
>   the same context, the last binding wins). Display/parsing: `crates/inkuno-ui/src/keys.rs`.
> - Generated doc: `docs/user/src/shortcuts.md` — do NOT hand-edit; regenerate with
>   `make docs` (test `shortcuts_page_is_current` enforces it).
> - In-app discovery: help panel `F1` / `Ctrl+/`; every line naming an action
>   can be re-bound in place in that panel (see `editable()` in
>   `crates/inkuno/src/shell/keymap.rs` + "Changing a shortcut" in settings doc).

### 11.1 Principles (borrowed, adapted)

1. Keyboard-first but calm (rex / hoocode).
   The mouse path always works (grips, `+` buttons, palette buttons, rail);
   the keyboard path is always full. No speed-run theatrics: one calm chord
   per intent, standard desktop chords win over clever ones.
2. Palette-first discoverability (hoocode / rex beta).
   `Ctrl+P` (find/open/create), `F10` / `Ctrl+Shift+P` (commands), `F1` /
   `Ctrl+/` (help) are the three doors every user learns. Anything without a
   chord must be reachable by typing what it DOES in `F10` or `Ctrl+P`
   (`› template`, `› capture`, `› terminal`, `› formatting bar`, `› theme`, …).
   A feature with neither chord nor palette entry is considered undiscoverable
   (a bug, not minimalism).
3. Leader-like consistency, desktop dialect (nvim / helix, translated).
   We do not force modal editing. We borrow the *consistency* idea: one
   modifier family per scope. `Ctrl+Shift+*` = vault/panel scope
   (`Ctrl+Shift+F` search vault, `Ctrl+Shift+S` sync, `Ctrl+Shift+O` outline,
   `Ctrl+Shift+P` commands, `Ctrl+Shift+B` list-cycle, `Ctrl+Shift+Z` redo).
   `Alt+*` = move/act on the thing under cursor
   (`Alt+Up/Down` block/row move, `Alt+=` autosum, `Alt+Enter` open embed,
   `Alt+Left/Right` back/forward, `Alt+Shift+arrows` pane travel).
   `Ctrl+Enter` = confirm/act in place (task toggle, palette open-aside,
   table insert). New bindings should follow these families.
4. Selection-first where sensible (helix).
   Select-then-act is already the model for `Ctrl+D` (select word, then next
   occurrence), `Alt+click` multi-cursor, table grips, and the formatting bar
   (lit button = selection has the style, click again removes it). Prefer
   extending selection verbs over new modes; `Esc` always collapses back to
   one cursor / dismiss.
5. Chords are stable API (nvim).
   Bindings are user muscle memory: renames need a deprecation path (keep old
   chord working, announce in help/palette), never a silent swap. `make docs`
   + help panel keep code, book, and screen in agreement.
6. macOS rule: `secondary` = `Cmd` on macOS, `Ctrl` elsewhere
   (`crates/inkuno/src/shell/keymap.rs` `defaults()` comment). Word motion uses `Alt` on
   macOS; pane travel is `Cmd+Alt+arrows`; back/forward are `Cmd+[ / Cmd+]`.
   This doc writes the Linux/Windows form (`Ctrl+*`); macOS readers
   substitute `Cmd`.

### 11.2 Current bindings (summary, not source)

If this table and `docs/user/src/shortcuts.md` disagree, the book + `defaults()`
win. Grouped by intent:

Editor (context `Editor` / `Find`):

| Keys | Action |
|---|---|
| `Ctrl+Alt+B` / `Ctrl+I` / `Ctrl+U` | Bold / italic / underline |
| `Ctrl+Shift+X` | Strikethrough |
| `Ctrl+Shift+H` | Highlight |
| `Ctrl+Shift+K` | Inline code |
| `Ctrl+K` | Link from selection / edit link address |
| `Ctrl+Enter` | Task check/uncheck (`[ ]` box) |
| `Ctrl+Shift+B` | List-cycle: bullets → numbered → tasks → plain |
| `Tab` / `Shift+Tab` | Indent / outdent (in tables: next cell / new row) |
| `Alt+Up` / `Alt+Down` | Move block (in tables: move row) |
| `Alt+=` | Autosum: total of cells above/left |
| `Ctrl+D` / `Alt+click` | Multi-cursor: next occurrence / add cursor (`Esc`: back to one) |
| `Ctrl+F` / `Ctrl+R` / `F3` (`Shift+F3`) | Find / find+replace / next (prev) |
| `Ctrl+Z` / `Ctrl+Shift+Z` (also `Ctrl+Y`) | Undo / redo |
| `Ctrl+Shift+M` | Comment selection/line (resolve on comment) |
| `Ctrl+Shift+L` / `E` / `R` | Table column align left / center / right |

App / vault / panes (global + `Shell` / `Nav` / `Palette` contexts):

| Keys | Action |
|---|---|
| `Ctrl+P` | Find by name/text, create, filter by `#tag` (`› word` sub-commands) |
| `F10` / `Ctrl+Shift+P` | Command palette (every command by what it does) |
| `F1` / `Ctrl+/` | Help (re-bindable in place) |
| `Ctrl+N` / `Ctrl+Alt+N` / `Ctrl+J` / `Ctrl+O` | Scratch note (2026-11-14) / note in the tree's folder / today's note / change vault |
| `Ctrl+T` / `Ctrl+E` / `Ctrl+G` / `Ctrl+L` | Tags / tree / graph / backlinks |
| `Ctrl+Shift+J` | Calendar |
| `Ctrl+Shift+F` | Vault search panel |
| `Ctrl+Shift+S` / `Ctrl+Alt+E` | Git sync / share-export |
| `Ctrl+\` / `Ctrl+Alt+\` | Split pane right / below |
| `Alt+Shift+arrows` | Travel panes (macOS: `Cmd+Alt+arrows`) |
| `Ctrl+Tab` / `Ctrl+W` / `Ctrl+1` / `Ctrl+2` | Next tab / close tab / focus pane 1/2 |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Zoom in / out / reset (85–150 %) |
| `Ctrl+Shift+O` | Outline: the headings of the note, in the right dock |
| `Ctrl+Shift+Enter` (in find bar) | Replace across the whole vault (after count) |

### 11.3 Gaps + discussion questions (no decision yet)

1. Outline toggle? DECIDED + BUILT (2026-11-08): `Ctrl+Shift+O` reuses the
   chord for the persistent dock — closed opens it and takes its keys, open
   takes its keys, Escape in its list goes back to the note; palette
   `› outline` does the same toggle. See §12.4.
2. Focus mode chord? No binding exists; "focus" is only page-width Narrow
   (§3). If a real focus mode (hide side panels) is proposed, which chord?
   Candidates: `Ctrl+M` is taken (panel full-window); needs a fresh chord.
3. Multi-cursor discoverability. `Ctrl+D` / `Alt+click` exist but have no
   visible affordance (no bar button, no `/` entry). Options: (a) palette
   entries (`› multicursor next`, `› add cursor`), (b) a hint in the
   formatting-bar overflow, (c) leave as power-user keys. Which?
4. Formatting-bar toggle chord? CURRENT: only via palette
   (`Ctrl+P › formatting bar`); no direct chord, and Phase 1 locks the bar
   SHOWN (§10). Should a toggle chord exist at all before Phase 2, or does
   that invite premature hiding? Proposal: no chord until Phase 2 reskin.
5. Table formula entry (`=` + `Alt+=`) is chord-complete but
   palette-invisible. Worth `› autosum` / `› column align` palette aliases?
6. Kanban/board chords (`Space` tick, `Alt+arrows` carry) live in `Board`
   context only — fine, but should the help panel visually group them under
   Editing so note users find them? Editorial question for `help_sections`.

## 12. Visual spec v1 (PROPOSED 2026-11-08)

> Superseded by §22 (Quiet window, 2026-11-21) wherever they differ: the regions, the sidebar (files only), the one right panel, the empty page and the Quiet tokens. Kept as history.

> Status: proposal anchored to the in-repo mockup build
> `docs/design/mockups/editor-current-v1.html` (which renders this spec; §9
> keeps the original PNG as the vision reference). Read both side by side:
> PNG = Typora-calm target; HTML = CURRENT-behavior build.
> Per-region notes below are annotated CURRENT (implemented, do not regress)
> vs TBD (mockup-only or Phase-2, do not mistake for shipped).
> Phase-1 locks from §10 apply throughout: page width Normal 800 (landed §10.4–5),
> formatting bar stays SHOWN, paragraph spacing stays as-is.

### 12.1 Explorer (left region, CURRENT layout + TBD polish)

- CURRENT: tree / recent / tags / links / calendar / graph live in the
dock-panel; tree marks the active file; arrows preview in an italic tab.
- CURRENT (§20, built 2026-11-21): the 40 px rail is gone. The sidebar's top band holds the sidebar toggle, search and new note; its foot holds the tree button and the title bar switch (the commands logo is gone; the palette is F10 or `>`).
- Spec: narrow column, small semibold section labels, one active-row tint
  (brand 20 %); folder chevrons, tag `#` prefix, counts right-aligned.
- TBD: exact icon set, active-row color token, preview-tab italic style —
  mockup shows intent only; do not hardcode from pixels.

### 12.2 Center canvas (CURRENT behavior, Typora-calm skin)

- CURRENT: single centered column, width = Normal 800 (§3, §10 lock);
  markdown renders as styled blocks (headings, lists, tasks, quote, code
  block with copy icon, table grid, `---` divider); `/` menu, block grip
  + `+` affordance, Tab-indent lists per §5 KEEP list.
- Spec: generous whitespace, `H1 28 / H2 22 / H3 17` semibold, body 15/1.7
  per §3; headings close to body, no card borders; code block tinted with
  mono 13.5; table header tinted, row separators hairline; link color =
  brand, no underline until hover.
- TBD: heading font choice (system serif vs sans in mockup), code-block tint
  token, table zebra (none in v1) — visual intent only.

### 12.3 Formatting bar + footer (CURRENT shown-locked)

- CURRENT (locked §10): formatting bar sits under the note, centered, SHOWN:
  bold/italic/underline/strike/highlight(5 colors)/code/link/lists/tasks/table;
  lit button = selection has style; narrow notes fold it into a bottom-center
  button that opens it; hide only via palette (`› formatting bar`).
- Spec: pill bar, icon buttons, hairline border + soft shadow, active button
  filled (brand 20 %); overflow: collapse to the single opener button.
- The word counter that sat at the bottom right of the note
  (`crates/inkuno/src/views/editor/mod.rs:653`) is REMOVED, decided 2026-11-14 (§16).
  The round Assistant button takes that corner.
- TBD: any other always-visible footer/status bar (word count etc.) is MOCKUP-ONLY
  (§5; §7 acceptance keeps cursor context). Do not present mockup status text as shipped.

### 12.4 Outline (BUILT 2026-11-08: right dock; palette jump retired; chord and palette command 2026-11-15)

- CURRENT: right-dock list of the note's headings (`Mode::Outline`,
  `crates/inkuno/src/views/nav.rs`), available but hidden by default (`right=[outline]`,
  `right_shown=[]`). Decision 2026-11-15 (owner): `Ctrl+Shift+O` and the palette command
  **Outline of the note** both toggle it. Hidden: it shows alone in the right dock, in place of
  the Assistant, and takes the keys; a full-window panel gives way to the split one. Shown: the
  press hides it (`toggle_outline`, `crates/inkuno/src/shell/sidebar.rs`). Click jumps the cursor
  and refocuses the note; parents fold; active section lit and followed;
  show/fold/width persist through the layout file (`crates/inkuno/src/shell/sidebar.rs`,
  `crates/inkuno/src/shell/tests/docks.rs`).
- Spec (as built): right-side list of note headings, follows active
  heading, click scrolls canvas; collapses with panel; keyboard reaching via
  the §11.3-Q1 decision.
- TBD: everything about the dock (width, active-heading style, nesting
  indent) — mockup shows placement intent only.

### 12.5 Mockup annotation key (for the HTML build)

- Mark each area badge-style: CURRENT (matches app today) vs TBD
  (proposal). Anything touching a §10 lock (Normal 800, bar shown,
  paragraph spacing) MUST be badged CURRENT and match the lock — the mockup
  illustrates the lock, never overrides it.

---

## 13. Strip-down v1 (user-approved 2026-11-08; supersedes §10.2)

> Superseded by §22 (Quiet window, 2026-11-21): the sidebar views, the right dock with its stacked tools, the comments column and the round Assistant button are replaced by the regions of §22. Kept as history.

> Direction: a calm single-pane writing app. Stripped components stay modular
> in code — nothing deleted — so any of them can come back anytime; they are
> just not in the main UI now. Bring-back = reverse the hide (rail button,
> pref default, or palette entry), never a re-implementation. North star stays
> the product direction (§2: Typora-inspired, three regions toggleable, quiet
> chrome, Inter + JetBrains Mono, 800 px landed as the Normal measure, phases). The §10 item-4
> geometry locks (Normal 800, bar default shown, paragraph spacing) stay in
> force; the Phase-2 measure proposal has landed as option (a).
>
> Status note: sibling workers have the T-fix, the tree-only rail, the
> visually-hidden format bar, and the comments dot in the working tree
> (uncommitted at time of writing) — this section records the decisions, not
> the merge state. Anything unverified against code stays marked TBD per §1.

### 13.1 Locked decisions

- **Single-pane default.** New windows open one pane; splits (right / below)
  stay code-complete and palette-only (`› split the pane…`, chords unchanged —
  see `keybindings.md` §4). Default-path mechanism: TBD — verify the pane-tree
  init path before claiming; until then "single-pane default" is the locked
  intent, splits-on-demand the verified behavior.
- **Format bar: hide-visually-first, default stays on.** The `bar` pref default
  does NOT flip (§10 lock); the bar hides visually only (sibling WIP:
  `opacity(0)` in `crates/inkuno/src/views/editor/format_bar.rs`, toggle still clickable,
  geometry and tests hold). Palette `› formatting bar` toggles. A toggle
  chord waits for the Phase-2 reskin (§11.3-Q4).
- **Comments collapse to a dot.** The comments column starts shut
  (`comments_shut: true`, `crates/inkuno/src/shell/mod.rs`); the badge becomes one quiet dot
  — no icon, no count — same hit area and corner (sibling WIP in
  `crates/inkuno/src/shell/render.rs`). Click the dot or palette `› comments` reopens.
- **T-fix accepted.** `Ctrl+Shift+T` = reopen last closed tab (browser
  standard); Recent keeps NO chord — `Ctrl+P` opens on recent + `› recent
  notes` command (see `keybindings.md` §8; sibling implemented + tests updated).

### 13.2 KEEP vs STRIPPED (bring-back path per item)

| Component | Verdict | Main UI now | Bring-back path (no re-implementation) |
|-----------|---------|-------------|----------------------------------------|
| Tabs / tab bar | KEEP | Unchanged (§5) | — |
| Panes / splits | KEEP, quiet | Single pane default; split via palette/chords | Re-show a split affordance (§1 proposal) |
| Command palette | KEEP | The way back for everything stripped | — |
| File tree / explorer | KEEP | Sole rail mode by default | — |
| Graph | STRIPPED from main UI | Rail button gone | Palette `› graph of the notes` / `Ctrl+T`-family chord `Ctrl+G`; `Mode::Graph` untouched |
| Calendar | STRIPPED from main UI | Rail button gone | Palette `› calendar` / `Ctrl+Shift+J`; mode untouched |
| Tags | STRIPPED from main UI | Rail button gone | Palette `› tags` / `Ctrl+T`; mode untouched |
| Recent | STRIPPED from main UI, UNBOUND | Rail button gone, no chord | Palette `› recent notes` (`Ctrl+P` opens on recent); re-bind via help panel anytime |
| Links / backlinks | STRIPPED from main UI | Rail button gone | Palette `› backlinks` / `Ctrl+L`; mode untouched |
| Vault search | STRIPPED from main UI | Rail button gone | Palette `› search the whole vault` / `Ctrl+Shift+F`; mode untouched |
| Outline dock | Right-dock tool, built (§12.4) | `Ctrl+Shift+O` and palette `Outline of the note` toggle it (2026-11-15) | Width and nesting stay TBD (§12.4); the palette jump is retired |
| Comments | DOT | Collapsed to dot | Click dot / `› comments` / flip `comments_shut` |
| Format bar | HIDDEN-VISUALLY | Invisible but present, toggle clickable | Palette `› formatting bar`; Phase-2 chord TBD |
| Counts | QUIET | No numbered badges (comment count → dot); the bottom-right word counter is removed 2026-11-14 (§16) | Status-bar counts stay mockup-only (§12.3); any footer count needs a §1 proposal |
| Tables / embeds / kanban / canvas / sheet / capture / sync / sharing / templates / updates | PALETTE-ONLY | No main-UI chrome; nothing unbound that was bound | Type what it does (`› template`, `› capture`, `› sync`, …) per `keybindings.md` §6 |
| Themes | CURATED light / dark / sepia | Offering narrows to three (§3) | Other 10 built-ins stay in the engine; re-listing is a one-line curation change + contrast-test re-verify (§7.7) |

### 13.3 Rules for stripped components

1. **Nothing deleted.** Modes stay in `Mode::ALL`, commands stay in the
   catalogue and help; rail buttons and pref defaults may change, code paths
   stay.
2. **Everything stripped stays reachable.** A palette entry and/or chord must
   exist (`keybindings.md` §2 principle 2: a feature with neither is a bug,
   not minimalism).
3. **Hide, don't remove.** Visually hidden beats conditionally compiled;
   geometry, keys, and tests keep holding (the format-bar `opacity(0)` pattern).
4. **Bring-back = one small proposal per component** via the §1 check-box flow,
   reversing the hide — never a silent re-add.

---

## 14. Typography step: Inter + JetBrains Mono bundled, 800 px landed (2026-11-08)

> LANDED 2026-11-08 — option (a): adjust Normal 720 → 800, per user final call.
> Detail companion: `docs/design/typography.md`.
> 2026-11-21: Quiet.md scale: body 15/22, headings 28/21/18/16/15 (H1 to H6), code 13/20; the reference size is `Prefs::SIZE` (15), so the default UI scale stays 1.0.

- [x] **Propose** — this section + `typography.md` (tokens table, bundling
      proof, 800 px migration path).
- [x] **User approves** — final call 2026-11-08: ADJUST Normal 720 → 800
      (option (a), north-star measure). Q2 (bold story for variable fonts)
      still open — blocks activating the staged fonts.
- [x] **Implement** — `theme.rs:50` Normal base 720→800 (+ width test
      px(800)/px(1600)); `editor/mod.rs:180-181` PILL_CLEAR 48→56, MAX_WIDTH
      720→800 (+ `mod.rs:795-805` table test: 952 stays, lead 116→76).
      `window.rs` narrow gate re-derives via MAX_WIDTH (threshold 768→848).
- [ ] **Verify** — `cargo check` + `cargo test` (theme/editor/md_tables/
      window/prefs) + `cargo fmt --check`, then `md_tables` wrap-offset
      re-tuning if red, then full suite. Blocked at landing time by the
      tabs-crew red in `sidebar.rs:146` (see Q3).

### What landed (no behavior change)

- Inter (four static cuts: regular, italic, bold, bold italic) and JetBrains Mono
  (variable regulars, static bold and bold italic) bundled under
  `crates/inkuno-ui/assets/fonts/` with OFL notices (`LICENSE-Inter.txt`,
  `LICENSE-JetBrains-Mono.txt`); the bundling record is `typography.md` §3 (historical).
- Wired in `FONT_FILES` (`crates/inkuno-ui/src/assets.rs:26`) and now the default families.
  The staging split (`INTER_FILES` / `JETBRAINS_MONO_FILES`) is gone.
- Bundling test: `bundled_fonts_are_real_fonts` (`crates/inkuno-ui/src/assets.rs:518`).
- Former blockers, now resolved: gpui 0.2 cannot embolden a variable font, so the
  bolds ship as static cuts (Q2 below). The weight/size tokens in `theme.rs` (tabs crew)
  are a separate follow-up.

### 800 px landing (option (a), landed 2026-11-08)

Target tokens: body 16/1.7, para 16 px, H1 30 / H2 23 / H3 19, code 14,
metadata 12, measure **800 px** (`typography.md` §1). Landing record
(`typography.md` §4): `theme.rs:47-56` measure + `theme.rs:530-537` test,
`editor/mod.rs:180-181` PILL_CLEAR 48→56 + `MAX_WIDTH` 720→800 +
`mod.rs:795-805` table test (952 stays, lead 116→76),
`window.rs:423` pill-clearance re-run (asserts at 459-460, 480, 539-542 —
narrow gate re-derives via MAX_WIDTH, no literal edits),
`md_tables` wrap-offset re-tuning, pref keys
(`prefs.rs:15,28,36,53`, `settings.rs:34,41` — no numeric change under
option (a), verified), book follow-ups (no 720 mentions found).
Normal 800 is the default (§10.4 lock released for this change).

### Questions for the user

- **Q1. 800 px: adjust Normal 720 → 800, or keep Normal and add 800 as the
  new default width?** → decided 2026-11-08: ADJUST (option (a)), landed.
- **Q2. Bold story for variable fonts:** ship static bold cuts next to the
  variables, or wait for a gpui upgrade with synthetic bold? → answered: static bold
  cuts ship next to the variables, and the fonts are live (see "What landed" above).
- **Q3. Note:** at the time of writing, `cargo test` could not run green
  tree-wide because `crates/inkuno/src/shell/sidebar.rs` (tabs crew) has a non-exhaustive
  `Mode::Outline` match (E0004 at `sidebar.rs:146`) — unrelated to this step;
  re-run `cargo test` (theme/editor/md_tables/window/prefs, then full suite)
  after their landing.

---

## 15. Steering log 2026-11-08 (user-agreed, additive — no code changed here)

> Steering session 2026-11-08. This section records the 7 decisions only;
> it changes no tokens, bindings, geometry, or code. Sibling-owned sections
> (`crates/inkuno/src/`, `docs/user/`, `mockups/*.html`, `keybindings.md`) are untouched —
> their crews land the work; this log is the pointer.

### Status snapshot

- **Done (armed / kept / swept):** (1) rebuild-trigger armed, (2) Rail kept,
  (6) docs sweep landed (docs-only, no commit).
- **Queued:** (3) split-hide, (4) F9, (5) GPUI upgrade, (7) Linux CI-confirm.

### The 7 decisions

1. **Rebuild + test-drive fires when inset + outline land (compile gate
   clears).** No rebuild on red; the trigger is armed and waits for the
   tabs-crew landing (`sidebar.rs:146` E0004) to clear the gate. Status:
   done (armed).
2. **First-run default stays Rail (Tree on first click, preserves
   `empty_page` geometry).** No default flip; Tree remains one click away
   and empty-state geometry is unchanged. Status: done (kept).
3. **SplitPane buttons hidden (splits palette-only, code intact).** Rail
   split buttons hide; splits stay typed-reachable via the palette; no
   split code paths removed. Status: queued.
4. **F9 binds now, focus-mode build starts.** `F9` is the focus/zen
   candidate (reserved per `keybindings.md`); binding lands now and the
   focus-mode build starts against it. Status: queued.
5. **GPUI upgrade for synthetic bold runs after green tree.** Per §14-Q2:
   variable-font bold waits for a gpui upgrade with synthetic bold; the
   upgrade run starts once the tree is green (after the tabs-crew red
   clears). Staged fonts stay staged until then. Status: queued.
6. **Docs sweep landed 2026-11-08 (docs-only, no commit).** `keybindings.md`
   §4 stale line fixed, book tint wording + T-chords follow-ups, mockup
   720 → 800 annotations. `docs/user/src/shortcuts.md` regen left for Linux CI
   (`make docs` can't run on macOS). Status: done.
7. **Linux CI confirms `make-docs` cells.** CI on Linux re-runs the
   `make-docs` cells as the confirmation gate for the sweep + typography
   landings. Status: queued.

### Built 2026-11-14 (clean focus chrome, owner brief)

- Built: `Ctrl+B` sidebar (Bold moves to `Ctrl+Alt+B`), `Ctrl+Shift+S` title bar (Linux; macOS/Windows keep the system bar, gpui 0.2.2 sets it at window creation only), `F9` focus mode (rail, docks, tab bar and title bar hidden; note edge to edge; the 8 px gaps of normal mode stay; lifted the same day, see §15). Two rail switches at its foot. Decisions and tests: `keybindings.md` §8.

### Scratch notes (built 2026-11-14, owner brief)

- Built: `Ctrl+N` makes a scratch note in a tab of its own (no vault needed); `Ctrl+Alt+N` makes the vault note
  filed in the tree's folder, as before. A scratch note lives in the app's own folder, never in the vault, and
  is written as it is typed.
- Closing a scratch note: a blank one goes silently with its file. One with text asks, in the palette's list
  (`Palette::choose`, the settings' list): **Keep as scratch** (the default, `Enter`), **Keep in vault…** (the
  note moves under its title, into the tree's folder or the vault root; its tab stays, as a note of the vault;
  only offered with a vault), **Discard** (last; moves the file to the scratch folder's `.trash`, nothing is destroyed), `Esc` cancels the close. Closing several
  tabs at once (`Close other tabs`, `Close all files`) keeps scratch notes without asking.
- Quitting or closing the window never asks: scratch notes are on disk already, and the scratch tabs open
  at quit come back with the other tabs, with or without a vault (`restore_panes`, the `panes` file).
- Decisions and tests: `keybindings.md` §8 (2026-11-14), `shell/tests/scratch.rs`.

### Owner decision 2026-11-14: edge to edge in normal mode (built)

- The §10 item 4 gap lock is lifted: normal mode takes the focus-mode geometry (panes flush with the
  window's edges, a 1 px hairline between neighbours, flat panes). Tests: `window.rs`
  (`sheets_tile_the_window_edge_to_edge_with_a_hairline`, `normal_mode_starts_at_the_content_origin_and_tiles_with_a_hairline`),
  and `end_to_end/cursors_comments.rs` (the comments column width and its right edge follow the window).
- macOS now hides the system title bar and its traffic lights with `Ctrl+Shift+S` (and with focus mode), and shows them again on the next press; the state is kept in the layout file like on Linux. The window is created with a transparent bar on macOS only (`shell/mod.rs`), and `platform/mac_chrome.rs` hides the three standard buttons through the NSWindow. Not yet run on a real Mac: see `backlog.md`.

---

## 16. Assistant v1: a reviewer that cannot change the text (built 2026-11-14)

> Status: approved by the owner on 2026-11-14, built. The code is the ground truth for names. Decisions
> for the command line side are in `cli.md` (Decisions).

- [x] **Propose**, **User approves**: this section, owner-approved 2026-11-14.
- [x] **Implement**: `shell/assistant.rs` (panel, state, process side), `platform/assistant.rs` (child process),
      `crates/inkuno-logic/src/assistant.rs` (the words, the findings, the placing of comments),
      `markdown::comment_on` (one comment, shared with `inkuno comment`).
- [x] **Test**: `shell/tests/assistant.rs` (events fed by hand, no process), `platform/assistant.rs`
      (a fake agent in `sh`), `inkuno-logic` `assistant.rs`.

### 16.1 The counter is removed

- The word and character counter at the bottom right of the note is removed completely
  (`crates/inkuno/src/views/editor/mod.rs:653`). It is not moved and not hidden.
- It is replaced by the Assistant button (§16.2). Nothing else takes its place.

### 16.2 The Assistant button

- A round button, 30 px, at the bottom right corner of the note. The comments bubble stays to its left.
- States:

| State | Shows |
|---|---|
| Idle | quiet |
| Working | accent colour, slow pulse |
| Needs you | an accent dot: an answer, a finished review or an error arrived while the panel was not shown |
| Not installed | dimmed; its tooltip says the program was not found |

- `Needs you` is cleared (back to Idle) when the panel is shown. While the panel is shown, the end of a run
  goes straight to Idle.
- A click opens the panel, or closes it.
- Not drawn in focus mode (`F9`, §15).

### 16.3 The panel: a tool of the right dock

- It is a tool of the right dock, like Outline (§4, §12.4). It is always in the right dock.
- Hidden by default. When Outline is shown too, it stacks under Outline. A saved layout gains it automatically.
- Opened by the button, or from the command palette by typing `assistant` (action `app::ToggleAssistant`).
  No default shortcut in v1 (owner decision).
- Content, top to bottom:
  1. the conversation: the person's questions, the agent's answers, quiet lines for errors and for a review in progress;
  2. **Review this note**, **Improve selection** or **Improve this note** (§18), and while the agent works, **Stop**;
  3. the prompt box.
- There is no Allow and no Deny. The agent has no tool that changes anything (§16.4), and its protocol never
  asks the client a question, so there is nothing to answer.
- The prompt box is one field. Long text wraps. `Enter` sends. `Escape` returns to the note.
  No multi-line entry in v1 (owner decision; a multi-line box is a later idea).

### 16.4 The agent process

- Inkuno starts the `hoocode` program, found on the PATH, as a child process with the flags
  `--mode rpc --no-session --disallowed-tools Shell,ShellKill,Write,Edit,WebFetch,WebSearch,Agent` after the user's own words
  (`platform/assistant.rs`, `arguments`). Its protocol is JSON Lines of hoocode's own, one object a line.
- So the read-only rule is enforced by the tools the agent is started with, not only by an instruction:
  it has no shell and no write tool. A review's findings are text in its answer; Inkuno writes them (§16.6).
- Its working folder is the open vault, or the scratch folder when no vault is open. A new folder means a new
  agent: the old one is stopped first.
- It starts on the first prompt. It stops when Inkuno quits, when the vault changes, or when the setting
  changes.
- When the program is not found, the panel says so quietly. Nothing else changes.
- hoocode is a separate MIT program. It is never linked into Inkuno.
- Layers: `platform/assistant.rs` starts the process, reads and writes its lines, and stops it (no window
  code). `crates/inkuno-logic/src/assistant.rs` holds the words sent and read back, pure.
  `shell/assistant.rs` holds the panel, its state and the process side.

### 16.5 The setting

- `assistant` in the settings file is the command that starts the agent. The default is `hoocode`.
- It is changed with **Assistant command…** in the palette. An empty entry returns to the default.
- Inkuno stores no keys and no model choice. Login and model stay in hoocode's own configuration.

### 16.6 The two actions of v1

- **Chat** about the current note. The note's unsaved edits are written first, so the agent reads what the
  person sees. The transcript shows the question; the words sent to the agent also name the note's file
  (`logic::assistant::chat_prompt`).
- **Review this note**: the note is saved, and the agent is asked for findings in a fixed text form
  (`logic::assistant::review_prompt`). When the note has open comments, the prompt lists them first, numbered `C1`,
  `C2`… (`logic::assistant::open_comments`: those whose last message is not by `assistant`), and says the person's
  comments come first: each question or request in them gets one `REPLY: C<n>` and `COMMENT:` (one line, a direct
  answer grounded in the note, or what is missing). Then at most 12 new findings: `PASSAGE:` a line copied from the
  note, `COMMENT:` the remark. Both item forms are alternatives in the same answer. The single line `NO FINDINGS` only
  when there is neither a reply nor a finding. The answer is not shown; its text is collected while the agent
  works, and read when the agent ends (`logic::assistant::findings`).
- **Replies**: a reply is written after the comment's replies, on its line (`markdown::reply_on`), by `assistant`. The
  agent writes its comments and replies under the name `assistant`, so it never answers itself.
- Placing the findings (`Shell::apply_review`, `logic::assistant::place`): replies first, last comment first; then each
  new comment with `markdown::comment_on_by` (or `comment_after`), from the last passage in the text to the first. A reply is written only where
  its listed comment still starts at the same byte with the same markup.
- **New-comment placement** (`logic::assistant::spot`): a passage is found by its text and must be in the text exactly
  once. It may lie inside the passage of a comment already there, with words after it: the new marker splits that passage,
  and the old comment keeps the words after the new marker (its author, time and text are untouched). When the passage ends
  the old passage (or only its punctuation is left), nothing is split: the new comment is written right after the old
  comment's marker (and after any markers adjacent to it), so the old passage stays whole and the new comment parses as its
  own comment with its author and time (`logic::assistant::comment_after`). Skipped when the passage lies in a comment's
  marker or text, or in a CriticMarkup comment. The skipped ones are counted.
  The summary: "Wrote N comments.", "Wrote N replies.", "Wrote N comments and M replies.", "1 finding could not be
  placed.", or "No findings.".
- When the review writes into the open note, the Review view opens on the first new comment or reply, expanded and scrolled
  into view (`Shell::reveal_review`). It is opened without moving the keys (`Shell::show_view_quiet`), so a person typing stays
  in the note.
- The words are written by Inkuno, never by the agent. A review that is stopped, or whose agent fails or
  exits, writes nothing.

### 16.7 Seeing the changes

- An open note is changed with one splice (`Editor::absorb`, the route an outside change takes): all the
  comments of one review are one undo step, `Ctrl + Z` takes them back, and the cursor and scroll stay where
  they were. The note becomes dirty and is saved by the usual debounce.
  `logic::assistant::edit_between` gives the one splice that turns the old text into the new one.
- When no tab shows the note any more, its file is read, the findings are placed, and the result is written
  with `vault::write` (atomic). The watcher then follows as for any outside write.
- If the person is typing in the note at the same time, the normal merge applies (see the Agents page of the guide).

### 16.8 Scratch notes and the conversation

- Review works on scratch notes too. `inkuno comment --file PATH` comments on a Markdown file by its path,
  with no vault (`cli.md`, Decisions); the Assistant itself does not use that command.
- The conversation is kept in memory only. It is gone when Inkuno restarts (owner decision).

### 16.9 Known limits

- The agent cannot ask before it uses a tool. hoocode's RPC mode has no approval messages (its `app-server`
  protocol has them). Its edits do not exist: its proposals reach the note as pending changes (§18), which the
  person accepts or rejects (§17). Accept and reject have chords inside a change (`Ctrl+Enter`, `Ctrl+Backspace`,
  §17.5); accept all, reject all and comment have none.
- The prompt box is one field without newlines.

## 17. Pending changes (built 2026-11-15)

> Status: the app's side of `inkuno write` (`cli.md`, 2026-11-15). The changes are beside the note, in
> `.inkuno/pending` (2026-11-21, `platform/sidecar.rs`); the editor shows them as marks in block form (`logic/pending.rs`),
> composed from the file and the list, and split again on save. This section is what the editor does with them. Chords: §17.5.

- [x] **Propose**, **User approves**: the flow of `revisions.md` (Decisions, 2026-11-14 and 2026-11-15).
- [x] **Implement**: `views/editor/layout.rs` (`Pend`, `pend_of`), `paint.rs` (the wash), `text_runs` (dim and
      strike), `ui/theme.rs` (`added`, `removed`), `views/editor/edit.rs` (the guard), `views/editor/changes.rs`
      (accept, reject, comment), `shell/changes.rs` (the comment prompt and the hint), `shell/menu.rs`.
- [x] **Test**: `shell/tests/pending.rs` (typing, paste, replace all, accept, reject, comment, an outside change,
      the hint), `inkuno-logic` `pending.rs`
      (`refuses`, one test per boundary case), `ui/theme.rs` (`pending_changes_show_and_read`, the pair test), `layout.rs`
      (`a_line_knows_its_pending_change`), `edit.rs` (`the_span_of_a_rewrite…`), `commands.rs` (the five entries).

### 17.1 Drawing

- An **added** line lies on the `added` wash (a green, quiet tint) across the page width, its text as it is.
- A **removed** line lies on the `removed` wash (red), its text dimmed to `soft` and struck through.
- The four **marker** lines are dimmed (`dim`) and drawn at 0.8 of the body size, their rows 0.8 of a body row
  too (rounded to a pixel). They stay real lines: no line is hidden, and a change takes less room. Each row has its
  own size, as a heading does (`layout.rs`, `MARK_SCALE`), so the caret, the hit test and the scroll follow it.
- The washes are theme tokens (`Theme::added`, `Theme::removed`), derived as `highlight` is: the strongest step
  from 22 % down that keeps the text at 4.5:1 on it. They are in the contrast tests of every theme and in both looks.
- The areas are read once per text change (`Editor::pending`, set in `changed` and `load`), never per frame. Each
  line's state is part of its styling cache key, so a changed area restyles its lines.
- Tables and figures inside a change carry the same wash as its text lines, behind the whole block: a table's rows
  take the state of their source lines, and a table's header keeps the wash instead of its own band. A removed table
  or figure is not struck through.

### 17.2 The read-only rule

- The person cannot change a pending change. The one rule is `logic::pending::refuses(text, range, inserted)`:
  `Editor::touches_pending` delegates to it entirely, and the command line uses it too (`cli.md`). It refuses an edit
  that changes a byte of an area, marker lines included, and an edit that would leave a marker line without its own
  line: one that ends where an area starts unless the inserted text ends a line (or the deletion takes a whole line
  that starts a line), and one that starts where an area ends when the area has no final line break and the inserted
  text does not start with one. So `Enter` at the edges works, a letter at the start of a marker line is refused, and
  a letter at the end of a note whose last marker has no break is refused (`Enter` first). The unit tests of
  `pending.rs` cover each case, CRLF included.
- A multi-range route (restyle of several lines, cell marks, comment lines moved with a table, completion that
  removes brackets, a table's rewrite with its comment lines) checks every range first, through
  `Editor::touches_pending_any`, and makes none of them when one is refused. The undo step is pushed only after the
  check, so a refusal leaves no undo step and keeps the redo.
- Allowed: the caret and the selection enter a change; copy works; undo and redo (`Editor::restore`, the `Undo` and
  `Redo` actions); an outside change (`absorb`, the file watcher and the Assistant); accept, reject and comment (they
  go through `absorb`, not the guard).
- The routes that pass the guard: `edit` and `splice` (typing, IME, delete, Enter, paste, replace one), `shift_lines`
  (Tab), `restyle` (format bar and block turns), `clear_cells`, `realign`/`align_at` and `set_table` (tables),
  `renumber`, the table branches of Enter, cut (`copy(cut)`, before the clipboard is written), replace all.
- A refusal emits `EditorEvent::Notice` with `This change is pending: accept or reject it first.`. The shell shows it
  once while it is on screen (`shell/notes.rs`).
- A renumbering or a table realign that would change a pending change is left out, silently: the edit that led
  there was refused and said so already. A mark line ends a list or a table, so a renumbering or a realign never
  crosses a change from kept text.

### 17.3 Accept, reject, and undo

- **Accept the change** / **Reject the change** act on the change the caret is in, or the selection touches. With the
  caret outside every change, the first press does not act: it moves the caret to the next change (after the last
  one, the first), scrolls it into view, and says `Caret moved to the next change. Repeat to accept or reject it.`.
  The right-click menu moves the caret into the clicked change first, so it acts on that change as before.
  **Accept all** / **Reject all** act on every change. With no change, the message is `No pending change.` and
  nothing changes.
- The new text comes from `logic::pending` (`accept`, `reject`, `accept_all`, `reject_all`), turned into one splice by
  `logic::assistant::edit_between`, and applied by `absorb`: one undo step. The note becomes dirty and is saved by the
  usual debounce. The caret goes to where the change was (its start, or the start of the first change for all).
- `Editor::restore` skips an undo step that left the text as it is, so an edit refused after its undo step was taken
  does not leave an undo that changes nothing. A refused edit pushes no undo step at all (§17.2).

### 17.4 Comment on the change

- **Comment on the change** asks for one line in the shell's prompt (`Palette::prompt`, as `Save a version` does).
  The prompt names the change by its text (`EditorEvent::AskChangeComment(text)`), and when it answers the change is
  found again by that exact text. An outside edit may have moved it meanwhile; if no change has the text any more,
  nothing is written and the message `That change is gone or changed: nothing was commented.` is shown.
  The comment is written by `markdown::comment_on` on the first added line of the change, or on its first removed
  line when nothing is added: `passage<!-- comment: text -->` (2026-11-21, Q1). The passage is that whole line, so it must be unique in the
  note, or the reason is said and nothing changes (empty line, comment marks already there, passage not unique).
- The change stays pending. One undo step, through `absorb`.

### 17.5 Where the actions are

- Declared in `views/editor/mod.rs` (`AcceptChange`, `RejectChange`, `AcceptAllChanges`, `RejectAllChanges`,
  `CommentChange`), handled in `views/editor/render.rs`.
- Palette: five entries in `inkuno-logic/src/commands.rs` (group Note; icons `check.svg`, `close.svg`,
  `c-comment.svg`).
- Chords (owner decision, 2026-11-15): **Accept the change** is `Ctrl+Enter` and **Reject the change** is
  `Ctrl+Backspace`, both only while the caret or the selection is in a change. The editor's key context is then
  `Editor Change` (`views/editor/render.rs`), and two bindings with the context `Editor && Change` come after the
  `Editor` ones in `shell/keymap.rs`, so they win there (GPUI takes the deepest matching context, then the later
  binding). Outside a change `Ctrl+Enter` is still `ToggleTask` and `Ctrl+Backspace` still deletes a word.
  **Accept all changes**, **Reject all changes** and **Comment on the change** have no chord. The palette shows the
  chords it has; the help panel lists the five in `Changes in a note`, and `shortcuts.md` is regenerated.
  Test: `shell/tests/pending.rs` (`ctrl_enter_…`, `ctrl_backspace_…`).
- Right-click menu: `EditorEvent::Menu` carries whether the click is inside a change; then the menu offers the three
  actions first, on the change under the pointer (`shell/menu.rs`, `views/nav.rs` `Do`).

### 17.6 The hint

- While the note has changes, a quiet pill at the note's foot, left of the corner buttons: `2 changes pending`
  (`1 change pending`). A click moves the caret to the first change and focuses the note. Not drawn in focus mode.
  It is drawn over the text like the corner buttons, so it moves no line.

### 17.7 Known limits

- Accept all, reject all and comment on a change have no chord (§17.5).
- Copy of the whole note (`app::CopyAll`) copies the marker lines as they are: the app has no reading view or export
  that resolves them.
- A change at the very end of a text without a final line break gets one break more on reject (the limit of
  `logic/pending.rs`).
- A rewrite that would change a change is left out without a message (renumbering): the numbers stay until the
  change is accepted or rejected.
- Rename rule: a rename or a title change that would rewrite a `[[link]]` inside a pending change of a linking note
  is refused before the file moves or is written, with a notice; the check is `pending::rewrite_refused`, as in
  `cli.md`. Links outside every change follow the rename as before.

## 18. Assistant v2: proposals through pending changes (built 2026-11-15)

> Status: built, on the owner's brief of 2026-11-15. The code is the ground truth for names. The agent still has no
> write tool (§16.4); its proposal reaches the note only through the pending-change flow (§17).

- [x] **Implement**: `shell/assistant.rs` (the job, the action, the write, the revisions), `crates/inkuno-logic/src/assistant.rs`
      (`improve_prompt`, `proposal`, `rewritten`), `commands.rs` (the palette entry), `shell/menu.rs` and `views/nav.rs`
      (the right-click entry), `shell/render.rs` (the action's handler).
- [x] **Test**: `shell/tests/assistant.rs` (the Improve tests: events fed by hand, the prompt read from `sent`), `inkuno-logic`
      `assistant.rs` (each word of the prompt, the labels, the fence, `NO CHANGE`, CRLF, the three kinds of range), `commands.rs`.

### 18.1 The action

- `app::AssistantImprove`, no chord. Reached from three places:
  - the panel's second button, next to **Review this note**: **Improve selection** when the note has a non-empty
    selection, else **Improve this note**;
  - the command palette: **Improve selection or note** (group Go, next to the Assistant entry; `improve` finds it);
  - the editor's right-click menu: **Improve selection**, offered only when the selection is not empty.
- Same gating as Review: the agent is free (not working, installed), and a note with a file path is open. Scratch notes
  work. Refused before anything is sent, with one quiet line: **Accept or reject the pending changes first.** The box
  keeps its text.

### 18.2 What is sent

- The scope: the selection of the active note when it is not empty, else the whole note.
- The instruction: the prompt box's text when it is not blank (the box is then cleared), else the default
  (`logic::assistant::IMPROVE_INSTRUCTION`: clearer and tighter, same meaning, same language, same Markdown structure).
- The note is saved first, as Review does (the same failure line, with the word *improved*). The path is read after that
  save, not before: a save renames a note whose `# Title` line changed (`flush_one`), and the prompt names the file as it is
  then. `base` is the note's full text at that moment, kept in memory with that path and the byte range of the selection in
  `base` (`None` for the whole note). Review reads its path the same way (§16.6). The transcript shows the question as a `You` line (**Improve selection** or **Improve this note**, with `: instruction`
  when the person typed one), and a quiet line **Working on a proposal…**.
- `improve_prompt` names the file to read for context, gives the instruction, and for a selection gives the passage verbatim
  between the lines `BEGIN PASSAGE` and `END PASSAGE`. It asks for an answer that is nothing but the new text between a line
  `BEGIN TEXT` and a line `END TEXT`, with no code fence, and the single line `NO CHANGE` when nothing should change.
  The prompt also says **Write in the language of the note.**, so the answer keeps the note's language whatever the
  instruction's.
- The box is emptied only once the run has started (the status is Working). A prompt that could not start (the command
  refuses, or the agent does not start) stays in the box, with its error line, and leaves no job behind.

### 18.3 How the answer comes back

- The answer is collected while the agent works and not shown, as a review's is (`Job`, §18.4).
- When the agent ends, `logic::assistant::proposal` reads the lines between the first `BEGIN TEXT` and the last `END TEXT`.
  The labels are tolerant: surrounding whitespace, bold, backticks, a trailing colon, any letter case. One code fence wrapped
  round the block is dropped. The answer gives nothing (`None`) when there is no such block, when the block is empty, or when
  it is `NO CHANGE`.
- `logic::assistant::rewritten` gives the whole proposed text. The block replaces the passage, or the whole note. Its leading
  and trailing line breaks go; its line breaks take the kind of `base` (CRLF when `base` has CRLF). A passage that ended with a
  line break keeps one, and so does a note that ended with one.
- The fence rule: `proposal` takes a second argument, whether the text the block replaces (the selected passage, or the whole
  note when there is no selection) starts with a fence line (`logic::assistant::starts_with_fence`). One code fence wrapped
  round the block is dropped only when that is false. So a fenced passage comes back with its fences, and an unfenced passage
  returned inside a wrapper fence has the wrapper removed.

### 18.4 The job

- The panel holds one job at a time: `Job::Review` (§16.6) or `Job::Improve`. The panel is not free while it runs.
- **Stop**, a refusal, a failure, an exit, or the kill after the grace period drops the job and writes nothing, as for a review.

### 18.5 Writing the proposal

- `pending::propose(base, current, proposed)`, with `current` the open tab's text, else the file's text.
- A refusal is said in one line in the person's words: nothing changed → **The assistant proposed no change.**; already
  pending → **Accept or reject the pending changes first.**; marks in the text → **The text holds pending-change marks, so
  nothing was proposed.**; the person changed the same lines → **You changed the same lines meanwhile, so nothing was
  proposed. Ask again.** A line of the answer that is `NO CHANGE`, or no block, gives **The assistant proposed no change.**
- Accepted, the proposal is one undo step: `logic::assistant::edit_between` gives the splice, applied by `Editor::absorb`
  when a tab shows the note; otherwise `vault::write` (atomic) writes the marked text, and a failed write says **Not written:
  the proposal was not saved:** with the reason. Other changes of the person since the agent read the note are kept, and the
  agent's are placed in the current text (as `inkuno write` does).
- Then one quiet line: **N changes are pending in the note: accept or reject them there.** (for one: **1 change is pending in
  the note: accept or reject it there.**), counted by `pending::areas`.
- When the proposal is written into the open note, the Review view opens on its run (one card, expanded; §23.5) with no
  focus moved (`Shell::reveal_review`, `Shell::show_view_quiet`). Not when the note is not the one shown.

### 18.6 Revisions

- Around a successful proposal two `kind: auto` revisions are saved, as `inkuno write` saves them (`revisions.md`, 2026-11-15):
  - the text before the change (`current`), by the person's author setting (the one `Save a version` uses), with the note
    **before a change by assistant** (fixed English, the same words as `inkuno write`: history data is not translated),
    skipped when it is already the latest revision's text;
  - the proposed text, `pending::accept_all(marked)`, by the author `assistant`, with the note `proposed, pending`.
- A failure of the first save stops the proposal with one line that says why; nothing is written. A failure of the second is a
  warning line; the changes stay pending.
- A note with no revisions home (`platform/revisions.rs` `home_of` gives none) skips both, silently.
- The command line keeps its own helper (`cli/ops.rs`, `snapshot`). The shell cannot call it (the layers), so the shell has the
  same few lines on `platform/revisions.rs` (`save_as`, `home_of`, `now`).

### 18.7 Known limits

- A whole-note improvement sends the note and gets the whole note back: a very long note is slow.
- One job at a time; a second improvement waits for the first answer, and is refused meanwhile.
- A line `END TEXT` after the block, in the agent's commentary, is taken as part of the text (the last one closes it).
- The default instruction is English. The prompt asks for the answer in the note's language (§18.2).
- The passage of a selection that holds a table or a figure is not special-cased.

## 19. Versions panel, compare and restore (built 2026-11-15; restore 2026-11-16)

> Status: built, on the owner's brief of 2026-11-15 (`revisions.md`, Decisions). The code is the ground truth for names.
> The panel lists the saved versions of a note; a compare shows one of them against the note as it is now.

- [x] **Propose**, **User approves**: the brief of 2026-11-15 (the panel, the compare in a read-only tab, the row menu).
- [x] **Implement**: `shell/versions.rs` (the list and its cache, the compare, the row gestures), `views/nav.rs` (`Mode::Versions`,
      `Do::CompareVersion`, `Do::CopyVersion`), `shell/sidebar.rs` (the rows, the empty states, the drawing), `shell/menu.rs` (the
      entries, `version_do`), `shell/tabs.rs` and `shell/render.rs` (the tab's and the window's name, the palette action, the
      hint), `shell/pane.rs` (`Pane::compare`), `shell/revisions.rs` (the save refused in a comparison), `shell/assistant.rs` (the list
      read again after a proposal), `views/editor/` (the `readonly` flag: `edit.rs`, `changes.rs`, `clipboard.rs`, `input.rs`,
      `layout.rs`, `mod.rs`), `inkuno-logic/src/revision.rs` (`row_name`, `short_date`, `compare`), `commands.rs` (the entry and its
      alias `history`), `views/palette.rs` (`offered`), `shell/keymap.rs` (the help line).
- [x] **Test**: `shell/tests/versions.rs` (the palette, newest first and the auto mark, the three empty states, the tab switch and the
      save, Enter, the read-only rules, not dirty and nothing written, equal texts, marks refused, one tab per version, the row
      menu and the copy, the menu cannot trash a version; the fix round: a version chosen in the panel is not the tree's selection,
      every file gesture is refused on a history file, one unreadable version among three, a folder link and a wiki link followed
      from a comparison, a note loaded into a comparison, a pasted image, the saved layout, the panel following the comparison, a
      revision written from a terminal), `inkuno-logic` `revision.rs` (row name, date and its hand-edited cases, the compare and its
      refusals), `platform/vault.rs` (a revision event reaches the history signal only; the real watcher does the same on disk),
      `commands.rs`, `views/palette.rs` (`history`), `views/nav.rs` (the saved word `versions`).

### 19.1 The panel

- `app::ShowVersions`, palette entry **Versions of the note** (group Go; `history` finds it too). No chord and no rail button, as
  the Scratch panel. It lives in the left dock, hidden by default; an older layout file names no `versions`, and nothing changes.
- Rows: the versions of the active tab's note, newest first. A row reads `#12 · author`, then `· auto` for an automatic version,
  then `· <note>` when it has one. On the right, the date and time in local time, to the minute (`logic::revision::local_date`,
  `platform::clock::local`: the platform's own zone rules, no zone read by hand). The stored time is UTC. An automatic version's
  name is drawn in the quiet colour.
- The list is read from disk when the panel is shown, after **Save a version**, after an Assistant proposal (Review or Improve), when
  the note of the active tab changes, and when a revision is written from outside the app (`inkuno rev save`, another window): the
  vault's watcher reports the `.inkuno/revisions` folder on its own signal, which only marks the list stale and redraws the panel
  (`versions_moved`). Nothing else is reread for it: no note reload, no tree refresh, no link index. It is cached in
  `Shell::versions` and keyed by the note's path; each row finds its version by a lookup, not a scan, so nothing is read per frame.
- The list reads each version's **header only** (`platform::revisions::headers`, `read_header`): the front matter up to its closing
  `---`, line by line, never the text. A version's text is read on demand, by a compare, a copy or a restore (`platform::revisions::get`);
  a long history therefore lists at once. A header that cannot be read is the unreadable row (above); a text that cannot be read
  when a compare, a copy or a restore needs it says `#7 · this version cannot be read, so it cannot be compared.` (or copied,
  or restored), the same words the row gives.
- While a comparison is the shown tab, the panel lists the versions of the note that comparison belongs to, and the row of the
  compared version is marked as the current one (the same mark a shown note gets in the tree). Enter on another row opens another
  comparison.
- A version whose file cannot be read is not hidden with the others: it is one quiet row, `#7 · cannot be read`. Enter on it opens
  nothing and says why: **#7 · this version cannot be read, so it cannot be compared.** The other versions still list, compare and
  copy.
- Empty states, in the panel's quiet text:
  - no note in the tab: **No note is open.**
  - a note outside the vault and the scratch folder: **This note has no history.**
  - no version yet: **No version yet. Save one with** the chord in force (`Ctrl+Alt+S`).
  - a history that cannot be read: the reason, not an empty list.
  - a history whose folder is readable but whose files are not: one quiet row per unreadable file (above), not a message.
- Keys: Up and Down move, Enter compares, Escape returns to the note, as in the other panels.

### 19.2 The compare

- Enter, a click, or **Compare with the note now** in the row's menu opens a tab of the active pane: a read-only note with no file,
  titled `<note> · #12 ↔ now` (the word `now` is `maintenant` in French).
- Its text is the version's text, with each difference written as a pending change: the lines the version had and the note no longer
  has are removed lines, the new ones are added lines. The drawing of §17 colours them: green for an added line, red and struck for a
  removed one. The note's text is the one shown now, unsaved edits included. Decision: `logic::revision::compare`, which is
  `pending::propose(version, version, now)`.
- Not opened, with one line and no tab:
  - the two texts are equal: **Nothing changed since version 12.**
  - the version or the note holds pending changes or their marks: **This version or the note holds pending changes: accept or reject
    them first.**
- Opening the same version again brings its tab to the front. When the note has changed since, that tab's text is replaced by the new
  compare.
- The compare reads the note's text from the tab that shows the note (its unsaved edits included), or from the file when no tab shows
  it: never from the comparison tab itself, whose text holds the marks.
- A comparison is never a note and never a list page. Every route that loads a note or a list into the active tab takes a tab of its
  own when that tab is a comparison (decision of the fix round). Checked, route by route: `Shell::open` and `fresh_tab`, through
  `spare_tab`, which refuses a comparison (`open_note`, the palette's note, the daily note through `open_wiki`, the wiki link, the
  reopen of a tab, the restored layout, a tree drop, Ctrl+N through `new_scratch`, `new_note_in` for a note of the tree);
  `open_listing`, which takes a tab when the active one is a comparison (the folder link, the list of a folder, the tree's list gesture,
  a folder dropped on a pane); `open_wiki`'s "create the note" branch, which goes through `fresh_tab`; `preview_note`, which uses a
  preview tab or `fresh_tab`, never the comparison; `reload` and `restore_panes`, which only touch the note they hold. The vault's
  opening (`library.rs`) and a file's deletion (`files.rs`) call `new_note` on their own tab; the second line below covers them.
  The second line of defence: `load_note` and `new_note` clear the tab's `compare`, and `Editor::load` clears the read-only flag, so a
  load that gets past the routes makes the tab an editable note, not a comparison with a note's text.
- The tab is read-only. The caret, the selection, copy, find and scrolling work. Typing, paste, cut, Enter, Tab, Delete, the format
  bar, the table and block gestures, find-and-replace, accept, reject, comment, undo and Save a version change nothing. Paste is
  refused first, before the clipboard's picture is written to the vault: no file appears for it. Each refusal
  says **This is a comparison: it cannot be changed.** once (Save a version says it cannot be saved as a version). The editor's
  `readonly` flag does this: `touches_pending` refuses every edit of a read-only note, and the change actions refuse before they act
  (§17.2, §17.3).
- The compare is a snapshot: it does not follow later edits of the note. It is never saved: no dirty mark, no autosave, no file. Closing
  it asks nothing. It is not added to the recent notes or to the reopen list (Ctrl+Alt+T), and the layout file skips it, so it is not
  restored at the next start. The "N changes pending" hint is not drawn for it. Review and Improve are not offered: they need a file.

### 19.3 The row menu

- **Compare with the note now** (the default: Enter does the same), **Copy this version's text**, and **Restore this version** (last).
- **Restore this version** is offered only when the row's header can be read: a version with no readable text has no restore.
- No other gesture applies to a version: the tree's trash, rename, move and duplicate act on nothing, since a version is history.
- The history is never a target of a file gesture, whatever the panel shows. `menu_do` refuses, at its top, any target under a history
  folder: the vault's `.inkuno/revisions` and its bin `.inkuno/trash`, and the scratch folder's `.revisions` (bin included), named by
  `logic::revision` and `platform::revisions::holds_history`. The refusal says **The history of the notes is kept by the app: nothing
  is done to it.** The selection is also cleared when the Versions panel is entered or left (`Nav::activate`): a version chosen in the
  panel is not the tree's selection, so Delete, F2 and Ctrl+D of the tree find nothing to act on.
- **Restore this version** (built 2026-11-16) proposes the version's text as the note's new text, through the pending flow, as an
  Assistant proposal lands (§18.5). It is `Shell::restore_version` in `shell/versions.rs`:
  - the note is the active tab's, read as it is in the editor now (its unsaved edits included). A comparison tab, or a read-only
    tab, is refused with **This is a comparison: it cannot be changed.**;
  - `pending::propose(current, current, version)`: a note with pending changes is refused with **Accept or reject the pending
    changes first.** (as Improve says it); a note that is this version already says **The note is this version already.** and
    changes nothing;
  - on success, one undo step (`Shell::absorb_marked`, shared with Improve) and two `kind: auto` revisions, as an agent's write
    leaves them (`revisions.md`, 2026-11-16): the text before, by the person's author, memo `before restoring version N`; the
    restored text with its changes accepted, by the same author, memo `version N restored, pending`. A failed save of the text
    before stops the restore with one line; a failed save of the restored text is a warning, the changes stay pending;
  - then the count line: **N changes are pending in the note: accept or reject them there.** The person accepts or rejects each
    change as usual.

### 19.4 Known limits

- Line-level compare only: a changed line is one removed line and one added line. No word-level tint.
- Tables and figures inside a change get no wash, as in §17.7.
- **Copy all** on a comparison copies the text with its marks (`{--`, `{++`): the app has no reading view that resolves them. The
  same holds for a copy of the whole note with pending changes (`backlog.md`).
- The layout file does not save a comparison (it has no note). While one is shown, the group's `*` mark goes to the note it compares
  when that note is a tab of the group, else to the nearest tab that holds a note (the left one on a tie), so the next start shows
  the note the person was on.
- A date is local time from the platform's zone rules on unix. On Windows no local-time call is made yet, so the date is UTC there
  (`platform/clock.rs`, `local`).
- A restore needs the note in a tab that is not a comparison (§19.3); the note of a comparison is not restored from its panel.

## 20. Window chrome: unified sidebar (APPROVED by owner 2026-11-21; BUILT 2026-11-21, pill not yet seen on screen)

Owner decisions, 2026-11-21 (answered in chat; these are owner approvals):

- **Direction: "A — Unified sidebar (Notes/Finder style)"**: APPROVED by owner 2026-11-21. Visual reference: artifact "Inkuno Chrome Options", https://claude.ai/artifact/WEnF3NT2oTgiyY56snn4g3, option A.
- **macOS traffic lights with the title bar hidden**: always hidden, as today (`crates/inkuno-platform/src/mac_chrome.rs`), with the overlap and the header misalignment fixed by the layout below. APPROVED by owner 2026-11-21.
- **IBM Plex Serif SemiBold**: kept bundled for now, unused (`app-ui.md` §3, line 63 today). The decision is deferred. APPROVED by owner 2026-11-21 as a deferral.

### 20.1 The layout (APPROVED)

- **No separate 40px icon rail.** Today the rail is `RAIL: f32 = 40.` (`crates/inkuno/src/views/nav.rs:55`), drawn in `crates/inkuno/src/shell/sidebar.rs` from about line 1211 (the commands logo `nav-commands` at 1226, `rail-logo` at 1241, buttons `nav-tree` 1265, `nav-search` 1280, `nav-new` 1294, `rail-sidebar` 1326, `rail-titlebar` 1342).
- **Left sidebar** runs full height. Its top 38px band holds the macOS traffic lights and, right-aligned in that band, the sidebar toggle and search. Below the band: the nav sections (Files tree, Tags).
- **Right sidebar** mirrors it: the same background token, the same 38px top band (holding the Outline/Assistant switcher), the same header style, a 1px border.
- **Sidebar hidden**: a 38px strip spans the editor. It holds the lights, then the toggle, then the note title, centred.
- **Title bar hidden**: the band collapses to 12px of padding.

### 20.2 Problems it fixes (from code reading, 2026-11-21)

- The lights overlap the rail logo when the panel is collapsed. `TRAFFIC_ROOM = 78.` (`crates/inkuno/src/shell/render.rs:23`) reserves the room; `traffic_pad` (`crates/inkuno/src/shell/tabs.rs:170-180`) pads the tabs, the pane and the first dock header but not the rail; the rail top is at `crates/inkuno/src/shell/sidebar.rs:1211-1228`.
- With the title bar hidden, content runs to the top edge, and the right dock header does not line up with the left one.

### 20.3 Implementation checklist (built 2026-11-21, writer crew B)

- [x] Remove the 40px rail. The rail's column is gone: `views/nav.rs` (the `RAIL` constant is deleted, `Panel::Rail` is now `Panel::Hidden`); `shell/sidebar.rs:1161` (`render_nav`) draws the left panel alone, with its band and foot.
- [x] Where each rail action went (owner to confirm; the placement is crew B's, from §20.1):

  | Former rail button (id) | What it did | New place |
  |---|---|---|
  | `nav-commands` (logo) | opens the command palette (`>`) | foot row, first (`shell/sidebar.rs`, `sidebar-foot`); F10 still opens it |
  | `nav-tree` | shows the vault tree | foot row, second; its key (`ShowTree`) still works |
  | `nav-search` | opens the palette (find or create) | band, right-aligned, second (`search_button`); with the title bar hidden, the foot |
  | `nav-new` | new note here | band, right-aligned, third (`new_button`); with the title bar hidden, the foot |
  | `rail-sidebar` (sidebar toggle, Ctrl+B) | shows or hides the left panel | band, right-aligned, first (`sidebar_toggle`); the top strip of the note while the panel is hidden (`render.rs`, `nav-sidebar`); the foot with the title bar hidden |
  | `rail-titlebar` (Ctrl+Shift+S) | shows or hides the title bar | foot row, last (`titlebar_button`); Ctrl+Shift+S unchanged |
  | `rail-free` (empty drag part) | moves the window | the empty part of the band and of the foot (`drag_window` on the spacer only) |

  Not on the rail before, so no change: theme, back and forward, trash, sync, backup and help stay in the palette (`›` word) and on their keys.
- [x] Band height token: `BAND` 38px, `BAND_HIDDEN` 12px, `LIGHTS_ROOM` 78px, in `ui/theme.rs:11`, `:14`, `:17`. The tab bar is the band (`shell/tabs.rs:9`, `TAB_BAR = BAND`); the single-page strip is the band too (`shell/render.rs:926`, `:935`).
- [x] Mirrored right band: the switcher (one button per right tool, `shell/sidebar.rs` `switcher`, `show_right`) is the header of the right dock's first tool (`render_dock`, `switch`), the same background as the left (`render_stack`), a 1px hairline on its inner edge (the existing dock divider).
- [x] Hidden-sidebar strip: `shell/render.rs` (the strip overlay, `strip-title` and the toggle, from about line 1260) for a single page; `shell/tabs.rs` (`render_tabs`, `toggle`) when the note has tabs. Lights room, then the toggle, then the note's title centred.
- [x] Title-bar-hidden state: the bands are `BAND_HIDDEN` of padding (`shell/sidebar.rs` `band`; `shell/render.rs:935`; `shell/tabs.rs:374`). macOS lights stay hidden (decision 2, unchanged in `platform/mac_chrome.rs`).
- [x] Linux client-decoration pill: drawn in the band row (`shell/render.rs:583`, `h(BAND)`). Built, not verified on screen: see 20.4.
- [x] `traffic_pad` removed; `strip_edge` (`shell/tabs.rs:168`) gives the top-left row its edge past the toggle. `TRAFFIC_ROOM` removed from `render.rs`, `sidebar.rs`, `tabs.rs`. The lights room is only in the band at the top left (`strip_edge`), not on the welcome page.
- [x] Tests (`crates/inkuno/src/shell/tests/window.rs`): `the_top_band_has_one_height_and_one_top_edge` (1282; band height and top edge in all four states), `the_lights_room_is_only_in_the_band_at_the_top_left` (1374), `every_former_rail_action_is_still_a_button` (1452; each button clicked), and the updated `the_title_bar_hides_and_comes_back_from_the_sidebar` (1009). Layout file: `views/nav.rs` test (about line 869): an old `rail` panel word loads as hidden.
- [x] Layout file: the panel word is `hidden` (`views/nav.rs:615`); `rail` and anything unknown read as hidden (`views/nav.rs:564`). The rail key was never saved as its own key, so no key is removed.
- [x] Update §4 and §12.1 to the unified sidebar (crew F3, 2026-11-21): §4 and §12.1 above name the band and foot, not the rail.

### 20.4 Open (TBD) and what is not verified

- Where the rail actions go: placed per the table above (20.3); the owner confirms or moves them.
- Whether the band token is also the title-bar strip's height: yes, `BAND` is the title bar strip's height too (`TAB_BAR = BAND`, `render.rs:583`).
- **Not verified on screen: the Linux pill.** The sandbox has no compositor, so gpui uses server decorations and draws no pill (the app's own bar needs compositor support: `gpui-0.2.2/src/platform/linux/x11/window.rs:1528`). The screenshots show the lights-room path instead. The pill's place is covered only by the existing test `window_buttons_leave_the_header_and_the_title_clear` (`shell/tests/window.rs:329`), which sets `client = true`. A run with a compositor (xcompmgr or picom) is needed.
- **Gap: sidebar hidden and title bar hidden.** Then the band is 12px, the sidebar is gone, and the foot with its toggle is gone too. The only way back is Ctrl+B and Ctrl+Shift+S. The spec does not cover this combination. Owner decision needed: backlog W1.
- **Help text** no longer says "the rail" (crew F3, 2026-11-21): `shell/keymap.rs` names the logo at the foot of the sidebar, a view's button and the sidebar; `docs/user/src/shortcuts.md` is regenerated (`INKUNO_WRITE_DOCS=1 cargo test shortcuts_page_is_current`).
- **Right-band title duplication:** the first right tool's name is in the switcher, and its header no longer repeats it. Confirm with the owner that the switcher alone is the header.
- Screenshots (Xvfb, the server-decoration path): `target/visual/chrome/1-hidden-title-shown.png` to `6-sidebar-hidden-title-hidden.png`, and the `scripts/visual-check.sh mixed` run `target/visual/20261121-052149/`. Note that capture 6 shows the sidebar open with the title bar hidden (the script's keys reopened the sidebar before it ran); the name is wrong, the image is right.
- Macintosh is not checked: the lights path is only in the tests (`client = false`).

## 21. Lists and the visual check

- **Lists: the north star is GitHub + Typora** (APPROVED by owner 2026-11-21). The reference table and Inkuno's target (§10.3) are in `docs/design/markdown_renderer.md` §10, both APPROVED by the owner 2026-11-21. The colour and nesting of list markers follow the owner's pick there.
- **Visual check findings, 2026-11-21**, from `scripts/visual-check.sh` (screenshots confirmed by the owner): the list markers, `~~~` fences, 4-space indented code and table cells. The list, fence, indented-code and table-cell findings are built on this branch (crew F3). The open ones are in `docs/backlog.md`, Follow-ups: rendering R1 to R7, and window chrome W1 and W2.

## 22. Quiet window (2026-11-21)

The shipped design of the window, after the Quiet packages P1 to P8. Since 2026-11-23 every theme has this layout; themes set colours, fonts, sizes and spacing only. The spec and its decision log are in `docs/design/prototypes/`: `v2.html` (the prototype, approved 2026-11-21) and `README.md` (its twenty decisions, the three open questions answered, the build packages and their notes). Where this section and the prototype differ, the code is the reference; the difference is recorded here.

### 22.1 Regions

The window has four regions and nothing else:

1. **Title band** (`BAND`, 38 px; 12 px hidden): the tab strip of each pane, the window controls and the right-panel switch. The strip is the band, even for one note. Ctrl+Shift+S hides the band; a pointer on its top edge brings it back over the note.
2. **Left sidebar, files only**: Search and Today rows, the tree, a folded Scratch section at the bottom, Settings at the foot. It sits on the window background, without a panel, border or shadow. Recent notes and search live in the palette (Ctrl+P).
3. **Panes**: one or two, side by side, each with its own tabs. Preview tabs are in italics. The focused pane's active tab is solid; the other pane's tabs are dimmed. A hairline divides the panes.
4. **Right panel**: one panel with four views, Outline, Review (§23; was Comments), Versions and Assistant, chosen by an icon switcher in the band; one view at a time, 280 px wide. Ctrl+Shift+O opens the outline, Ctrl+Alt+1 to 4 pick a view.

The note is a flat sheet, flush with the top, right and bottom edges of the window, without gutters, rounded corners or borders. Its left edge casts one faint shadow (the only structural exception).

### 22.2 The empty page (Quiet)

A centred column of three cards, **Today**, **New note** and **Search**, each 200 px wide with a 1 px border at 8 % of the text, 10 px radius, 14 px padding, 10 px gaps and the hover fill. The title is 13 px medium in the text colour; the description is 12 px in the soft colour. Under them, **From a month ago** with up to three recall cards: the title, then the note's first body line (headings skipped, Markdown markers removed, one line with an ellipsis), or the title alone when the note has none. Cards with no vault: New note creates a scratch note. The other themes keep their rows (`render_empty_rows`). Code: `shell/render.rs` (`render_empty`, `render_cards`), `inkuno-logic/src/recall.rs` (`first_line`, the cap of three).

### 22.3 Quiet tokens

Defined per theme in `fixed_extras` (`ui/theme.rs`), and derived for the other themes, for `Light` and `Dark`. Each is a semantic token of `Theme`, and nothing else in the UI sets a colour for Quiet:

| Token | Meaning |
|---|---|
| `link` | text of a link and a tag |
| `ground` | the window background under the sheets |
| `syntax` | the six code colours (comment, keyword, type, function, string, number) |
| `todo_box` | the border of an unchecked task box (text at 40 %) |
| `dots` | the dots of a rule (text at 40 %) |
| `quote_bar` | the bar beside a quote |
| `chip`, `chip_line` | the fill and the 1 px rim of inline code |
| `code_line`, `gutter`, `gutter_num` | the box, the line-number gutter and its numbers of a code block |
| `th` | a table's header fill |
| `ink`, `table_alpha`, `ring_alpha` | the table's grid and outline, and an image's ring: `ink` at the alpha |
| `alerts` | the five alert tints (NOTE, TIP, IMPORTANT, WARNING, CAUTION), made legible on their fill |
| `popover` | the fill of menus and the palette |
| `caution` | the red of destructive items |

### 22.4 Dark pass (2026-11-21)

Every fixture of `scripts/visual-check.sh` was rendered with `Dark` pinned, and the code, table and media pages were re-checked after the last change. Text must reach 4.5:1 against the sheet, the code block and the gutter. Decorative lines stay subtle hairlines, close to the light theme's (about 1.4 to 1.6:1 on the sheet), except the quote bar, which is a meaningful mark and reaches 3:1. Fixes (Quiet Dark only):

| Token | Was | Now | Ratio before → after | Why |
|---|---|---|---|---|
| `gutter_num` | `#5b5d60` | `#868686` | 2.49 → 4.52 | line numbers are text |
| `quote_bar` | `#4a4c4f` | `#676767` | 1.98 → 3.01 | a meaningful mark |
| `code_line` | `#2e2f31` | `#36373a` | 1.27 → 1.43 | hairline of a code box |
| `chip_line` | `#353638` | `#3a3b3e` | 1.41 → 1.35 | hairline rim of inline code |
| `table_alpha` | 0.09 | 0.12 | 1.29 → 1.43 | table grid, hairline |
| `ring_alpha` | 0.08 | 0.12 | 1.26 → 1.43 | image ring, hairline |

The first pass set the decorative lines to 3:1 (`#6a6a6a`, `#6f6f6f`, alpha 0.34); the owner judged that too heavy, and they were softened to the values above. No theme test pinned the old values.

Already at or above target, unchanged: body and secondary text, the five syntax colours (4.9 to 7.8), `link`, `caution`, the alert tints, the task box (`todo_box`, 3.3), the rule dots (3.3), the highlight (`Theme::highlight` enforces 4.5). Light themes were not changed.

### 22.5 Status

- Built: P1 (title band, tabs), P5 (menus, palette, slash menu), P6 (links), P7 (comments as `passage<!-- comment: text -->`; pending changes beside the note in `.inkuno/pending`), P8 (empty page, dark pass, these docs).
- P4 (Settings tab: Ctrl+, opens Appearance, Editor, Files, Shortcuts, About; F1 opens Shortcuts; the help overlay is gone) is merged.
- Not yet merged when this section was written: P2 (sidebar) and P3 (right panel). The user guide describes them as the prototype specifies, and the lines that depend on them are listed in the P8 report.
- Open questions of the prototype (Q1 to Q3) are answered as in its log. The gap in §20.4 (sidebar and title bar both hidden) is still open in backlog W1.

---

## 23. Review feed (APPROVED 2026-11-24, not built)

> Status: agreed design, recorded 2026-11-24. Not built. It replaces the Comments view and the
> pending-change cards of §22.1. Until it lands,
> the code is as §17 and §22 describe. Decision log: `revisions.md`, 2026-11-24.

### 23.1 One panel

- One right-dock panel, **Review · N open**, replaces the Comments view and the pending-change cards.
  N counts the open comments and pending changes of the note. 280 px (`PANEL_W`,
  `crates/inkuno/src/views/nav.rs:60`). The editor stays the primary region.
- No filter row. Count only. No "show resolved".

### 23.2 The feed

- One scrollable feed, oldest first.
- Sort key: a comment's timestamp, or a pending change's new `created` field (UTC, set when the
  change is proposed). `at` is a text position, not a time. Items without a time (legacy comments,
  pending changes without `created`) sort first, in document order.
- Only one card is expanded at a time.
- Selecting a card highlights its anchored passage in the editor and scrolls to it.
- No full diff in a collapsed card.

Collapsed card:

- Author: avatar initial and name.
- Type icon: comment, replace, insert or delete.
- Relative time.
- Content: at most 3 lines.
- Location: `Section · Line N`. It is computed at render from the text anchor and never stored.

Expanded comment:

- Full text, replies, a reply field, and **Resolve**.

Expanded change:

- Full old and new text, and **Accept** / **Reject**.

### 23.3 Resolve

- **Resolve** deletes the comment from the note: its markup goes, the passage text stays.
- Undo and the revisions recover it. There is no "show resolved".

### 23.4 Comment markup

The markup travels with the `.md` file. A reply is a line inside the same block.

```text
passage<!-- comment @name 2026-11-24T10:02Z: text ↳ @name 2026-11-24T10:05Z: reply -->
```

- The comment block, replies included, stays on one physical line, so file line numbers are stable.
- Replies are separated by ` ↳ `. The panel shows each reply as its own row.
- A literal `↳` in text is escaped as `\↳`.
- The time is UTC, to the minute (`Z`).
- A legacy `<!-- comment: text -->` reads as author `Unknown`, with no time.
- A legacy comment is rewritten in the new form only when it is edited, never on read. (Reading still migrates CriticMarkup to `<!-- comment: -->`, as before.)

### 23.5 Pending changes

- Storage unchanged (§17): changes stay in `.inkuno/pending/<note>.json` in the vault and sync with it.
- Each list entry gains `created`: the UTC time the change was proposed (`2026-11-24T10:02Z`). It is used only for feed order. `at` stays a text position.
- Entries without `created` sort with untimed items (§23.2).
- **A run is one card.** Two or more pending changes in a row with the same author and the same `created` minute (one
  Improve proposes them together; `logic::review::runs`) show as one card, **N changes**, with the first change's text.
  Expanded, each change has its own Accept and Reject, and **Accept all N** / **Reject all N** act on that run only
  (`Editor::decide_changes`: applied last to first, one undo step; the others stay pending). The title's count stays per change.

### 23.6 Text

- All interface text goes through `tr("English", "Français")`.

### 23.7 Line numbers

- The editor gutter shows file line numbers: the lines of the `.md` on disk, the same numbers as the feed's `Line N`, git and the CLI. Lines hidden by the editor (front matter, comment markup, table syntax) leave gaps; accepted.
- A wrapped line shows its number on its first visual row only.
- Own column left of the block and list drag handles (`blocks.rs` gutter). Not selectable, not copied.
- Washed out: the theme's faint text colour, about 0.85× body size, right-aligned. The current line is one step darker. No other decoration.
- On by default. A settings toggle hides it (text via `tr()`).
- When on, Quiet's per-code-block line numbers (`layout.rs` gutter) are not drawn: file lines only.

---

## 24. iPhone app (APPROVED by owner 2026-11-24)

The v1 iPhone UI. Native SwiftUI app. Prototype: `docs/design/prototypes/v3-ios.html`.
Build decisions live in `docs/design/ios.md`.

- Navigation: Home is the NavigationStack root. Cards (Write here… opens Today), a 2×2 grid
  (Today, New note, Search, Notes), then From a month ago with recall cards. The tree pushes:
  folders (icon, count, chevron), then notes (icon, title, age), then a Scratch section.
  Empty folders say "This folder is empty." No tab bar. Pull-down or search opens the palette.
- Editor: live-styled Markdown. Only the caret line shows raw Markdown (Settings toggle).
  A tap on a link opens it: a note opens on the stack, a URL opens the browser, a heading
  scrolls there. A tap on a #tag opens the palette with it. A tap on an @date opens that day.
  An accessory bar sits above the keyboard: / # [[ task indent outdent undo hide.
- Comments in the editor: a commented passage is washed, and its line has a gold dot in the
  right inset. A tap on the passage opens a popover with the comment, its replies and Resolve.
- Callouts draw as tinted boxes with a label. A wide table scrolls sideways inside its box.
- Slash sheet: medium detent, 3-column grid. Heading, Bulleted, Numbered, Task, Quote, Code,
  Table, Divider. No Math, no Mermaid.
- Palette: full sheet with a search field. Empty query shows 6 recent notes. Groups:
  Recent, Names, Content. A ">" prefix runs commands: Formatting (Bold, Italic, Heading, for an
  open note), This note, Notes, Window. Window has Accept all changes and Reject all changes
  when the note has pending changes. A "Create «q»" row creates the note. No match says "No matches".
  Pull-down opens the palette on Home and in the tree. The tree's "+" button makes a new note.
- Panel sheet: bottom sheet (medium/large detents) with four segments. Outline lists indented
  headings; tap scrolls there ("No headings" when empty). Comments show the passage quote,
  who·time, text, replies and Resolve; adding is long-press on a block only. Versions has "Save version"
  and the list; tap asks to confirm the restore, current saved first ("No saved versions yet").
  Backlinks lists linking notes ("No notes link here yet").
- Pending review: a bottom pill ("N changes · Review") opens a sheet with Reject all / Accept all
  and one card per change (old struck, new). Swipe leading accepts, trailing rejects.
  The Accept and Reject confirmations show as a toast after the sheet closes. No inline washes.
- Context menus are long-press. Tree row: Rename, Move, Duplicate, Copy link, separator,
  Move to Trash (destructive). Block: the long-press that selects a block's text opens the
  edit menu, which holds Comment, Show Markdown (Edit as Markdown for a table), Copy, Duplicate,
  separator, Delete (destructive). There is no separate block context menu.
- Settings is a Form. Notes folder (change). Appearance: Theme Light/Dark/System segmented,
  Text size 14/15/16/18, Page width narrow/normal/wide. Editor: "Show Markdown on the live line"
  toggle. Files: Trash (count) opens the Trash list. About: version.
- Trash is a list with Restore buttons. The palette command "Restore from Trash" restores too.
  A note goes back to the folder it came from: the origin is recorded in
  `.inkuno/trash-origin/<name>.txt` when it is trashed. Move to Trash shows an Undo toast.
- Themes: Light, Dark, System only. The desktop's other themes are out of v1.
- Type: Inter (fallback system), JetBrains Mono for code (fallback SF Mono). Note body 16 pt,
  line-height 1.6. H1 1.9em/700, H2 1.45em/650, H3 1.15em/600, H4–H6 1em/600. Rows are at
  least 44 pt. Large nav title 34 pt/700. Push .32 s, sheets .25–.3 s. Motion goes through InkMotion, which honours Reduce Motion.
- Tokens: the full table is the prototype. iPhone-only additions: toast + toast-ink, add +
  add-ink, del + del-ink, comment wash + comment dot, selection highlight, scrim. Dark
  code-line/gutter-num/quote use the softened §22.4 values.
- Out of v1: Compare/diff screen, inline pending washes, table cell editing, Mermaid and math
  rendering (shown as code), Focus mode, tar.gz backup, the other themes, Assistant,
  text-selection comments.

## 25. HTML pages (macOS: a tab)

- An HTML page of the vault opens as a tab on macOS: a native webview is laid over the tab's area (`views/html_page.rs`, `inkuno_view::embed`).
- Elsewhere it opens in its own window, `inkuno view` (`shell/notes.rs`, `open_html_in_viewer`), as before.
- Why: gpui does not run the GTK loop that the webview needs on Linux, and Windows has no viewer, so the separate window stays the fallback there.
- A tab out of sight hides its webview each frame; the page is not restored at startup, as before.
