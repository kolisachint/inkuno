# Compatibility

| System | Status |
|---|---|
| Linux, GNOME on Wayland | first-class: this is what Inkuno is built and used on (Arch Linux) |
| Linux, other desktops, Wayland or X11 | should work, with the same built-in title bar. Not tested. |
| macOS 11 or later | built and tested by CI on every commit, and the download is built there too. Not used day to day by the author. |
| Windows | not built: CI runs on Linux and macOS only, and no download ships. |

## Linux

- Inkuno needs a working Vulkan driver (Mesa or the vendor one).
- Inkuno draws its own title bar, shadow and resize edges, because GNOME under Wayland draws none for applications. Drag the window by the pill at the top right, or by the empty part of the title band. A double click maximizes it. You can also hold `Alt` and drag from anywhere. `Ctrl+Shift+S` hides that title bar, with its window buttons; pointing at the top edge of the window brings it back over the note while the pointer stays there, and the same key shows it again. The choice is kept.

On macOS and Windows, the system title bar is used. On macOS, `Ctrl+Shift+S` hides it with its traffic lights, and shows them again.

## Fonts

Inter (text and titles) and JetBrains Mono (code) are built into the app. Ideograms and emoji come from a font of your system. See [Fonts](settings.md#fonts).

## Accessibility

- Every view can be reached from the keys.
- Every color pair stays readable in the 13 themes.
- A screen reader cannot read the window yet: the toolkit Inkuno is drawn with exposes no accessibility tree.
- Composing text with an input method (Chinese, Japanese, Korean, dead keys) works in notes. In a table cell, the text is inserted as it is typed.
