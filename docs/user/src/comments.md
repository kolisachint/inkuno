# Comments

`Ctrl + Shift + M` annotates a word, a line or the line of an image, without changing its text. Without a selection, it comments the line.

## In the file

The comment is kept in the note itself, as an HTML comment that GitHub hides:

```text
the text<!-- comment @Ana 2026-11-24T10:02Z: the comment -->
```

The comment names its author and the minute it was written (UTC). Its replies follow it on the same line, each with its author and minute, separated by ` ↳ `. A comment written before this has no author: it shows as **Unknown**, and it keeps its old form until someone replies to it.

The passage is the text from the start of its line (or of its table cell, or of the comment before it) up to the comment. In Inkuno, the passage has a yellow wash, a gold underline and a gold dot in the margin of its line, and the comment is faded; off the line you are editing, the comment's markup is hidden. Notes written with the earlier CriticMarkup comments (`{==text==}{>>comment<<}`) are read as before, and are written in this form when the note is next saved.

A review by the [Assistant](assistant.md) writes its findings in this form too, and its answers to your open comments as replies. All of them are one undo step.

## The Review view

The right panel has a **Review** view, `Ctrl + Alt + 2`. It lists the comments of the note and its pending changes as one feed, oldest first. Its head counts them: **Review · 4 open**. It shares the panel with the outline, the versions and the Assistant: see [Navigation](navigation.md#the-right-panel).

When the Assistant writes comments or proposes changes in the note you have open, this view opens by itself on the first new card, expanded. Keyboard focus does not move: you keep typing where you were.

The feed is in time order. An item with no time (an older comment, or a change with none) comes first, in the order of the note. A comment is dated by its minute; a pending change, by the minute it was proposed.

Each card shows who it is by, its kind (a comment, a replacement, an insertion or a deletion), how long ago, the first lines of what it says, and where it is: its section and its line in the file, as `Section · Line 12`.

- Only one card is open at a time. Click a card to open it: its passage is selected in the note.
- A comment opens with its full text and its replies. Type a reply in the field and press `Enter`: it is written after the replies, on the same line. **Resolve** takes the comment's markup out and keeps the passage; `Ctrl + Z` brings the comment back.
- A pending change opens with **Before** (what it removes) and **After** (what it adds). **Accept** and **Reject** do what the keys do in the note.
- Changes from one Assistant proposal form one card, with **Accept all N** and **Reject all N** for its N changes. See [Assistant](assistant.md#improve).

A new comment opens a draft card at the top of the feed: the passage it is on, a field for what is said, and **Comment** and **Cancel** buttons. `Enter` writes the comment; `Escape` or **Cancel** takes it out again and the passage stays as it was.

- Click a comment's passage in the note to open its card.
- The bubble at the note's foot, and the switcher's button, count the open items (`9+` beyond nine).
- The pill at the foot of the note that says how many changes are pending opens the feed on the first change.
- `review` in the palette lists the comments, to jump to one from the keyboard.

On a comment, `Ctrl + Shift + M` resolves it. On a line of its own (see below), the line goes.

## In a table

- **A cell**: the comment is on the text of the cell. A selection over several cells gets one comment per cell, never across a `|`. A `|` typed in the comment is written `\|`.
- **A row, a column or the whole table**, taken by its grip: the comment goes on a line of its own under the table, named by its content. A row is named by its first cell, a column by its header: `<!-- comment: Column "Age": in years -->`.

Because the comment is named by content, a sort leaves it where it belongs:

- Renaming that first cell or header renames the comment.
- Deleting the row, the column or the table deletes it.
- Moving, inserting, duplicating, sorting or swapping rows and columns keeps it with what it is about. One `Ctrl + Z` brings both back.

How it shows:

- A commented cell is tinted, with a mark in its corner.
- A commented row, column or table is outlined, with a dot on its grip. It is stronger while the cursor is in the comment.
- The Review view says where each comment is: `Cell C2`, `Row Léa`, `Column Age`, `Table`.

A comment on an empty cell is refused: there is nothing to highlight.
