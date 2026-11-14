# Keybindings — source, contract, changelog

Status: implements `docs/design/app-ui.md` §11 (which stays the WHY; this doc is the
HOW and the log). Ground truth for WHAT the keys are is code + the generated
page, never this doc: if this doc and `defaults()` disagree, `defaults()` wins.

## 1. Source of truth + regen flow

- `defaults()` in `crates/inkuno/src/shell/keymap.rs:326` is the whole default table. Order
  matters: for the same context, **the last binding wins** (GPUI overlay order;
  `line::bindings` in `crates/inkuno/src/ui/line.rs:239` is appended after, so input-line keys
  take precedence over the fields that use them).
- Display, parsing, and user overrides live in `crates/inkuno/src/ui/keys.rs`:
  `canon`/`spell`/`show` (§7), `meets` (`:133`), `effective` (`:153`),
  `refusal` (`:180`), `system` (`:191`), `clashes` (`:203`), `read`/`write`
  (`:227`/`:220`), `shown` (`:258`).
- What the help panel shows is `help_sections()` (`crates/inkuno/src/shell/keymap.rs:8`).
- `docs/user/src/shortcuts.md` is GENERATED from `help_sections` — do NOT hand-edit.
  Regenerate with `make docs` (`INKUNO_WRITE_DOCS=1 cargo test shortcuts_page_is_current`
  writes the page, then `mdbook build`). The test
  `shortcuts_page_is_current` (`crates/inkuno/src/shell/tests/shortcuts_page.rs`) fails until
  the page matches. CONTRIBUTING flow: change `help_sections`/`defaults()`,
  run `make docs`, commit both.
- Which actions can be re-bound at all is gated by `editable()`
  (`crates/inkuno/src/shell/keymap.rs:320`): app/palette/nav/outline/search/table actions are
  open; view-internal caret, gesture and menu plumbing is not.

## 2. Principles (from app-ui.md §11.1, one line each + what each costs)

1. **Calm, standard chords win** — one calm chord per intent; browser/OS
   conventions beat clever ones. Cost: power users get fewer chords, not more.
2. **Palette-first discoverability** — `Ctrl+P` notes, `F10`/`Ctrl+Shift+P`
   commands, `F1`/`Ctrl+/` help. Anything chordless MUST be typed-reachable
   (`› word`, `/` menu, grips). A feature with neither chord nor palette entry
   is a bug, not minimalism.
3. **Modifier families, desktop dialect** — `Ctrl+Shift` = vault/panel scope,
   `Alt` = act on the thing under the cursor, `Ctrl+Enter` = act in place (§3).
4. **Selection-first (helix)** — select-then-act (`Ctrl+D`, grips, lit
   format-bar buttons); `Esc` always collapses back.
5. **Chords are stable API (nvim)** — renames need a deprecation path (§8),
   never a silent swap. `make docs` + help panel keep code, book, screen in sync.
6. **macOS rule** — this doc writes the Linux/Windows form (`Ctrl+*`); on macOS
   read `Cmd` for `Ctrl` (§5).

## 3. Modifier contract

| Family | Scope | Examples |
|---|---|---|
| `Ctrl+Shift+*` | Vault / panel / list scope | `Ctrl+Shift+F` search vault, `S` sync, `O` outline, `P` commands, `Z` redo, `B` list-cycle, `M` comment, `C` copy block/selection, `N` new folder, `D` new diagram, `J` calendar, `T` reopen tab |
| `Alt+*` | Move / act on the thing under the cursor | `Alt+Up/Down` move block (row in a table), `Alt+=` autosum, `Alt+Enter` open embed, `Alt+Left/Right` back/forward, `Alt+Shift+arrows` travel panes |
| `Ctrl+Enter` | Confirm / act in place | Task toggle, palette open-aside, find-bar replace-all, sheet insert-below |
| Plain `Ctrl+letter` | The note itself | `Ctrl+I/U` styles, `Ctrl+Alt+B` bold, `Ctrl+F/R` find, `Ctrl+D` next occurrence, `Ctrl+K` link; `Ctrl+B` is the sidebar (§8) |

**Context splits (intentional, not bugs).** Two bindings may share keys when
their contexts never meet (`meets()`, `crates/inkuno/src/ui/keys.rs:133`: a binding with no
context or `Shell` meets everything; two views never meet each other, except a
`Line` field with its carrying view). Documented cases:

- `Ctrl+D` ×3: `editor::SelectNext` (Editor) / `nav::Duplicate` (Nav) /
  `canvas::Duplicate` (Diagram). Help shows three rows; only one can fire.
- `Ctrl+Shift+Enter` ×2: `ReplaceInVault` (Find bar) / `sheet::InsertAbove`
  (CSV grid).
- `Ctrl+\` / `Ctrl+Alt+\`: split pane right / below.
- `Ctrl+=` + `Ctrl++`: both zoom in (layout coverage).
- `Ctrl+R` in note vs find bar: `FindReplace` is bound in BOTH `Editor` and
  `Find` contexts so the bar and the note share it; `nav::ShowRecent` is NOT
  among them (see §8).
- `TabToLeft/Right/Up/Down`, `EvenPanes`, `MoveTool`, `ToolAsTab`:
  chordless-by-design (palette commands, §6); calm over completeness.

## 4. Binding tables (mirror of `help_sections` groups)

Notation: keys in Linux/Windows form; context `—` = everywhere (`None`),
`Shell` = the whole window. `Rebindable` = in `editable()`. `Palette` = how to
type it (`› word` = command palette, `Ctrl+P › word` = notes palette command
mode). Keyless rows (`—`) are grips/`/`/menu-only by design (§6).

### Typing

| Keys | Action | Context | Rebindable | Palette |
|---|---|---|---|---|
| (typing) | `editor::Type` | Editor | no | — |
| `Backspace` / `Delete` (+ `Ctrl` word variants) | `editor::Backspace/Delete/DeleteWordLeft/Right` | Editor | no | — |
| arrows, `Home/End`, `Ctrl+Home/End`, `PageUp/PageDown`, `Shift`+… select variants | caret + selection | Editor | no | — |
| `Ctrl+A` | `editor::SelectAll` | Editor | no | — |
| `Enter` / `Tab` / `Shift+Tab` | `editor::Newline/Indent/Outdent` | Editor | no | — |

### Editing (context `Editor`, all rebindable, `Format`/`Note` palette groups)

| Keys | Action | Palette |
|---|---|---|
| `Ctrl+Alt+B` / `Ctrl+I` / `Ctrl+U` | `editor::Bold/Italic/Underline` | `› bold/italic/underline` |
| `Ctrl+Shift+X` | `editor::Strike` | `› strikethrough` |
| `Ctrl+Shift+H` | `editor::Highlight` | `› highlight` |
| `Ctrl+Shift+K` | `editor::InlineCode` | `› inline code` |
| `Ctrl+K` | `editor::InsertLink` | `› link` |
| `Ctrl+Enter` | `editor::ToggleTask` | `› tick the task` |
| `Ctrl+Shift+B` | `editor::ListCycle` | `›` (catalogue `Format`; help-rebindable) |
| `Ctrl+D` | `editor::SelectNext` | `› multicursor: select the next occurrence` (alias, `crates/inkuno/src/shell/palette_open.rs:211`) |
| `Alt+click` | add cursor (mouse) | — |
| `Ctrl+F` / `Ctrl+R` / `F3` / `Shift+F3` | `editor::Find/FindReplace/FindNext/FindPrev` | `› find/replace in the note` |
| `Ctrl+Shift+M` | `editor::Comment` | `› comment` |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | `editor::Copy/Cut/Paste` | `› copy/cut/paste` (table group in grids) |
| `Ctrl+Z` / `Ctrl+Shift+Z`, `Ctrl+Y` | `editor::Undo/Redo` | `› undo/redo` |
| — | `editor::FindFromSelection` (`Ctrl+E`) | — (chord exists, no palette line; help-rebindable) |
| `Ctrl+Shift+A` | `editor::CopyResults` (block/table results) | — |
| `Ctrl+Shift+C` | `editor::Copy` (block) | — |
| `Ctrl+V` (after copy) | `editor::PasteUpdate` (paste, renumbering) | — |

Column alignment (context `Editor`, rebindable, `Table` palette group):

| Keys | Action | Palette |
|---|---|---|
| `Ctrl+Shift+L` / `E` / `R` | `editor::AlignLeft/AlignCenter/AlignRight` | `› align/center the column…` (aliases, `palette_open.rs:211`) |
| `Alt+=` | `editor::TableAutoSum` | `› write a total…` |
| `Ctrl+Shift+Backspace` | `editor::TableDeleteSpan` | `› delete the table/row/column` family |

Find bar extras (context `Find`): `Enter` first match, `Tab` switch field,
`Alt+C/W/R` case/word/regex toggles, `Ctrl+Enter` replace-all,
`Ctrl+Shift+Enter` replace-in-vault.

### Tables in a note (context `Editor`, all rebindable)

Chorded: `Alt+Up/Down` move row, `Alt+=` autosum, `Ctrl+Shift+L/E/R` align,
`Ctrl+Shift+Backspace` delete span, `Ctrl+Enter` task toggle in cells.
Keyless-by-design (grip / `+` / `›` command, `Table` group first when the
cursor is in a table via `here_group`, `crates/inkuno/src/shell/help.rs:223`): insert row
above/below, insert column left/right, move row/column, duplicate row/column,
sort A→Z / Z→A, transpose, autosum, copy-with-results, delete row/column/table,
select row/column, table CSV import/export, delimiter/encoding/header.

### Blocks (context `Editor`, keyless, rebindable)

Turn into heading 1/2/3, bulleted/numbered/task list, quote, code block, text;
move block up/down (`Alt+Up/Down` chorded); duplicate; delete. Discoverability:
`/` slash menu + grip in-canvas (blessed path) plus `›` aliases as an equal
path (overrule 2026-11-08, see §6), plus `Block` palette group
(`here_group` returns `Block` in a note).

### CSV / TSV grid (context `Sheet`)

Arrows/`Tab`/`Enter` move, `F2` edit, `Esc` cancel, `Backspace/Delete` clear,
`Ctrl+C/X/V/A/Z` (+`Shift+Z`/`Y` redo), `Ctrl+Enter` insert below,
`Ctrl+Shift+Enter` insert above, `Ctrl+Delete` delete rows,
`Shift+Space` select row, `Ctrl+Space` select column,
`Ctrl+Shift+Backspace` delete span. Picks (delimiter/encoding/header/copy-as/export):
palette-only, `›` + help-rebindable.

### Navigation

| Keys | Action | Context | Palette |
|---|---|---|---|
| `Ctrl+O` | `nav::Open` (go to file) | Nav | `›` |
| `Ctrl+E` | `nav::ShowTree` | Shell | `› vault tree` |
| — | `nav::ShowRecent` | Shell | `› recent notes` (Ctrl+P opens on recent) |
| `Ctrl+T` | `nav::ShowTags` | Shell | `› tags` |
| `Ctrl+G` | `nav::ShowGraph` | Shell | `› graph of the notes` |
| `Ctrl+L` | `nav::ShowLinks` | Shell | `› backlinks` |
| `Ctrl+Shift+J` | `nav::ShowCalendar` | Shell | `› calendar` |
| `Ctrl+Shift+F` | `app::SearchVault` | — | `› search the whole vault` |
| `Alt+Left/Right` | `app::GoBack/GoForward` | — | `› back/forward` |
| `Ctrl+Shift+N` | `nav::NewFolder` | Shell | `› new folder` |
| `F2` | `nav::Rename` | Nav | — |
| `Ctrl+D` | `nav::Duplicate` | Nav | — |
| `Ctrl+Backspace` (macOS `Cmd+Backspace`) | `nav::Trash` | Nav | — |
| `Ctrl+N` | `app::NewScratch` (scratch note, no vault needed) | — | `› new scratch note` |
| — | `nav::ShowScratch` (the Scratch panel: scratch notes, newest first) | — | `› scratch notes` |
| `Ctrl+Alt+N` | `app::NewNote` (note in the tree's folder, in the vault) | — | `› new note in folder` |
| `Ctrl+J` / `Ctrl+O` | `app::Today/OpenVault` | — | `› today/change vault` |
| `Ctrl+Shift+D` | `app::NewDiagram` | — | `› new diagram` |
| `Ctrl+Shift+S` | `app::ToggleTitleBar` (the bar the app draws, Linux: shown or hidden; its buttons come back from the rail) | — | `› title bar: show or hide it` |
| `Ctrl+M` | `nav::ToggleFull` (panel full-window) | Shell | `› panel on the whole window` |
| `Ctrl+B` | `app::ToggleSidebar` (left dock shown or hidden, the rail stays; works in the note) | — | `› sidebar: show or hide the side panel` |

Tree/list keys (context `Nav`): arrows move, `Right/Left` fold/unfold,
`Enter` open, `Ctrl+Enter` preview (peek), `Ctrl+Up` parent, `Ctrl+F` search,
`Ctrl+P` focus search, `F1` help, `F10` commands, `Ctrl+D` duplicate,
`Ctrl+Z/Y` undo/redo, `Ctrl+X/C/V`, `Ctrl+A`, month `PageUp/PageDown`,
`Esc` close.

### Panes, tabs, docks

| Keys | Action | Context | Palette |
|---|---|---|---|
| `Ctrl+Shift+T` | `app::ReopenTab` | — | (help row, rebindable) |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | `app::NextTab/PrevTab` | — | — |
| `Ctrl+Shift+PageUp/PageDown` | `app::MoveTabLeft/MoveTabRight` | — | — |
| `Ctrl+W` | `app::ClosePane` (tab; saves first) | — | — |
| `Ctrl+\` / `Ctrl+Alt+\` | `app::SplitPane/SplitDown` | — | `› split the pane…` |
| `Alt+Shift+arrows` | `app::PaneLeft/Right/Up/Down` | — | — |
| `Alt+Shift+Enter` | `app::MaximizePane` | — | `› pane on the whole window` |
| `F9` | `app::ToggleFocus` (focus mode: the note alone, edge to edge; `F9` or `Esc` in the note goes back) | — | `› focus mode` |
| `Ctrl+1` / `Ctrl+2` | `app::FocusLeft/FocusRight` | — | — |
| `Ctrl+Enter` | `palette::ConfirmAside` | Palette | — |
| — | `app::TabToLeft/Right/Up/Down`, `EvenPanes`, `MoveTool`, `ToolAsTab` | — | `› move the tab…/even out/move the panel…/open as tab` |

### Diagrams (context `Diagram`/`Canvas`)

`Ctrl+A` select all, `Ctrl+D` duplicate, `Delete/Backspace` erase,
`Ctrl+Z/Y` undo/redo, arrows nudge, `F2` edit text, `Enter` confirm,
`Esc` cancel, zoom `Ctrl+=/-/0` (global).

### Appearance (`Look` group)

`Ctrl+=` / `Ctrl++` bigger, `Ctrl+-` smaller, `Ctrl+0` default (85–150%);
`› theme…` (no chord). Font/mono/page/width/bar/animations: `›` settings only.

### Notes + App

`Ctrl+Shift+O` outline (`› outline of the note`); `Ctrl+P` palette (notes,
`#tag` filter, `› word` commands, create); `F10`/`Ctrl+Shift+P` commands;
`F1`/`Ctrl+/` help (rebindable in place); `Ctrl+Q` quit; `Ctrl+S` save now.

## 5. macOS translation

`secondary` = `Cmd` on macOS, `Ctrl` elsewhere (`crates/inkuno/src/ui/keys.rs` `spell`/`show`;
`MOD` constant). This doc's `Ctrl+*` reads as `Cmd+*` on macOS, except:

| Linux/Windows | macOS | Why |
|---|---|---|
| `Alt+Left/Right` (back/forward) | `Cmd+[` / `Cmd+]` | browser convention |
| `Alt+Shift+arrows` (pane travel) | `Cmd+Alt+arrows` | `Alt+arrows` is word motion |
| `Alt+arrows` (word motion) | `Ctrl+arrows` | macOS word-motion convention (`word` in `defaults()`) |
| `Cmd+Left/Right/Up/Down` etc. | line/doc ends + selection | macOS text conventions (`crates/inkuno/src/shell/keymap.rs`, `#[cfg(target_os = "macos")]` tail of `defaults()`) |
| `Ctrl+Backspace` (trash) | `Cmd+Backspace` | macOS convention |

`system()` (`crates/inkuno/src/ui/keys.rs:191`) flags OS-reserved chords to warn, not refuse.

## 6. Palette-alias convention

- `› word` (or `Ctrl+P › word`): type what the action DOES; matching is
  per-word prefix on the folded label (`command()`, `crates/inkuno/src/views/palette.rs:333`),
  in both languages (`tr()` labels).
- Catalogue entries live in `commands::catalog()` (`crates/inkuno-logic/src/commands.rs:56`);
  the palette shows only actions the current view offers
  (`palette_commands()`, `crates/inkuno/src/shell/palette_open.rs:211`), with the view's
  group first (`arrange()`): `Block` in a note, `Table` in a table
  (`here_group()`, `crates/inkuno/src/shell/help.rs:223`), `Diagram` on a drawing, CSV picks
  on a grid.
- In command queries, catalogue `editor::Table*`/`sheet::Pick*` lines are
  hidden as doubled (`doubled()`, `crates/inkuno/src/views/palette.rs:654`): the same gestures
  are offered as grid items when a table is under the cursor.
- **Aliases** (same function; most name actions the catalogue does not):
  `editor::SelectNext` → `› multicursor…` (`Note` group, everywhere the editor
  chord works); `editor::AlignLeft/Center/Right` → `› align/center the
  column…` (`Table` group, only in a table). Keys badges come from the bindings
  in effect, so aliases never rot when chords change.
- **Block transforms ARE `›` aliases too** (overrule 2026-11-08 of the
  `/`+grip-only stance): heading 1/2/3, bulleted/numbered/task list, quote, code
  block, duplicate block (`editor::BlockHeading1/2/3`, `BlockBullet`,
  `BlockOrdered`, `BlockTask`, `BlockQuote`, `BlockCode`, `BlockDuplicate`;
  `Block` group, `palette_open.rs` aliases, deduped against the catalogue so no
  action lists twice). `/` + grip stay; `›` is an equal path.

## 7. Editable mechanism (help panel + `keys` file)

- Any help line naming an action can be re-bound in place (help `F1`, type to
  filter, `Enter` a new chord). Gate: `editable()` (`crates/inkuno/src/shell/keymap.rs:320`).
- `refusal()` (`crates/inkuno/src/ui/keys.rs:180`): single keys / `Shift`-only letters type
  text (`Types`); bare `Esc` belongs to the panel (`Panel`); unreadable stays
  unreadable. `system()` flags OS-kept chords as warnings.
- `clashes()` (`:203`): who already has these keys where this action applies
  (`meets()` contexts). The panel offers Replace (takes them) or keeping both
  when contexts never meet.
- `effective()` (`:153`): a choice replaces the default in EACH of the action's
  contexts, at the place it held (equal context: last wins); a choice wins over
  a changeable default holding the same keys (that action goes keyless until
  re-bound). Empty choice = explicitly unbound.
- The `keys` file: one `action=keys` line per changed action (`write()`); read
  back fault-tolerant (`read()`): unknown/unchangeable actions, unreadable or
  refused keys, keys taken by unchangeables are set aside as faults, defaults
  stay. Swaps (A takes B's keys and vice versa) are allowed.

## 8. Changelog + deprecation policy

- Policy (from §11.1-P5): chords are stable API. A rename keeps the old chord
  working ≥1 release, announced in help + palette; never a silent swap.
  Hand-written guide pages (`docs/user/src/navigation.md`, `tabs-and-panes.md`)
  follow in the same change.
- **2026-11-08 — T-fix (accepted): `Ctrl+Shift+T` = reopen last closed tab**
  (browser standard). `nav::ShowRecent` loses its chord entirely: recent notes
  stay the default `Ctrl+P` view and keep the `› recent notes` command + help
  row (rebindable). `Ctrl+Alt+T` freed (reserved, see §9). Fallout updated in
  the same change: `tabs.rs`, `save_close.rs` (reopen chord),
  `end_to_end/panel.rs` + `find_bar.rs` (recent via palette/action).
- **2026-11-08 — aliases, no new chords:** `› multicursor: select the next
  occurrence`, `› align/center the column…` (×3) in `palette_commands()`.
  Table row/column/sort/transpose/delete + autosum were already catalogue
  `Table` commands; block transforms (heading 1/2/3, bulleted/numbered/task,
  quote, code block, duplicate) gained `›` aliases by overrule of the
  `/`-menu-first decision (same `Block` rules, deduped against the catalogue).
- **2026-11-08 — focus mode on `F9`:** `app::ToggleFocus` (new action, no new
  `Mode`) folds the left dock onto its rail, empties the right dock through
  `hide_tool`, shuts the comments, centers the note; `F9` again — or `Esc`
  in the note — restores the stashed chrome. Palette alias `› focus mode`
  (`Look` group, `palette_open.rs`); help row in `Navigation`, next to
  `nav::ToggleFull`. Binds the §9 reservation, with the §1 proposal this line is.
- **2026-11-08 — docs sweep (coherence pass, docs-only, no commit):** §4
  Blocks wording fixed to match the §6 overrule (`›` aliases an equal path
  next to `/` + grip); book tint wording + T-chords fixed in
  `tabs-and-panes.md` / `navigation.md` / `settings.md`; mockup 720 → 800
  annotations. `docs/user/src/shortcuts.md` untouched — regen owned by Linux CI
  (`make docs` can't run on macOS).

- **2026-11-14 — scratch notes on `Ctrl+N`, the vault note moves to `Ctrl+Alt+N`:** `app::NewScratch`
  (new) opens a scratch note in a tab, the cursor in it, with no vault needed. Its file is plain Markdown
  in the app's own folder (`inkuno/scratch` under the config folder, never in the vault), written as it is
  typed, named by its moment (`2026-11-14-103200`, a number after a collision). `app::NewNote` keeps its
  name and its behaviour (the note filed in the tree's folder, in the vault) and its chord moves to
  `Ctrl+Alt+N`: a chord is stable API, so `Ctrl+N` now means something else for the same action name.
  Closing a scratch note with text (`app::ClosePane`) asks in a list: Keep as scratch (default, Enter),
  Keep in vault… (only with a vault), Discard; Esc cancels. The Scratch panel (`nav::ShowScratch`,
  palette only, like Recent) lists them by title with their moment; its menu has Open, Keep in vault…
  and Delete (`Delete` key too: the file goes, an open note leaves with its tab; no `.trash` for
  scratch notes). Tests:
  `scratch.rs` (`ctrl_n_writes_a_scratch_note_and_no_vault_file`, `ctrl_n_works_without_a_vault`,
  `ctrl_alt_n_still_creates_a_vault_note`). Tests that made vault notes with `Ctrl+N` now use `Ctrl+Alt+N`:
  `end_to_end/panel.rs`, `tabs.rs`, `window.rs`, `save_close.rs`.

- **2026-11-14 — sidebar on `Ctrl+B`, bold moves to `Ctrl+Alt+B`:** `app::ToggleSidebar` (new) shows or
  hides the left dock (`Panel::Rail` ↔ `Split`, saved in the layout file) from any view, the note
  included, so the note keeps the keys. `editor::Bold` moves to `Ctrl+Alt+B`; `Ctrl+B` in the note no
  longer makes text bold. Help row in `Navigation`, palette `› sidebar`. Tests: `keyboard.rs`
  (`ctrl_b_toggles_the_sidebar_from_the_note`), `end_to_end/history_panes.rs` (menu hint).
- **2026-11-14 — title bar on `Ctrl+Shift+S`:** `app::ToggleTitleBar` (new) hides or shows the bar the
  app draws on Linux (window buttons and the top strip of a note that stands alone). The choice is
  saved in the layout file (`titlebar=hidden`). The rail's foot keeps two switches, sidebar and title
  bar, so the bar always comes back from there. macOS and Windows draw no bar of their own here, so
  the key changes nothing on screen there: gpui 0.2.2 sets the native bar at window creation only.
  `Ctrl+Shift+S` was free: the sync chord (`app::SyncVault`) left with the November trim, and its row
  is replaced. Tests: `window.rs` (`the_title_bar_hides_and_comes_back_from_the_rail`).

- **2026-11-14 — focus mode on `F9` (built; supersedes the 2026-11-08 focus proposal where they differ):**
  `app::ToggleFocus` hides the rail, the panel, the right dock, the tab bar and the title bar, and the note
  takes the window edge to edge. Owner brief 2026-11-14: the rail and the title bar go too (the 2026-11-08
  line kept the rail). It is a view flag (`Shell::focus_mode`, not saved): nothing of the chrome is changed
  to hide it, so `F9` again, or `Esc` in the note (`editor::Cancel` propagates when nothing is open), shows
  the same layout as before. Edge to edge only in this mode: in normal mode the 8 px gaps stay (§10 item 4,
  `window.rs` sheets test). Palette line moved from `palette_open.rs` to the catalogue (`commands.rs`).
  Tests: `window.rs` (`focus_mode_gives_the_note_the_window_and_restores_the_chrome`).

## 9. Reserved / gaps log

- `F9`: BOUND 2026-11-08 to `app::ToggleFocus` (focus mode: docks hidden, note
  centered; `F9` or `Esc` exits; `› focus mode`). Was the focus/zen chord candidate
  (§11.3-Q2); the §1 proposal this line is. (Only `F1/F2/F3/F9/F10` are taken today.)
- `Ctrl+Alt+T`: freed by the T-fix; do NOT reassign casually — it is the
  fallback if the T-fix ever regresses.
- Outline dock (§11.3-Q1): no chord decision. `Ctrl+Shift+O` stays palette
  jump-to-heading; a future dock reuses or splits per a new proposal.
- Format-bar toggle chord: deferred to Phase 2 reskin (bar stays SHOWN,
  app-ui.md §10 lock).
- Kanban `Space` tick + `Alt+arrows` carry stay `Board`-only by design; the
  help panel groups them under Diagrams — revisit grouping only, not chords.
- Remaining stale reference outside this sweep's scope:
  `crates/inkuno/src/shell/pane.rs` doc comments (`Ctrl+Alt+T`) — book follow-ups fixed by
  the 2026-11-08 docs sweep (`tabs-and-panes.md`, `navigation.md`).

## 10. Strip-down cross-ref (2026-11-08)

`app-ui.md` §13 (Strip-down v1) takes graph / calendar / tags / recent / links /
search out of the main UI but NOT out of this doc: every stripped mode keeps
its palette entry and (except Recent) its chord, so the tables above stay the
contract. Bring-back = re-show, never re-bind. (Added by the docs worker;
sibling changelog §§8–9 untouched.)
