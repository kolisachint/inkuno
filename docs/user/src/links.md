# Links, embeds and files

## Links between notes

- `[[` suggests your notes. A link to a note that does not exist creates it.
- `[[Groceries]]` finds the note by file name, in any subfolder, ignoring case.
- `[[Note|alias]]` shows another text. `[[Note#Heading]]` points at a heading.
- `@` suggests days and links to the daily note of that day: `@today`, `@tomorrow`, `@yesterday`, a weekday (`@monday`: the next one, never today), or a date (`@2026-10-09`). French names work too (`@demain`, `@lundi`). An address such as `name@domain` suggests nothing.
- `Ctrl + click` opens a link. `Ctrl + Shift + click` opens it in the pane beside.
- Each name in the `aliases` of a note's front matter is another name for it: suggested after `[[`, and followed by links.

When a note is renamed, the links to it follow. See [Renaming](vault.md#names-and-renaming).

## Tags

A `#tag` in the text, or a `tags` key in the front matter, tags a note.

- `#` in the palette lists the notes carrying a tag.
- `Ctrl + click` opens a tag of the text.

## Pictures

`![](picture.png)` or `![[picture.png]]` shows the picture under its line: PNG, JPEG, GIF, WebP, SVG, BMP, TIFF or AVIF. Inkuno looks for it next to the note, then at the top of the vault, then by its file name anywhere in the vault.

- `Ctrl + V` with a picture in the clipboard saves it next to the note and inserts its `![](…)` line.
- Typing `![[` suggests the pictures of the vault.
- **Drop a file** on the note: an image is copied into the `assets/` folder next to the note and inserted as `![name](assets/name)`, each on its own line. Another file becomes a link, `[name](assets/…)`. A name already taken is never overwritten: a different file gets ` 2`, ` 3`… before its extension, and the same file is reused. One `Ctrl + Z` takes the drop back.
- **Web pictures**: `![](https://…)` and `<img src="https://…">` are fetched once, then kept on disk. The fetch needs the system `curl` and gives up after 10 seconds or a picture over 20 MB. The `width` and `height` of an `<img>` are honoured. A picture is never wider than the text column.
- A picture that is still loading, is not found, cannot be fetched or is not an image shows a box with its alternative text and what is wrong.
- **Enlarge**: point at a picture (PNG, JPEG, GIF, WebP…), a formula, an SVG or a Mermaid diagram, and a magnifier button appears at its corner, or beside it when it is too small to hold it. It opens a viewer: `Ctrl + wheel` zooms around the pointer, and dragging moves the picture.
- A click on the picture itself still places the cursor. A missing or unreadable picture has no button.

Pictures are part of the vault. See [Files on disk](vault.md#pictures-and-other-files).
