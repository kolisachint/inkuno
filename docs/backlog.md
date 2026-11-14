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

- Revisions: `inkuno rev` on a vault that has no `.inkuno` folder and is not open in the app treats the note's own folder as the vault, so history can be split by folder; the app creates `.inkuno` only at the first version. Create it when a vault opens, or document it.
- Revisions: restoring from the trash brings the history back only when the trash holds the same name; a trash entry removed by hand breaks the pairing (`platform/revisions.rs`, `restore_along`).
- Revisions: `Keep in vault` still deletes the scratch copy once the vault note is written (a move, not a loss).
- Revisions: the tree and the Scratch panel are not yet showing history; the Versions side panel and the coloured compare are next (`docs/design/revisions.md`).
- `Ctrl+Alt+S` is not tried on macOS or on a desktop that keeps it; the screen was not run here.

- Help-panel shortcut changes are saved to the `keys` file (`crates/inkuno/src/shell/help.rs:227`), but once `keybindings.json` exists it wins at launch and nothing reads `keys` again: they are lost at the next start.
- GitHub Pages must be set to deploy from GitHub Actions, else the Docs deploy job fails (`.github/workflows/docs.yml:3`).
- `shortcuts_page_is_current` is compiled out on macOS; the page cannot be regenerated on a Mac (`crates/inkuno/src/shell/tests/shortcuts_page.rs:66`).
- Two macOS-only clippy warnings fail `-D warnings` locally (`crates/inkuno/src/platform/mod.rs:7`, `crates/inkuno/src/shell/tests/shortcuts_page.rs:7`).
- Template machinery is reachable only through `inkuno new --template`; decide app UI or keep the command line only (`crates/inkuno/src/platform/vault.rs:254`, `crates/inkuno-logic/src/daily.rs:15`).
- `base64` in `crates/inkuno/Cargo.toml` is no longer used by the code since the server was removed: removing it changes the fragile `Cargo.lock`, so it needs the owner's go-ahead.
- A mistyped command suggests up to three names, nearest first: `inkuno lsit` says ``did you mean `edit` or `list`?``. Expected: the suggestions come from the spelling of the command names only (`crates/inkuno/src/cli/mod.rs`, `suggest`).
- `packaging/windows/` remains though Windows is not built (`packaging/windows/`).
- Scratch notes (2026-11-14): `Discard`, the panel's `Delete` and the `Delete` key move the file to `.trash` inside the scratch folder (`platform/scratch.rs`, `trash`), so nothing is destroyed on disk. Only a blank scratch file is still removed.
- `docs/design/*.md` still describes cut actions; listed for the owner, not rewritten.
- Merge: `identical_change_inside_a_larger_human_block_merges` (`crates/inkuno-logic/src/merge.rs:1015`, ignored): expected, an identical change inside a larger block the person edited merges, no text lost and no conflict copy; today it clashes.
- Merge: `fifty_hunks_each_side_spread_over_the_note_merge` (`merge.rs:1118`, ignored): expected, 50 hunks on each side of a 5,000-line note merge, no text lost and no spurious conflict copy; today the LCS cap folds the middle into one clash.
- AltGr layouts on Windows may fire Ctrl+Alt+N, S or B: expected, a character typed with AltGr is text and never a shortcut.
- Ctrl+B also toggles the sidebar inside the palette and find inputs: expected, in those inputs Ctrl+B leaves the sidebar alone (it toggles it from the note and the tree).
- The pane divider's drag handle covers the last 4 px of the left pane, its scrollbar included: expected, the left pane's scrollbar takes the pointer, and a divider drag starts only past it.
- `crates/inkuno/src/dbg_test.rs` is a leftover debug test that prints and asserts nothing: expected, removed, or turned into an assertion on `ui::keys::canon`.
- A corrupt latest revision file blocks later saves with no repair hint: expected, the refused save names the file and says how to repair it.
- Moving a note onto a path whose history folder already exists leaves the history behind: expected, the move is refused before the note moves, or the history moves with it (today the note moves, its history stays, and a "Revisions not moved" message says so).
- Blank scratch files are removed rather than trashed: intended, they hold nothing; listed so it is not taken for a loss.
- `tr()` second (French) args are still at call sites; removal is in progress (`crates/inkuno-logic/src/lang.rs:6`).
- `Pane::viewer()` has no non-test callers; remove or keep (`crates/inkuno/src/shell/pane.rs:179`).
- macOS chrome hiding is unverified on a real Mac until the owner tries it: the window's bar is transparent and its traffic lights are hidden with `setHidden:`; check that the note's top row is clickable, that the lights come back on the next press, and that a full-screen toggle does not show them again (`crates/inkuno/src/platform/mac_chrome.rs`).
