# Daily notes and templates

## Today's note

The [blank page](first-steps.md#the-blank-page) shows today's date: one click opens the note of the day, `2026-10-09.md`, wherever it is in the vault. If there is none yet, it creates it, with the date as its title.

- `@today`, `@tomorrow`, `@yesterday`, `@monday` or `@2026-10-09` in the text links to the daily note of that day. See [Links between notes](links.md#links-between-notes).
- The [blank page](first-steps.md#the-blank-page) offers the daily notes of a week, a month and a year ago.

## The calendar

The **Calendar** view of the panel (`Ctrl + Shift + J`) shows one month.

- Today is a round pill. A dot under a day means it has its daily note.
- The last days of the neighbouring months are dimmed, and open their note too.
- Arrows change the day. `Page Up` and `Page Down` change the month.
- `Enter` or a click opens the note of the day, or creates it.
- The week starts on Monday in French, on Sunday in English.

## Templates

Put notes in a folder named `Templates` (or `Modèles`) at the root of the vault: each one is a template.

- Case and accents of the folder name do not matter, and templates may sit in sub-folders.
- Templates are left out of the graph, the tags, the links and the notes to read again. A `[[link]]` never points at one.

**New note from a template**: **new note from a template** in the palette, or in the right-click menu of a folder of the tree.

1. Choose the template.
2. Type the name of the note.
3. The note is written in the folder selected in the tree (the root of the vault otherwise) and opened, with the cursor where the template says.

A name already taken is refused; nothing is overwritten.

**Placeholders**

| Placeholder | Becomes |
|---|---|
| `{{title}}` | the name you typed |
| `{{date}}` | `2026-10-09` |
| `{{time}}` | `09:05` |
| `{{cursor}}` | where the cursor goes (the placeholder disappears) |

Any other `{{word}}` stays as written.

**Insert a template**: **insert a template** in the palette puts one at the cursor, filled with the name of the open note. One `Ctrl + Z` takes it back.

**Make a template of a note**: **make a template of this note** in the palette saves the open note in the templates folder, created if there is none. Its first `#` title becomes `{{title}}`.
