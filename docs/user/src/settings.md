# Settings and appearance

Every choice is a command of the palette: type its name, and see the result at once.

## Theme

The half-filled circle at the bottom of the rail, or `theme` in the palette.

- 13 themes: Inkuno Dark and Inkuno Light, Dracula, One Dark, Gruvbox (dark and light), Nord, Catppuccin (Mocha and Latte), Tokyo Night, Rosé Pine, Solarized (dark and light).
- Each theme is applied as you move through the list, so you see it before choosing. `Enter` keeps it, `Esc` goes back to the one you had.
- `Default` follows the light or dark setting of your system, with Inkuno Light or Inkuno Dark.
- In every theme, links, secondary text and the titles of colored panels are adjusted to a contrast of at least 4.5:1 (WCAG AA).

**The look.** The window is a warm ground, a little darker than the page in every theme. The note, the panel and the right dock lie on it as rounded sheets, eight pixels apart. Page titles are in a serif, while note titles follow your font.

- The active pane of a split has a thin accent ring.
- The open row of a list, a tag in the text, a commented passage and the current tab take the accent colour as text.
- Tables and code blocks have a rounded band.
- Menus, the palette, help and notices float above, with a shadow.

## Fonts

Inkuno carries IBM Plex Sans (text), IBM Plex Mono (code) and IBM Plex Serif (page titles), under the SIL Open Font License (`crates/inkuno/assets/fonts/`). So it looks the same everywhere, with nothing to install.

- Type `font` in the palette to choose another font, for the app or for the code, among those installed on your system. Type in the list to filter it.
- Ideograms and emoji, which Plex does not draw, come from a font of your system.
- A variable font cannot be emboldened. See [Troubleshooting](troubleshooting.md#bold-text-is-not-bold).

## Text size

`Ctrl + +` and `Ctrl + -` change the size, `Ctrl + 0` restores the default.

Headings, code and the width of the text column scale with it. The interface around the note (panel, rail, menus, palette, help) follows too, from 85 % to 150 %, so it stays usable at either end.

## Page width

`page width` in the palette sets how wide the text column is: narrow, normal, wide, or the full window.

`wide tables` in the palette chooses what a table wider than the text does:

- **center** (the default): it spills out on both sides of the column, centered on the window;
- **column**: it stays in the column, and scrolls sideways.

A small table keeps its own size, and nothing goes past the window.

## Formatting bar and animations

- `formatting bar` in the palette shows or hides the [formatting bar](writing.md#the-formatting-bar) under the note.
- `animations` in the palette turns the short fades off. When on, the panel, the formatting bar and a folder of the tree fade in over about 0.13 s as they open. Nothing moves on typing, scrolling or the caret, and no fade delays a result. Hovers and closing are instant.

## Language

The interface is in English.

## The settings file

These choices are kept in a file named `settings`, in the [configuration folder](vault.md#configuration-files). It holds one `key=value` per line:

| Key | Values | Set by |
|---|---|---|
| `theme` | the name of a theme | `theme` |
| `font`, `mono` | the font of the app, of the code | `font` |
| `size` | the text size | `Ctrl + +`, `Ctrl + -`, `Ctrl + 0` |
| `page` | `narrow`, `normal`, `wide`, `full` | `page width` |
| `wide` | `column`, `center` | `wide tables` |
| `bar` | `on`, `off` | `formatting bar` |
| `motion` | `on`, `off` | `animations` |

## Changing a shortcut

The help panel (`F1`) lists every shortcut, and lets you give any of them other keys.

- **Change**: click the keys of a row, or move to it with `Down` and press `Enter`. The row waits, and the next combination you press becomes its shortcut, at once. `Esc` cancels. `Backspace` leaves the action with no shortcut.
- **Conflicts**: a combination already used where the action applies is not taken. The row says by which action. `Enter` (**Replace**) gives it to the new one, and the other is left without a shortcut. The same keys in two views that never meet (a table and a diagram) are not a conflict.
- **Refused keys**: a key alone that types text, and the keys the text and the views need (`Enter`, `Tab`, the arrows…), are refused, with the reason. A combination your system usually keeps for itself is accepted with a warning.
- **Reset**: a changed row is highlighted and has a **default** button. **Reset all**, at the top of the panel, puts every shortcut back.
- The right-click menus and the palette show the keys in effect next to each action, so a change made here shows there too.
- Rows without a button are not key bindings (a click, a palette command, something you type). The search finds them, but they cannot be changed.

**The `keys` file.** Only what you changed is kept, one `action=keys` line each, in the `keys` file of the configuration folder, next to `settings`. `secondary` stands for `Cmd` on macOS and `Ctrl` elsewhere. A line that cannot be used (unknown action, unreadable keys, keys already taken) is ignored and reported at launch; the default stays.

## Changing shortcuts from a file

The help panel is the easy way. A JSON file does the same, for every action, and can be edited with any text editor.

- **Where**: `keybindings.json` in the [configuration folder](vault.md#configuration-files): `~/.config/inkuno/` on Linux, `~/Library/Application Support/inkuno/` on macOS.
- **Which file wins**: when `keybindings.json` is there and not empty, it is the one read, and it is read at launch: restart Inkuno to apply an edit. The older `keys` file is only read while it is absent. The help panel saves its changes to `keys`, so once `keybindings.json` exists, edit that file to change shortcuts.
- **Shape**:

```json
{
  "version": 1,
  "bindings": { "app::ToggleFocus": ["f8"], "nav::ShowTree": [] },
  "reset": ["app::ToggleTitleBar"]
}
```

- **Action names**: `group::Name`, as the code names them: `app::` for the window (`app::OpenPalette`, `app::ToggleFocus`), `editor::` for the note (`editor::Bold`), `nav::` for the panel (`nav::ShowTree`). Only an action that has a line in the help panel can be changed. The code keeps the list (`help_sections` in `crates/inkuno/src/shell/keymap.rs`).
- **Chords**: one chord per action, in the form `secondary-shift-p`, `alt-left`, `ctrl-tab` or `f9`. `secondary` is `Cmd` on macOS and `Ctrl` elsewhere. `""` or `[]` leaves the action with no shortcut. `reset` restores the default keys of the actions it names.
- **Faults**: an entry that cannot be used is ignored on its own, and the default stays. `inkuno --check-config` reports an unreadable chord, or a chord given to two actions, and exits with `1`. An unknown action name is not caught there: the app reports it when it starts.
