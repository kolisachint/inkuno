# Command line

The `inkuno` binary has two faces: the window, and the command line. The command line is how a
person or an agent with a shell works on a vault: it reads and writes the note files, draws a
diagram, and prints the result. Agents use it, and the server they used before is removed (see
Decisions). The user's view is in `docs/user/src/agents.md`; this page is the design.

Code: `crates/inkuno/src/cli/`. `main.rs` routes every argument list but the app's own to `cli::run()`.
The app's own are none (the window opens), `--help`, `-h`, `--check-config` (both stay with
`main.rs`) and the `-psn_…` macOS passes at launch. `--vault DIR` and `--json` may come before the
command word; `cli::run` reads them and counts them as if they came after it.

## Principles

- **One binary, no window.** Every command runs, prints and exits. It works whether or not the app
  is open. An open window sees what a command wrote through its file watcher, so no second channel
  is needed for writing.
- **Files only.** A command opens the vault from disk once (`store::Vault::open`), does its work,
  and exits. Nothing is kept between commands: no index on disk, no daemon, no socket. The one
  exception is `show`, which asks the open window to show a file (see Decisions).
- **The app's rules, not new ones.** A taken name is refused. A write is atomic. A rename rewrites
  the `[[links]]` to the note, unless another note still has the old name. Nothing is deleted:
  `trash` moves a file to `.trash`. Nothing outside the vault is reached.
- **Standard input only on request.** `--text -` (`new`, `append`, `comment`) and `--mermaid -`
  (`render`) read it; no other option does. Without `--text`, `new` makes a note with no body and
  `append` is a usage error. Nothing waits on a pipe that was not asked for.
- **No new dependency.** The command line reads its arguments by hand: `options()` in `cli/mod.rs`.
  `--help` is a switch of every command, `--json` of those that print a result; an unknown option
  is a usage error, and a bare `--` ends the options.
- **Wrong values are refused before the work.** A value outside an option's choices or range (a
  `--kind`, a `--limit`, a `--date`, a `--width`) is a usage error, exit 2, before the vault is read.
- **Text by default, `--json` for the structure.** The result goes to standard output. The reasons
  for a failure go to standard error.

## Output and exit codes

| Exit | Meaning |
|---|---|
| 0 | The work is done. The result is on standard output. |
| 1 | The work failed. `inkuno: <reason>` on standard error. A refused command changes nothing: a bad argument is exit 2, a taken name, a stale token or a path outside the vault is exit 1. `move` also rewrites the links to the note and reports how many. |
| 2 | A wrong command line: an unknown command or option, a missing argument. The reason, and `try inkuno help`, on standard error. |

`inkuno render --check` exits 1 when the Mermaid source does not draw; `--json` says why.

A mistyped command name is suggested: `inkuno lsit` names `edit` or `list`. The suggestions are the
names within a small edit distance, nearest first, at most three (`suggest` in `cli/mod.rs`).

## The vault

Every command but `rev` and `keys` works on one vault, found in this order (`cli/commands.rs`,
`vault_of`):

1. `--vault DIR`.
2. The nearest folder at or above the current one that holds an `.inkuno` folder.
3. The vault the app has open (`platform::vault::load_config`).

A note is named by its title, an alias, or its path inside the vault, with or without `.md`
(`Vault::find_note`). A path that climbs out of the vault, or a link that leads outside, is refused
(`Vault::inside`). `.trash` is reachable only by `restore`. A link into a hidden folder is refused as well: the rule holds
for the place the path really reaches. `--folder ""` and `--folder .` are the vault root, and a
listing's `--folder` is read by the same rule as `new` and `move` (`Vault::inside`). `rev` takes no `--vault`: it finds the
vault from the path of the note (`rev.rs`, `revisions.md`). `keys` works on the shortcut file of the
config folder, not on a vault.

## The token

`read --json` returns a `token`: the SHA-256 of the note's bytes as they were read
(`store::token`). `edit` and `append` take it with `--token`. When the file on disk no longer has
that hash, the write is refused: `the note changed since it was read; read it again and retry`.
Without `--token`, the write applies to the file as it is.

The token is not a saved version. The field was called `revision` in the agent's JSON; it is
`token` now, so that "revision" and "version" mean a saved version only (`inkuno rev`).

## The engine

| File | What it holds |
|---|---|
| `cli/mod.rs` | the top help (`TOP`), the dispatch, the option reader, the suggestions for a mistyped command |
| `cli/commands.rs` | every vault command: its options, its `--help` text with an example, and how its options become the arguments of the operation |
| `cli/ops.rs` | what each command does on the vault (`list_notes`, `read_note`, `edit_note`, `rename_note`, `trash_note`, `daily_note`, `add_comment`…), each returning an `Outcome` |
| `cli/store.rs` | the vault as the commands see it: the index read once, the path rules, the lookup of a note, the token |
| `cli/outcome.rs` | the result type, the readers of the arguments, the disk errors in words |
| `cli/draw.rs` | `inkuno render` and `render --check`: a Mermaid source, an SVG file of the vault, or a note's Mermaid block, to PNG |
| `cli/guide.rs` | `inkuno guide`: the pages of `docs/user/src/`, built into the binary |
| `cli/rev.rs` | `inkuno rev save`, `list`, `show`, `diff`, over `platform/revisions.rs` (`docs/design/revisions.md`) |
| `cli/keys.rs` | `inkuno keys list`, `set`, `reset`: the app's own rules for the shortcut file (`ui/keys.rs`, `shell/keymap.rs`) |
| `cli/check.rs` (test) | the saved versions follow a note through rename, trash and restore |
| `crates/inkuno/tests/cli.rs` | the binary run end to end, with the examples of the help |

## Commands

Reading: `vault`, `list`, `search`, `read`, `links`, `tags`, `show`.
Writing: `new`, `edit`, `append`, `move`, `trash`, `restore`, `daily`, `comment`.
Diagrams: `render`, `render --check`.
Guide: `guide [page]`.
Versions: `rev save`, `rev list`, `rev show`, `rev diff`.
Shortcuts: `keys list`, `keys set`, `keys reset`.
Help: `help [command]`, and `--help` after any command.

The options of each one are in `cli/commands.rs`, and in `docs/user/src/agents.md`.

## Decisions

| Date | Decision (owner-approved, 2026-11-14 unless noted) |
|---|---|
| 2026-11-14 | The command line is the agents' interface: one binary, no window, files only. It replaces the MCP server, and the server is removed. |
| 2026-11-14 | The six prompts, the `--read-only` option and the note and diagram resources are dropped. A prompt was a request written in advance that a client sent; `read` and plain files give what the resources gave. `--read-only` promised what could not be enforced: an agent with a shell can write files anyway. |
| 2026-11-14 | The token of `read --json` is called `token`, not `revision`. "Revision" and "version" now mean a saved version only (`inkuno rev`). |
| 2026-11-14 | `show` keeps a local door to the open window: `platform/link.rs`, a TCP port on 127.0.0.1 with a secret, both in `<config>/inkuno/agent` while the window is open. It is the only command that talks to the app. It fails with a sentence when the app is not open, and it changes nothing. |
| 2026-11-14 | Text by default, `--json` for the structure. Errors on standard error with exit 1; a wrong command line exits 2. |
| 2026-11-14 | Arguments are read by hand, with no new dependency. |
| 2026-11-14 (planned, not built) | An Assistant panel in the right dock. It starts the `hoocode` binary over its RPC mode. The first version is a read-only reviewer: chat about the note, and "Review this note" writes comments. The command that starts the agent is a setting. Inkuno stores no keys: the agent's own program holds them. |
| 2026-11-14 (planned, not built) | `inkuno write <note> --author NAME`. Each agent write is bracketed by automatic single-author snapshots (`kind: auto`). The change is pending: the agent's text is tinted until the human accepts, rejects or comments on it. Only the pending area is read-only for the human. A write that touches text the human changed since the agent read it is refused. |
| 2026-11-14 (planned, not built) | Later, the same commands talk to the running app over a local socket, instead of only writing files and asking through `show`. |
