# Backlog — parked, out of session scope

Decided 2026-11-09: documented here, dropped from active work. Each item says where it came from and what unblocks it.

## Product calls
- **Root `inkuno.ico` orphan** — zero references repo-wide (verified by brand-theme crew). Safe to delete. One-line removal.
- **Dark-theme nib contrast** — charcoal nib `#20232B` on `#171A22` ≈ 1:1; only the cobalt stroke shows on Inkuno Dark. Kit-compliant as shipped; needs human eyes to confirm acceptable.
- **Linux icon default** — no action needed: `Theme::of` already follows system appearance. Closed.
- **Collapsed code header line count** — Typora shows one; ours doesn't. One-line paint addition in `feat-codehide` follow-up.
- **Chevron glyph vs SVG icon** — code-fold chevron is a text glyph (▸/▾); find-bar has SVG chevron assets if HiDPI/CJK fallback looks wrong.
- **Folded blocks hide the closing fence** — shipped header-only; confirm desired vs showing trailing ` ``` `.
- **Mermaid exemption from folding** — code-hide crew's call so figure slots stay intact; confirm with editor use.
- **Wikilink bracket hiding** (`[[note]]`) — left readable; easy follow-up in the link-hide `urls` mechanism.
- **Mermaid kind coverage** — display engine already renders flowcharts/ER; only the canvas converter is limited to class/sequence/state. Sketch filed by link-hide crew: extend converter incrementally, flowchart first (LARGE slice, own workstream). Renderer swap explicitly not recommended.
- **`↩` backlink glyph for footnotes** — v1 uses clickable-label cycling; glyph insertion needs untested `subs` path. Follow-up.
- **Multi-ref footnote cycling, def-without-ref no-op** — shipped per spec; confirm feel in use.

## Code cleanups (integrator or later call)
- `Editor::lookups` (`views/editor/mod.rs:591`) — dead after PDF-trim; remove or keep.
- `logic/history.rs` — consumer-less after travel-trim; delete or keep for old-layout tolerance.
- `pane.tool` machinery (`tool_of`, `panes_state` `@tool`) — unreachable after obsidian-trim; remove or keep.
- `Pane::viewer()` — no non-test callers after canvas-trim; remove or keep.
- `logic/prefs.rs` `updates` field — silently ignored after sync-trim; migrate or drop.
- `shortcuts.md` regen via `make docs` — needs a real build; CI must do it (repo fmt baseline also predates toolchain — normalize decision needed).
- `book/src/img/git-sync.svg` orphaned asset; `README.md:25` save-as-pdf anchor dangling.

## Structural follow-ups (spec'd, not started)
- **Platform crate** — audit verdict: extract `clock`/`export`/`link` now, `vault.rs` stays until logic crate lands. Re-check `export` (only caller is sheet clipboard — delete-don't-extract if sheet trim stands).
- **Screenshots/og-image regen** — needs a display runner (`scripts/record-demo.sh`).

## Product gaps (from e2e walk, 2026-11-10)
- **ReplaceInVault live UI, dead engine** — `secondary-shift-enter` binding (`keymap.rs:55,468`) and the find-bar replace-vault button (`views/editor/find.rs:519-530`) fire a documented no-op (`replace_in_vault`). Remove the affordance or re-implement on a new vault-search home.
- **Daily note has no chord** — `open_today` reachable only from the toolbar button (`render.rs:709`); Ctrl+J/palette entry cut with the nav modes. Confirm the button is the intended sole home.
- **`inkuno --capture` gone** — desktop capture shortcut removed; `vault::capture` platform helper + tests remain (unused by the app).

## Process (standing)
- 20-minute permission gate on all workers; fast-lane protocol (CI compiles, small slices, shared target, heartbeats).
- `scripts/prune-agents.sh` runs on any disk-full stall.
