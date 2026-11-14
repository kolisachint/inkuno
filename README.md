# Inkuno

[![CI](https://github.com/kolisachint/inkuno/actions/workflows/ci.yml/badge.svg)](https://github.com/kolisachint/inkuno/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/kolisachint/inkuno)](https://github.com/kolisachint/inkuno/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

A fast, minimal note-taking app. Notes are plain Markdown files in a folder you choose, styled as you type. Written in Rust with [GPUI](https://www.gpui.rs), a GPU-accelerated UI framework.

Inkuno continues [Bref](https://github.com/alarboulletmarin/bref) by Andrea Larboullet Marin under a new name; its home is now [github.com/kolisachint/inkuno](https://github.com/kolisachint/inkuno).

![Inkuno, light and dark](docs/assets/inkuno.png)

**[Install](#install)** · **[User guide](docs/user/src/introduction.md)** · [Shortcuts](docs/user/src/shortcuts.md) · [Website](https://kolisachint.github.io/inkuno/) · [Contributing](CONTRIBUTING.md)

## Why Inkuno

- **Fast.** The window opens in about 75 ms. A keystroke costs about a millisecond, even in a note of 5,000 lines. [Figures](docs/user/src/performance.md)
- **Plain files.** One `.md` file per note, in an ordinary folder. Sync it with git, Syncthing or a cloud folder. An existing folder of Markdown notes works as is.
- **Styled as you type.** Headings grow, bold and italics render, markers stay visible but dimmed. The file on disk stays plain Markdown.
- **Tables.** Cells wrap, `Tab` moves between them, and pasting from a spreadsheet works.
- **Linked notes.** `[[Links]]`, tags and daily notes. The tree follows the active file, including diagrams, pictures and tables.
- **More than notes.** List pages and Mermaid diagrams in notes.
- **Agents.** `inkuno` on the command line lets an agent with a shell read notes, write them and add Mermaid diagrams, with Inkuno's links, trash and diagrams. [Guide](docs/user/src/agents.md)
- **Safe tab closing.** Close a tab, the other tabs or a whole pane from a tab's right click, or every file from the palette. If saving fails, the document stays open with its edits and undo history.
- **Local sharing.** Copy a whole note to the clipboard, or the code block under the cursor.
- **Keyboard first.** One palette for notes and commands. Every shortcut is listed in the app (`F1`) and can be changed.
- **Yours to dress.** 13 themes, IBM Plex built in, any installed font. Light and dark, in English.

![Typing a note: the title grows, styles render, a table adds up its column](docs/assets/demo.gif)

## Install

| System | Get it |
|---|---|
| Linux, x86_64 | the ready-made binary, below |
| macOS 11 or later | [`inkuno-macos.dmg`](https://github.com/kolisachint/inkuno/releases/latest/download/inkuno-macos.dmg) |
| Ubuntu, Debian, Fedora, other Linux | build from source, below |

```bash
# Linux, x86_64: the ready-made binary
curl -LO https://github.com/kolisachint/inkuno/releases/latest/download/inkuno-linux-x86_64.tar.gz
tar -xzf inkuno-linux-x86_64.tar.gz
sudo install inkuno /usr/local/bin/
```

```bash
# Other Linux: needs Rust 1.98 or later, and the libraries listed in the install guide
git clone https://github.com/kolisachint/inkuno.git
cd inkuno
make && sudo make install
```

The macOS download is not signed. The first launch needs one extra click: see [the install guide](docs/user/src/install.md), which also covers the packages of each distribution, building on macOS and Linux, and upgrading. Inkuno never checks for updates: fetch a new release yourself when you want one.

## Getting started

1. Launch Inkuno and click **Start in ~/Documents/Notes**, or open a folder you already have.
2. Type. The first line is the title of the note and the name of its file. Saving is automatic.
3. `Ctrl + P` finds or creates a note, `F10` lists every command, `F1` lists every shortcut.

On macOS, read `Cmd` for `Ctrl`. More in [First steps](docs/user/src/first-steps.md).

## Documentation

The [user guide](docs/user/src/introduction.md) covers everything. It is also published as a website: [kolisachint.github.io/inkuno/guide](https://kolisachint.github.io/inkuno/guide/).

| Page | What it covers |
|---|---|
| [Writing notes](docs/user/src/writing.md) | Markdown, styles, lists, slash commands, blocks, several cursors |
| [Links, embeds and files](docs/user/src/links.md) | links, tags, pictures, dropped files |
| [Comments](docs/user/src/comments.md) | annotating a passage, in a format other apps read |
| [Daily notes](docs/user/src/daily-notes.md) | today's note |
| [The palette and the panel](docs/user/src/navigation.md) | the tree, the trash, icons |
| [Tabs, panes and docks](docs/user/src/tabs-and-panes.md) | arranging the window |
| [Search and replace](docs/user/src/search.md) | in a note |
| [Files on disk](docs/user/src/vault.md) | naming, front matter, backup, configuration files |
| [Syncing a vault](docs/user/src/sync.md) | git or a cloud folder |
| [Settings and appearance](docs/user/src/settings.md) | themes, fonts, page width, changing a shortcut |
| [Updates](docs/user/src/updates.md) | no automatic updates; how to upgrade |
| [Keyboard shortcuts](docs/user/src/shortcuts.md) | every shortcut, as the help panel lists them |
| [Agents (command line)](docs/user/src/agents.md) | `inkuno read`, `new`, `edit`…: an agent reads notes, writes notes and adds Mermaid diagrams |
| [Compatibility](docs/user/src/compatibility.md), [Troubleshooting](docs/user/src/troubleshooting.md), [Performance](docs/user/src/performance.md) | systems, known problems, measurements |

## Contributing

Bug reports and small fixes are welcome. For a feature, open an issue first. Read [CONTRIBUTING.md](CONTRIBUTING.md) and the map of the code, [docs/design/architecture.md](docs/design/architecture.md). For a security problem, see [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
