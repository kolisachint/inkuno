# Backlog

Lean follow-ups only. Done work is listed once below, then dropped.

## Done in the November cleanup

- Backlog round for 0.0.9 (2026-11-16). Pending changes: a mark inside a code fence is plain text, and a proposal that changes a fenced block marks the whole block; a proposal keeps the note's own final line break, so reject is exact; an edit that would hide or create a change is refused; marker lines are drawn at 0.8 size and a table in a change carries the wash; a renumbering or realign never crosses a change (`app-ui.md` §17). Command line: `inkuno write --replace` lets an agent replace its own last proposal; a move, rename or title change that would rewrite a `[[link]]` inside a pending change is refused, in the app too (`cli.md`). Versions: **Restore this version** in a row's menu proposes the version as pending changes; the list reads headers only; dates are local time (`app-ui.md` §19, `revisions.md`). Keys: a rebinding is checked against the accept and reject chords (`keybindings.md`). Watcher: own and hidden folders are known in both spellings of the vault path (`architecture.md`). One helper saves an automatic version (`revisions::save_auto`). An `end_to_end` chapter walks the Assistant, pending changes and the Versions panel (`shell/tests/end_to_end/assistant_versions.rs`). The owner tried 0.0.8 on macOS with a real hoocode and on a real screen on 2026-11-16: the Assistant (Improve included), the washes, the chords, the Versions rows and the compare work as designed.

- Versions panel and compare (2026-11-15, `app-ui.md` §19): **Versions of the note** in the palette (`history` finds it too) lists the active note's versions, newest first; Enter or a click compares one with the note now, in a read-only tab, the difference drawn as pending changes; the row menu copies a version's text. Tests: `shell/tests/versions.rs`, `inkuno-logic` `revision.rs`, `commands.rs`, `views/palette.rs`.

- Assistant v2 (2026-11-15, `app-ui.md` §18): **Improve selection** / **Improve this note** (the panel's button, the palette, the right-click menu) sends the passage, or the note, with an instruction; the answer comes back as pending changes, one undo step, with two `kind: auto` revisions as `inkuno write` makes them. Chords: `Ctrl+Enter` accepts and `Ctrl+Backspace` rejects a pending change (`keybindings.md`, `app-ui.md` §17.5). Tests: `shell/tests/assistant.rs`, `shell/tests/pending.rs`, `inkuno-logic` `assistant.rs`, `commands.rs`.

- Pending changes in the app (2026-11-15, `app-ui.md` §17): an added line lies on a green wash, a removed one on a red wash, struck through; typing in a change is refused with a message; accept, reject (one change or all) and comment on a change are one undo step each, from the palette and the right-click menu; a hint at the note's foot counts them. `inkuno write` writes the marks (2026-11-15, `cli.md`). Tests: `shell/tests/pending.rs`, `inkuno-logic` `pending.rs`, `ui/theme.rs`.

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
- Assistant v1 (2026-11-14, `app-ui.md` §16): the bottom-right word counter is removed and a round Assistant button takes its corner; the panel is a right-dock tool, opened by the button or `assistant` in the palette; it starts `hoocode --mode rpc --no-session --disallowed-tools Shell,ShellKill,Write,Edit,WebFetch,WebSearch,Agent` as a child process (`platform/assistant.rs`), so the agent has no shell and no write tool; "Review this note" gets its findings as text, and Inkuno writes them as comments in one undo step (`shell/assistant.rs`, `inkuno-logic` `assistant.rs`); `inkuno comment --file PATH` comments on a Markdown file with no vault. Decisions: `cli.md`, `app-ui.md` §16, `architecture.md`.
- macOS case-only rename (`Zz` → `zz`) confirmed working by the owner on 2026-11-14, in the app and with `inkuno move`.

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

- Rendering R1 (setext headings, found 2026-11-21): a line underlined with `===` or `---` is not drawn as a heading; the `---` reads as a divider. Expected: the underline makes the line above a heading, as CommonMark says. Where: TBD (`crates/inkuno-logic/src/markdown/mod.rs`, `classify`).
- Rendering R2 (nested quotes, found 2026-11-21): a quote inside a quote shows its raw `> ` markers. Expected: the markers hide off the line being edited and the depth is drawn as nested bars (`markdown_renderer.md`). Where: TBD (`crates/inkuno/src/views/editor/layout.rs`, quote rows).
- Rendering R3 (lists in quotes, found 2026-11-21): a list inside a blockquote shows its raw `- ` and `1. ` markers. Expected: drawn as the lists outside a quote are (`markdown_renderer.md` §10.3). Where: TBD (`views/editor/layout.rs`, list kinds).
- Rendering R4 (long code lines, found 2026-11-21): a long line in a fenced code block wraps instead of scrolling sideways. Expected: the block scrolls horizontally, as code does on GitHub. Where: TBD (`views/editor/layout.rs`, code rows and their wrap width).
- Rendering R5 (table caret jump, found 2026-11-21): table columns move as the caret goes from row to row. `grid.rs` measures with the markup hidden only when a row is not live (`hide = !live`, about line 773), so the widths follow the caret. Expected: the widths are measured the same way whatever row holds the caret.
- Rendering R6 (large tables, found 2026-11-21): tables over `grid::BIG` never hide their cell markup, whatever the caret. Expected: the same hide rule at any size, or a stated limit (`views/editor/grid.rs`, `BIG`).
- Rendering R7 (indented code and the outline, found 2026-11-21): the outline, tags and footnotes logic (`prose_lines`, headings, index) does not know indented code, so a `#` or `#tag` inside an indented block is read as prose. Expected: indented code is skipped there as fenced code is (`crates/inkuno-logic/src/markdown/mod.rs`, `prose_lines`).
- Window chrome W1 (found 2026-11-21, owner decision needed): with the sidebar hidden and the title bar hidden, no on-screen toggle remains: the 12 px band has none and the foot is gone, so only `Ctrl+B` and `Ctrl+Shift+S` bring things back. Options: a toggle in the 12 px pad, or accept the keys (`app-ui.md` §20.4).
- Window chrome W2 (found 2026-11-21): the Linux client-decoration pill is not seen on screen, because the CI sandbox has no compositor and gpui draws server decorations. Expected: a run with a compositor (xcompmgr or picom) checks it (`app-ui.md` §20.4).
- Pending changes: the 0.8 marker lines and the wash on a table are not yet seen on a real screen (`views/editor/layout.rs`, `MARK_SCALE`; `views/editor/paint.rs`).
- Pending changes: a code fence cannot be opened above a pending change by typing its first line alone (the edit would hide the change, so it is refused): expected, the message says why, or the fence is allowed once it is closed (`crates/inkuno-logic/src/pending.rs`, `refuses`, `same_areas`).
- Pending changes: a change typed by hand whose added lines open a fence and do not close it hides the changes after it once accepted; a proposal never writes one (`pending.rs`, `areas`).
- Pending changes: a fence inside a quote or a list item (`> ```) is not seen as a fence, so a proposal may write marks inside it; accept and reject stay exact (`pending.rs`, `fence_opens`).
- Pending changes: a title change refused because of a link in another note's pending change leaves the note unsaved, and the notice may come back at each save: expected, the text is saved and only the file's rename waits (`crates/inkuno/src/shell/notes.rs`, `flush_one`, `links_pending`).
- `inkuno write --replace`: a proposal cannot be withdrawn (the person's own text is "nothing changed"), and when the first write's proposed version could not be saved the agent cannot replace it (`crates/inkuno/src/cli/ops.rs`, `own_last_proposal`).
- Keys: a `keybindings.json` line that takes the keys of the accept or reject chord leaves the chord without keys and says nothing at launch; the help panel asks first (`crates/inkuno/src/ui/keys.rs`, `read`).
- Versions: dates are UTC on Windows, which has no local-time call yet (`crates/inkuno/src/platform/clock.rs`, `local`).
- Versions: a restore saves its "before" version from the editor's text; an outside write not yet merged is in no version (`crates/inkuno/src/shell/versions.rs`, `restore_version`).
- Pending changes: copy of the whole note (`app::CopyAll`, `shell/notes.rs`) copies the marker lines as they are: the app has no reading view or export that resolves them.
- Pending changes: `append --under` into a section that holds a pending area is covered only by the general rule (`crates/inkuno/src/cli/ops.rs`, `append_under`): expected, a section with a change is refused by name.

- Revisions: `inkuno rev` on a vault that has no `.inkuno` folder and is not open in the app treats the note's own folder as the vault, so history can be split by folder; the app creates `.inkuno` only at the first version. Create it when a vault opens, or document it.
- Revisions: restoring from the trash brings the history back only when the trash holds the same name; a trash entry removed by hand breaks the pairing (`platform/revisions.rs`, `restore_along`).
- Revisions: `Keep in vault` still deletes the scratch copy once the vault note is written (a move, not a loss).
- Versions: a word-level compare; today a changed line is one removed and one added line (`crates/inkuno-logic/src/pending.rs`, `propose`).
- `Ctrl+Alt+S` is not tried on macOS or on a desktop that keeps it; the screen was not run here.

- Help-panel shortcut changes are saved to the `keys` file (`crates/inkuno/src/shell/help.rs:227`), but once `keybindings.json` exists it wins at launch and nothing reads `keys` again: they are lost at the next start.
- GitHub Pages must be set to deploy from GitHub Actions, else the Docs deploy job fails (`.github/workflows/docs.yml:3`).
- `shortcuts_page_is_current` is compiled out on macOS; the page cannot be regenerated on a Mac (`crates/inkuno/src/shell/tests/shortcuts_page.rs:66`).
- Two macOS-only clippy warnings fail `-D warnings` locally (`crates/inkuno/src/platform/mod.rs:7`, `crates/inkuno/src/shell/tests/shortcuts_page.rs:7`).
- Template machinery is reachable only through `inkuno new --template`; decide app UI or keep the command line only (`crates/inkuno/src/platform/vault.rs:254`, `crates/inkuno-logic/src/daily.rs:15`).
- CLI: link rewriting after a move swallows a write error on another note (`crates/inkuno/src/cli/store.rs`, `relink`), and the H1 rewrite error of a renamed note is ignored (`crates/inkuno/src/cli/ops.rs`, `rename_note`): expected, a note that cannot be written is reported, and the move says which links were left.
- CLI: `trash` returns an error after the file has moved when the history step fails (`crates/inkuno/src/cli/ops.rs`, `trash_note`): expected, the reason says where the note is now, or the move is undone.
- CLI: `inkuno links` lists a note embed `![[B]]` under pictures (`crates/inkuno/src/cli/ops.rs`, `note_links`): expected, an embedded note is an embed only.
- CLI: tag counting is written twice, in `vault_info` and `list_tags`, and the nested `pub mod wiki` in `ops.rs` shares its name with `crate::logic::wiki`: expected, one counting function, and a module name that does not clash.
- CLI: `inkuno rev` needs the `.md` in a path and takes no `--vault`, unlike the other commands, and `rev list` on a trashed path prints nothing with exit 0 (`crates/inkuno/src/cli/rev.rs`): expected, `rev` accepts a note name as the other commands do, and `rev list` on a path that has no versions says so.
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
- Vault scan: a note in a symlinked hidden folder is still listed (only typed paths are guarded): expected, the scan follows the same rule as typed paths.
- CLI: `inkuno help --json` says "unknown command": expected, the help as JSON or a clear usage error.
- Assistant: the prompt box is one field without newlines: expected, a multi-line box is a later idea.
- The Assistant cannot ask before a tool call: hoocode's RPC mode has no approval messages (its `app-server` protocol has them).
- CLI: `inkuno comment` still matches a passage inside a code fence or inside existing comment markup: expected, those are skipped or refused.
- macOS chrome hiding is unverified on a real Mac until the owner tries it: the window's bar is transparent and its traffic lights are hidden with `setHidden:`; check that the note's top row is clickable, that the lights come back on the next press, and that a full-screen toggle does not show them again (`crates/inkuno/src/platform/mac_chrome.rs`).
- Assistant: the `assistant` setting splits on spaces, so a program path with spaces cannot be set (`crates/inkuno/src/platform/assistant.rs`, `program`): expected, a path with spaces works, or the limit is stated in the setting's help.
- Assistant on Windows: only `name` and `name.exe` are looked up on `PATH`; `.cmd` and `.bat` are not (`crates/inkuno/src/platform/assistant.rs`, `find_in`): expected, the shims are found too, or the limit is written down.
- Assistant Improve: a whole-note improvement sends the whole note and gets all of it back: a very long note is slow (`crates/inkuno/src/shell/assistant.rs`, `assistant_improve`; `crates/inkuno-logic/src/assistant.rs`, `rewritten`): expected, a selection of the note, or a limit with a message.
- Assistant Improve: the proposal block ends at the last `END TEXT` line, so commentary after the block that holds that line is taken in (`crates/inkuno-logic/src/assistant.rs`, `proposal`): expected, the block is checked against the passage's own lines, or the answer is refused.
- Assistant Improve: the right-click entry **Improve selection** has no test; the button and the palette do (`crates/inkuno/src/shell/menu.rs`).
- Assistant: prompts are written to the agent on the window's thread (`crates/inkuno/src/shell/assistant.rs`, `assistant_run`): expected, a writer thread takes them if prompts grow.
- Assistant: the agent may still read any file it is pointed at, since `Read` is not limited to the vault: expected, an allow-list of tools and a folder limit, to be looked at with hoocode.
- Assistant: the shell's tests start no process, so the non-test path of `assistant_run` that starts the agent (`start_agent`) has no test there: expected, the process is tested in `platform/assistant.rs`, and a shell test with a fake agent program.
