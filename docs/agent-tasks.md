# Agent task list (theme/UI orchestration, 2026-11-23)

Recoverable plan. Status: [ ] todo, [~] in progress, [x] merged. Branch: `claude/theme-ui-orchestration-qwnfet`.
Shared build cache: `CARGO_TARGET_DIR=/home/user/tgt`. Worktrees: `~/.hoocode/worktrees/inkuno/<label>` (AGENTS.md).

## Decisions
- One layout for every theme: the former Quiet layout. Themes only set colours, fonts, sizes, line spacing.
- `Quiet Dark` -> `Dark`, `Quiet Light` -> `Light` (old names still read from settings).
- Agent-editable config, hot-reloaded: `~/.config/inkuno/themes.json`, `~/.config/inkuno/keybindings.json`.

## Wave 1 (parallel, disjoint files)
- [x] T1 theme: every theme uses the Quiet look (derived extras), rename, themes.json takes optional extra tokens — `crates/inkuno-ui/src/theme.rs` (+ name literals in tests/docs)
- [x] T2 hot reload of themes.json + keybindings.json — `inkuno-platform/src/vault.rs`, `main.rs`, `shell/config.rs` (new)
- [x] T3 shared scroll container with auto scrollbar for palette, slash/completion, menus, popovers — `inkuno-ui/src/components/`, `views/palette.rs`, `views/editor/completion.rs`, `shell/menu.rs`
- [x] T5 (picker not built: chip click puts caret on the fence line) code block: closing fence hidden off-cursor, language chip/picker; mermaid + latex: view always, source+view when cursor inside — `views/editor/layout.rs`, `paint.rs`

## Wave 2
- [x] T6 remove the `quiet` flag and every non-Quiet branch
- [x] T4 settings (spacing, code line numbers, restore, config folder; skipped spell check, autosave delay): common line spacing (settings file + settings UI, applied to every kind via one multiplier) + other essential settings
- [x] T7 (agent/t7-width, tgt-t7) + per-block word-wrap toggle top-right (owner) width discipline: tables fit (shrink columns / wrap cells), else horizontal scrollbar; long code lines scroll (backlog R4)

## Wave 3
- [x] T10 language picker popup on the code chip (owner) — start when a target dir frees (disk); shell menu + editor event, rewrite fence info string, one undo step
- [x] T11 inline math in the sentence + no mermaid band (agent/t11-math, tgt-t4) LaTeX fixes (owner): inline `$E = mc^2$` figure clipped (superscript cut) and drawn on a separate line under the source; `$\sum…$` inline not rendered; no border/ring around rendered math (and mermaid box). Start on tgt-t4 when T4 is done
- [x] T12 (opus) blocks never wider than the text column; Centered removed; switch/scrollbar never overlap (agent/t12-tables, tgt-t7)
- [x] T13 images: drop into assets/ next to the note + link; all formats incl. svg; https images via system curl + cache (agent/t13-images, tgt-t13)
- [x] T14 final Opus review of the whole session diff against the owner requests (sense check), then fix findings
- [x] T15 fix review findings F1-F9 (wrap state keys, picker staleness, mermaid scrollbar, base names, img sizes, reload faults/self-writes, $$ in fences, prefs theme names, big drops) (agent/t15-fixes, tgt-t7)
- [x] T16 code chip/wrap switch inside the box (not on its top border); footnote reference drawn as a raised marker; draggable scrollbar thumbs (popups, tables, code) (agent/t16-polish, tgt-t7)
- Open (owner call): scrollbar thumbs not draggable; remote images load automatically (privacy; no opt-out); unsaved-note drops go to <vault>/assets
- [x] T8 typography (inline code size + tracking impossible in gpui 0.2.2): [x] Inter Medium/SemiBold (+italics, v3.19 to match) bundled; table cells in Inter [owner ok]; weights/sizes/line heights per component; JetBrains Mono size next to Inter; screenshots
- [ ] T9 critical leftovers, final review, docs

## Scout notes (for wave 2/3)
- Settings page: `shell/settings_tab.rs` (card_row, segmented, switch, button); logic `shell/settings.rs` choose_setting; `Setting` enum `views/palette.rs:76`; prefs `inkuno-logic/src/prefs.rs`.
- No line-spacing pref: leading per kind at `views/editor/layout.rs:1936-1941`, `lh` at ~2021; tables `grid.rs:104` line_height.
- Tables: `grid.rs:184 column_widths` (fit, shared level, NARROWEST 36); big tables (>BIG 1500) clip; Wide::Column scrolls sideways via Shift+wheel only (`mouse.rs:891`), no visible scrollbar.
- Fonts `inkuno-ui/src/assets.rs`: Inter static R/I/B/BI (no semibold cut!), JetBrains Mono variable + static bold. Sizes: H1 28 H2 21 H3 18 H4 16 body 15@22, code 13@20, table cells mono 14@20.

## Lessons for next time
- Give each worktree its own CARGO_TARGET_DIR (copy a warm one): a shared dir mixes artifacts across worktrees.
- Run `cargo fmt --all` + clippy before every merge (CI `fast` job).
- A layout-wide change (T1) breaks layout tests: pair it with the cleanup (T6) before merging.

## Follow-ups noticed
- `views/nav.rs` `ROW` is 28 px; tree rows are 30 px in the one layout (listing.rs uses ROW).
- `end_to_end/blocks.rs` picker test still names "Quiet Light" (works via the compat mapping).
- GPUI 0.2.2 keeps stale `debug_bounds` for rows not painted this frame: tests must scroll a row into view before measuring.

## Status 2026-11-23
All tasks merged into PR #19 (labels rust:patch, cargo). Full CI (fast + test matrix ubuntu/macos) green on 962fef4. Waiting for the owner's merge; then the release gate bumps, tags and ships.
