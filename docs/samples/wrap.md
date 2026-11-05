# Soft wrap sample

The paragraph below is long enough to wrap onto several rows at the default window width. Long prose like this is how most notes look, and a plain editor that lets it run off the right edge makes every sentence hard to read and edit. The text keeps going for a while so that there are several visual rows to click into, and the middle of it carries **bold text in the middle of the sentence** so that a wrapped row can contain a styled run that spans a row break. Click in the second row to check where the caret lands, then press Shift+Right across the row break to check the selection. Lines are not split in the file, so the wrapping is only visual, and the source keeps one long line for this paragraph. Moving the caret with Up and Down goes row by row through the wrapped text, while Home and End still stop at the ends of the source line. Words such as configuration, interoperability and characterization are long enough to show how a single word is handled when it does not fit in the remaining space of a row. The end of the paragraph is here.

- A long list item that should wrap under its text, not under its bullet, so the second and later rows line up with the first line of the item text after the marker. This item is deliberately wordy to force a wrap at the default width.
- A short item.

> A long blockquote line that should wrap inside the quote and keep its bar on the left edge of every row, so the quote reads as one block even when the text spans several rows of the window.
