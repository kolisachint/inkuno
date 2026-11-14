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
| `kind` | `checkpoint` (written now) or `auto` (reserved: read, never written yet). Defaults to `checkpoint`. |
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

The same binary, with no window; it works whether or not the app runs (the app's file watcher
sees the files). Code: `crates/inkuno/src/cli/` (`rev.rs`, `keys.rs`); end-to-end tests:
`crates/inkuno/tests/cli.rs`.

```
inkuno rev save <note> [--author NAME] [-m TEXT]     prints the new revision number
inkuno rev list <note> [--json]                      newest first
inkuno rev show <note> <n> [--json]                  the text of revision n
inkuno rev diff <note> [<a>] [<b>] [--format unified|critic]
inkuno keys list [--json]
inkuno keys set <action> <chord>                     "" removes the shortcut
inkuno keys reset <action>...
inkuno help [rev|keys]                               and --help on each command
```

- The vault of a note: the nearest folder above it with an `.inkuno` folder; else the vault the app
  has open, when the note is in it; else the note's own folder. A scratch note (in the scratch
  folder) uses the scratch folder.
- `--author` defaults to the app's author setting, else `INKUNO_AUTHOR`, else the system user name,
  else `unknown`.
- Errors: stderr, exit 1. A wrong command line: stderr, exit 2.
- `keys set` reuses the app's rules (`ui/keys.rs`, `shell/keymap.rs`): the actions that can change,
  refused chords, clashes; it writes `keybindings.json` only when the app's own reader takes it back
  without a fault.
- One chord per action, as the file holds one. `keys set` takes a single chord (or `""`).

## Decisions

| Date | Decision (owner-approved, 2026-11-14 unless noted) |
|---|---|
| 2026-11-14 | Full snapshots with a front matter block; diffs are derived, never stored. |
| 2026-11-14 | Revisions live hidden in `.inkuno/revisions/` inside the vault (scratch: `.revisions/`). |
| 2026-11-14 | Explicit checkpoints only, by the chord or the command line. Files are live on disk, so saving a file never makes a revision. |
| 2026-11-14 | One author per snapshot. Multi-author views are derived from automatic single-author snapshots taken around each agent write (`kind: auto`, planned, not built). |
| 2026-11-14 | Agent changes stay tinted and pending until the human accepts, rejects or comments; the human edits them only after accepting; an agent write that touches text the human changed since the agent read it is refused (planned, not built). |
| 2026-11-14 | The command line replaces the MCP server; the MCP server is removed in a later step (planned). The MCP server is not changed by this work. |
| 2026-11-14 | An in-app agent talks over a local socket with the same command line as version 2 (planned). |

## Not built yet

- The Versions side panel and the coloured inline compare.
- Automatic (`auto`) snapshots and agent live edits.
- Restoring a trashed note's history from the Trash panel (the undo of a trash brings it back; the
  Trash panel's own restore does too, when the bin has it).
- The MCP server's rename does not move the history (the server is not changed by this work).
