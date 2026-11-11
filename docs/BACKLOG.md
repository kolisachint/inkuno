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

## Uncommitted worktree slices (surveyed 2026-11-11, carry into next session)

Verified: every agent branch is fully merged into `origin/main` — zero commits ahead anywhere. Everything below is UNCOMMITTED working-tree state: crews stopped mid-task and never committed. All worktrees sit on Nov-09 base `4104d2b` (two days stale); anything revived needs a rebase first, and some may already be superseded by 0.0.2.

Trim crews (debloat order):
- `trim-canvas` (~66 files) — cut canvas/diagram editor, keep mermaid engine. Deletions staged-but-uncommitted across `logic/diagram/*`, `views/canvas/*`, shell registries + tests. Largest open slice. Resume-or-drop call.
- `trim-tables` (~70 files) — cut sheets/boards/CSV/kanban/tables. `logic/table.rs`, `formula.rs`, `views/sheet/*`, `views/kanban.rs` deleted uncommitted; stubs re-created (`views/sheet.rs`, `views/kanban.rs` untracked). DEFERRED by user over runtime — needs explicit permission to resume.
- `trim-obsidian` (49 files) — cut embeds/graph/templates/calendar, keep markdown. Partial per crew reports, uncommitted; overlaps canvas/chrome territory — reconcile before resuming.
- `trim-chrome` (23 files) — cut docks/capture/search/travel chrome. Shell modules deleted uncommitted.
- `trim-sync` (15 files) — cut git sync + auto-update (`platform/git.rs`, `platform/update.rs`, `shell/sync.rs`, `shell/updates.rs` deleted uncommitted).
- `trim-deps` (16 files) — drop PDF-only deps (`platform/pdf.rs`, `shell/print.rs`, `ui/print.rs` deleted uncommitted).
- `trim-tables-basecheck` — clean detached-HEAD reference checkout. Nothing pending; deletable with its worktree.

Feature crews (GitHub-parity / Typora-feel orders):
- `feat-codehide` (6 files) — Typora-style code folding, then REVERTED late Nov-10; worktree still holds uncommitted *forward* edits. Decide: drop worktree or re-do. (Makes the `feat-codehide` follow-up bullets under Product calls above stale — re-point or close them.)
- `feat-footnotes` (15 files + new `shell/tests/end_to_end/footnotes.rs`) — second-wave footnotes work, uncommitted; "shipped per spec" above may only cover wave one.
- `feat-linkhide` (1 file) — wikilink/URL hiding, single `views/editor/layout.rs` edit uncommitted.
- `feat-logic-crate` (23 entries) — extract `logic/` into `crates/`; old `src/logic/*` deleted, new `crates/` untracked. Mid-surgery, uncommitted; `vault.rs`/platform-crate verdict waits on this.
- `ui-tree-header` (5 files) — Zed-style header (drop folder name, left-align icons); small uncommitted slice in `shell/render.rs`, `sidebar.rs`, `tabs.rs`, `mod.rs` + `tests/window.rs`.
- `macos-build` — committed work merged; only untracked `dist/` build output remains. Deletable.

Brand/design crews (deferred to design session):
- `brand-icons` — new `Inkuno.iconset/`, rebuilt `.icns`, both SVGs, bundle-script fix, new `packaging/windows/`. All uncommitted.
- `brand-theme` — `ui/theme.rs`, `ui/assets.rs`, `main.rs`, help + settings e2e. All uncommitted.

CI/release-speed crews (30-minute pipeline plan, never proven end-to-end):
- `ci-tier`, `ci-cache`, `ci-artifacts`, `ci-release-fast`, `macos-ci-test` (2–5 files each) — tiered CI, sccache/nextest, artifact reuse, release-profile cuts. Small uncommitted workflow diffs.
- `update-check-inkuno` (2 files) — update-check repointed at Inkuno (`RELEASING.md`, `scripts/release.sh`), uncommitted.
- `ops-prune` — untracked `scripts/prune-agents.sh` only; verify against main (may already exist) before treating as new.

Fully merged, nothing to carry: `json-config`, `release-gate`, `inkuno-docs`, `inkuno-rename`, `inkuno-0.0.1`, `ci-fix` (`c63451a` already in main history).

Stash/state leftovers:
- Main tree: clean except untracked 3-line scratch `Sachin.md` (`# Sachin / ## koli`). Delete or ignore.
- `stash@{0}` (`agent/features` WIP): mostly `.hoocode/dispatch/` run-logs (junk) + `ARCHITECTURE.md`, `docs/design/markdown_renderer.md`, `book/src/img/architecture-layers.svg` — check the docs for salvageable decisions, drop the rest.
- `stash@{1}` (old `main` WIP, bref-swap era): keymap/sidebar/tabs/render/nav edits + `docs/design/ui.md`. Pre-rename vintage, likely superseded; `ui.md` may predate current design docs — skim before dropping.

## Deferred by explicit user decision
- **Design session** — brand-icons + brand-theme slices above, plus dark-theme nib-contrast human check and tree-header/folder-naming restyle. Not this session.
- **Finish tables** — deferred over 1hr+ runtime; resumes only with explicit permission.
- **Parked small calls** — the Product calls / Code cleanups / Product gaps sections above stand as parked.

## Stale-or-unverified (confirm before next session builds on it)
- 30-minute pipeline shape approved but never run end-to-end; `ci-*` slices unproven.
- `scripts/prune-agents.sh` existence on main vs `ops-prune` untracked copy — dedupe.
- `feat-codehide` references in this backlog postdate the Nov-10 revert — re-point or close.
- 29 worktrees on Nov-09 base: prune dead ones per keep/kill before reviving anything (rebase required).
