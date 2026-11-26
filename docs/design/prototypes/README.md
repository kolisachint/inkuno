# Prototypes and design log

Clickable HTML prototypes of the Inkuno window, made to agree on the design before it is built in GPUI.
Each version is a single self-contained file: open it in a browser. A version is never edited once a
newer one exists; changes go into the next version and are logged here. When a decision lands in the
app, it moves into `app-ui.md` and this log says where.

Ground rules for every version:

- **GitHub Markdown only.** Nothing the window offers may write syntax that github.com does not render.
- **Quiet look, Inkuno behaviour.** Colours, type and spacing follow the Quiet theme (`ui/theme.rs`,
  `app-ui.md` §3); keys and workflows follow `shell/keymap.rs` unless this log changes them.
- **Prototype limits.** Note text is not editable; actions that would touch files show a short toast.

| Version | Date | File | Published |
|--|--|--|--|
| v1 | 2026-11-21 | `v1.html` | https://claude.ai/artifact/JoPD7oYaJom8B39UeCn8eV |
| v2 | 2026-11-21 | `v2.html` | same link, artifact version 2 |
| v3 | 2026-11-22 | `v3.html` | https://claude.ai/artifact/JM2rwBkZdjCzSuN3fhQTpM (the v1/v2 link keeps showing v2) |
| v3-ios | 2026-11-23 | `v3-ios.html` | https://claude.ai/artifact/68n8SgCXhVCDZft9GCEbgh |
| v4-ios | 2026-11-26 | `v4-ios.html` | https://claude.ai/artifact/6nNK2eL2oJ3vEFquVjaHPB |

The artifact link always shows the newest version; older versions stay in this folder and in the
artifact's own version history.

---

## v1: first proposal (2026-11-21)

Quiet.md's structure with Inkuno's editing.

- Grey sidebar straight on the window ground; one white sheet, flush right, faint left shadow.
- Header: breadcrumb, becoming tabs when more than one note is open; Version history, Outline, More.
- Click a line to show its Markdown; other lines show the result.
- Blocks: 17 px rounded to-do boxes, code with line numbers, GitHub alerts as tinted boxes, dotted rule.
- Search (⌘K) with snippets and `>` commands; slash menu with only GitHub-safe blocks; selection
  toolbar without underline; right-click menus with red, undoable delete.
- Settings as a page that took over the sidebar; version history as a drawer.

**Feedback (owner):** keep Inkuno's hideable title bar; files open as tabs; the right side should hold
version history, outline, assistant and comments as separate views of one unified panel; check every
other part of the app; log and document every prototype.

---

## v2: unified window (2026-11-21)

Built from an inventory of what the app does today (`shell/`, `views/nav.rs`, `keymap.rs`).

### Regions

The window has four regions and nothing else:

1. **Title bar** (38 px band): the tab strip of each pane, window controls, the right-panel toggle.
2. **Left sidebar**: files only.
3. **Panes**: one or two, side by side, each with its own tabs.
4. **Right panel**: one panel, four views.

### Decisions

| # | Area | Today (code) | v2 | Why |
|--|--|--|--|--|
| 1 | Title bar | Shown; Ctrl+Shift+S hides it down to a 12 px strip | Same key and strip. When hidden, pointing at the strip slides the bar back over the note until the pointer leaves | Hidden must not mean lost: tabs stay one gesture away |
| 2 | Focus mode | F9 hides docks, tabs, title bar; Esc leaves | Same; the note alone on the sheet, centred | Unchanged behaviour, Quiet surface |
| 3 | Tabs | Per pane; preview tab marked with a trailing • | Always a tab strip in the title bar, even for one note. Preview tab in *italics*, replaced by the next single click; editing or double-click keeps it | • usually means unsaved; italics is the common convention |
| 4 | Breadcrumb | None | Dropped from the band; the folder shows as the tab's tooltip and in the window title | Tabs and a breadcrumb compete for one line |
| 5 | Split panes | Ctrl+\ right, Ctrl+Alt+\ below, Ctrl+1/2 focus | Same keys. The focused pane's active tab is solid; the other pane's tabs are dimmed. A hairline divides the panes | Focus shown without an outline box |
| 6 | Left sidebar modes | Tree, Recent, Search, Outline, Scratch, Versions, Assistant | **Files only**: Tree, with a collapsible *Scratch* section at the bottom. Recent and search live in ⌘K/Ctrl+P. Outline, Versions and Assistant move right | One job per side: left is *where*, right is *about this note* |
| 7 | Sidebar keys | Ctrl+B toggles | Kept (v1 used ⌘\\, which is Inkuno's split key; fixed) | Keep Inkuno's keymap |
| 8 | Right panel | Outline and Assistant stack in a dock; Versions is a left mode; comments are a column beside the note | **One right panel** with an icon switcher: Outline, Comments, Versions, Assistant. One view at a time, same width (280 px). Ctrl+Shift+O opens Outline; Ctrl+Alt+1–4 pick a view; the band button toggles the panel | One place for everything about the current note |
| 9 | Comments | CriticMarkup `{==passage==}{>>comment<<}`, shown raw on GitHub | Panel view lists comment cards; the passage is underlined in the note with a dot in the margin; resolve removes it. **Proposed syntax: `passage<!-- comment: text -->`**, which GitHub hides | CriticMarkup breaks the GitHub rule. Needs owner decision (Q1) |
| 10 | Versions | Ctrl+Alt+S saves with a name; compare opens a read-only tab; restore is a proposal | Same flow, from the Versions view: list newest first, *Save version* button, clicking one opens a compare tab `Week plan · v2 ↔ now` with additions green and removals struck red, and *Restore* | Matches the app; moves the list to the right panel |
| 11 | Assistant | Dock panel plus a round button at the note's bottom right; changes arrive as pending marks | Assistant view in the panel (actions, prompt). Its changes show in the note as pending washes with Accept / Reject on each, and a foot pill "2 changes pending · Accept all · Reject all". The round button is dropped | One entry point; pending changes stay visible where they apply |
| 12 | Formatting | Bottom format bar removed; chords and palette only | Selection toolbar: Text ▾, Bold, Italic, Strike, Code, Link, **Comment** | Discoverable without a permanent bar |
| 13 | Settings | Palette choice lists (Theme…, Font…, Page width…) | A **Settings tab** (Ctrl+,) with Appearance, Editor, Files, Shortcuts, About. The palette entries stay and jump into it | A tab keeps the sidebar and panes as they are |
| 14 | Shortcuts | Help overlay F1, editable keys | Settings › Shortcuts, opened by F1; keys editable there | One home for settings |
| 15 | Empty pane | Today card and recall cards | Kept: *Today*, *New note*, *Search*, and up to three "from a month ago" cards | Already good |
| 16 | Wiki links `[[ ]]` | Writes `[[Note]]` | Keep the `[[` picker, but it **writes a relative link** `[Note](Note.md)` | Same gesture, GitHub-readable |
| 17 | Date links `@today` | Writes `@today` | The `@` picker writes `[Saturday, 21 November](Daily/2026-11-21.md)` | Same reason |
| 18 | Tags `#tag` | Plain text, filtered in the palette | Kept: GitHub shows them as plain text, nothing breaks | Compatible as is |
| 19 | Save state | Autosave, no indicator | No indicator (v1's "Saved" removed) | Autosave is the promise; a label adds noise |
| 20 | Zoom | Ctrl+= / Ctrl+- / Ctrl+0 | Kept; changes text size, shown in Settings | — |

### Open questions for the owner

- **Q1 Comments syntax.** Switch to `<!-- comment: … -->` (hidden on GitHub, anchored to the text just
  before it), or keep CriticMarkup (anchors a passage precisely, but shows as raw text on GitHub)?
- **Q2 Pending changes.** The assistant's pending changes are written into the file as CriticMarkup
  until accepted. Keep that, or hold them beside the file so the `.md` is always clean?
- **Q3 Preview tabs.** Keep preview tabs at all, or open every note as a normal tab?

### v2 as built (review notes, 2026-11-21)

All twenty decisions are clickable. Approximations, to settle in v3 or in the app:

- The commented passage is the whole line before the comment marker, not the exact selection.
- The assistant's prompt field and "Tighten the wording" only show a toast; "Review this note" makes
  two pending changes.
- Scroll position is kept per tab; the app keeps it per pane and tab.
- Key rebinding in Settings › Shortcuts has no conflict check.
- Pending changes are shown as written into the file (CriticMarkup), as today; Q2 is still open.
- The window title (vault — Inkuno) is not drawn; the folder path is the tab's tooltip.

### v2 approved (owner, 2026-11-21) — Phase 2 build plan

The owner approved v2 as the spec for the window. The three open questions take the recommended
answers (owner may overturn any of them):

- **Q1 Comments:** `passage<!-- comment: text -->`, hidden on GitHub. Existing CriticMarkup comments
  are read and rewritten to the new form when a note is saved.
- **Q2 Pending changes:** held beside the note (in `.inkuno/`), never written into the `.md`; the
  file on disk is always clean Markdown.
- **Q3 Preview tabs:** kept, shown in italics.

Build packages, in order (each: Quiet themes only where it is styling, Inkuno themes unchanged, full
tests green except the two sandbox-as-root tests, screenshots against `v2.html`):

| # | Package | Main files | Status |
|--|--|--|--|
| P1 | Title band and tabs: Quiet tab pills, preview tabs in italics, dimmed unfocused pane, hidden-band peek on hover, focus mode surface | `shell/tabs.rs`, `shell/render.rs`, `shell/pane.rs` | built, uncommitted |
| P2 | Left sidebar, files only: Search and Today rows, tree, Scratch section, Settings at the foot; Quiet rows | `shell/sidebar.rs`, `views/nav.rs` | |
| P3 | One right panel: icon switcher with Outline, Comments, Versions, Assistant; versions and assistant leave the left; the comments column becomes the Comments view | `shell/sidebar.rs`, `shell/render.rs`, `shell/versions.rs`, `shell/assistant.rs` | |
| P4 | Settings tab with Appearance, Editor, Files, Shortcuts (F1), About | new `views/settings*`, `shell/settings.rs`, `shell/help.rs` | |
| P5 | Menus and palette in the Quiet popover; snippets in search; slash menu with the icon grid; selection toolbar with Comment | `views/palette.rs`, `views/editor/completion.rs`, new toolbar | |
| P6 | Links: `[[`/`@` pickers write relative links; relative links clickable with hidden brackets | `views/editor/completion.rs`, `links.rs`, `layout.rs` | |
| P7 | Comments syntax (Q1) and pending changes beside the file (Q2) | `inkuno-logic` markdown + pending, `views/editor/changes.rs` | built 2026-11-21 (see below) |
| P8 | Empty pane cards, dark-mode pass, docs (`app-ui.md`, user guide) | `shell/render.rs`, `docs/` | |

### Build notes (2026-11-21)

- **P5 (merged 7513c73):** daily notes stay where the app keeps them, at the vault root
  (`2026-11-21.md`); the prototype's `Daily/` folder was illustrative only. `/date` left the slash
  menu (it wrote `@date`); the `@` picker writes a relative link instead.
- **P6 (merged 107e989):** folder renames do not rewrite relative links yet.

### P7 as built (2026-11-21)

- **Q1 comments:** `passage<!-- comment: text -->`. The passage is the text from the start of the line, the table
  cell or the previous comment up to the marker, trimmed (so a comment narrows nothing: a passage in the middle of a
  line covers the text before it on that line). Read: both forms. Written: the new form only. A note with
  CriticMarkup comments is rewritten to the new form on its next save (or the next `inkuno comment`/`write`). The
  marker and its text are hidden off the live line and shown dimmed on it. A `-->` in a comment is written `-- >`.
- **Q2 pending changes:** `.inkuno/pending/<note>.json`, beside the note; the file is clean. Accept writes the new text
  into the note, reject drops the entry. Autosave, undo, external-change merge and copy (no mark is copied) work on
  the composed text as before.
- Not yet: a move or trash of a note leaves its pending list behind in `.inkuno/pending` (a rename by its title
  moves it); a note outside the vault and the scratch folder keeps its list beside its own file's folder.

## Where to pick up (handoff, 2026-11-22)

- **The built app matches v2.** P1–P9 and three review rounds are merged (PR kolisachint/inkuno#17).
- **Working on the prototype:** copy `v2.html` to `v3.html`, edit, and log each decision here under a new
  "v3" heading. It is self-contained HTML/CSS/JS: open it in a browser. The published copy is the
  claude.ai artifact linked in the table above (republish the new file to the same link).
- **Known follow-ups, candidates for v3 or the app:**
  - folder renames do not rewrite relative links;
  - a title rename scans the vault on the UI thread;
  - the editor's per-line comment highlight can mark a comment-like line inside indented code;
  - comment cards show no author or time, and comments have no replies;
  - code fences, rules and tables inside a block quote render as text;
  - the remaining CommonMark gaps listed in `markdown_renderer.md` §11.5 (indented code and pipe-less
    tables need a block-level classifier).
- **Screenshots:** `scripts/visual-check.sh` renders every fixture on Xvfb; pin a theme by writing
  `theme=Dark` (or any theme name) into the temporary config it creates.

---

## v3: parity with the built app (2026-11-22)

Built from `v2.html` (frozen). Each row is a decision from the owner's list. Keys follow the app's keymap.

| # | Area | v2 | v3 | Why |
|--|--|--|--|--|
| 1 | Tabs | Every tab shows a cross | An unsaved note shows a 6 px accent dot in place of the cross. Hover shows the cross. Ctrl+S clears the dot and toasts "Saved" | The app marks unsaved tabs (app-ui C23). The dot reads faster |
| 2 | Tabs | No way to reopen a closed tab | Ctrl+Shift+T reopens the last closed tab | The app has it |
| 3 | Empty pane | Cards only | A "Write here…" line sits above the cards. Key chords are chips on one line | The app shows the line (app-ui C21). Chips stop chords wrapping |
| 4 | Right panel | Open on first load | Closed on first load. The band button opens it on Outline | The app starts closed (app-ui C03). The note gets the room first |
| 5 | Palette key | Ctrl+K | Ctrl+P (⌘P on mac) everywhere it shows. Ctrl+K stays the note's Link key, as in the app | Matches the app keymap. One search key |
| 6 | Palette ranking | Open notes ranked first, so "road" + Enter opened another note | Name matches rank above content matches. Open notes lead within each group. "Create «…»" is always the last row | Fixes a v2 inconsistency: "road" + Enter now opens Roadmap |
| 7 | Command mode | One flat list | The ">" stays in the field. Commands sit under This block, Formatting, Notes and Window | Matches the app. Easier to scan |
| 8 | Palette rows | No age | Note rows show an age at the right ("today", "2 d") in faint text | An age helps tell apart notes with similar names |
| 9 | Slash menu | Block added at the end of the note | Block goes in after the caret line | Fixes a v2 inconsistency: the block landed far from the caret |
| 10 | Segmented control | Dark active chip barely lighter than the track | Dark: track #202022, active chip #3a3a3c | The active choice reads at a glance |
| 11 | Text fields | Faint border, weak focus | 1 px border at 3:1 against the sheet in both themes. 2 px accent focus ring | Field edges and focus meet contrast needs |
| 12 | Switch | Off track faint | Off track is ink at 22 % (light) or 28 % (dark) | Off state reads against the sheet |
| 13 | Toasts | Centred on the window, under the right panel | Centred at the bottom of the note sheet | Feedback sits over the note it is about. Fixes a v2 inconsistency |
| 14 | Outline | Flat list | Rows with children get a 20 px fold chevron. Clicking it folds the rows below | The app has the fold (app-ui RP-09) |
| 15 | Comments | Draft card at the top of the view | Kept as v2: passage quote, field, Comment and Cancel at the top | Already matches. No change |
| 16 | Comment anchor | Passage taken from the selection | Passage runs from the line start (or the previous comment) to the marker | Follows the app's passage rule (README P7). The selection could cover text the comment does not anchor |
| 17 | Compare | Replaces the note view | Opens as its own read-only tab named "<note> · <version time>", with the compare bar on top (Restore, Close) | The app does this (app-ui §19). The note stays open beside it |
| 18 | Pending changes | Inline Accept and Reject pills | Washes: green for an addition, red for a removal. Ctrl+Enter accepts and Ctrl+Backspace rejects the change at the caret. The foot pill moves the caret to the first change. Accept all and Reject all are palette commands | The app draws washes and uses chords, not pills (app-ui §17) |
| 19 | Text measure | Normal 720 px | Normal 800 px (`--measure:800px`) | The app landed 800 px as Normal (app-ui §14) |

Notes:

- Palette commands for the block types and the change actions were added with items 7 and 18. They use the app's names.
- The design log window still shows the v2 decisions. It is a record, not a spec.

---

## v3-ios: iPhone app (2026-11-23)

Clickable iPhone prototype for the planned SwiftUI app (`v3-ios.html`, one self-contained file). Built from v3.html's tokens, icons and Markdown approach. Deferred features are built and clickable with a Deferred or Simplified chip; the **v1 scope** switch in the design log replaces them with their v1 fallback.

### Owner decisions

- iPhone only. The native app is SwiftUI, built later on inkuno-logic (Rust) through UniFFI. The assistant is deferred and not shown.
- The vault is a folder picked in the Files app (iCloud Drive, Dropbox, Working Copy). It is the same folder as on the desktop.
- Editor: live styling as on the desktop. Only the tapped line shows its Markdown. Accessory bar above the keyboard: / # [[ task, indent, outdent, undo, hide.
- Home is the desktop empty-page cards. The file tree is push navigation with a nav bar and no tab bar; Scratch sits at the foot of the root. Settings is reached from Home.
- Commands: pull down (or tap search) for a sheet with note search and > commands. Long-press a row or block for the context menu.
- The right panel is one bottom sheet with four segments (Outline, Comments, Versions, Backlinks) and half and full detents.

### Decisions and complexity

| # | Area | iPhone design | Complexity | v1 suggestion | Why |
|--|--|--|--|--|--|
| 1 | Vault | Folder picked in the Files app; the same folder as on the desktop. Placeholder downloads and conflict copies | High | Keep (foundation) | Every note reads and writes through it; placeholders and conflicts are the hard part |
| 2 | Live-styled editor | Rendered Markdown; only the tapped line shows its syntax. Accessory bar above the keyboard | High | Keep (core; spike first) | Biggest risk: TextKit 2 with the Rust parser, IME and undo |
| 3 | Home cards, tree, Scratch, Today | Desktop empty-page cards; push tree; Scratch at the foot of the root; Today note | Low | Keep | Mostly layout; Today is the main entry point |
| 4 | Pull-down palette and > commands | Pull down or tap search: note names, content matches, > commands in a sheet | Med | Keep | Replaces Ctrl+P; one search surface |
| 5 | Accessory bar and slash sheet | / # [[ task, indent, outdent, undo above the keyboard; / opens a grid sheet | Low | Keep | Native input affordance |
| 6 | Links, backlinks, tags, @date | [[ writes relative links; backlinks list; #tags; @today | Med | Keep (logic in inkuno-logic) | Same rules as the desktop; the logic is shared through the core |
| 7 | Outline sheet | Outline segment of the one bottom sheet | Low | Keep | Same sheet as the other panel views |
| 8 | Comments view and resolve | Comments segment; inline badge; resolve | Med | Keep | Comment syntax already decided (v2 Q1) |
| 9 | Adding comments | Long-press a block, then Comment. No text selection on the phone | Med | Simplify (long-press only) | Selecting text on iPhone is costly; block comments cover v1 |
| 10 | Versions list, save, restore | Versions segment; tap a version; restore asks first | Med | Keep | The history promise |
| 11 | Compare diff screen | Green adds, red strikes, Restore | Med | Defer (restore confirm instead) | A second screen to build; the confirm covers the need |
| 12 | Pending changes review sheet | Pill opens a sheet; swipe right accepts, left rejects | Med | Keep | Desktop agent edits need a review path on the phone |
| 13 | Inline washes in the editor | Green and red washes on the changed lines | High | Defer (pill and sheet only) | Needs styled change spans in TextKit; the sheet covers review |
| 14 | Tables render | Horizontal scroll inside the note | Med | Keep | Common in notes; reads well on a phone |
| 15 | Table cell editing | Tap a cell: sheet with the cell text | High | Defer (raw Markdown on the live line) | Needs a cell model and sync; raw editing is safe |
| 16 | Mermaid render | Flowchart as inline SVG; full-screen view | High | Defer (code block) | Needs the Rust renderer to output SVG |
| 17 | Math render | Formula as styled HTML; full-screen view | High | Defer (code) | Needs a formula layout engine |
| 18 | Context menus | Long-press on tree rows and blocks (replaces right-click) | Low | Keep | Native pattern |
| 19 | Settings | Folder, appearance, editor, files, about. Reached from Home | Low | Keep | Plain list |
| 20 | Theme set | Light, Dark, System and Quiet only | Low | Simplify (15 more themes later) | Keeps the theme code small for v1 |
| 21 | Focus mode | Hides the navigation bar | Low | Defer | The phone is already focused |
| 22 | Backup tar.gz | Row in Settings › Files | Low/Med | Defer | Files already sync through iCloud |
| 23 | Trash restore UI | Settings › Files › Trash; palette command Restore from Trash | Low/Med | Keep (palette command) | Keeps undo for deleted notes |

### Suggested v1 deferrals (owner to reconsider)

- Compare screen: restore asks for confirmation instead.
- Inline washes: the pill and the review sheet only.
- Table cells: edit as Markdown on the live line.
- Mermaid and math: show as code blocks.
- Focus mode and Backup (tar.gz).
- Theme set beyond Light, Dark, System and Quiet.

**Feedback (owner, 2026-11-24):** limited v1 scope approved as suggested above: every row marked
Defer or Simplify stays out of v1. Next steps are in [`../ios.md`](../ios.md).

---

## v4-ios: Mermaid, math, conflict copies (2026-11-26)

Built on v3-ios. The switch "Show built app (v1)" shows what ships today. v4 proposals carry a "v4" chip.

| # | Area | iPhone design | Why |
|--|--|--|--|
| 1 | Mermaid | Drawn under the block, borderless, column width. Too wide: scale to 0.75, then scroll inside the column with a right fade. Never widens the note | Same container rule as the desktop (`markdown_renderer.md` §9.1) |
| 2 | Math | Inline `$…$` drawn in the line (≤1.6 line heights). Display `$$` centred under the block | Desktop behaviour |
| 3 | Edit | Tap a figure: the source shows, the figure stays below as Preview | Phone has no caret-line hover; keeps the result visible while editing |
| 4 | Full screen | Magnifier (top-right, 36 px band reserved) opens a viewer: pinch zoom, Copy image, Share | Desktop magnifier |
| 5 | Errors | "Unreadable figure" / "Unreadable formula" + Show source. Invalid inline math stays as source | Desktop labels; phone adds the way back to the source |
| 6 | Long-press | Block menu adds Copy image, View full screen | Native pattern |
| 7 | Slash sheet | Math and Mermaid entries (hidden in v1 mode) | `ios.md` open item 6 |
| 8 | Conflict copy | Tree row with warning dot; sheet "Two versions of …", neutral highlights, Merge both / Keep iPhone / Keep Mac | Core `merge_onto` exists (`inkuno-ffi` `merging.rs`); Swift never calls it |

### Fixed from v3

- goto and outline jumps hit the top page only.
- Scratch is removed from the Files picker. It lives outside the vault.
- The pull-down hint says Home or a note.
- v1 mode hides Backup, Quiet theme, and slash Math and Mermaid.

### Build prerequisite (critical)

- The figure engine (`crates/inkuno-ui/src/figure.rs`: mermaid-rs-renderer 0.3.1, latex-rust 2.1.0, resvg) imports gpui, so iOS cannot link it.
- Move it to a gpui-free module. Export `figure_mermaid` and `figure_latex` (SVG or PNG at device scale) through `inkuno-ffi`.
- Render off the main thread. Cache by source, theme and width.
- Measure binary size (STIX Two Math font, renderer crates) before committing to it.

### Still not designed

- Dynamic Type (fixed 14–18 pt today).
- VoiceOver per-block labels.
- Images and attachments (the tree shows .md only).
- Share and export.
- Landscape.
- Hardware keyboard shortcuts.
- Review feed (`app-ui.md` §23).
- Outline folding.
- Footnote taps.

### Stale docs found

- `markdown_renderer.md` §4 says only class, sequence and state Mermaid kinds draw, and only the first formula per line. The code draws every kind and every formula.
