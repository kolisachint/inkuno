# UI parity: the app against the v2 prototype

Scripts and logs that compare the real app (driven on Xvfb) with `docs/design/prototypes/v2.html`
(driven with Playwright), state by state and workflow by workflow, in the light and dark themes.

- `SCENARIOS.md`: the workflows (W1–W8), with the same step names on both sides so that screenshots pair up.
- Screenshots are not committed: the scripts write them under `target/` or a scratch folder.
