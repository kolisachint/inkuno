# Assistant

The Assistant is an AI agent that reads the note you have open. You can chat with it about the note, or ask it to review the note. A review's findings are written into the note as comments, in one step: one `Ctrl + Z` takes them back. The agent never changes your text itself.

The agent is not part of Inkuno. Inkuno starts the `hoocode` program and talks to it. Login and model stay in hoocode's own settings.

## What you need

- **hoocode**, installed and logged in on its own. Inkuno finds it on your `PATH`.

Without it, the Assistant still opens, and its panel says that the program was not found. Nothing else changes.

## The state

The Assistant's button in the switcher of the right panel shows the state of the agent. There is no button at the bottom of the note.

| State | What you see |
|---|---|
| Idle | quiet |
| Working | the accent colour, pulsing slowly |
| Needs you | an accent dot: an answer came while the view was not on display |
| Not installed | dimmed; the tooltip says `Assistant: hoocode is not installed`, with the command set in the settings |

A click on the button shows the Assistant view. Showing it clears the dot. The panel is hidden in [focus mode](navigation.md) (`F9`).

## The panel

The panel is the Assistant view of the right panel, beside the outline, comments and versions. Open it with its button in the switcher, `Ctrl + Alt + 4`, or the command palette: type `assistant`.

From top to bottom, the panel has:

- the conversation;
- **Review this note**, and **Improve selection** (or **Improve this note**), and **Stop** while the agent works;
- the prompt box.

## Asking about the note

- Type in the prompt box and press `Enter`. The agent answers about the note that is open.
- Inkuno saves the note first, so the agent reads what you see.
- The conversation shows your question only. The agent is also told which file the note is.
- The prompt box is one line. Long text wraps. A new line is not possible in this version.
- `Escape` takes you back to the note.

## Review this note

**Review this note** asks the agent to read the note and list what it finds: unclear or wrong statements, gaps, contradictions and typos that change the meaning. The agent answers with a short list of passages and comments. Inkuno then writes each one as a comment on its passage, in the [comment syntax](comments.md): `passage<!-- comment: text -->`.

- The answer itself is not shown. While the agent works, the panel says **Reviewing…**. When it ends, the panel says how many comments were written.
- All the comments of one review are one undo step. `Ctrl + Z` takes the whole review back.
- A finding is left out when its passage is not in the note exactly once, or when it lies inside a comment that is already there. The panel says how many were left out.
- The comments show in the Review view, with the ones you write. Click one to go to it, and resolve it as usual.

If you edit the note while the agent works, the comments go into the text as it is when the agent ends.

## Improve

**Improve selection** asks the agent to rewrite the text you have selected. With nothing selected, the button says **Improve this note** and rewrites the whole note. The same action is in the command palette as **Improve selection or note**, and in the right-click menu as **Improve selection**.

- The agent is asked to make the writing clearer and tighter, with the same meaning, the same language and the same Markdown structure. To ask for something else, type it in the prompt box first, such as `make it formal`. The box is then cleared, and the instruction is used for this one request.
- The new text is not written at once. It comes back as **pending changes** in the note: you see the removed lines and the added lines, and you accept or reject them as usual (see the [Agents page](agents.md#accepting-rejecting-and-commenting)). One undo takes the whole proposal back.
- Inkuno saves two versions around the proposal: the text before, and the proposed text. They are kept in the note's history, which `inkuno rev list` shows.
- A note with pending changes cannot be improved yet. Accept or reject them first.
- If you change the same lines while the agent works, nothing is proposed, and the panel asks you to ask again. Changes to other lines are kept.
- If the agent says it has no change to make, the panel says so, and the note stays as it is.

## Stop

**Stop** ends the agent's work while it is running. A review that is not finished writes nothing. If the agent has not stopped after three seconds, Inkuno ends its process, and the panel is free again.

## The setting

`assistant` in the [settings file](settings.md#the-settings-file) is the command that starts the agent. It is `hoocode` by default. To use another command, type `Assistant command…` in the palette. An empty entry returns to `hoocode`. Changing the command stops the agent that runs.

## What it does not do

- It does not change your text on its own. It cannot run a program or write a file: it is started without those tools. Its findings become comments only through Inkuno, and its proposals become pending changes only through Inkuno.
- It does not ask you before it does something. The agent has no tool that could change anything, so there is nothing to approve.
- It stores no keys and no model choice. Those stay in hoocode's configuration.
- It does not keep the conversation. The conversation is kept in memory only, and it is gone when Inkuno restarts.
- It stops when Inkuno quits, or when you change the vault.

## Scratch notes

Scratch notes work too. When no vault is open, the agent works in the scratch folder. A review of a scratch note writes its comments into that note, the same way.
