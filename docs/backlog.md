# Backlog

Lean follow-ups only. Done work is listed once below, then dropped.

## Done in the November cleanup

- The November trim slices (canvas, tables, obsidian, chrome, sync, deps) are all merged into main.
- Root `inkuno.ico`, the git-sync illustration and the README save-as-PDF anchor are gone.
- `ReplaceInVault`, `--capture`/`vault::capture` and the `pane.tool` machinery are cut.
- The daily note opens from the blank-page button only (`crates/inkuno/src/shell/render.rs:709`); no chord, no palette entry.
- `logic/history.rs`, prefs `updates` and `Editor::lookups` are gone.
- `shortcuts.md` is generated and checked on Linux CI only.
- Docs and packaging folders restructured: `book/` -> `docs/user/`, `docs/screenshots` and `docs/site` -> `docs/assets/`, Linux packaging -> `packaging/linux/`.
- The app moved to `crates/inkuno`, logic to `crates/inkuno-logic`; the root manifest is workspace-only.

## Kept ideas (not built)

- Code folding in the editor.
- Wikilink and URL bracket hiding.
- Footnote backlink glyph.
- Mermaid converter coverage (flowcharts next).
- Tree header restyle.
- Brand icons and theme.
- Platform crate extraction.
- Screenshots regeneration.

## Follow-ups

- GitHub Pages must be set to deploy from GitHub Actions, else the Docs deploy job fails (`.github/workflows/docs.yml:3`).
- `shortcuts_page_is_current` is compiled out on macOS; the page cannot be regenerated on a Mac (`crates/inkuno/src/shell/tests/shortcuts_page.rs:66`).
- Two macOS-only clippy warnings fail `-D warnings` locally (`crates/inkuno/src/platform/mod.rs:7`, `crates/inkuno/src/shell/tests/shortcuts_page.rs:7`).
- Template machinery is reachable only through MCP; decide app UI or keep MCP-only (`crates/inkuno/src/platform/vault.rs:254`, `crates/inkuno-logic/src/daily.rs:15`).
- `packaging/windows/` remains though Windows is not built (`packaging/windows/`).
- `docs/design/*.md` still describes cut actions; listed for the owner, not rewritten.
- `tr()` second (French) args are still at call sites; removal is in progress (`crates/inkuno-logic/src/lang.rs:6`).
- `Pane::viewer()` has no non-test callers; remove or keep (`crates/inkuno/src/shell/pane.rs:179`).
