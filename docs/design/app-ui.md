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
  (`crates/inkuno/src/ui/assets.rs:16`), exposed as `sans()` (`crates/inkuno/src/ui/assets.rs:38`) and
  `mono()` (`crates/inkuno/src/ui/assets.rs:42`), overridable from prefs
  (`crates/inkuno/src/ui/assets.rs:70`). → **RESKIN**: swap the bundled families to
  Inter + JetBrains Mono; the override mechanism stays.
- Fallback chain must cover CJK (current code notes Plex has no CJK glyphs,
  `crates/inkuno/src/ui/assets.rs:46`).

### Sizes (target, px at scale 1)

| Role      | Size | Line-height | Current state |
|-----------|------|-------------|---------------|
| Body      | 16   | 1.7         | Default body size is 16 (`crates/inkuno-logic/src/prefs.rs:24`); line-height TBD — verify in `crates/inkuno/src/views/editor/style.rs` |
| H1        | 30   | TBD         | TBD — verify heading scale in editor style |
| H2        | 23   | TBD         | TBD |
| H3        | 19   | TBD         | TBD |
| UI chrome | 13–14| —           | UI scale follows body size: `ui = size/16` clamped 0.85–1.5 (`crates/inkuno/src/ui/theme.rs:118`, `crates/inkuno/src/ui/theme.rs:116`) |
| Code      | 14   | TBD         | TBD — verify code run sizing |
| Metadata (counts, hints) | 12 | — | Secondary text uses `soft` color token, WCAG AA 4.5:1 (`crates/inkuno/src/ui/theme.rs:77`) |

### Measure + spacing (target)

- **Doc max-width: 800 px** at body 16.
- **Paragraph spacing: 16 px** between blocks.
- **Current state (Inkuno):** page widths are Narrow 560 / Normal 800 / Wide 960 /
  Full (`crates/inkuno/src/ui/theme.rs:49-51`), width follows zoom
  (`Page::measure`, `crates/inkuno/src/ui/theme.rs:44-54`), default Normal
  (`crates/inkuno-logic/src/prefs.rs:28`, `Page::of` fallback `crates/inkuno/src/ui/theme.rs:34`). 800 px
  landed 2026-11-08 as the Normal measure (option (a), see §14).
- Interface spacing unit is the rem at UI scale (`crates/inkuno/src/shell/render.rs:27-28`
  `window.set_rem_size(t.px(16.))`).

### Color + shape (target: quiet neutrals, one accent)

- Three themes: **light, dark, sepia**. Warm paper for sepia (new).
- **Current state (Inkuno):** 13 built-in themes in `THEMES`
  (`crates/inkuno/src/ui/theme.rs:129`), picked from prefs or system appearance
  (`Theme::of`, `crates/inkuno/src/ui/theme.rs:134`), with WCAG-AA enforcement for text,
  accent, and callouts (`crates/inkuno/src/ui/theme.rs:300-330` approx; tests
  `every_theme_reads`, `every_pair_reads` in `crates/inkuno/src/ui/theme.rs:290+`).
  → **RESKIN**: keep the `Theme` engine + contrast tests; curate the default
  offering down to light/dark/sepia (whether the other 10 stay as extras: **TBD**).
- Radii: small 6, tab pill 9, large 8, sheet 12 (`crates/inkuno/src/ui/theme.rs:133-155`
  approx: `small()`, `tab()`, `large()`, `sheet()`). Keep.
- Shadows: `lifted` for floating bars, `floating` for palette/menus
  (`crates/inkuno/src/ui/theme.rs:150-162` approx). Keep; no new elevation levels without §1 flow.
- All tokens centralize in `crates/inkuno/src/ui/theme.rs` (`Theme` struct,
  `crates/inkuno/src/ui/theme.rs:100-124`); views must not hard-code colors/sizes.
  (Inkuno rule: layers depend downwards only, `docs/design/architecture.md:19-27`.)

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
  drawing `crates/inkuno/src/shell/sidebar.rs`). Default: visible, narrow rail + panel.
- **Center region — canvas.** The note(s): tab bar + editor, arranged by the
  pane tree (`render_layout`, `crates/inkuno/src/shell/render.rs:990`; placed by
  `logic/layout.rs`, `docs/design/architecture.md:48`). Always present; it is the focus.
- **Right region — outline.** Document outline (headings of the active note)
  plus comments panel. Current: outline opens through the palette
  (`open_outline`, `crates/inkuno/src/shell/palette_open.rs:29-38`); the comments panel sits
  right of everything (`crates/inkuno/src/shell/render.rs:88-90` approx); right-dock stacking
  via `Nav::right_shown` (`crates/inkuno/src/shell/docks.rs:42-51`). → Outline as a
  persistent right-dock view: **TBD** (design proposal needed via §1 flow).
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
| Tabs + tab bar | Tab bar 34 px high (`crates/inkuno/src/shell/tabs.rs:6`); tab pills radius 9 (`crates/inkuno/src/ui/theme.rs:tab()`); cycle/reorder/close actions (`crates/inkuno/src/actions.rs:6`: `NextTab/PrevTab/MoveTabLeft/...`) | KEEP | Multi-note work needs tabs; restyle only via tokens |
| Panes / splits | Split right/down, move tabs between panes, even/maximize/zoom (`split`, `crates/inkuno/src/shell/pane.rs:604`; `toggle_zoom`, `crates/inkuno/src/shell/pane.rs:457`; actions `SplitPane/SplitDown/MaximizePane`, `crates/inkuno/src/actions.rs:6`) | KEEP | Power layout; quiet by default (single pane) |
| Command palette | Views `crates/inkuno/src/views/palette.rs`; opens for notes/commands/outline/questions (`docs/design/architecture.md:53`, `crates/inkuno/src/shell/palette_open.rs`); ranked note search (`docs/design/architecture.md:108-111` approx, Palette section) | KEEP | The keyboard-first core; central to vision |
| File tree / explorer | `views/nav.rs` state + `shell/sidebar.rs` drawing (`docs/design/architecture.md:90`); left-dock stacking (`crates/inkuno/src/shell/docks.rs:42-51`) | KEEP | Vault navigation; collapses to rail |
| Outline nav | Palette list of headings (`crates/inkuno/src/shell/palette_open.rs:29-38`); action `Outline` (`crates/inkuno/src/actions.rs:6`) | KEEP + TBD dock view | Jump-to-heading is vision (§4 right region); persistent view needs proposal |
| Graph view | Force-directed, off-UI-thread (`crates/inkuno/src/views/graph.rs:261-263`); a `Mode` in nav | HIDE (tool, not default) | Useful but anti-chrome-quiet; open on demand only |
| Calendar / Tags / Recent / Links | Nav modes (`crates/inkuno/src/views/nav.rs`; toggles `ShowRecent/ShowTags/ShowCalendar/ShowLinks`, `crates/inkuno/src/shell/render.rs:150-170` approx) | HIDE (tools, not default) | Same as graph: available, never forced |
| Format bar | Bottom-of-note bar (`crates/inkuno/src/views/editor/format_bar.rs`, `docs/design/architecture.md:90`); pref `bar`, `off` hides it (`crates/inkuno-logic/src/prefs.rs:17-18`; `Theme::bar`, `crates/inkuno/src/ui/theme.rs:122`) | HIDE by default | Vision: no permanent toolbar; keep pref, default `off` (**TBD** user confirms default) |
| Focus / typewriter modes | **Not in codebase** (no matches for typewriter/focus-mode) | Vision-only, TBD | Typora parity wants them; needs full §1 proposal |
| Word / char counts | **Not found** (no counter in shell/editor) | TBD | Vision wants quiet counts; placement (status/footer) needs proposal |
| Auto-save | `EditorEvent::Changed` → `dirty=true` → 400 ms debounce → `flush` (`crates/inkuno/src/shell/notes.rs:491-510`); atomic writes, crash never truncates (`crates/inkuno/src/platform/vault.rs:889`) | KEEP | Already local-first; surface a subtle saved indicator (proposal) |
| Session restore | Layout persisted on change (`panes_state`, `crates/inkuno/src/shell/render.rs:137-140`); vault backup/restore archives (`crates/inkuno/src/platform/vault.rs:590`, `:703`) | KEEP + TBD | Layout restore exists; unsaved-text-after-crash recovery: **TBD** — verify `dirty` tabs reload path |
| Zoom (text) | `ZoomIn/Out/Reset` (`crates/inkuno/src/actions.rs:6`); `resize_text` (`crates/inkuno/src/shell/settings.rs:96`); page width follows size (`crates/inkuno/src/ui/theme.rs:44-54`) | KEEP | Accessibility + vision zoom |
| Themes UI | `ChooseTheme` (`crates/inkuno/src/actions.rs:6`); 13 themes (`crates/inkuno/src/ui/theme.rs:129`) | RESKIN | Curate to light/dark/sepia per §3 |
| Motion | `motion` pref, instant when off (`Theme::motion`, `crates/inkuno/src/ui/theme.rs:124`) | KEEP | Respects reduced-motion spirit |
| Help / shortcuts | Help panel + generated shortcuts page (`docs/design/architecture.md:95`, `docs/design/architecture.md:99`) | KEEP | Keyboard-first needs discoverability |
| Toasts / update banner | `render_toasts`, `render_update_banner` (`crates/inkuno/src/shell/render.rs`) | KEEP, quiet | Transient only; never modal for routine info |

## 6. User preferences (EVOLVING — the user edits this section directly)

This section is explicitly unfinished. The user changes values here; agents
implement what it says. Current preference keys (all in `crates/inkuno-logic/src/prefs.rs`):

| Key | Today | User's choice |
|-----|-------|---------------|
| `theme` | theme name, else system (`crates/inkuno/src/ui/theme.rs:134`) | TBD — pick default (sepia? light?) |
| `font` / `mono` | family names, default Plex (`crates/inkuno-logic/src/prefs.rs:7-8`, `crates/inkuno/src/ui/assets.rs:70`) | TBD — Inter / JetBrains Mono (see §3) |
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
      user's keystrokes (atomic write `crates/inkuno/src/platform/vault.rs:889`; merge rule
      `docs/design/architecture.md:100`).
4. [ ] **Offline.** No network needed for any editing/viewing path (vault is a
      local folder; update check is opt-out background, `crates/inkuno-logic/src/prefs.rs:50-51`).
5. [ ] **Consistent fonts.** Inter for UI/body/headings, JetBrains Mono for
      code, everywhere (replacing `crates/inkuno/src/ui/assets.rs:16` defaults).
6. [ ] **Hideable sidebars.** Left and right regions toggle independently and
      the choice persists (`settle_nav` saves layout, `crates/inkuno/src/shell/sidebar.rs:73-82`).
7. [ ] **Three themes.** Light, dark, sepia; all text ≥ 4.5:1 (extend the
      `every_theme_reads` test family, `crates/inkuno/src/ui/theme.rs:290+`).
8. [ ] **Large-doc responsive.** Typing cost stays per-damaged-line
      (`views/editor/layout.rs` + `paint.rs`, `docs/design/architecture.md:91`); tables >
      1500 rows use light layout (`docs/design/architecture.md:105-107` approx).
9. [ ] **Auto-save + recovery.** 400 ms debounce flush (`crates/inkuno/src/shell/notes.rs:504-510`)
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
   in the binary like today (`crates/inkuno/src/ui/assets.rs:26-28`).
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
   Touch `crates/inkuno/src/ui/theme.rs`, `crates/inkuno/src/ui/assets.rs`, `views/editor/style.rs`,
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
   Normal page measure is now 800 (option (a) landed, item 5 below);
   pill clearance went PILL_CLEAR 48→56 with it. Inset crew owns
   `render.rs` + `layout.rs` + `shell/mod.rs`: any traffic math depending on
   the width there is their follow-up, verified against `window.rs:497` and
   `md_tables.rs:176` on a green tree.
5. **Phase 2 plan: Normal 800px (LANDED 2026-11-08, option (a)):**
   - `crates/inkuno/src/ui/theme.rs:50` Normal base 720→800; theme.rs width test to px(800)/px(1600). ✅
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
>   the same context, the last binding wins). Display/parsing: `crates/inkuno/src/ui/keys.rs`.
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
| `Ctrl+B` / `Ctrl+I` / `Ctrl+U` | Bold / italic / underline |
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
| `Ctrl+N` / `Ctrl+J` / `Ctrl+O` | New note / today's note / change vault |
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
- TBD: any always-visible footer/status bar (word count etc.) is MOCKUP-ONLY
  (§5: RESKIN — word counts not found in app; §7 acceptance keeps cursor
  context). Do not present mockup status text as shipped.

### 12.4 Outline (BUILT 2026-11-08: right dock; palette jump retired)

- CURRENT: right-dock list of the note's headings (`Mode::Outline`,
  `crates/inkuno/src/views/nav.rs`), available but hidden by default (`right=[outline]`,
  `right_shown=[]`); `Ctrl+Shift+O` / palette `› outline` toggles it
  (closed opens + takes the keys, open takes the keys, Escape goes back to
  the note — `toggle_outline`, `crates/inkuno/src/shell/docks.rs`); click jumps the cursor
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
| Outline dock | TBD (§12.4) | Palette jump `Ctrl+Shift+O` stays | Persistent dock needs a §1 proposal (§11.3-Q1) |
| Comments | DOT | Collapsed to dot | Click dot / `› comments` / flip `comments_shut` |
| Format bar | HIDDEN-VISUALLY | Invisible but present, toggle clickable | Palette `› formatting bar`; Phase-2 chord TBD |
| Counts | QUIET | No numbered badges (comment count → dot) | Status-bar counts stay mockup-only (§12.3); any footer count needs a §1 proposal |
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

- Inter Variable (upright + italic) + JetBrains Mono Variable (upright +
  italic) bundled under `crates/inkuno/assets/fonts/` with OFL notices
  (`LICENSE-Inter.txt`, `LICENSE-JetBrains-Mono.txt`); proof + sources in
  `typography.md` §3.
- Staged in code as `INTER_FILES` / `JETBRAINS_MONO_FILES`
  (`crates/inkuno/src/ui/assets.rs:326-343`), **not** in `FONT_FILES` (`crates/inkuno/src/ui/assets.rs:22`)
  — Plex stays the bundled default; `apply_fonts` (`crates/inkuno/src/ui/assets.rs:62-67`)
  already prefers an installed Inter / JetBrains Mono.
- Bundling test `north_star_fonts_are_bundled` (`crates/inkuno/src/ui/assets.rs:355`);
  `cargo check` clean for `assets.rs` (see Q3 note on unrelated tree reds).
- Blockers for activating the staged fonts: gpui 0.2 cannot embolden a
  variable font (`crates/inkuno/src/ui/assets.rs:10-12`); weight/size tokens live in
  `theme.rs` (tabs crew) — see `typography.md` §3.

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
  variables, or wait for a gpui upgrade with synthetic bold? (Blocks
  activating the staged fonts.)
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

- Built: `Ctrl+B` sidebar (Bold moves to `Ctrl+Alt+B`), `Ctrl+Shift+S` title bar (Linux; macOS/Windows keep the system bar, gpui 0.2.2 sets it at window creation only), `F9` focus mode (rail, docks, tab bar and title bar hidden; note edge to edge; the 8 px gaps of normal mode stay). Two rail switches at its foot. Decisions and tests: `keybindings.md` §8.
