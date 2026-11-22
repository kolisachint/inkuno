# UI parity: the app against the v2 prototype

Scripts and logs that compare the real app (driven on Xvfb) with `docs/design/prototypes/v2.html`
(driven with Playwright), state by state and workflow by workflow, in the light and dark themes.

- `SCENARIOS.md`: the workflows (W1–W8), with the same step names on both sides so that screenshots pair up.
- Screenshots are not committed: the scripts write them under `target/` or a scratch folder.
- `proto-shots.js`: every state of a prototype in both themes (`node tests/visual/parity/proto-shots.js`; `PROTO=` another prototype file, `OUT=` another folder; default `target/visual/proto`). States: `PROTO-STATES.md`.
- `wf-proto.js`: the W1–W8 workflows in the prototype (default output `target/visual/wf-proto`); last run's log: `WF-PROTO-LOG.md`.
- `ui-check.sh`: the app in the same states as the prototype (`--theme light|dark|both`, `--only NAME…`; `BIN=`, `OUT=`, default `target/visual/app`). Needs Xvfb, xdotool, ImageMagick, lavapipe (see `scripts/visual-check.sh`).
- `wf-app.sh`: the W1–W8 workflows in the real app on Xvfb (`BIN=`, `WORK=`; default `target/visual/wf-app`); last run's log: `WF-APP-LOG.md`.
- `WF-APP-LOG-FINAL.md`: the same workflows on the finished branch (47 of 48 OK).

## Status (2026-11-22)

- **Done**: 112 findings over four areas (`findings/`), each decided in `DECISIONS.md`; the "fix" rows
  are in the app (chrome, right panel, popovers and settings, Markdown rendering, polish), the "keep"
  rows are drawn in `docs/design/prototypes/v3.html`.
- **Structure**: `crates/inkuno-platform` and `crates/inkuno-ui` split out of the app;
  reusable pieces in `crates/inkuno-ui/src/components/`.
- **Checks**: workspace tests green apart from three sandbox-only failures (root user, a terminal
  installed): `an_unreadable_folder_does_not_hide_the_rest`, `an_outside_write_while_typing_is_merged`,
  `end_to_end` (the "No terminal found" toast). Workflows: `WF-APP-LOG-FINAL.md`, 47 of 48 OK.

## Follow-ups

- F1 does nothing while a settings text field has the focus (W8-04).
- From v3, not yet in the app: the palette keeps the `>` in command mode; an age column on note rows;
  the slash menu's query line; the compare bar on the compare tab.
- Comment replies (RP-13) need a format decision; Files page buttons (SET-13).
- `ui-check.sh`: settings pages need a 4 s settle; `40-titlebar-peek` in `proto-shots.js` reports a false miss.
- Undoing a written comment takes three steps (resolve, text, empty marker); the draft's marker and its text should be one undo step.
- `layout.rs` (4,100+ lines), `sidebar.rs` and `inkuno-platform/src/vault.rs` are past the 1,500-line rule: split by subject.
