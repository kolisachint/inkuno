# Syncing a vault

A vault is a folder of plain files. To have the same notes on every computer, and a copy somewhere else, sync that folder with the tool of your choice: git, Syncthing, iCloud, Dropbox or Google Drive. Inkuno itself syncs nothing: it [follows what changes in the folder](vault.md#changes-from-other-programs), whatever put it there.

Pick one tool, never two on the same folder: a git repository inside a cloud folder ends up corrupted.

## With a cloud folder

Put the vault in a folder synced by iCloud, Dropbox, Google Drive or Syncthing. Nothing more is needed.

## With git

Keep the vault in a git repository and commit, pull and push with git itself, in a terminal or with the graphical tool you prefer. Any empty repository works: create one on GitHub, GitLab or Codeberg, or on any server you reach by SSH with `git init --bare notes.git`.

A repository that already has a history is never overwritten by a clone: fetch it first, then move your notes in.

## Conflicts

When the same note changed on both sides between two syncs, both versions are kept, as Dropbox and Syncthing do: the note keeps the version of this computer, and the other one is written next to it as `Note (conflict 2026-10-09).md`. You never get `<<<<<<<` markers in a note, and nothing is overwritten.

## Limits

- Syncing happens in your sync tool, not in Inkuno: there is no timer, no sync on quit, and no sync button.
- git syncs computers. There is no Inkuno on a phone, where the repository stays readable with another app.
