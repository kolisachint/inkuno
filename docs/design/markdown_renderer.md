# Markdown renderer compatibility contract

> Docs-only contract. No code, no behavior change.
> Per `docs/design/app-ui.md` §1 (rule 3): every claim about what the app does today
> cites a `file:line`; anything unverified is marked **needs-eyes**, never asserted.
> Grounding refs: `docs/design/app-ui.md` §§3 (`app-ui.md:48-106`, tokens + typography
> system) and 12 (`app-ui.md:411-479`, visual spec v1, esp. §12.2 center canvas).
> Parser ground truth: `crates/inkuno-logic/src/markdown/` (`mod.rs`, `tables.rs`, `lists.rs`,
> `front.rs`, `dates.rs`, `search.rs`); rendering ground truth:
> `crates/inkuno/src/views/editor/` (esp. `layout.rs`, `render.rs`, `paint.rs`, `style.rs`,
> `embed.rs`, `formulas.rs`, `grid.rs`, `table.rs`); user-facing spec:
> `docs/user/src/writing.md`.

## 1. Tier table (user-set contract)

> **Amended 2026-11-21 (owner decision, §11): Inkuno's Markdown standard is CommonMark 0.31.2 +
> GFM 0.29-gfm, nothing else.** The table below is the pre-amendment tiering, kept for history; §11
> supersedes it where they differ (alerts, footnotes, math and Mermaid are GitHub.com extensions, not
> part of the standard; `<u>`/`<mark>`/`==` are outside it).

| Tier | Spec | What's in scope |
|------|------|-----------------|
| Core | CommonMark | Paragraphs, ATX headings (6 levels), bullet / ordered / task lists, blockquotes, fenced code blocks, thematic breaks, inline emphasis (bold, italic, code, links), autolinks, hard/soft breaks |
| Extended | GFM | Tables, strikethrough, task-list items (`- [ ]` / `- [x]`), autolink extension, `> [!NOTE]`-style callout panels |
| Optional | Beyond GFM | Math (`$…$` inline, `$$…$$` display), Mermaid diagrams (`` ```mermaid ``), footnotes |
| Sanitized | HTML support | Inline `<u>`, `<mark>` (incl. `<mark class="…">` colors); everything else: kept as typed, never executed |

Non-goals of this contract: editing interactions (marker hiding lives with the
editor crew), export fidelity (PDF rules live in `docs/user/src/writing.md` "Save as
PDF"), and any new syntax. Source on disk stays plain Markdown
(`docs/user/src/writing.md:3-5`; `app-ui.md` vision, `app-ui.md:30-46`).

## 2. Core (CommonMark) — status vs gaps

What exists today (verified by reading code):

- **Block classification** is a line-by-line classifier: `Kind` enum
  (`crates/inkuno-logic/src/markdown/mod.rs:21`) + `classify`
  (`crates/inkuno-logic/src/markdown/mod.rs:103`) covering `Para`, `Heading(1-6)`,
  `Bullet`, `Ordered`, `Task(bool)`, `Quote`, `Fence`, `Code`, `Rule`,
  `Table`, `Meta` (front-matter lines). Fences detected by `is_fence`
  (`mod.rs:80` area: `count_fences_and_dollars`; fence predicate nearby).
- **Code blocks**: content range via `code_block` (`mod.rs:751`); single-line
  per-language lexer `code` (`mod.rs:683`) coloring comments / strings /
  numbers / keywords; language name taken from the info string, with
  `"" | "text" | "txt" | "plain" | "mermaid"` treated as uncolored
  (`crates/inkuno/src/views/editor/layout.rs:1568`); copy affordance is documented behavior
  (`docs/user/src/writing.md:30`).
- **Inline styles**: `inline` (`mod.rs:278`) over `emphasis` (`mod.rs:295`)
  for `**` bold, `*`/`_` italic, `` ` `` code, `~~` strike, `==` highlight,
  `<u>` underline, `<mark>` colors; markers are flagged `DIM`
  (`crates/inkuno-logic/src/markdown/mod.rs`, `DIM` flag; user-visible rule:
  "markers stay visible but dimmed", `docs/user/src/writing.md:5`). Bars between
  blocks, checkbox styling, and arrow substitutions (`->` → `→`, etc. via
  `symbols`) follow the same flag pipeline.
- **Links**: protocol-gated `is_url` / `web_link` + `links`
  (`crates/inkuno-logic/src/markdown/mod.rs`, `is_url`/`web_link`/`links` block) for
  `[text](address)`, bare `http(s)://` URLs, `[[Wiki]]` targets, `#tag`
  ranges; `[ ]`/`[x]` boxes tracked separately (`checkboxes`).
- **Lists**: Enter continuation / renumber / restyle live in
  `crates/inkuno-logic/src/markdown/lists.rs` (`on_enter`, `renumber`, `restyle`).
  The list north star (GitHub + Typora) and Inkuno's proposed target are in §10.
- **Front matter**: `front_matter` / `body_start` / keys / values
  (`crates/inkuno-logic/src/markdown/front.rs`); `Meta` lines are excluded from heading
  treatment. Title-in-first-line convention per `body_start`.
- **Canvas contract**: single centered column, Normal width 800
  (`app-ui.md:78-88`, `app-ui.md:259-290`); headings / lists / tasks / quote / code /
  table / `---` render as styled blocks (`app-ui.md:431-443`, §12.2).

Gaps / **needs-eyes** (not verified — do not claim compliance):

- No CommonMark spec suite (e.g. `spec.txt` / `smart_punct`) was found in the
  repo — **needs-eyes**: confirm whether one exists or compliance is
  best-effort-by-construction.
- Setext headings, reference-style links (`[a][b]`), link titles, HTML blocks,
  backslash escapes, entity decoding: **needs-eyes** (no matches spotted in
  `classify`/`links`; verify before promising).
- Nested-list indent width and list-interrupt-paragraph rules: **needs-eyes**
  (verify `lists.rs` + `classify` against spec cases).

## 3. Extended (GFM) — status vs gaps

What exists today:

- **Tables**: pipe parsing via `bars` / `cells`
  (`crates/inkuno-logic/src/markdown/tables.rs:7`, `:24`); grid ops (insert/delete/move/
  sort/transpose) in the same file; source-side column alignment capped by
  `ALIGNED = 100` (`tables.rs:472-494`, `format_table` at `tables.rs:475` —
  beyond that width columns are no longer padded but display is unchanged).
  Editor display: a table line becomes a grid whose cells wrap
  (`crates/inkuno/src/views/editor/layout.rs:283`); grid lines themselves never wrap
  (`layout.rs:1600`); non-table lines wrap at the measure
  (`layout.rs:1752-1754`). Cell styling reuses the inline pipeline per cell
  (`highlights` / `tags` cell paths in `mod.rs`).
- **Strikethrough + task items**: `STRIKE` flag + `Task(bool)` kind (see §2);
  `Ctrl+Enter` toggles boxes (`docs/user/src/writing.md:12`, `:62`).
- **Callout panels**: `> [!NOTE]` etc. with per-kind colors, GitHub-style
  (`docs/user/src/writing.md:29`); the five `/note…/caution` slash commands insert
  them (`docs/user/src/writing.md:72-80`).
- **Autolinks**: bare `http(s)://` handled in `links` (§2).

Gaps / **needs-eyes**:

- GFM table alignment colons (`:---:`) round-trip: partially handled in
  `format_table` (colon preservation) — **needs-eyes**: verify center/right
  alignment renders, not just survives.
- Discontinuous tables (blank line inside a table), multiple header rows:
  **needs-eyes**.
- GFM alert titles / custom titles beyond the five kinds: not supported
  (only `NOTE TIP IMPORTANT WARNING CAUTION`, `docs/user/src/writing.md:29`).
- Large tables: `app-ui.md` acceptance criterion cites "> 1500 rows use light
  layout" (`app-ui.md:190-216`) — **needs-eyes**: verify the threshold constant
  and path in `layout.rs`/`grid.rs` before citing it as contract.

## 4. Optional (Math, Mermaid, footnotes) — status vs gaps

What exists today:

- **Math**: `$$` fence-line counting in `count_fences_and_dollars`
  (`crates/inkuno-logic/src/markdown/mod.rs:80`); formulas drawn as figures *under* their
  line, first formula of a line only (`docs/user/src/writing.md:32`); display
  figures built via `figure::latex` (`crates/inkuno/src/views/editor/layout.rs:1374`,
  `:1796`). Inline `$x^2$` is styled/kept inline, not drawn; PDF draws only
  `$$` formulas (`docs/user/src/writing.md`, "Save as PDF" limits).
- **Mermaid**: only ` ```mermaid ` blocks whose source is one of
  class / sequence / state diagrams convert (`crates/inkuno/src/logic/mermaid.rs`: `kind`,
  `to_diagram`); anything else (flowcharts, `graph TD`, erDiagram, …) counts
  skipped lines, not a figure. Rendering: the block reserves a figure slot
  when the cursor has left it (`layout.rs:1341-1356`), drawn off-thread via
  `figure::mermaid` (`layout.rs:1348`, engine in `crates/inkuno-ui/src/figure.rs:41-48`);
  unreadable source shows "Figure illisible" / "Unreadable figure"
  (`layout.rs:1356`, `:1862`) — covered by `crates/inkuno/src/shell/tests/feedback.rs:150-172`
  (good diagram draws, bad/empty diagram says so, `figure_count` asserted).
  User rule: diagram under its block once the cursor leaves; magnifier opens
  full-size (`docs/user/src/writing.md:31`; `docs/user/src/writing.md:23`).
- **Footnotes**: GitHub-style — `[^label]` reference + `[^label]:` definition (see §7-Q3, decided).
  Parsing in `crates/inkuno-logic/src/markdown/mod.rs`: references `footnote_ref` (`mod.rs:194`),
  definitions `footnote_def` (`mod.rs:238`), document collection `footnotes` (`mod.rs:278`)
  with `count_footnotes` pre-scan (`mod.rs:346`) and per-line defined/undefined styling
  `footnote_flags` (`mod.rs:370`). Defined references render as links with hidden brackets
  off the edited line (`crates/inkuno/src/views/editor/layout.rs:687`, `:1629`); undefined ones stay literal.
  `Ctrl+click` jumps reference → definition and cycles definition → references
  (`crates/inkuno/src/views/editor/mouse.rs:696`, hand cursor `crates/inkuno/src/views/editor/blocks.rs:184`).
  Export: sharing passes them through literally (`crates/inkuno-logic/src/sharing.rs:286`); printing keeps
  the label in place with no link entry (`crates/inkuno-ui/src/print.rs:279`); PDF prints them in place
  with no collected section (`docs/user/src/writing.md`, "Save as PDF" limits).

Gaps / **needs-eyes**:

- Mermaid kind coverage is a *bridge*, not a renderer (`mermaid.rs` header):
  three kinds only. Flowchart/graph support: open question (§6), not a gap
  to silently fill.
- Multi-formula lines (only first drawn), `$$` inside tables/cells,
  `\(...\)` syntax: **needs-eyes**.
- Footnote syntax: RESOLVED 2026-11-09 by §7-Q3 (GitHub-style `[^label]` + `[^label]:`).

## 5. Sanitized (HTML support) — status vs gaps

What exists today:

- Supported inline HTML is a closed set: `<u>…</u>` for underline (Markdown
  has none — `docs/user/src/writing.md:42`) and `<mark>` / `<mark class="…">`
  with the five `MARKS` colors (`crates/inkuno-logic/src/markdown/mod.rs:241`,
  `mark_tag` / `mark_color` / `mark_signs`); `<mark>` portability note:
  "plain HTML, so other apps still show the text highlighted"
  (`docs/user/src/writing.md`, inline-styles list).
- Rendering pipeline treats unknown HTML as ordinary text (no tag execution
  path was found in `paint.rs`/`layout.rs`) — **needs-eyes**: confirm there
  is no `Html`/`Embed` branch that interprets arbitrary tags.
- SVG figures from elsewhere are rasterized by resvg, which *ignores*
  `<foreignObject>` HTML and `light-dark()`; `readable` substitutes SVG text
  (`crates/inkuno-ui/src/svg.rs:1-9`). Sharing/export strips to readable plain text
  (`crates/inkuno-logic/src/sharing.rs:1-34`, rules in `docs/user/src/writing.md`
  "Plain-text rules").

Gaps / **needs-eyes**:

- No dedicated HTML sanitizer (e.g. ammonia-like allowlist) was found —
  sanitizer choice is open question §6-Q2. Current posture appears to be
  "allowlist-by-construction" (only `<u>`/`<mark>` parsed, rest literal), but
  verify `paint.rs` + export paths (`sharing.rs`, `print.rs`) before
  contracting it.
- `<mark>` inside table cells / headings / PDF output: **needs-eyes**
  (PDF path per `docs/user/src/writing.md` limits).

## 6. Test strategy per tier

| Tier | Unit (pure logic) | Integration (editor/shell) |
|------|-------------------|----------------------------|
| Core | `crates/inkuno-logic/src/markdown/mod.rs` tests (classify/inline/code/boxes); `lists.rs` Enter/renumber/restyle; `front.rs` matter parsing | `crates/inkuno/src/shell/tests/end_to_end/writing.rs` (typing/lists/blocks); `outline.rs` (headings skip code blocks); `find_mouse.rs` (marker searchability) |
| Extended | `crates/inkuno-logic/src/markdown/tables.rs` tests (cells/paste/format); formula tests with `table_formulas.rs` | `md_tables.rs` (paste/align/wrap thresholds — note `app-ui.md:259-290` recalibration rule), `table_handles.rs`, `table_styles.rs`, `csv.rs` |
| Optional | `crates/inkuno/src/logic/mermaid.rs` tests (kind/convert/skip counts); formula evaluator tests (`logic/formula.rs`) | `feedback.rs:150-172` (mermaid good/bad/empty); `diagrams.rs` (bridge diagrams); `media.rs` (pictures/figures); PDF tests for `$$` vs `$…$` |
| Sanitized | `mod.rs` mark-tag tests (`mark_tag`/`mark_color`); `sharing.rs` plain-text rules | Export/copy tests (Markdown vs plain-text copy, `docs/user/src/writing.md` "Copy and share"); PDF "not printed" list |

Rule for all tiers: when a threshold or pixel value changes (e.g. Normal 800,
`PILL_CLEAR`, `ALIGNED`), re-verify in the order window → page-width →
`md_tables` → boards/cursors → full suite (`app-ui.md:259-290`).

## 7. Open questions

1. **Mermaid overflow**: ~~currently has no contracted overflow rule~~ RESOLVED in principle 2026-11-09 by §9.1 (container-inviolable; scroll/clip internally, magnifier as escape hatch; exact per-figure mechanics with editor crew). Candidates were: (a) shrink-to-measure like PDF
   ("shrunk to fit the page", `docs/user/src/writing.md`), (b) horizontal scroll
   region, (c) clip with magnifier affordance (magnifier already exists,
   `docs/user/src/writing.md:31`). Which is canonical for the canvas? (Related
   live work: mermaid figures under blocks; Typora-style marker hiding must
   not fight the figure slot.)
2. **HTML sanitizer choice**: allowlist-by-construction (today's apparent
   posture, §5) vs an explicit sanitizer pass over pasted/imported HTML?
   Which tags/attributes are contractual (`<u>`, `<mark class>`), and what
   happens to the rest on paste — escaped, stripped, or literal? (Blocks
   import/copy paths: `clipboard.rs`, `sharing.rs`.)
3. **Footnote syntax**: RESOLVED 2026-11-09 — GitHub-style `[^label]` reference +
   `[^label]:` definition (at most three spaces of indent). Undefined references stay
   literal (no definition collected); definitions print and share in place, with no
   collected section. Marker-hiding follows the editor's reveal rule (markers hidden off
   the edited line, source never rewritten); navigation is `Ctrl+click` both ways.
4. **Table wrap/clip behavior** (companion to Q1): ~~needs contracting~~ RESOLVED 2026-11-09 by §9.1 (tables/code get own overflow rules; container inviolate; internal scroll/clip; alignment preserved). Was:
   (`layout.rs:1600`) while cells wrap (`layout.rs:283`); source alignment
   stops at `ALIGNED` (`tables.rs:472`). What is the canvas rule when a table
   exceeds the measure — PDF shares width and wraps cells
   (`docs/user/src/writing.md`), but the live canvas rule needs contracting
   (related live work: table wrap/clip).
5. **Code-fence rendering scope**: single-line lexer per language
   (`mod.rs:683`, `/* */` and multi-line strings explicitly out of scope in
   the ponytail comment) — is that the permanent contract, or is a per-language
   grammar (tree-sitter) on the roadmap? Info-string edge cases (extra args
   after lang, ` ```mermaid ` vs ` ``` mermaid`) also need pinning.
6. **Marker-hiding interplay** (for the editor crew, recorded here): markers
   are `DIM` today; the arrows-substitution precedent hides replacements only
   off the editing line (`docs/user/src/writing.md:33`). If Typora-style hiding
   lands, this contract's Core section needs a "source line vs display"
   amendment — hiding must never rewrite source (vision: `app-ui.md:30-46`).

## 8. Amendment procedure

Propose the change in this file first (new section or edit, marked
`PROPOSED` + date) → user approves by editing → implement citing this doc —
the `app-ui.md` §1 check-box flow (`app-ui.md:7-28`) applies to renderer changes too.

## 9. Decided rules (2026-11-09)

> Locked in by user decision 2026-11-09. Docs-only: no code changed here;
> implementation is a follow-up for the editor crew (§9.3).

### 9.1 Wide-table / code-block overflow rule — DECIDED 2026-11-09

Tables and fenced code blocks get their OWN overflow/wrapping rules,
distinct from prose and from figure slots (math, Mermaid).

Container-inviolable statement: a wide table (or wide code block) must NEVER
force the document beyond its container. The document width is inviolate;
overflow is contained internally by the block that overflows.

Suggested mechanics (for the editor crew to implement, §9.3):

- Internal scroll/clip: the overflowing table / code block scrolls (or clips)
  on its own axis inside the measure instead of widening the canvas.
- Alignment preserved: source-side column alignment (`format_table`,
  `tables.rs:475`) and cell styling keep working; containment must not
  re-justify columns or break the grid's cell-wraps-cell / grid-line-never-wraps
  split (`layout.rs:283`, `layout.rs:1600`).
- Same container rule extends to wide figures under blocks (Mermaid, math):
  shrink, scroll, or clip internally — never widen the doc (resolves §7-Q1
  in principle; exact per-figure mechanics stay with the editor crew, with
  the existing magnifier affordance, `docs/user/src/writing.md:31`, as the
  full-size escape hatch).

Resolves §7-Q4 (table wrap/clip) and §7-Q1 (Mermaid overflow) at the
principle level; per-block pixel/threshold work remains implementation.

### 9.2 Layout-agnostic renderer principle — DECIDED 2026-11-09

The renderer (engine) is layout-AGNOSTIC: it emits content + metrics only.
The APPLICATION owns presentation context. Responsibility split:

- Engine owns: block/inline classification, cell content + wrap metrics,
  figure descriptors, marker/flag pipeline (`DIM`, `STRIKE`, etc.).
- Application owns (knobs the engine must NOT hard-code): width / measure
  (e.g. Normal 800 today, `app-ui.md:78-88`), typography scale, padding,
  alignment.

Rationale: keeps the engine reusable across consumers — notes, agent-chat
(agent conversations), documents — each with its own width, type scale,
padding, and alignment, without forking rendering logic.

Consequence for this contract: any current pixel/measure citations (e.g.
Normal 800, `ALIGNED = 100`, light-layout row thresholds) are application
settings consumed by the engine, not engine constants. Future amendments
must state which side of the split a new knob lives on.

### 9.3 Implementation follow-up (for the editor crew — DO NOT implement here)

- [ ] Implement §9.1 containment (table / code internal scroll/clip,
  alignment preserved) in `crates/inkuno/src/views/editor/` (`layout.rs`, `grid.rs`,
  `table.rs`) citing this doc.
- [ ] Confirm §9.2 split: no hard-coded width / type-scale / padding /
  alignment left in engine paths; application passes them in per consumer
  (notes, agent-chat, documents).
- [ ] Re-verify UI thresholds per `app-ui.md:259-290` (window → page-width →
  `md_tables` → boards/cursors → full suite) and report back here.

## 10. List rendering — north star (GitHub + Typora)

Status:

- **North star: GitHub and Typora.** APPROVED by owner 2026-11-21.
- **Inkuno's target (§10.3): APPROVED by owner 2026-11-21.** The owner answered §10.4 (answers inline there). The editor follows §10.3.

Scope: bullet, ordered and task list items in the editor. The reference behaviour in §10.1 is as stated in the owner's brief of 2026-11-21 (GitHub via github-markdown-css; Typora). It is not re-checked in the live products here: entries marked TBD are unverified.

### 10.1 Reference table

| Item | GitHub (github-markdown-css) | Typora |
|------|------------------------------|--------|
| Bullet markers | `ul` disc → circle → square by depth | Same CSS-rendered markers (disc, circle, square) |
| Marker colour | Text colour | Text colour |
| Source marker | Not shown (rendered output) | Revealed only while editing that item |
| Ordered markers | Decimal; nested ordered: lower-roman, then lower-alpha | Decimal |
| Start number | `start` attribute respected | Numbers come from the source; start respected |
| Indent | List `padding-left: 2em` | TBD |
| Task items | Checkbox replaces the bullet (no bullet drawn); native checkbox | Checkbox replaces the bullet; clickable |
| Checked task items | Not struck through | Not struck by default (theme-dependent) |
| Item spacing | 0.25em | TBD |
| Loose lists | Paragraph spacing | TBD |
| Wrapped lines | TBD | Hanging indent: wrapped lines align with the text |

### 10.2 Inkuno today (verified 2026-11-21 unless marked TBD)

- List kinds are `Kind::Bullet`, `Kind::Ordered` and `Kind::Task(bool)` (`crates/inkuno-logic/src/markdown/mod.rs:27` region, the `Kind` enum).
- Restyle cycles bullet → ordered → task (`crates/inkuno/src/views/editor/style.rs:75-76`).
- Off the live line, a list item's marker is drawn in the margin of its depth, not hidden: a bullet shape, a number, or a checkbox (`lead` in `crates/inkuno/src/views/editor/layout.rs`, painted in `paint.rs`). Wrapped lines hang under the item text. The live line shows its source. Continuation paragraphs (indented lines under an item) hang at the item text too. Depth comes from `list_depth` in `crates/inkuno-logic/src/markdown/lists.rs`.
- Line height is 1.7 times the body size for body text (`layout.rs`, the `lh` computation). The blank-line gap between paragraphs is unchanged.

### 10.3 Inkuno target — APPROVED by owner 2026-11-21

Follows GitHub (shapes, numbering, colour) and Typora (indent, reveal of the source on the live line):

- **Bullet shapes by depth, as GitHub:** depth 0 a filled disc, depth 1 a hollow circle (a 1px ring), depth 2 and deeper a filled square. About 0.35em across (5 to 6px at 16px), centred vertically on the first visual line of the item, in the **text colour** (not the soft colour). The shape sits 0.5em before the text of the item.
- **Ordered numbering by depth, as GitHub's `ol`, `ol ol` and `ol ol ol`:** depth 0 decimal (`1.`), depth 1 lower-roman (`i.`), depth 2 and deeper lower-alpha (`a.`). The depth is the same list depth used for indent, so an ordered list nested in a bullet at depth 1 is roman, as GitHub's `ul ol`. The displayed value is computed from the source number (`3.` at depth 1 shows `iii.`; alpha: 1 is `a`, 26 is `z`, 27 is `aa`). The delimiter (`.` or `)`) stays. The source text never changes; the live line shows the raw source.
- **Number and box colour:** the text colour for bullets and numbers. A checkbox's border keeps its soft colour; a checked box is an accent fill with a white check.
- **Indent, as Typora's default theme (GitHub-derived):** each level indents 30px at a 16px body, that is 1.875em, and scales with the text size. The marker sits in that gutter, right-aligned 0.5em before the item text, so the text of an item starts at 1.875em × (depth + 1). A number too wide for the gutter (`viii.`, `100.`) makes its item's gutter grow to fit. An ordered task (`1. [ ] x`) shows its number, then its checkbox, then its text.
- **Checked tasks:** struck through and dimmed (kept as it is).
- **Line spacing:** the body line height and the blank-line paragraph gap stay as they are. Tight lists are consecutive lines, loose lists have blank lines between items. No extra item spacing.
- **Continuation paragraphs:** a non-list line indented by at least two columns under the items of a list hides its indent and hangs at the text of its item. Fenced code blocks inside an item keep their own layout.
- **Source reveal, as Typora:** the line the caret is on (clicked or edited) shows its Markdown source, the marker included. Every other line renders. Source is never rewritten (§7, question 6).

### 10.4 Open questions (answered by owner 2026-11-21)

1. **Nested ordered lists:** GitHub's lower-roman, then lower-alpha (`i.`, `a.`), by depth. Answer: GitHub's style (§10.3).
2. **Checked task items:** struck through and dimmed. Answer: kept as it is (§10.3).
3. **Indent per level:** Typora's 1.875em (30px at 16px). Answer: Typora's indent (§10.3).
4. **Bullet shapes by depth:** GitHub's disc, circle, square, drawn as shapes (not glyphs). Answer: GitHub's shapes (§10.3).
5. **Loose lists and spacing:** keep the existing spacing; no extra item spacing. Answer: existing spacing (§10.3).
6. **Source marker:** Typora's: the live line shows its source, the other lines render. Answer: Typora's reveal (§10.3). This changes the dimmed-marker rule of `docs/user/src/writing.md`, which is updated for list markers.

### 10.5 Status note

Approved 2026-11-21. The shapes, numbering, colour, indent and continuation rules of §10.3 are implemented in the editor (`layout.rs`, `paint.rs`, `lists.rs`) and checked with `scripts/visual-check.sh` on the `lists-bullet`, `lists-ordered` and `lists-task` fixtures. Remaining: none for lists from this section. Later changes to the list look go through an amendment (§8).

## 11. The standard: CommonMark + GFM — DECIDED by owner 2026-11-21

### 11.1 Decision

Inkuno reads and writes **two specifications and no others**:

| Spec | Version (pinned) | Source |
|--|--|--|
| CommonMark | 0.31.2 (2024-01-28) | https://spec.commonmark.org/0.31.2/ |
| GitHub Flavored Markdown | 0.29-gfm (2019-04-06) | https://github.github.com/gfm/ |

GFM is a strict superset of CommonMark; its additions are exactly five: **tables**, **task list
items**, **strikethrough**, **autolinks (extended)** and **disallowed raw HTML** (the tag filter).

Why these two (owner's brief): maximum practical compatibility; precise specifications with official
example suites; a strong parser ecosystem, Rust included; the least implementation complexity.
Pandoc and Jupyter come **later, as separate capabilities** (import/export), never as core syntax.

The feel stays as decided in §10: **render like GitHub** (github-markdown-css shapes, numbering,
alerts), **edit like Typora** (the caret's line shows its source; every other line renders).

### 11.2 Three tiers from now on

| Tier | Contents | Rule |
|--|--|--|
| **Standard** | Everything in CommonMark 0.31.2 and GFM 0.29-gfm | Must conform; measured by the official example suites (§11.4) |
| **GitHub.com extensions** | Alerts `> [!NOTE]` (five kinds), footnotes `[^1]`, math `$…$` / `$$…$$`, Mermaid fences | Kept because github.com renders them. Each degrades to valid Standard Markdown (a blockquote, literal text, a fenced code block), so no file ever breaks elsewhere. No new ones without an owner decision |
| **Outside** | `[[wiki]]` links, `@date` links, `==highlight==`, `<mark>`, `<u>`, CriticMarkup comments and pending marks, arrow substitution | Nothing new may be written in these. Existing ones are converted or retired per `prototypes/README.md` (v2 decisions #9, #16, #17) and §11.5. Arrow substitution is display-only and writes nothing, so it may stay |

### 11.3 Parser strategy

- **Keep Inkuno's line classifier for the editor.** It is what makes per-line reveal and incremental
  relayout possible (`inkuno-logic/src/markdown/mod.rs`); a tree parser would re-parse the note on
  every keystroke and lose the line identity the reveal rule needs.
- **Use `comrak` as the conformance oracle**, as a *dev-dependency only*. comrak is a Rust port of
  cmark-gfm, the parser github.com uses, and implements CommonMark 0.31.2 + GFM exactly. The app
  binary does not link it.
- Later, when export to HTML is wanted, comrak can produce it; that is a separate capability decision.

### 11.4 Conformance tests

- Vendor the official example suites as test data: CommonMark `spec.json` (652 examples) and the GFM
  extension examples, under `crates/inkuno-logic/tests/spec/`, with their licences.
- For each example, compare **block structure per source line** — Inkuno's `classify` result against
  the block comrak assigns to that line (heading level, list item and depth, quote, code, table,
  thematic break, paragraph, HTML block) — and **inline spans** for emphasis, strong, code, links,
  strikethrough and autolinks.
- Keep a checked-in **expected-failures list**. The test fails if an example in it starts passing
  (remove it) or one not in it fails (a regression). The list only shrinks; its size is the
  conformance score, reported in this section.

Score 2026-11-21: CommonMark 334/652, GFM extensions 9/24 (`tests/conformance.rs`; the failing
examples are listed in `crates/inkuno-logic/tests/spec/expected-failures.txt`).

Score 2026-11-21, after the first fixes of §11.5: CommonMark 384/652, GFM extensions 20/24. Gains by
section: Emphasis and strong emphasis 57 to 82/132, Links 22 to 32/90, Setext headings 9 to 12/27,
Autolinks 5 to 6/19, Images 15 to 16/22, Tabs 2 to 3/11, Code spans 8 to 9/22; GFM Autolinks
(extension) 0 to 11/11. Unchanged: Indented code blocks 3/12, List items 26/48, HTML blocks 23/44.
One example newly fails and is listed as an explained conflict: cm 612 (`foo@bar.example.com` is
plain text in CommonMark, but GFM example 629 requires the same form to be a `mailto:` link; Inkuno
ships both, so it reads GFM).

Worst sections, top 8 by failures:

| Spec | Section | Passed | Failing |
|--|--|--|--|
| CommonMark | Emphasis and strong emphasis | 57/132 | 75 |
| CommonMark | Links | 22/90 | 68 |
| CommonMark | List items | 26/48 | 22 |
| CommonMark | HTML blocks | 23/44 | 21 |
| CommonMark | Setext headings | 9/27 | 18 |
| CommonMark | Link reference definitions | 9/27 | 18 |
| CommonMark | Code spans | 8/22 | 14 |
| CommonMark | Autolinks | 5/19 | 14 |

### 11.5 Known gaps to close, in order

1. Setext headings (`Title` underlined with `===` / `---`).
2. Backslash escapes and entity / numeric character references.
3. Reference links and link reference definitions; link titles.
4. Angle autolinks `<https://…>` and GFM extended autolinks (`www.`, e-mail).
5. Hard line breaks (two trailing spaces, trailing backslash).
6. List rules: interrupting a paragraph, lazy continuation, loose vs tight.
7. HTML blocks and inline HTML: shown as dimmed source, never executed; GFM's tag filter applies.
8. Converting the Outside tier: `[[ ]]` and `@` pickers write relative links; comments move to
   `passage<!-- comment: … -->` (done 2026-11-21, Q1 in `prototypes/README.md`: the CriticMarkup form is still
   read, and rewritten on save; pending changes are beside the note, Q2).

Found by the first conformance run (2026-11-21), folded into the order above as quick wins first:

- Quick wins: `__strong__` (done 2026-11-21); empty ATX heading `#` (done 2026-11-21); spaced rules
  `- - -` / `* * *` (done 2026-11-21); four-space indent stops a heading or rule (done 2026-11-21);
  relative links `[x](/path)` styled, not clickable, not only `http(s)` (done 2026-11-21); Unicode-aware
  intraword `_` (cm 362) (done 2026-11-21).
- Indented code blocks (four spaces) recognised as code.
- GFM tables: rows without a leading pipe; a delimiter row with the wrong column count is not a table
  (gfm 203).
- GFM extended autolinks (`www.`, e-mail, bare URLs with trailing punctuation rules): 0/11 today (done 2026-11-21: 11/11).
- **Multi-line inline spans** (emphasis, code or a link crossing a line break, e.g. cm 121, 405) fail
  by construction of the per-line classifier. Report them as a separate count, not as rendering
  failures, until the editor's inline pass can look across soft breaks.

### 11.6 Where it is stated

`docs/user/src/writing.md` opens with one line: Inkuno writes CommonMark with GitHub's extensions,
so every note reads the same on github.com. The README says the same.

Follow-ups decided 2026-11-21 (orchestrator):

- Relative links `[x](Note.md)`, `[x](/path)`, `[x](#heading)` must also be **clickable** and hide
  their brackets off the live line like web links: they replace `[[wiki]]` links (prototype decision
  #16), so they need the same treatment. App crate: `links()` callers and `layout.rs` bracket hiding.
- Indented code and pipe-less tables need a **block-level classifier API** (previous-line state and a
  one-line lookahead) that both the editor and the conformance test call; the editor already keeps
  this state in `views/editor/listing.rs` (`starts_indented_code`). Move that logic into
  `inkuno-logic` rather than duplicating it.
