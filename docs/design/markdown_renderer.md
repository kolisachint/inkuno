# Markdown renderer compatibility contract

> Docs-only contract. No code, no behavior change.
> Per `docs/design/app-ui.md` §1 (rule 3): every claim about what the app does today
> cites a `file:line`; anything unverified is marked **needs-eyes**, never asserted.
> Grounding refs: `docs/design/app-ui.md` §§3 (`app-ui.md:48-106`, tokens + typography
> system) and 12 (`app-ui.md:411-479`, visual spec v1, esp. §12.2 center canvas).
> Parser ground truth: `src/logic/markdown/` (`mod.rs`, `tables.rs`, `lists.rs`,
> `front.rs`, `dates.rs`, `search.rs`); rendering ground truth:
> `src/views/editor/` (esp. `layout.rs`, `render.rs`, `paint.rs`, `style.rs`,
> `embed.rs`, `formulas.rs`, `grid.rs`, `table.rs`); user-facing spec:
> `book/src/writing.md`.

## 1. Tier table (user-set contract)

| Tier | Spec | What's in scope |
|------|------|-----------------|
| Core | CommonMark | Paragraphs, ATX headings (6 levels), bullet / ordered / task lists, blockquotes, fenced code blocks, thematic breaks, inline emphasis (bold, italic, code, links), autolinks, hard/soft breaks |
| Extended | GFM | Tables, strikethrough, task-list items (`- [ ]` / `- [x]`), autolink extension, `> [!NOTE]`-style callout panels |
| Optional | Beyond GFM | Math (`$…$` inline, `$$…$$` display), Mermaid diagrams (`` ```mermaid ``), footnotes |
| Sanitized | HTML support | Inline `<u>`, `<mark>` (incl. `<mark class="…">` colors); everything else: kept as typed, never executed |

Non-goals of this contract: editing interactions (marker hiding lives with the
editor crew), export fidelity (PDF rules live in `book/src/writing.md` "Save as
PDF"), and any new syntax. Source on disk stays plain Markdown
(`book/src/writing.md:3-5`; `app-ui.md` vision, `app-ui.md:30-46`).

## 2. Core (CommonMark) — status vs gaps

What exists today (verified by reading code):

- **Block classification** is a line-by-line classifier: `Kind` enum
  (`src/logic/markdown/mod.rs:21`) + `classify`
  (`src/logic/markdown/mod.rs:103`) covering `Para`, `Heading(1-6)`,
  `Bullet`, `Ordered`, `Task(bool)`, `Quote`, `Fence`, `Code`, `Rule`,
  `Table`, `Meta` (front-matter lines). Fences detected by `is_fence`
  (`mod.rs:80` area: `count_fences_and_dollars`; fence predicate nearby).
- **Code blocks**: content range via `code_block` (`mod.rs:751`); single-line
  per-language lexer `code` (`mod.rs:683`) coloring comments / strings /
  numbers / keywords; language name taken from the info string, with
  `"" | "text" | "txt" | "plain" | "mermaid"` treated as uncolored
  (`src/views/editor/layout.rs:1568`); copy affordance is documented behavior
  (`book/src/writing.md:30`).
- **Inline styles**: `inline` (`mod.rs:278`) over `emphasis` (`mod.rs:295`)
  for `**` bold, `*`/`_` italic, `` ` `` code, `~~` strike, `==` highlight,
  `<u>` underline, `<mark>` colors; markers are flagged `DIM`
  (`src/logic/markdown/mod.rs`, `DIM` flag; user-visible rule:
  "markers stay visible but dimmed", `book/src/writing.md:5`). Bars between
  blocks, checkbox styling, and arrow substitutions (`->` → `→`, etc. via
  `symbols`) follow the same flag pipeline.
- **Links**: protocol-gated `is_url` / `web_link` + `links`
  (`src/logic/markdown/mod.rs`, `is_url`/`web_link`/`links` block) for
  `[text](address)`, bare `http(s)://` URLs, `[[Wiki]]` targets, `#tag`
  ranges; `[ ]`/`[x]` boxes tracked separately (`checkboxes`).
- **Lists**: Enter continuation / renumber / restyle live in
  `src/logic/markdown/lists.rs` (`on_enter`, `renumber`, `restyle`).
- **Front matter**: `front_matter` / `body_start` / keys / values
  (`src/logic/markdown/front.rs`); `Meta` lines are excluded from heading
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
  (`src/logic/markdown/tables.rs:7`, `:24`); grid ops (insert/delete/move/
  sort/transpose) in the same file; source-side column alignment capped by
  `ALIGNED = 100` (`tables.rs:472-494`, `format_table` at `tables.rs:475` —
  beyond that width columns are no longer padded but display is unchanged).
  Editor display: a table line becomes a grid whose cells wrap
  (`src/views/editor/layout.rs:283`); grid lines themselves never wrap
  (`layout.rs:1600`); non-table lines wrap at the measure
  (`layout.rs:1752-1754`). Cell styling reuses the inline pipeline per cell
  (`highlights` / `tags` cell paths in `mod.rs`).
- **Strikethrough + task items**: `STRIKE` flag + `Task(bool)` kind (see §2);
  `Ctrl+Enter` toggles boxes (`book/src/writing.md:12`, `:62`).
- **Callout panels**: `> [!NOTE]` etc. with per-kind colors, GitHub-style
  (`book/src/writing.md:29`); the five `/note…/caution` slash commands insert
  them (`book/src/writing.md:72-80`).
- **Autolinks**: bare `http(s)://` handled in `links` (§2).

Gaps / **needs-eyes**:

- GFM table alignment colons (`:---:`) round-trip: partially handled in
  `format_table` (colon preservation) — **needs-eyes**: verify center/right
  alignment renders, not just survives.
- Discontinuous tables (blank line inside a table), multiple header rows:
  **needs-eyes**.
- GFM alert titles / custom titles beyond the five kinds: not supported
  (only `NOTE TIP IMPORTANT WARNING CAUTION`, `book/src/writing.md:29`).
- Large tables: `app-ui.md` acceptance criterion cites "> 1500 rows use light
  layout" (`app-ui.md:190-216`) — **needs-eyes**: verify the threshold constant
  and path in `layout.rs`/`grid.rs` before citing it as contract.

## 4. Optional (Math, Mermaid, footnotes) — status vs gaps

What exists today:

- **Math**: `$$` fence-line counting in `count_fences_and_dollars`
  (`src/logic/markdown/mod.rs:80`); formulas drawn as figures *under* their
  line, first formula of a line only (`book/src/writing.md:32`); display
  figures built via `figure::latex` (`src/views/editor/layout.rs:1374`,
  `:1796`). Inline `$x^2$` is styled/kept inline, not drawn; PDF draws only
  `$$` formulas (`book/src/writing.md`, "Save as PDF" limits).
- **Mermaid**: only ` ```mermaid ` blocks whose source is one of
  class / sequence / state diagrams convert (`src/logic/mermaid.rs`: `kind`,
  `to_diagram`); anything else (flowcharts, `graph TD`, erDiagram, …) counts
  skipped lines, not a figure. Rendering: the block reserves a figure slot
  when the cursor has left it (`layout.rs:1341-1356`), drawn off-thread via
  `figure::mermaid` (`layout.rs:1348`, engine in `src/ui/figure.rs:41-48`);
  unreadable source shows "Figure illisible" / "Unreadable figure"
  (`layout.rs:1356`, `:1862`) — covered by `src/shell/tests/feedback.rs:150-172`
  (good diagram draws, bad/empty diagram says so, `figure_count` asserted).
  User rule: diagram under its block once the cursor leaves; magnifier opens
  full-size (`book/src/writing.md:31`; `book/src/writing.md:23`).
- **Footnotes**: GitHub-style — `[^label]` reference + `[^label]:` definition (see §7-Q3, decided).
  Parsing in `src/logic/markdown/mod.rs`: references `footnote_ref` (`mod.rs:194`),
  definitions `footnote_def` (`mod.rs:238`), document collection `footnotes` (`mod.rs:278`)
  with `count_footnotes` pre-scan (`mod.rs:346`) and per-line defined/undefined styling
  `footnote_flags` (`mod.rs:370`). Defined references render as links with hidden brackets
  off the edited line (`src/views/editor/layout.rs:687`, `:1629`); undefined ones stay literal.
  `Ctrl+click` jumps reference → definition and cycles definition → references
  (`src/views/editor/mouse.rs:696`, hand cursor `src/views/editor/blocks.rs:184`).
  Export: sharing passes them through literally (`src/logic/sharing.rs:286`); printing keeps
  the label in place with no link entry (`src/ui/print.rs:279`); PDF prints them in place
  with no collected section (`book/src/writing.md`, "Save as PDF" limits).

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
  has none — `book/src/writing.md:42`) and `<mark>` / `<mark class="…">`
  with the five `MARKS` colors (`src/logic/markdown/mod.rs:241`,
  `mark_tag` / `mark_color` / `mark_signs`); `<mark>` portability note:
  "plain HTML, so other apps still show the text highlighted"
  (`book/src/writing.md`, inline-styles list).
- Rendering pipeline treats unknown HTML as ordinary text (no tag execution
  path was found in `paint.rs`/`layout.rs`) — **needs-eyes**: confirm there
  is no `Html`/`Embed` branch that interprets arbitrary tags.
- SVG figures from elsewhere are rasterized by resvg, which *ignores*
  `<foreignObject>` HTML and `light-dark()`; `readable` substitutes SVG text
  (`src/ui/svg.rs:1-9`). Sharing/export strips to readable plain text
  (`src/logic/sharing.rs:1-34`, rules in `book/src/writing.md`
  "Plain-text rules").

Gaps / **needs-eyes**:

- No dedicated HTML sanitizer (e.g. ammonia-like allowlist) was found —
  sanitizer choice is open question §6-Q2. Current posture appears to be
  "allowlist-by-construction" (only `<u>`/`<mark>` parsed, rest literal), but
  verify `paint.rs` + export paths (`sharing.rs`, `print.rs`) before
  contracting it.
- `<mark>` inside table cells / headings / PDF output: **needs-eyes**
  (PDF path per `book/src/writing.md` limits).

## 6. Test strategy per tier

| Tier | Unit (pure logic) | Integration (editor/shell) |
|------|-------------------|----------------------------|
| Core | `src/logic/markdown/mod.rs` tests (classify/inline/code/boxes); `lists.rs` Enter/renumber/restyle; `front.rs` matter parsing | `src/shell/tests/end_to_end/writing.rs` (typing/lists/blocks); `outline.rs` (headings skip code blocks); `find_mouse.rs` (marker searchability) |
| Extended | `src/logic/markdown/tables.rs` tests (cells/paste/format); formula tests with `table_formulas.rs` | `md_tables.rs` (paste/align/wrap thresholds — note `app-ui.md:259-290` recalibration rule), `table_handles.rs`, `table_styles.rs`, `csv.rs` |
| Optional | `src/logic/mermaid.rs` tests (kind/convert/skip counts); formula evaluator tests (`logic/formula.rs`) | `feedback.rs:150-172` (mermaid good/bad/empty); `diagrams.rs` (bridge diagrams); `media.rs` (pictures/figures); PDF tests for `$$` vs `$…$` |
| Sanitized | `mod.rs` mark-tag tests (`mark_tag`/`mark_color`); `sharing.rs` plain-text rules | Export/copy tests (Markdown vs plain-text copy, `book/src/writing.md` "Copy and share"); PDF "not printed" list |

Rule for all tiers: when a threshold or pixel value changes (e.g. Normal 800,
`PILL_CLEAR`, `ALIGNED`), re-verify in the order window → page-width →
`md_tables` → boards/cursors → full suite (`app-ui.md:259-290`).

## 7. Open questions

1. **Mermaid overflow**: ~~currently has no contracted overflow rule~~ RESOLVED in principle 2026-11-09 by §9.1 (container-inviolable; scroll/clip internally, magnifier as escape hatch; exact per-figure mechanics with editor crew). Candidates were: (a) shrink-to-measure like PDF
   ("shrunk to fit the page", `book/src/writing.md`), (b) horizontal scroll
   region, (c) clip with magnifier affordance (magnifier already exists,
   `book/src/writing.md:31`). Which is canonical for the canvas? (Related
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
   (`book/src/writing.md`), but the live canvas rule needs contracting
   (related live work: table wrap/clip).
5. **Code-fence rendering scope**: single-line lexer per language
   (`mod.rs:683`, `/* */` and multi-line strings explicitly out of scope in
   the ponytail comment) — is that the permanent contract, or is a per-language
   grammar (tree-sitter) on the roadmap? Info-string edge cases (extra args
   after lang, ` ```mermaid ` vs ` ``` mermaid`) also need pinning.
6. **Marker-hiding interplay** (for the editor crew, recorded here): markers
   are `DIM` today; the arrows-substitution precedent hides replacements only
   off the editing line (`book/src/writing.md:33`). If Typora-style hiding
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
  the existing magnifier affordance, `book/src/writing.md:31`, as the
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
  alignment preserved) in `src/views/editor/` (`layout.rs`, `grid.rs`,
  `table.rs`) citing this doc.
- [ ] Confirm §9.2 split: no hard-coded width / type-scale / padding /
  alignment left in engine paths; application passes them in per consumer
  (notes, agent-chat, documents).
- [ ] Re-verify UI thresholds per `app-ui.md:259-290` (window → page-width →
  `md_tables` → boards/cursors → full suite) and report back here.
