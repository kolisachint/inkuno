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
- macOS title bar and traffic lights (2026-11-14): `Ctrl+Shift+S` and focus mode hide the system bar and its three window buttons (`platform/mac_chrome.rs`, transparent bar at creation); the state is kept in the layout file. Tested by compiling the module for `aarch64-apple-darwin` only.
- Edge to edge in normal mode (2026-11-14, owner decision): panes take the window's edges, a 1 px hairline between neighbours, no gap ground; `app-ui.md` §10 item 4 lifted, §15. Tests: `shell/tests/window.rs`.

- Revisions (2026-11-14, `docs/design/revisions.md`): `Ctrl+Alt+S` (`app::SaveRevision`, palette `Save a version`) saves a numbered version of the note with its author and an optional line; `revision author…` sets the author; the history lives in `.inkuno/revisions/` (scratch: `.revisions/`), follows renames and moves, and goes to the trash with a trashed note. Scratch deletes go to `.trash` in the scratch folder. The command line: `inkuno rev save|list|show|diff` and `inkuno keys list|set|reset`, with `inkuno help`. Tests: `inkuno-logic` `revision.rs`, `platform/revisions.rs`, `tests/cli.rs`, `shell/tests/revisions.rs`.

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

- Revisions: the MCP server's `rename_note` does not move the history (the server is not changed by this work; it is removed in a later step, `docs/design/revisions.md`).
- Revisions: `inkuno rev` on a vault that has no `.inkuno` folder and is not open in the app treats the note's own folder as the vault, so history can be split by folder; the app creates `.inkuno` only at the first version. Create it when a vault opens, or document it.
- Revisions: restoring from the trash brings the history back only when the trash holds the same name; a trash entry removed by hand breaks the pairing (`platform/revisions.rs`, `restore_along`).
- Revisions: two saves at the same moment: the second is refused (`saved meanwhile`), there is no lock.
- Revisions: `Keep in vault` still deletes the scratch copy once the vault note is written (a move, not a loss).
- Revisions: the tree and the Scratch panel are not yet showing history; the Versions side panel and the coloured compare are next (`docs/design/revisions.md`).
- `Ctrl+Alt+S` is not tried on macOS or on a desktop that keeps it; the screen was not run here.
- `docs/design/app-ui.md` still says Discard deletes the file (line ~670); the design notes are listed for the owner.

- Help-panel shortcut changes are saved to the `keys` file (`crates/inkuno/src/shell/help.rs:227`), but once `keybindings.json` exists it wins at launch and nothing reads `keys` again: they are lost at the next start. Also `--help` says the JSON is hot-reloaded; it is read at launch only (`crates/inkuno/src/main.rs`).
- GitHub Pages must be set to deploy from GitHub Actions, else the Docs deploy job fails (`.github/workflows/docs.yml:3`).
- `shortcuts_page_is_current` is compiled out on macOS; the page cannot be regenerated on a Mac (`crates/inkuno/src/shell/tests/shortcuts_page.rs:66`).
- Two macOS-only clippy warnings fail `-D warnings` locally (`crates/inkuno/src/platform/mod.rs:7`, `crates/inkuno/src/shell/tests/shortcuts_page.rs:7`).
- Template machinery is reachable only through MCP; decide app UI or keep MCP-only (`crates/inkuno/src/platform/vault.rs:254`, `crates/inkuno-logic/src/daily.rs:15`).
- `packaging/windows/` remains though Windows is not built (`packaging/windows/`).
- Scratch notes (2026-11-14): `Discard`, the panel's `Delete` and the `Delete` key move the file to `.trash` inside the scratch folder (`platform/scratch.rs`, `trash`), so nothing is destroyed on disk. Only a blank scratch file is still removed.
- Scratch notes live under the config folder: on macOS that is `~/Library/Application Support`, which sits inside a vault that is the home folder, where the scan would see it (it skips only dot-folders). Check the scratch folder is outside any vault before a release.
- `docs/design/*.md` still describes cut actions; listed for the owner, not rewritten.
- `tr()` second (French) args are still at call sites; removal is in progress (`crates/inkuno-logic/src/lang.rs:6`).
- `Pane::viewer()` has no non-test callers; remove or keep (`crates/inkuno/src/shell/pane.rs:179`).
- macOS chrome hiding is unverified on a real Mac until the owner tries it: the window's bar is transparent and its traffic lights are hidden with `setHidden:`; check that the note's top row is clickable, that the lights come back on the next press, and that a full-screen toggle does not show them again (`crates/inkuno/src/platform/mac_chrome.rs`).
