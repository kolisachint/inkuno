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
| P1 | Title band and tabs: Quiet tab pills, preview tabs in italics, dimmed unfocused pane, hidden-band peek on hover, focus mode surface | `shell/tabs.rs`, `shell/render.rs`, `shell/pane.rs` | started 2026-11-21 |
| P2 | Left sidebar, files only: Search and Today rows, tree, Scratch section, Settings at the foot; Quiet rows | `shell/sidebar.rs`, `views/nav.rs` | |
| P3 | One right panel: icon switcher with Outline, Comments, Versions, Assistant; versions and assistant leave the left; the comments column becomes the Comments view | `shell/sidebar.rs`, `shell/render.rs`, `shell/versions.rs`, `shell/assistant.rs` | |
| P4 | Settings tab with Appearance, Editor, Files, Shortcuts (F1), About | new `views/settings*`, `shell/settings.rs`, `shell/help.rs` | |
| P5 | Menus and palette in the Quiet popover; snippets in search; slash menu with the icon grid; selection toolbar with Comment | `views/palette.rs`, `views/editor/completion.rs`, new toolbar | |
| P6 | Links: `[[`/`@` pickers write relative links; relative links clickable with hidden brackets | `views/editor/completion.rs`, `links.rs`, `layout.rs` | |
| P7 | Comments syntax (Q1) and pending changes beside the file (Q2) | `inkuno-logic` markdown + pending, `views/editor/changes.rs` | |
| P8 | Empty pane cards, dark-mode pass, docs (`app-ui.md`, user guide) | `shell/render.rs`, `docs/` | |
