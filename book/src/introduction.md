# Inkuno

Inkuno is a fast, minimal note-taking app. Your notes are plain Markdown files in a folder you choose, styled as you type. It is written in Rust with [GPUI](https://www.gpui.rs), a GPU-accelerated UI framework. It continues Bref by Andrea Larboullet Marin under a new name.

New here? [Install Inkuno](install.md), then read [First steps](first-steps.md).

## What it does

| | |
|---|---|
| **Fast** | The window opens in about 75 ms. A keystroke costs about a millisecond, even in a note of 5,000 lines. See [Performance](performance.md). |
| **Plain files** | One `.md` file per note, in a folder called the *vault*. An existing folder of Markdown notes works as is, front matter included. See [Files on disk](vault.md). |
| **Styled as you type** | Headings grow, bold and italics render, markers stay visible but dimmed. The file stays plain Markdown. See [Writing notes](writing.md). |
| **Linked notes** | `[[links]]`, backlinks, tags, embedded sections and a graph. See [Links, embeds and files](links.md). |
| **Comments** | Annotate a passage without changing it, in a format other apps can read. See [Comments](comments.md). |
| **Daily notes** | Today's note from the blank page, and a calendar. See [Daily notes](daily-notes.md). |
| **Tabs and panes** | As many tabs and split panes as the window holds. See [Tabs, panes and docks](tabs-and-panes.md). |
| **Sync** | The vault is a folder of plain files: sync it with git or a cloud folder. See [Syncing a vault](sync.md). |
| **Keyboard first** | One palette for notes and commands. Every shortcut is listed in the app and can be changed. See [Keyboard shortcuts](shortcuts.md). |
| **Yours to dress** | A dozen themes, IBM Plex built in, any installed font, light and dark. See [Settings and appearance](settings.md). |

Inkuno speaks English and French, following the language of your system.

## Conventions in this guide

- Keys are written for Linux and Windows. On macOS, read `Cmd` for `Ctrl`, and `Alt` for `Ctrl` when moving by word.
- The keys shown are the defaults. The help panel (`F1`) always shows the ones in effect, and lets you [change them](settings.md#changing-a-shortcut).
- **Bold** names a button, a menu entry or a palette command, as written in the app.
