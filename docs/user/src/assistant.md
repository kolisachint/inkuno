# Assistant

The Assistant is an AI agent that reads the note you have open. You can chat with it about the note, or ask it to review the note. A review writes its findings as comments. It does not change your text.

The agent is not part of Inkuno. Inkuno starts the `hoocode` program and talks to it. Login and model stay in hoocode's own settings.

## What you need

- **hoocode**, installed and logged in on its own. Inkuno finds it on your `PATH`.

Without it, the Assistant still opens, and its panel says that the program was not found. Nothing else changes.

## The button

A round button sits at the bottom right corner of the note, 30 pixels wide. The comments bubble stays to its left. The button shows the state of the agent:

| State | What you see |
|---|---|
| Idle | quiet |
| Working | the accent colour, pulsing slowly |
| Needs you | an accent dot: the agent needs you |
| Not installed | dimmed; the tooltip says hoocode was not found |

A click opens the panel, or closes it. The button is not shown in [focus mode](navigation.md) (`F9`).

## The panel

The panel is a tool of the right dock, next to the outline. It is hidden at first. Open it with the **Assistant** button, or type `assistant` in the palette (`F10`). There is no key for it by default.

From top to bottom, the panel has:

- the conversation;
- a request from the agent, when it asks permission: **Allow** or **Deny**;
- **Review this note**, and **Stop** while the agent works;
- the prompt box.

## Asking about the note

- Type in the prompt box and press `Enter`. The agent answers about the note that is open.
- The prompt box is one line. Long text wraps. A new line is not possible in this version.
- `Escape` takes you back to the note.

## Review this note

**Review this note** asks the agent to read the note and write what it finds as comments. Each comment is on one passage, in the [comment syntax](comments.md): `{==passage==}{>>comment<<}`. They show in the comments panel, with the ones you write.

- Click a comment to go to it, and resolve it as usual.
- The agent is asked to change nothing but comments. This is an instruction, not a lock: an agent with a shell can write files.

Comments appear in the open note within about half a second. See [Agents](agents.md#while-you-have-the-note-open) for what happens when you type at the same time.

## Allow and Deny

When the agent asks permission for something, the request shows in the panel.

- **Allow** lets it go ahead.
- **Deny** refuses.

## Stop

**Stop** ends the agent's work while it is running.

## The setting

`assistant` in the [settings file](settings.md#the-settings-file) is the command that starts the agent. It is `hoocode` by default. To use another command, type `Assistant command…` in the palette. An empty entry returns to `hoocode`.

## What it does not do

- It does not edit your text silently. In this version it is asked to write comments only.
- It stores no keys and no model choice. Those stay in hoocode's configuration.
- It does not keep the conversation. The conversation is kept in memory only, and it is gone when Inkuno restarts.
- It stops when Inkuno quits, or when you change the vault.

## Scratch notes

Scratch notes work too. When no vault is open, the agent works in the scratch folder. A review of a scratch note writes its comments into that note, the same way.
