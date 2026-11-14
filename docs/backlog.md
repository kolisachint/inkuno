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
- Clean focus chrome (2026-11-14): `Ctrl+B` sidebar (Bold on `Ctrl+Alt+B`), `Ctrl+Shift+S` title bar, `F9` focus mode (edge to edge), shortcuts from `keybindings.json` documented in the settings page.
- Scratch notes (2026-11-14): `Ctrl+N` makes one outside the vault (no vault needed; `inkuno/scratch` in the config folder, written as typed); closing one with text asks Keep as scratch / Keep in vault… / Discard; the Scratch panel (`scratch notes` in the palette) lists them; `Ctrl+Alt+N` is the vault note as before. Decisions and tests: `keybindings.md` §8, `app-ui.md` (Scratch notes), `shell/tests/scratch.rs`.

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

- Help-panel shortcut changes are saved to the `keys` file (`crates/inkuno/src/shell/help.rs:227`), but once `keybindings.json` exists it wins at launch and nothing reads `keys` again: they are lost at the next start. Also `--help` says the JSON is hot-reloaded; it is read at launch only (`crates/inkuno/src/main.rs`).
- GitHub Pages must be set to deploy from GitHub Actions, else the Docs deploy job fails (`.github/workflows/docs.yml:3`).
- `shortcuts_page_is_current` is compiled out on macOS; the page cannot be regenerated on a Mac (`crates/inkuno/src/shell/tests/shortcuts_page.rs:66`).
- Two macOS-only clippy warnings fail `-D warnings` locally (`crates/inkuno/src/platform/mod.rs:7`, `crates/inkuno/src/shell/tests/shortcuts_page.rs:7`).
- Template machinery is reachable only through MCP; decide app UI or keep MCP-only (`crates/inkuno/src/platform/vault.rs:254`, `crates/inkuno-logic/src/daily.rs:15`).
- `packaging/windows/` remains though Windows is not built (`packaging/windows/`).
- Scratch notes: `Discard`, the panel's `Delete` and the `Delete` key delete the file for good, with no `.trash` (the owner asked for deletion; CONTRIBUTING says nothing is destroyed on disk). Owner to confirm, or move them to a `.trash` in the scratch folder (`crates/inkuno/src/shell/scratch.rs`, `delete_scratch`).
- Scratch notes live under the config folder: on macOS that is `~/Library/Application Support`, which sits inside a vault that is the home folder, where the scan would see it (it skips only dot-folders). Check the scratch folder is outside any vault before a release.
- `docs/design/*.md` still describes cut actions; listed for the owner, not rewritten.
- `tr()` second (French) args are still at call sites; removal is in progress (`crates/inkuno-logic/src/lang.rs:6`).
- `Pane::viewer()` has no non-test callers; remove or keep (`crates/inkuno/src/shell/pane.rs:179`).
