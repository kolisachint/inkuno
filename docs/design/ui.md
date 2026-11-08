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
- **Current state (Inkuno):** the bundled fonts are **IBM Plex Sans + IBM Plex Mono**
  (`src/ui/assets.rs:16`), exposed as `sans()` (`src/ui/assets.rs:38`) and
  `mono()` (`src/ui/assets.rs:42`), overridable from prefs
  (`src/ui/assets.rs:70`). → **RESKIN**: swap the bundled families to
  Inter + JetBrains Mono; the override mechanism stays.
- Fallback chain must cover CJK (current code notes Plex has no CJK glyphs,
  `src/ui/assets.rs:46`).

### Sizes (target, px at scale 1)

| Role      | Size | Line-height | Current state |
|-----------|------|-------------|---------------|
| Body      | 16   | 1.7         | Default body size is 16 (`src/logic/prefs.rs:24`); line-height TBD — verify in `src/views/editor/style.rs` |
| H1        | 30   | TBD         | TBD — verify heading scale in editor style |
| H2        | 23   | TBD         | TBD |
| H3        | 19   | TBD         | TBD |
| UI chrome | 13–14| —           | UI scale follows body size: `ui = size/16` clamped 0.85–1.5 (`src/ui/theme.rs:118`, `src/ui/theme.rs:116`) |
| Code      | 14   | TBD         | TBD — verify code run sizing |
| Metadata (counts, hints) | 12 | — | Secondary text uses `soft` color token, WCAG AA 4.5:1 (`src/ui/theme.rs:77`) |

### Measure + spacing (target)

- **Doc max-width: 800 px** at body 16.
- **Paragraph spacing: 16 px** between blocks.
- **Current state (Inkuno):** page widths are Narrow 560 / Normal 720 / Wide 960 /
  Full (`src/ui/theme.rs:49-51`), width follows zoom
  (`Page::measure`, `src/ui/theme.rs:44-54`), default Normal
  (`src/logic/prefs.rs:28`, `Page::of` fallback `src/ui/theme.rs:34`). 800 px
  sits between Normal and Wide → **TBD (user decides)**: adjust Normal to 800,
  or add the 800 target as the new default. Do not invent; see §6.
- Interface spacing unit is the rem at UI scale (`src/shell/render.rs:27-28`
  `window.set_rem_size(t.px(16.))`).

### Color + shape (target: quiet neutrals, one accent)

- Three themes: **light, dark, sepia**. Warm paper for sepia (new).
- **Current state (Inkuno):** 13 built-in themes in `THEMES`
  (`src/ui/theme.rs:129`), picked from prefs or system appearance
  (`Theme::of`, `src/ui/theme.rs:134`), with WCAG-AA enforcement for text,
  accent, and callouts (`src/ui/theme.rs:300-330` approx; tests
  `every_theme_reads`, `every_pair_reads` in `src/ui/theme.rs:290+`).
  → **RESKIN**: keep the `Theme` engine + contrast tests; curate the default
  offering down to light/dark/sepia (whether the other 10 stay as extras: **TBD**).
- Radii: small 6, tab pill 9, large 8, sheet 12 (`src/ui/theme.rs:133-155`
  approx: `small()`, `tab()`, `large()`, `sheet()`). Keep.
- Shadows: `lifted` for floating bars, `floating` for palette/menus
  (`src/ui/theme.rs:150-162` approx). Keep; no new elevation levels without §1 flow.
- All tokens centralize in `src/ui/theme.rs` (`Theme` struct,
  `src/ui/theme.rs:100-124`); views must not hard-code colors/sizes.
  (Inkuno rule: layers depend downwards only, `ARCHITECTURE.md:19-27`.)

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
  `Nav::left_shown`, `src/shell/docks.rs:70-75`; tree state `src/views/nav.rs`,
  drawing `src/shell/sidebar.rs`). Default: visible, narrow rail + panel.
- **Center region — canvas.** The note(s): tab bar + editor, arranged by the
  pane tree (`render_layout`, `src/shell/render.rs:990`; placed by
  `logic/layout.rs`, `ARCHITECTURE.md:48`). Always present; it is the focus.
- **Right region — outline.** Document outline (headings of the active note)
  plus comments panel. Current: outline opens through the palette
  (`open_outline`, `src/shell/palette_open.rs:29-38`); the comments panel sits
  right of everything (`src/shell/render.rs:88-90` approx); right-dock stacking
  via `Nav::right_shown` (`src/shell/docks.rs:42-51`). → Outline as a
  persistent right-dock view: **TBD** (design proposal needed via §1 flow).
- **Independently toggleable.** Panel states Rail / Split / Full
  (`Panel`, `src/views/nav.rs:95-99`); full-panel toggle
  (`toggle_full`, `src/shell/sidebar.rs:64-70`); closing the last left tool
  collapses to rail (`src/shell/docks.rs:118-120`); right dock renders only
  when non-empty (`src/shell/docks.rs:47-51`). Every region must be hideable
  so the canvas can own the window (zen: both docks hidden).
- **Small-screen rules.** When panes crowd, the window promotes one group to
  alone/full (`alone()`, `crowded_in()`, `src/shell/pane.rs:467-474`;
  `crowded()`, `src/shell/pane.rs:30`); the side panel keeps `NOTE_MIN`
  (`src/views/nav.rs:35`) room for the note and floors the panel at 120 px
  (170 with calendar) (`src/shell/sidebar.rs:88-95`). Keep this behavior;
  exact narrow breakpoints (< 700 px etc.): **TBD** — measure, don't invent.

## 5. Component + interaction standards

Verdict per component: **KEEP** (as-is) · **RESKIN** (same behavior, new look) ·
**HIDE** (exists but off/quiet by default). Reason required in each row.

| Component | Today (with citation) | Verdict | Reason |
|-----------|----------------------|---------|--------|
| Tabs + tab bar | Tab bar 34 px high (`src/shell/tabs.rs:6`); tab pills radius 9 (`src/ui/theme.rs:tab()`); cycle/reorder/close actions (`src/actions.rs:6`: `NextTab/PrevTab/MoveTabLeft/...`) | KEEP | Multi-note work needs tabs; restyle only via tokens |
| Panes / splits | Split right/down, move tabs between panes, even/maximize/zoom (`split`, `src/shell/pane.rs:604`; `toggle_zoom`, `src/shell/pane.rs:457`; actions `SplitPane/SplitDown/MaximizePane`, `src/actions.rs:6`) | KEEP | Power layout; quiet by default (single pane) |
| Command palette | Views `src/views/palette.rs`; opens for notes/commands/outline/questions (`ARCHITECTURE.md:53`, `src/shell/palette_open.rs`); ranked note search (`ARCHITECTURE.md:108-111` approx, Palette section) | KEEP | The keyboard-first core; central to vision |
| File tree / explorer | `views/nav.rs` state + `shell/sidebar.rs` drawing (`ARCHITECTURE.md:90`); left-dock stacking (`src/shell/docks.rs:42-51`) | KEEP | Vault navigation; collapses to rail |
| Outline nav | Palette list of headings (`src/shell/palette_open.rs:29-38`); action `Outline` (`src/actions.rs:6`) | KEEP + TBD dock view | Jump-to-heading is vision (§4 right region); persistent view needs proposal |
| Graph view | Force-directed, off-UI-thread (`src/views/graph.rs:261-263`); a `Mode` in nav | HIDE (tool, not default) | Useful but anti-chrome-quiet; open on demand only |
| Calendar / Tags / Recent / Links | Nav modes (`src/views/nav.rs`; toggles `ShowRecent/ShowTags/ShowCalendar/ShowLinks`, `src/shell/render.rs:150-170` approx) | HIDE (tools, not default) | Same as graph: available, never forced |
| Format bar | Bottom-of-note bar (`src/views/editor/format_bar.rs`, `ARCHITECTURE.md:90`); pref `bar`, `off` hides it (`src/logic/prefs.rs:17-18`; `Theme::bar`, `src/ui/theme.rs:122`) | HIDE by default | Vision: no permanent toolbar; keep pref, default `off` (**TBD** user confirms default) |
| Focus / typewriter modes | **Not in codebase** (no matches for typewriter/focus-mode) | Vision-only, TBD | Typora parity wants them; needs full §1 proposal |
| Word / char counts | **Not found** (no counter in shell/editor) | TBD | Vision wants quiet counts; placement (status/footer) needs proposal |
| Auto-save | `EditorEvent::Changed` → `dirty=true` → 400 ms debounce → `flush` (`src/shell/notes.rs:491-510`); atomic writes, crash never truncates (`src/platform/vault.rs:889`) | KEEP | Already local-first; surface a subtle saved indicator (proposal) |
| Session restore | Layout persisted on change (`panes_state`, `src/shell/render.rs:137-140`); vault backup/restore archives (`src/platform/vault.rs:590`, `:703`) | KEEP + TBD | Layout restore exists; unsaved-text-after-crash recovery: **TBD** — verify `dirty` tabs reload path |
| Zoom (text) | `ZoomIn/Out/Reset` (`src/actions.rs:6`); `resize_text` (`src/shell/settings.rs:96`); page width follows size (`src/ui/theme.rs:44-54`) | KEEP | Accessibility + vision zoom |
| Themes UI | `ChooseTheme` (`src/actions.rs:6`); 13 themes (`src/ui/theme.rs:129`) | RESKIN | Curate to light/dark/sepia per §3 |
| Motion | `motion` pref, instant when off (`Theme::motion`, `src/ui/theme.rs:124`) | KEEP | Respects reduced-motion spirit |
| Help / shortcuts | Help panel + generated shortcuts page (`ARCHITECTURE.md:95`, `ARCHITECTURE.md:99`) | KEEP | Keyboard-first needs discoverability |
| Toasts / update banner | `render_toasts`, `render_update_banner` (`src/shell/render.rs`) | KEEP, quiet | Transient only; never modal for routine info |

## 6. User preferences (EVOLVING — the user edits this section directly)

This section is explicitly unfinished. The user changes values here; agents
implement what it says. Current preference keys (all in `src/logic/prefs.rs`):

| Key | Today | User's choice |
|-----|-------|---------------|
| `theme` | theme name, else system (`src/ui/theme.rs:134`) | TBD — pick default (sepia? light?) |
| `font` / `mono` | family names, default Plex (`src/logic/prefs.rs:7-8`, `src/ui/assets.rs:70`) | TBD — Inter / JetBrains Mono (see §3) |
| `size` | body px, default 16 (`src/logic/prefs.rs:24`) | 16 (confirm) |
| `page` | narrow/normal/wide/full (`src/logic/prefs.rs:53`) | TBD — 800 px target vs Normal 720 (see §3) |
| `wide` | wide-table behavior (`src/logic/prefs.rs:53`) | TBD |
| `bar` | `off` hides format bar (`src/logic/prefs.rs:17-18`) | default `off` (confirm) |
| `motion` | `off` = instant (`src/logic/prefs.rs:39`) | TBD |
| `updates` | update checks on/off (`src/logic/prefs.rs:50-51`) | TBD |

Wanted (not yet prefs — proposals via §1): sepia theme definition; default
docks open/closed; outline pinned or not; focus/typewriter toggles; keybinding
overrides file/format (**TBD** — check `src/shell/keymap.rs:326` defaults and
whether user overrides exist).

## 7. Acceptance criteria

From the vision (§2); each must hold before calling the UI work done:

1. [ ] **The document is the focus.** With both docks hidden and the bar off,
      the window is the note and nothing else (Panel Rail/Full paths,
      `src/views/nav.rs:95-99`).
2. [ ] **Unified editing.** No edit/preview mode split; typing renders in place
      (custom editor element, `ARCHITECTURE.md:103-111` approx, Editor section).
3. [ ] **Source preserved.** Round-trip: file on disk unchanged except the
      user's keystrokes (atomic write `src/platform/vault.rs:889`; merge rule
      `ARCHITECTURE.md:100`).
4. [ ] **Offline.** No network needed for any editing/viewing path (vault is a
      local folder; update check is opt-out background, `src/logic/prefs.rs:50-51`).
5. [ ] **Consistent fonts.** Inter for UI/body/headings, JetBrains Mono for
      code, everywhere (replacing `src/ui/assets.rs:16` defaults).
6. [ ] **Hideable sidebars.** Left and right regions toggle independently and
      the choice persists (`settle_nav` saves layout, `src/shell/sidebar.rs:73-82`).
7. [ ] **Three themes.** Light, dark, sepia; all text ≥ 4.5:1 (extend the
      `every_theme_reads` test family, `src/ui/theme.rs:290+`).
8. [ ] **Large-doc responsive.** Typing cost stays per-damaged-line
      (`views/editor/layout.rs` + `paint.rs`, `ARCHITECTURE.md:91`); tables >
      1500 rows use light layout (`ARCHITECTURE.md:105-107` approx).
9. [ ] **Auto-save + recovery.** 400 ms debounce flush (`src/shell/notes.rs:504-510`)
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
   shell → views → ui → logic (`ARCHITECTURE.md:19-27`).
5. **No new dependencies without asking** (`ARCHITECTURE.md:133`). Fonts ship
   in the binary like today (`src/ui/assets.rs:26-28`).
6. **No untranslated interface text.** All UI strings go through `tr()`
   (`ARCHITECTURE.md:131`).
7. **No invented behavior.** If the code doesn't show it and this doc doesn't
   approve it, it's TBD — not implemented.
8. **No drive-by refactors.** Touch only what the approved proposal names.

## 9. Visual reference — Markscribe mockup (PRIMARY)

File: `Downloads/a_clean_minimal_modern_markdown_editor_ui_mockup.png`
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
   Touch `src/ui/theme.rs`, `src/ui/assets.rs`, `views/editor/style.rs`,
   `views/editor/render.rs`. Shell keeps working as-is until Phase 2.
2. **Inkuno power features are KEEP + reskin, none hidden:** tabs, panes,
   palette, tree, graph, docks, calendar/tags all stay, dressed in §3 tokens.
3. **Mockup (§9) wins ties** against the written brief on look/feel questions.
4. **Deferred to Phase 2** (2026-11-08, keep e2e green): format-bar default stays
   shown — flipping it moves the bottom-right corner button that
   `boards.rs`/`cursors_comments.rs` click at `height-31`; paragraph spacing
   stays as laid out (a blanket Text pad would also gap list items);
   Normal page measure stays 720, not 800 — 800 caused a real 5 px traffic-light
   overlap in 860 px windows (`window.rs:497`) and shrank table-wrap distances
   (`md_tables.rs:176`). Proper 800 needs PILL_CLEAR 48→56+, wrap-threshold
   rework, and a full suite re-verify.
5. **Phase 2 plan: Normal 800px (proposal, not yet implemented):**
   - `src/ui/theme.rs:50` Normal base 720→800; theme.rs width test to px(800)/px(1600).
   - `src/views/editor/mod.rs:180` PILL_CLEAR px(48)→px(56) (pill bottom is 53px in 860 windows; check render.rs:434-444 at ui>1).
   - `src/views/editor/mod.rs:181,489-490` MAX_WIDTH px(720)→px(800).
   - `src/shell/tests/window.rs:480` narrow gate re-derives (threshold 848); asserts keep logic.
   - `src/shell/tests/end_to_end/md_tables.rs:176` wrap threshold recalibrate (~15-25 or longer filler) + dependent offsets :178-185.
   - Leave shell-chrome click tests (boards, cursors_comments) untouched, re-verify only.
   - Re-verify order: window test, page-width test, md_tables/boards/cursors_comments/tags, then full suite.
