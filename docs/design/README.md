# docs/design — index

Design ground truth for the note app. App code follows these docs, never the
reverse (`CONTRIBUTING.md`).

- **`app-ui.md` — philosophy home.** Vision, tokens, layout, components, prefs,
  acceptance, locks (§10), keybinding philosophy (§11, the WHY), visual spec
  v1 (§12), strip-down v1 (§13, user-approved 2026-11-08 — supersedes the
  §10.2 KEEP-all stance; §10 item-4 geometry locks still hold). North star:
  the product direction in §2 (Typora-inspired, three regions toggleable,
  quiet chrome, Inter + JetBrains Mono, 800 px target, phases).
- **`keybindings.md` — chords.** Source-of-truth contract, binding tables,
  palette-alias convention, changelog (T-fix: `Ctrl+Shift+T` = reopen tab,
  Recent unbound), reserved keys (`F9` focus/zen candidate). Implements
  `app-ui.md` §11; if it disagrees with `defaults()`, code wins.
- **Steering log:** `app-ui.md` §15 (2026-11-08 session — 7 user-agreed decisions;
  done: rebuild-trigger armed, Rail kept; queued: split-hide, F9, GPUI
  upgrade, docs sweep, Linux CI-confirm). Pointer only — crews land the work
  in their own files.
- **`mockups/` — visual truth.** `editor-current-v1.html` (CURRENT-behavior
  build; CURRENT vs TBD badges per `app-ui.md` §12.5). Reference only: never edit
  markup to change behavior — change code + doc first.

Conventions: one calm chord per intent, palette-first discoverability
(everything stripped stays typed-reachable), chords are stable API (renames
need a deprecation path). Stripped components stay modular in code —
bring-back anytime, just not in main UI now.
