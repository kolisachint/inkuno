# Revisions

Idea refinement: a person keeps evolving a note and saves explicit **revisions** of it. Each one is
a complete copy of the text at that moment, with its author and an optional decision note. The
files are plain Markdown, readable by people and by agents. Pure format and diff logic:
`crates/inkuno-logic/src/revision.rs`. Disk: `crates/inkuno/src/platform/revisions.rs`.

## Format

A revision is one file, `NNNN.md` (`0001.md`, `0002.md`, … and wider past 9999). It opens with a
front matter block that Inkuno writes, then the note text **verbatim** (even when the text starts
with its own front matter):

```
---
revision: 3
parent: 2
author: sachin
date: 2026-11-14T10:32:00Z
kind: checkpoint
note: dropped the plugin idea, keep it local-first
---
<the full note text exactly as it was>
```

| Field | Meaning |
|---|---|
| `revision` | 1-based number. Required. |
| `parent` | The revision saved before it. Left out for the first. |
| `author` | One line. Defaults to `unknown` when missing. |
| `date` | UTC, RFC 3339, to the second. |
| `kind` | `checkpoint` (a person's save) or `auto` (written around each `inkuno write`). Defaults to `checkpoint`. |
| `note` | Optional, one line. Any text: a value with colons, quotes, `\`, tabs or line breaks is written as a quoted string with escapes. Empty means no note. |

Unknown keys are ignored. The header ends at the first line that is exactly `---` after the opening
one; everything after it is the text. A file that does not open with `---`, is never closed, or has
a bad `revision`, `parent` or `kind` is not a revision and is reported, never guessed at.

A save whose text equals the latest revision's is refused (`nothing changed since revision N`): no
file is written.

## Where the files are

| The note | Its revisions folder |
|---|---|
| `<vault>/ideas/plan.md` | `<vault>/.inkuno/revisions/ideas/plan.md/` |
| a scratch note `<scratch>/X.md` | `<scratch>/.revisions/X.md/` |

- `.inkuno` (vault) and `.revisions` (scratch) are dot-folders: the tree, recent notes, search,
  wikilinks and the watcher ignore them (test `the_history_is_invisible_to_the_vault_scan`), and the
  Scratch panel lists only the `.md` files of the scratch folder.
- A rename or a move (tree, context menu) moves the folder with the note, or with the folder that
  holds it. A scratch note kept in the vault moves its folder into the vault.
- A trashed note or folder: its revisions folder goes to the trash of its home, next to the trash
  (`<vault>/.inkuno/trash/`, or `<scratch>/.revisions/.trash/`), under the name the trashed item got
  there. Nothing is deleted. An undo of the trash puts it back with the note.
- The vault backup archive holds the whole vault, so `.inkuno/` is in it, as `.trash` is.

## Diffs

Diffs are derived when asked, never stored. Line-level, from the same diff the merge uses
(`merge::hunks`).

- `unified`: a standard unified diff, `--- a/<name>@<n>` and `+++ b/<name>@<m|current>`, three lines
  of context. It applies with `patch` (tested when `patch` is installed).
- `critic`: CriticMarkup over the whole text: removed lines in `{--…--}`, added lines in `{++…++}`.

## Command line

The versions are driven by the command line, which works whether or not the app runs (the app's file
watcher sees the files). The design of the command line, its exit codes and the other commands
(`list`, `read`, `new`, `keys`…) are in [cli.md](cli.md). Code: `crates/inkuno/src/cli/rev.rs`;
end-to-end tests: `crates/inkuno/tests/cli.rs`.

```
inkuno rev save <note> [--author NAME] [-m TEXT]     prints the new revision number
inkuno rev list <note> [--json]                      newest first
inkuno rev show <note> <n> [--json]                  the text of revision n
inkuno rev diff <note> [<a>] [<b>] [--format unified|critic]
```

- The vault of a note: the nearest folder above it with an `.inkuno` folder; else the vault the app
  has open, when the note is in it; else the note's own folder. A scratch note (in the scratch
  folder) uses the scratch folder.
- `--author` defaults to the app's author setting, else `INKUNO_AUTHOR`, else the system user name,
  else `unknown`.
- Errors and exit codes: as in [cli.md](cli.md).

## In the app

- `Save a version` (`Ctrl+Alt+S`) saves a version of the note in the tab, with its author and the optional line.
- **Versions of the note** (palette; `history` finds it too) opens the Versions panel: the versions of the active note, newest first,
  each as `#12 · author`, with its line and its date. Enter or a click compares a version with the note as it is now, in a read-only
  tab; the difference is drawn as pending changes (green added, red removed). The row menu copies a version's text. Design and
  limits: `app-ui.md` §19. **Restore this version** (the row menu's last entry) proposes the version's text as the note's new
  text, as pending changes, and saves two `kind: auto` revisions around it (see Decisions, 2026-11-16).

## Decisions

| Date | Decision (owner-approved, 2026-11-14 unless noted) |
|---|---|
| 2026-11-14 | Full snapshots with a front matter block; diffs are derived, never stored. |
| 2026-11-14 | Revisions live hidden in `.inkuno/revisions/` inside the vault (scratch: `.revisions/`). |
| 2026-11-14 | Explicit checkpoints only, by the chord or the command line. Files are live on disk, so saving a file never makes a revision. |
| 2026-11-14 | One author per snapshot. Multi-author views are derived from automatic single-author snapshots taken around each agent write (`kind: auto`). |
| 2026-11-14 | Agent changes stay tinted and pending until the human accepts, rejects or comments; the human edits them only after accepting; an agent write that touches text the human changed since the agent read it is refused. The write and the refusals are built (2026-11-15, `inkuno write`); the tint, accept, reject and comment are built in the app (2026-11-15, `app-ui.md` §17). |
| 2026-11-15 | An agent write is bracketed by two `kind: auto` revisions: the text before it (the person's default author), and the proposed text (the agent's `--author`, note `proposed, pending`). A write that changes nothing in a snapshot's text saves no revision. |
| 2026-11-15 | Pending changes are CriticMarkup in block form, one area per removed and added block pair, in the note file itself, so that the note stays plain text for every other reader. (Superseded 2026-11-21: they are beside the note, see below.) A note with pending changes takes no new agent write. |
| 2026-11-15 | The app's side of pending changes is built: a green or red wash on the added and removed lines, the marks dimmed; typing in a change is refused with a message, while the caret, the selection, copy, undo and outside changes are not; accept, reject (one change, or all) and comment on a change, each one undo step, from the palette and the right-click menu, with a hint at the note's foot. No chords yet. `app-ui.md` §17. |
| 2026-11-15 | The first snapshot of a write (the text before) is skipped when the text is the latest saved version already: the write's JSON has `"before": null`. The second (the proposed text) is written whenever it differs from the latest. |
| 2026-11-15 | The Assistant's proposal (Improve, `app-ui.md` §18) is bracketed the same way: two `kind: auto` revisions, the text before by the person's author setting, then the proposed text by `assistant` (note `proposed, pending`). The shell writes them itself (`shell/assistant.rs`), as the command line does. |
| 2026-11-15 | The proposed version is the note as it would be once every change is accepted (`pending::accept_all` of the marked text). When the agent's base was older than the note, it includes the person's other edits, not only the agent's literal text. |
| 2026-11-15 | `write` never renames the note file, even when the proposed text changes its `#` title line: the title change is pending like any other. |
| 2026-11-15 | A failed save of the proposed version does not undo the write: the changes stay pending, `proposed` is null, and one warning goes to standard error (`cli.md`, Decisions). The first save, of the text before, still stops the write. |
| 2026-11-14 | The command line replaces the MCP server, and the server is removed: done, see [cli.md](cli.md). |
| 2026-11-14 | An in-app agent talks over a local socket with the same command line as version 2 (planned). |
| 2026-11-15 | The Versions panel lists the active note's versions, newest first, and a compare shows one against the note now, in a read-only tab with no file: its text is the version's with the difference as pending changes (`pending::propose`), a snapshot, never saved, not restored at the next start. No restore yet; a restore would be a proposal through the pending flow (`app-ui.md` §19). |
| 2026-11-16 | A restore of a version is a proposal through the pending flow, as an Assistant proposal lands: `pending::propose(current, current, version)`, one undo step, and two `kind: auto` revisions around it, by the person's author: the text before (memo `before restoring version N`), then the restored text with its changes accepted (memo `version N restored, pending`). It needs the note in its tab (not a comparison) and no pending changes in it. |
| 2026-11-16 | The Versions panel reads each version's header only (the front matter, line by line up to its closing `---`); a version's text is read when a compare, a copy or a restore needs it. A header that cannot be read is its own row, as before. |
| 2026-11-16 | The panel shows each version's date in local time, to the minute (the platform's zone rules). The stored time stays UTC: the file and `rev list` do not change. |
| 2026-11-21 | Pending changes are held beside the note, in `.inkuno/pending/<note>.json` (a scratch note: `<scratch>/.inkuno/pending/<name>.json`): a list of `{id, author, at, old, new}`, where `old` is the text the note holds at `at` and `new` the text it becomes once accepted. The note's file never holds a mark, so it is clean Markdown for every reader; the editor and `read` show the marks, composed from the file and the list (`platform/sidecar.rs`, `logic::pending::split` and `compose`). Supersedes the 2026-11-15 row on marks in the file. A note file written with marks before this is shown as it is, and its marks move to the list on the next save. Owner's answer Q2 in `prototypes/README.md`. |
| 2026-11-21 | Snapshots (`rev save`, the window's save of a version, the automatic ones) hold the note's own text: no pending change is in them. A copy of marked text holds no mark: it is the text the note will have once accepted. |
| 2026-11-22 | A pending change keeps the text around its anchor: the line before and the line after, in `before` and `after` of each list entry (`logic/pending.rs`, `split`). `at` is only a hint: when the note changed outside Inkuno, an entry is placed by its removed text (when it is in the note once) or by its context, at a line start outside code fences, and a change that cannot be placed cleanly is kept in the list and shown nowhere. A list whose changes are not shown is kept on save, and a change that was shown and is gone is dropped (`platform/sidecar.rs`, `split`). A list in the way of a move is moved aside, never an error. A note's history moves first; its pending list follows when it can. The command line checks a move and rewrites the links through the text as the editor shows it (`read_shown`, `write_shown`), so a pending change is never written into the file. |

## Not built yet

- Restoring a trashed note's history when the bin holds it under another name (the undo of a trash
  and the Trash panel's restore bring it back by name).
