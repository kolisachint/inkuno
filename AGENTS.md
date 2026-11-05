# AGENTS.md — handoff notes for coding agents

Read this first, then `docs/IMPLEMENTATION_PLAN.md` (task list, Definition of Done, subagent template) and `docs/DESIGN.md`.

## How work is done here

- Dispatch small-scoped **Haiku** subagents, one plan task each, using the template at the end of `docs/IMPLEMENTATION_PLAN.md`.
- Run each subagent in its own git worktree, then review and merge it into the working branch.
- Worktrees can start from a stale commit. Tell every subagent to first run `git merge --ff-only <working branch>` (or `git reset --hard <working branch>` while it has no work yet), and to run `cargo clean` when done, because disk space is limited.
- Every task ends with these passing at the workspace root:
  `cargo build`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check`.
- Do not change the GPUI git rev (root `Cargo.toml`) except in a dedicated bump change.
- `rust-toolchain.toml` must match the `rust-toolchain.toml` of the pinned zed rev (currently 1.98.1). An older toolchain fails to compile `gpui_macos`, which only shows up in the macOS CI job. Bump both together.
- Targets are **macOS and Linux**. Linux is where an agent can build, run and screenshot the app. macOS is built in CI only.
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

## Status (see the plan for acceptance criteria)

| Area | State |
|------|-------|
| P0 scaffold | Done. CI workflows enabled in `.github/workflows/`. |
| P1 spikes | S4 done (provisional GO). S2: runs on Linux (X11/Xvfb, screenshot in `docs/spikes/`), no macOS run yet. S1 GO (split host: MathJax in embedded QuickJS, Mermaid needs a WebView). S3 GO (conditional on the IME check). |
| P2 md_core | 2.1–2.8 done (156 unit tests + 5 proptests). |
| P3 editor basic | Interfaces merged (`docs/P3-interfaces.md`); the app opens an empty Editor window. 3.1 and 3.6 in progress. |
| P4 live preview | Not started (GPUI). |
| P5 code fences | 5.1, 5.2 done; 5.3 not started (GPUI). |
| P6 render host / math | Decided: split host (MathJax v4 in QuickJS in-process, WebView for Mermaid only; DESIGN §6.1). 6.2 done; 6.4 core done (`md_render::raster`; screenshot with bundled fonts waits on 6.3); 6.1, 6.3, 6.5, 6.6 not started. |
| P7 Mermaid | 7.3 core done (debounce 300 ms, last-good fallback, `BlockView`); 7.3 UI part, 7.1, 7.2 not started. |
| P8 modes | Not started (GPUI). |
| P9 workspace | 9.3 done; 9.1, 9.4 core done (`Expansion`, `visible_rows`, `MatchSpan`; sidebar and search panel UI left); gitignored notes hidden from tree, search and watcher; 9.2 core done (`watcher::VaultWatcher`, `plan_buffer_updates`; app must apply updates; 1 s latency not asserted in tests); 9.5 not started. |
| P10 theming | Not started. |

## Blockers and open items

Each item names what is blocked and what unblocks it. Remove an item when it is resolved.

1. **S2 go/no-go pending.** `docs/spikes/S2-gpui-hello.md` needs a macOS run and a screenshot. CI cannot take an interactive screenshot.
   Unblock: run the `nightly` macOS build on a Mac (`xattr -dr com.apple.quarantine noteapp.app`) and attach a screenshot. Alternatively, accept the Linux run plus a green macOS CI build as the gate.
2. **macOS-only checks from S1/S3.** (a) The hidden WKWebView host that Mermaid needs (DESIGN §14 item 1) is unverified. (b) IME composition across hidden markup ranges is not evaluated, since md_core has no UTF-16 or marked-text API.
   Unblock: a macOS run during 6.1/7.1 for (a), and during 3.4 for (b). See `docs/spikes/S1-render-host.md` and `S3-display-map.md`.
3. **macOS signing.** The `nightly` release publishes macOS and Linux builds (macOS compiles in CI since the 1.98.1 toolchain bump). Release builds are ad-hoc signed and not notarized, so users must clear quarantine.
   Unblock: an Apple Developer ID certificate and notarization credentials stored as repo secrets. This needs the repo owner.

## Next tasks to dispatch (in order)

1. P3: interfaces and task briefs are in `docs/P3-interfaces.md` (merged). Order: 3.1 and 3.6, then 3.2 and 3.5, then 3.3 and 3.4. Check the git log for which ones are merged. Each task is verified with a Linux screenshot under `docs/screenshots/`.
2. Then P4, then P5.3, P6, P7, P8, P9 UI, P10, following the plan's dependency graph.
