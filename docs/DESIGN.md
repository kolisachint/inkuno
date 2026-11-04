# noteapp — Design

> **Status:** v1 design baseline. Decisions written here are binding for v1. Anything not yet decided is listed in [§16 Open Questions](#16-open-questions).

## Contents

1. [Goals and non-goals](#1-goals-and-non-goals)
2. [Core principle](#2-core-principle)
3. [Architecture](#3-architecture)
4. [Live preview semantics](#4-live-preview-semantics)
5. [Modes](#5-modes)
6. [Rich block rendering](#6-rich-block-rendering-crate-md_render)
7. [Editor element](#7-editor-element-crate-md_editor)
8. [Workspace](#8-workspace-crate-workspace)
9. [Theming](#9-theming)
10. [Crate layout](#10-crate-layout)
11. [Licensing](#11-licensing)
12. [Performance targets](#12-performance-targets)
13. [Risks and spikes](#13-risks-and-spikes)
14. [Verified facts and sources](#14-verified-facts-and-sources)
15. [Reference repositories](#15-reference-repositories)
16. [Open Questions](#16-open-questions)

---

## 1. Goals and non-goals

### 1.1 Goals

| # | Goal | Where it is designed |
|---|------|----------------------|
| G1 | Typora-style real-time WYSIWYG live preview of Markdown | §4 |
| G2 | Inline rendering of code fences, syntax highlighted | §6.6 |
| G3 | Math with KaTeX-compatible LaTeX syntax | §6.4 |
| G4 | Mermaid diagrams rendered inline | §6.3 |
| G5 | Source Code Mode, Focus Mode, Typewriter Mode | §5 |
| G6 | Folder-tree sidebar and full-text search | §8 |
| G7 | Light / Dark / Auto theme. Auto is the default and follows macOS appearance | §9 |
| G8 | Pasted images saved into an `assets/` folder next to the note, linked relatively | §7, §8 |

**Platform:** macOS first, using the GPUI Metal renderer.
**License:** GPL-3.0-or-later.

### 1.2 Non-goals (v1)

- Wiki-links and backlinks.
- PlantUML. A future opt-in will run a local JAR behind the `DiagramRenderer` trait (§6.7).
- Custom CSS themes.
- Linux and Windows.
- Sync.
- Export.

---

## 2. Core principle

**The raw Markdown text in a rope buffer is the only source of truth. Rendering is a projection of that text.**

Every edit, including WYSIWYG input rules such as heading shortcuts and list continuation, is a text edit applied to the buffer. This has three consequences:

- Undo/redo, save, and Source Code Mode come for free, because they all operate on the same text.
- The file on disk is the buffer's text. There is no separate document model to keep in sync.
- Rich views (rendered blocks, reveal state, focus dimming) are derived state. They can be discarded and rebuilt from the buffer at any time.

Any behaviour that cannot be expressed as a text edit or as a projection of text is out of scope for the editor core.

---

## 3. Architecture

### 3.1 Pipeline

```
+--------------------------------------------------------------+
|  Rope buffer  (Zed text / rope / sum_tree)                   |
|  anchors, transactions, undo/redo        <-- source of truth |
+--------------------------------------------------------------+
                              |
                              |  edits -> dirty byte ranges
                              v
+--------------------------------------------------------------+
|  Block splitter                                              |
|  splits buffer into top-level blocks with source ranges      |
+--------------------------------------------------------------+
                              |
                              v
+--------------------------------------------------------------+
|  Per-block parse                                             |
|  pulldown-cmark offset iterator, cached by content hash;     |
|  only dirty blocks are reparsed                              |
+--------------------------------------------------------------+
                              |
                              v
+--------------------------------------------------------------+
|  Display map                                                 |
|  source ranges <-> visual positions;                         |
|  hidden-syntax ranges are collapsed here                     |
+--------------------------------------------------------------+
                              |
                              v
+--------------------------------------------------------------+
|  GPUI virtualized, variable-height list()                    |
|  only visible blocks are laid out                            |
|                                                              |
|  Paragraph  Heading  List  Blockquote  Table  CodeFence      |
|  MathBlock  Mermaid  Image  ThematicBreak                    |
+--------------------------------------------------------------+
```

### 3.2 Stages

- **Rope buffer.** Built on Zed's `text`, `rope` and `sum_tree` crates. Provides anchors that survive edits, transactions, and undo/redo.
- **Block splitter.** Splits the buffer into top-level Markdown blocks (paragraph, heading, list, quote, table, code fence, math block, thematic break, HTML block). Each block records its source byte range. After an edit, only blocks overlapping the dirty range are re-split.
- **Per-block parse.** The pulldown-cmark offset iterator yields events with source ranges. Results are cached by content hash, so only dirty blocks are reparsed.
- **Display map.** Translates between source ranges and visual positions. Hidden-syntax ranges (§4) are collapsed here. Caret movement and hit-testing operate on the display map, so they skip hidden ranges.
- **GPUI list.** A virtualized, variable-height `list()` of block elements. Only visible blocks are laid out.

---

## 4. Live preview semantics

The target behaviour is Typora-like. The reference for expected behaviour is the MarkText `muya` engine (§15).

### 4.1 Inline reveal

Each inline span has marker ranges (for example the two `**` runs in `**bold**`) and a content range. The rule is:

- If the caret is inside a span, show that span's markers.
- Otherwise, hide that span's markers.

| Span | Syntax | Markers hidden when caret is outside |
|------|--------|--------------------------------------|
| Strong | `**text**` | the two `**` runs |
| Emphasis | `*text*` | the two `*` runs |
| Inline code | `` `code` `` | the backticks |
| Link | `[text](url)` | `[`, and `](url)` |
| Image | `![alt](src)` | `![`, and `](src)` |
| Inline math | `$x$` | the two `$` runs |
| Strikethrough | `~~text~~` | the two `~~` runs |

The display map collapses hidden ranges, so caret movement and hit-testing skip them (§3.2).

### 4.2 Block reveal

- **Code fences** always show as highlighted text. The fence lines (opening and closing fence, info string) are hidden when the caret is outside the block and revealed when the caret is inside it.
- **Math and Mermaid blocks** show their rendered result until the caret enters the block. When the caret enters, they show the source, with the live preview below it.

### 4.3 Input rules

Every rule below is a text edit applied through the buffer's transaction system (§2).

| Trigger | Effect |
|---------|--------|
| `# ` through `###### ` at block start | Block becomes a heading, levels 1 to 6 |
| `- `, `* `, `1. ` at block start | Block becomes a list item |
| `> ` at block start | Block becomes a blockquote |
| `- [ ] ` at block start | Block becomes a task list item |
| ` ``` ` then Enter | Code fence (opening and closing fence lines inserted) |
| `$$` then Enter | Math block |
| `---` then Enter | Thematic break |
| Enter inside a list item | Continues the list with a new item |
| Enter on an empty list item | Exits the list |
| Backspace at block start | Demotes the block |

### 4.4 Behaviour reference

MarkText `muya` engine (see §15). It is the behavioural reference for expected editing feel. Its code is not a dependency.

---

## 5. Modes

Modes are views over the same buffer. Focus Mode and Typewriter Mode can be combined with each other, and both also work in Source Code Mode.

### 5.1 Source Code Mode

- No ranges are hidden. Every marker is visible.
- Highlighting uses tree-sitter-markdown.
- Toggling modes preserves the caret offset and the scroll anchor, both expressed as source offsets. The caret keeps its source byte offset. The scroll position is anchored to the source offset of the top visible block.

### 5.2 Focus Mode

Every block other than the caret's block is drawn at reduced opacity.

### 5.3 Typewriter Mode

- After each caret move, the view scrolls so the caret line sits at a fixed fraction of the viewport. The default fraction is 0.5.
- The document gets half-viewport overscroll padding at the top and bottom, so the first and last lines can reach the fixed position.
- Scrolling is smooth.

### 5.4 Combinations

- Focus Mode and Typewriter Mode can be enabled together.
- Both work in Source Code Mode.

Key bindings for modes are an open question (§16).

---

## 6. Rich block rendering (crate `md_render`)

### 6.1 Render host

One shared hidden WKWebView, called the **render host**, runs:

- Bundled `mermaid.js`.
- Bundled MathJax v3.

It uses the system WebKit, reached through `wry` or `objc2`. Both libraries are bundled with the app and pinned to exact versions. The render host is the only place WebKit is used.

### 6.2 Request contract

- **Request:** `(kind, source, theme)`, where `kind` is Mermaid or Math, `source` is the diagram or LaTeX text, and `theme` is the active theme.
- **Response:** an SVG string.
- Requests are asynchronous and run off the UI thread.
- Results are cached by `hash(kind, source, theme)`. Each theme has its own cache entries.
- A placeholder is shown until the render is ready.
- While the source is invalid, the last good render is kept on screen.
- Render errors are displayed inline.

### 6.3 Mermaid

Rendering uses the Mermaid render API:

```js
const { svg } = await mermaid.render(id, src)
```

- Top-level config sets `htmlLabels: false`. The `flowchart.htmlLabels` option is deprecated, so the top-level option is used.
- Themes: `default` for Light, `dark` for Dark.

### 6.4 Math

- MathJax v3 renders with `tex2svgPromise`.
- `fontCache: 'none'` is set so each SVG is standalone.
- The math fill colour comes from the active theme.

**Why MathJax and not KaTeX:** KaTeX outputs HTML and CSS, which GPUI cannot render. MathJax outputs SVG and accepts the same LaTeX syntax.

### 6.5 SVG to pixels

- SVG is rasterized with resvg/usvg, and the result is handed to GPUI as an image.
- Images are re-rasterized when the window's scale factor changes.
- The GPUI `svg()` element is designed for monochrome icons, so it is not used here.
- Fonts are bundled so that WebKit's text measurement matches resvg's drawing.
- Each theme has separate cache entries (§6.2).

### 6.6 Code fences

Code fences are highlighted with tree-sitter grammars, through the `Highlighter` trait (§6.7). The language comes from the fence's info string.

### 6.7 Traits

| Trait | Purpose | v1 implementation | Future |
|-------|---------|-------------------|--------|
| `DiagramRenderer` | Renders diagram source to SVG | Mermaid | PlantUML, opt-in via local JAR (§1.2) |
| `MathRenderer` | Renders LaTeX to SVG | MathJax v3 | Native mitex to Typst (§6.8) |
| `Highlighter` | Tokenizes code for highlighting | tree-sitter | none planned |

### 6.8 Fallbacks

- **Render host.** If the hidden WKWebView proves unreliable (UNVERIFIED, §14), attach the webview to a real NSWindow positioned offscreen.
- **Math only.** Run MathJax v3 in embedded rquickjs with liteAdaptor, which needs no DOM.
- **Mermaid.** No non-DOM fallback exists.
- **Future.** A native mitex to Typst path, placed behind `MathRenderer`.

---

## 7. Editor element (crate `md_editor`)

This is a custom GPUI `Element`. It implements:

- `EntityInputHandler`, for IME composition and marked text.
- Caret and selections.
- Hit-testing through the display map (§3.2).
- Keyboard navigation.
- Clipboard. Copy produces Markdown.
- Image paste:
  1. Write the image to `assets/<note-stem>-<timestamp>.png`.
  2. Create `assets/` if it is missing.
  3. Insert a relative link `![](assets/...)` at the caret.

**References:**

- Zed `editor` crate, for display map, selections and IME.
- `longbridge/gpui-component` editor component.

The base for `md_editor` is chosen in spike S4 (§13).

**Risk:** this is the highest-risk component in the project.

---

## 8. Workspace (crate `workspace`)

- **Vault.** A vault is a folder of plain `.md` files.
- **Sidebar.** A folder-tree sidebar shows the vault.
- **File watching.** The `notify` crate watches the vault.
- **File operations.** Create, rename and delete.
- **Full-text search.** Built on the ripgrep crates `ignore` and `grep-searcher`. There is no index in v1. Results appear in a results panel, and selecting a result jumps to the match.
- **Images.** Pasted images follow the rule in §7.

---

## 9. Theming

- **Modes:** Light, Dark, and Auto.
- **Default:** Auto. It follows the macOS appearance through GPUI's window appearance-change events.
- **Override:** the user can set a manual theme in settings.
- **Colours** are defined as tokens. The render cache is theme-aware (§6.2).

---

## 10. Crate layout

The project is a Cargo workspace.

| Crate | Responsibility | Notes |
|-------|----------------|-------|
| `md_core` | Buffer, transactions, block splitter, parser, display map, input rules | **No GPUI dependency.** Heavily unit-tested. |
| `md_render` | Render host, resvg, tree-sitter highlighting, caches | §6 |
| `md_editor` | GPUI editor element, modes | §7, §5 |
| `workspace` | Folder tree, file watcher, search | §8 |
| `app` | Window, menus, theme, settings, file IO | §9 |

**GPUI dependency.** GPUI is a git dependency on `zed-industries/zed`, pinned to a specific revision. The crates.io `gpui` release (0.2.2) is stale and is not used. The pinned revision is bumped deliberately, as its own change with build and test verification.

---

## 11. Licensing

| Component | License |
|-----------|---------|
| gpui | Apache-2.0 |
| sum_tree | Apache-2.0 |
| rope | GPL-3.0-or-later |
| text | GPL-3.0-or-later |
| editor | GPL-3.0-or-later |
| markdown | GPL-3.0-or-later |
| gpui-component | Apache-2.0 |
| noteapp (this project) | GPL-3.0-or-later |

Apache-2.0 code can be incorporated into a GPL-3.0 work, so the project as a whole is distributed under GPL-3.0-or-later.

---

## 12. Performance targets

- **Keystroke to paint:** under 16 ms on documents of 10,000 lines.
- **Reparse:** only dirty blocks are reparsed.
- **Layout:** only visible blocks are laid out.
- **Rich rendering:** Mermaid and math renders are asynchronous and cached.

These targets are checked in the performance pass (Phase 10, task 10.4 in the implementation plan).

---

## 13. Risks and spikes

Four spikes run before the corresponding implementation phases. Each spike ends with a go/no-go decision recorded in `docs/spikes/`.

| Spike | Question | Decides |
|-------|----------|---------|
| **S1** Render host | Hidden WKWebView (`wry` with `with_visible(false)`) or offscreen NSWindow? Do Mermaid and MathJax produce SVG? Does Mermaid output contain no `<foreignObject>` with `htmlLabels: false`? Does resvg render the output correctly, given that `foreignObject` support is believed absent but unverified? What is the round-trip latency? | §6.1, §6.8 |
| **S2** GPUI | Does a pinned git dependency on GPUI build, and does a hello window run on macOS? | Pinned revision (§10) |
| **S3** Display map | Can caret movement and IME work across hidden `**` markers? | Display map design (§3.2, §4.1) |
| **S4** Editor base | Is the `gpui-component` editor or the Zed `editor` crate the better base for `md_editor`? | §7 |

---

## 14. Verified facts and sources

**Verified:**

- GPUI on crates.io is stale, so the project pins a git revision. Sources: https://github.com/zed-industries/zed/issues/66275 and https://crates.io/crates/gpui/versions
- Mermaid render API and the `htmlLabels` option. Sources: https://mermaid.js.org/config/usage.html and https://mermaid.js.org/config/schema-docs/config.html
- resvg unsupported features. Source: https://github.com/linebender/resvg/blob/main/docs/unsupported.md
- gpui-component. Source: https://github.com/longbridge/gpui-component
- MathJax. Source: https://github.com/mathjax/MathJax

**UNVERIFIED (must be settled in spike S1):**

- Layout behaviour of a hidden WKWebView.
- resvg support for `<foreignObject>`. It is believed absent.
- Whether `htmlLabels: false` removes all `<foreignObject>` elements from Mermaid output.

---

## 15. Reference repositories

- https://github.com/zed-industries/zed (display map, selections, IME, GPUI)
- https://github.com/marktext/marktext (`muya` engine, live preview behaviour)
- https://github.com/longbridge/gpui-component (editor component)
- https://github.com/mermaid-js/mermaid
- https://github.com/mathjax/MathJax
- https://github.com/linebender/resvg

---

## 16. Open Questions

1. **Mode key bindings.** The proposals are Cmd+/ for Source Code Mode, F8 for Focus Mode, and F9 for Typewriter Mode. None of these is decided.
2. **Table editing UX.** Should tables use a Typora-style grid, or reveal source when the caret enters the table? Undecided.
3. **Export.** Not in v1. Formats and scope are undecided.
4. **Render host approach.** Hidden WKWebView or offscreen NSWindow. Decided by spike S1.
5. **Pinned versions.** The exact versions of mermaid.js and MathJax v3. Chosen during S1.
6. **GPUI revision.** The exact Zed revision to pin. Chosen during S2.
7. **Editor base.** `gpui-component` editor or Zed `editor` crate. Chosen by spike S4.
8. **Mermaid debounce interval.** The delay before a re-render after an edit. Chosen in Phase 7.
9. **Settings storage.** The location and format of the settings file. Undecided.
