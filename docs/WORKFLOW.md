# The 30-Minute Shape — parallel-agent operating protocol

Proven in the session that built it (24h → 30min target). Follow this literally.

## 0. Preconditions (one-time, before the clock starts)

- Warm caches: sccache + rust-cache in CI, shared target dir locally.
- Tiered CI: fast tier (fmt, check, clippy, unit tests, ~4 min) gates the full matrix.
- `scripts/prune-agents.sh` runs on any disk-full stall.
- This box is orchestrator + reviewer. CI (or a metal-capable machine) is the
  compiler. Never attempt full local builds; they cannot link without Xcode's
  `metal` toolchain.

## 1. Minutes 0–5: decide everything upfront

- One batched Q&A: trim list, keep/cut calls, version number. No interleaved
  questions later — a mid-wave scope flip (cut/keep/cut) costs a worktree.
- Dispatch read-only scouts in parallel (brand, debloat, renderers, rename).
  Scouts stay on the main tree; they never edit.

## 2. Minutes 5–20: waves of 3, green between waves

- Max 3 writer crews per wave, each in its own worktree off the recorded base
  (`git worktree add -b agent/<label> ~/.hoocode/worktrees/<repo>/<label> <base>`).
- Partition by FILE, not by feature: one crew owns shared hubs
  (`keymap.rs`, registries, `mod.rs` files); everyone else REPORTs registry
  lines instead of touching them.
- Features frozen while trims land. New code goes in only after cuts merge.
- Each wave merges and CI-proves before the next starts. Three small merges
  cost a tenth of one big one. Never accumulate 10+ unmerged workstreams.
- Every crew: 15-minute slices, no local builds (grep + rustfmt + review only),
  20-minute HARD GATE — any worker passing 20 minutes is halted with its
  edits kept, and the owner gets a continue / defer / drop decision.

## 3. Minutes 20–25: CI judges, once per wave

- Batch fixes by review; burst pushes with one CI run judging the batch, never
  a push per fix. Background `gh run watch` on every run; heartbeat on landing.
- **Format gate is `cargo fmt --all -- --check; echo $?` (exit code).** It parses
  only, no compile/metal — run it locally before every push. Do NOT grep
  `rustfmt --check` output for `error`: format diffs print as `-`/`+` lines, so a
  grep-for-error check silently passes while CI fails. This exact mistake cost
  two CI round-trips.
- Fast tier first (fmt → check → clippy → unit tests), full matrix only on green.
- Run `scripts/prune-agents.sh` between waves (dry-run default, `--apply` when
  free space is under the threshold) so disk never blocks the next crew.

## 4. Minutes 25–30: PR, then the label gate releases

- **PR rule: when the owner says "create a PR", create it and STOP. Merging is
  the owner's decision.** Merge only when the owner explicitly confirms. Never
  auto-merge to keep a pipeline moving.
- Release flow (already wired in `.github/workflows/release-gate.yml`): apply
  exactly one of `rust:patch`, `rust:minor`, `rust:major` to the PR; merging to
  `main` triggers the gate → bump Cargo.toml, commit `chore: release X.Y.Z`,
  tag, draft release with author notes, then `release-native.yml` builds
  macOS + Linux and publishes only after every download verifies. Labels must
  exist in the repo (created 2026-11-10; recreate with `gh label create` if
  missing). Any merge without exactly one label is a no-op. The FIRST release
  (no previous tag) stays manual via `scripts/release.sh`.

## Standing rules

- Never push unless asked. Never merge a PR unless the owner explicitly says
  so — stop at the PR and report the link.
- Never commit on the user's branch unasked — writers
  leave diffs uncommitted; the integrator commits grouped merges.
- Batch all independent tool calls and subagent dispatches in one turn.
- Keep a live todo list; park small calls in `docs/BACKLOG.md` instead of
  expanding scope.
