# AGENTS.md — handoff notes for coding agents

Read this first. Then read the plan for the current iteration: `docs/IMPLEMENTATION_PLAN.md` (iteration 1, P0 to P3) or `docs/IMPLEMENTATION_PLAN_ITERATION_2.md` (iteration 2, P4 to P10, starts after iteration 1). The DoD and subagent template are in each plan. Also read `docs/DESIGN.md`.

> **ON HOLD: do NOT work on math or Mermaid.** This covers math (MathJax, QuickJS; P6 tasks 6.1, 6.3, 6.5 and 6.6) and Mermaid (P7), including the existing math and diagram code in `md_render` (`math*`, `diagram`, `host`, Mermaid parts of `queue`/`fallback`). Do not start, extend, refactor or test them until the repo owner explicitly confirms. Their task text is held in `docs/IMPLEMENTATION_PLAN_ITERATION_2.md` under "On hold". Tasks 6.2 and 6.4 are not on hold.

## Current iteration

- **Iteration 1 = P0 to P3:** a plain-markdown editor. It opens, edits, selects, navigates, takes IME input, scrolls and saves a `.md` file, on macOS and Linux, released as nightly binaries.
- **Iteration 1 is done apart from the macOS gate run** (Linux gate passed, see `docs/P3-gate-linux.md`). The next session starts from `docs/IMPLEMENTATION_PLAN_ITERATION_2.md`, in its suggested order.

## How work is done here

- Dispatch small-scoped **Haiku** subagents, one plan task each, using the template at the end of the plan file.
- Run each subagent in its own git worktree, then review and merge it into the working branch.
- Worktrees can start from a stale commit. Tell every subagent to first run `git merge --ff-only <working branch>` (or `git reset --hard <working branch>` while it has no work yet), and to run `cargo clean` when done, because disk space is limited.
- Every task ends with these passing at the workspace root:
  `cargo build`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`.
- Do not change the GPUI git rev (root `Cargo.toml`) except in a dedicated bump change.
- `rust-toolchain.toml` must match the `rust-toolchain.toml` of the pinned zed rev (currently 1.98.1). An older toolchain fails to compile `gpui_macos`, which only shows up in the macOS CI job. Bump both together.
- Targets are **macOS and Linux**. Linux is where an agent can build, run and screenshot the app. macOS is built in CI only. (DESIGN §1.2 still lists Linux as a non-goal; the owner must reconcile this.)
- Running the app headless on Linux: install the apt packages listed in `.github/workflows/release.yml` plus `xvfb mesa-vulkan-drivers imagemagick`, then
  `xvfb-run -n 99 -s "-screen 0 1280x800x24" cargo run -p app &` and `DISPLAY=:99 import -window root shot.png`.
  Use a fixed display number (`-n 99`); `xvfb-run -a` alone opened no window in testing. Only X11 + llvmpipe has been verified, not Wayland or a real GPU.

## Releases

- `.github/workflows/release.yml` builds `-p app` on `macos-14` (arm64) and `ubuntu-22.04` (x86_64).
  - A push to `main` or `claude/**` replaces the `nightly` pre-release.
  - A push of a `v*` tag creates a versioned release.
  - Assets: `noteapp-macos-arm64.zip` (ad-hoc signed `.app`), `noteapp-macos-arm64.tar.gz`, `noteapp-linux-x86_64.tar.gz`.
- `.github/workflows/md_core.yml` runs fmt, clippy and tests for `md_core` on every push.
- Pushing changes under `.github/workflows/` needs a GitHub credential with **workflow** scope. If a push is rejected for that reason, park the file under `docs/ci/` and add a line to Blockers below.

## Status

### Iteration 1 (P0 to P3)

| Area | State |
|------|-------|
| P0 scaffold | Done. CI workflows enabled in `.github/workflows/`. |
| P1 spikes | S4 done (provisional GO). S3 GO (conditional; IME check open, macOS-only). S1 GO (split host; math path on hold). S2 runs on Linux (X11/Xvfb, `docs/spikes/S2-linux-hello.png`); macOS run and go/no-go still PENDING (blocker 1). |
| P2 md_core | 2.1–2.8 done (156 unit tests + 5 proptests). |
| P3 editor basic | 3.1 to 3.7 done. Linux gate passed (`docs/P3-gate-linux.md`, 11 of 11 checks). macOS gate run (screenshot, IME composition) still PENDING (blockers 1 and 2b). Save is `cmd-s` on macOS and `ctrl-s` on Linux. |

Iteration 1 is complete on Linux (gate passed). The macOS gate run (screenshot, IME composition) is still pending and needs a Mac: blockers 1 and 2(b).

### Iteration 2 (P4 to P10): work already in the repo

These pieces exist ahead of their phases. Treat them as reusable starting points, see the "Starting point" section of the iteration 2 plan.

- P4 to P5: 5.1 and 5.2 done (`md_render::highlight`, `lang`). 5.3 not started.
- P6, ON HOLD: 6.2 done (`cache`, `queue`, render traits); 6.4 core done (`md_render::raster`). 6.1, 6.3, 6.5, 6.6 are on hold and must not be touched.
- P7, ON HOLD: 7.3 core done (`queue` debounce 300 ms, `fallback` last-good). 7.1, 7.2 and the 7.3 UI part are on hold.
- P8 modes: not started.
- P9 workspace: 9.3 done (`ops`). 9.1 and 9.4 core done (`tree`, `search`; sidebar and search panel UI not started). 9.2 core done (`watcher::VaultWatcher`, `plan_buffer_updates`; the app must apply updates). 9.5 not started.
- P10 theming: not started.

## Blockers and open items

Each item names what is blocked and what unblocks it. Remove an item when it is resolved.

1. **S2 go/no-go pending (iteration 1).** `docs/spikes/S2-gpui-hello.md` needs a macOS run and a screenshot. CI cannot take an interactive screenshot.
   Unblock: run the `nightly` macOS build on a Mac (`xattr -dr com.apple.quarantine noteapp.app`) and attach a screenshot. Alternatively, accept the Linux run plus a green macOS CI build as the gate.
2. **macOS-only checks from S1/S3.** (a) The hidden WKWebView host that Mermaid needs (DESIGN §14 item 1) is unverified. Held with P6/P7 (iteration 2, on hold). (b) IME composition across hidden markup ranges is not evaluated, since md_core has no UTF-16 or marked-text API. This blocks the macOS P3 gate (3.4).
   Unblock for (b): a macOS run of the P3 gate. Unblock for (a): a macOS run, only when the owner lifts the hold. See `docs/spikes/S1-render-host.md` and `S3-display-map.md`.
3. **macOS signing.** The `nightly` release publishes macOS and Linux builds (macOS compiles in CI since the 1.98.1 toolchain bump). Release builds are ad-hoc signed and not notarized, so users must clear quarantine.
   Unblock: an Apple Developer ID certificate and notarization credentials stored as repo secrets. This needs the repo owner.

## Next tasks to dispatch (in order)

1. **Iteration 1.** Done apart from the macOS gate run (blockers 1 and 2b). Nothing to dispatch here until a Mac run is recorded.
2. **Iteration 2.** The next session starts from `docs/IMPLEMENTATION_PLAN_ITERATION_2.md`, in its suggested order: P4 live preview (4.1 to 4.5), then P5.3, then P8 modes, P9 workspace UI, and P10 theming.
3. **Do NOT start** math (P6 6.1, 6.3, 6.5, 6.6) or Mermaid (P7), including the existing code. These are on hold until the repo owner explicitly confirms.
