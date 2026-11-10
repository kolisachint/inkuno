# Links, embeds and files

## Links between notes

- `[[` suggests your notes. A link to a note that does not exist creates it.
- `[[Groceries]]` finds the note by file name, in any subfolder, ignoring case.
- `[[Note|alias]]` shows another text. `[[Note#Heading]]` points at a heading.
- `@` suggests days and links to the daily note of that day: `@today`, `@tomorrow`, `@yesterday`, a weekday (`@monday`: the next one, never today), or a date (`@2026-10-09`). French names work too (`@demain`, `@lundi`). An address such as `name@domain` suggests nothing.
- `Ctrl + click` opens a link. `Ctrl + Shift + click` opens it in the pane beside.
- Each name in the `aliases` of a note's front matter is another name for it: suggested after `[[`, and followed by links, backlinks and the graph.

When a note is renamed, the links to it follow. See [Renaming](vault.md#names-and-renaming).

## Backlinks

The **Backlinks** view of the panel (`Ctrl + L`) lists the notes that link to the open one, each with the line that holds the link.

- Links written with an alias, and links inside tables, count.
- Embeds count too, marked *embedded*.
- The list follows the note you open, not the one you preview while moving through the list. So you can look at each linking note in turn.

## Tags

A `#tag` in the text, or a `tags` key in the front matter, tags a note.

- `#` in the palette lists the notes carrying a tag.
- The **Tags** view of the panel (`Ctrl + T`) lists every tag of the vault with its number of notes.
- `Ctrl + click` opens a tag of the text.

## Embedded sections

A line holding only `![[Note#Heading]]` shows that heading, and what is under it up to the next heading of the same level, in place and live.

| Write | Shows |
|---|---|
| `![[Note#Heading]]` | the section under that heading |
| `![[Note]]` | the whole note, without its front matter |
| `![[Note#^id]]` | the block that ends with `^id` (a common Markdown convention) |

- **Look**: the section is drawn in a frame, with a thin bar on its left in the accent color and the name of its note at the top right. It follows the theme and wraps to the page.
- **Open the source**: click the name, double click the frame, or press `Alt + Enter` on its line. The note opens at that section. `Ctrl + click` on a `[[Note#Heading]]` link does the same.
- **Suggestions**: typing `![[Note#` suggests the headings of the note, and `![[Note#^` its blocks. This works after `[[` too.
- **Read-only**: to change the text, change it in its note.
- **Loops**: an embed that leads back to a note already in the chain says *circular embed*. Three levels are shown at most.
- **Renames**: renaming the note rewrites the embeds and links to it. Renaming the heading (the same headings but one, saved) rewrites `#Heading` in the other notes.
- **Only alone on its line**: inside a sentence, a list item or a code block, it is a plain link. `![[picture.png]]` is still a picture.

The file stays plain Markdown: other apps show the line as a link.

## Pictures

`![](picture.png)` or `![[picture.png]]` shows the picture under its line: PNG, JPEG, GIF, WebP, SVG, BMP, TIFF or AVIF. Inkuno looks for it next to the note, then at the top of the vault, then by its file name anywhere in the vault.

- `Ctrl + V` with a picture in the clipboard saves it next to the note and inserts its `![](…)` line.
- Typing `![[` suggests the pictures of the vault.
- **Enlarge**: point at a picture (PNG, JPEG, GIF, WebP…), a formula, an SVG or a Mermaid diagram, and a magnifier button appears at its corner, or beside it when it is too small to hold it. It opens a viewer: `Ctrl + wheel` zooms around the pointer, and dragging moves the picture.
- A click on the picture itself still places the cursor. A missing or unreadable picture has no button.

Pictures are part of the vault. See [Files on disk](vault.md#pictures-csv-and-other-files).
