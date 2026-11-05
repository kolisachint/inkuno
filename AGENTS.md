# AGENTS.md — handoff notes for coding agents

Read this first. Then read the plan for the current iteration: `docs/IMPLEMENTATION_PLAN.md` (iteration 1, P0 to P3) or `docs/IMPLEMENTATION_PLAN_ITERATION_2.md` (iteration 2, P4 to P10, starts after iteration 1). The DoD and subagent template are in each plan. Also read `docs/DESIGN.md`.

> **ON HOLD: do NOT work on math or Mermaid.** This covers math (MathJax, QuickJS; P6 tasks 6.1, 6.3, 6.5 and 6.6) and Mermaid (P7), including the existing math and diagram code in `md_render` (`math*`, `diagram`, `host`, Mermaid parts of `queue`/`fallback`). Do not start, extend, refactor or test them until the repo owner explicitly confirms. Their task text is held in `docs/IMPLEMENTATION_PLAN_ITERATION_2.md` under "On hold". Tasks 6.2 and 6.4 are not on hold.

## Current iteration

- **Iteration 1 = P0 to P3:** a plain-markdown editor. It opens, edits, selects, navigates, takes IME input, scrolls and saves a `.md` file, on macOS and Linux, released as nightly binaries.
- **Iteration 1 is done apart from the macOS gate run** (Linux gate passed, see `docs/P3-gate-linux.md`).
- **Iteration 2 (P4 to P10) is complete on Linux for every task that is not on hold.** Done: P4 live preview (4.1 to 4.5), P5 code fences (5.1 to 5.3), P8 modes (8.1 to 8.5), P9 workspace (9.1 to 9.5), P10 theme tokens and Auto (10.1, 10.2), settings persistence (10.3, see below), the performance part of 10.4, and soft word wrap. The performance part of 10.4 has benchmarks in `crates/md_core/benches/core.rs` and `crates/md_editor/benches/pure.rs`. Results and follow-up fixes are in `docs/spikes/perf-pass.md` (incremental block list and cached caret hidden ranges landed). The math and diagram part of 10.4 is on hold with P6 and P7. P6 and P7 stay on hold. Iteration 2 was merged to `main` via a PR from `claude/iteration2-live-preview`. The plan is `docs/IMPLEMENTATION_PLAN_ITERATION_2.md`.

## How work is done here

- Dispatch small-scoped **Haiku** subagents, one plan task each, using the template at the end of the plan file.
- Each session's working branch is created from `main` and named `claude/<topic>`. Run each subagent in its own git worktree. Review each worktree commit, then merge it into the working branch in the main checkout.
- Worktrees can start from a stale commit. Tell every subagent to first run `git merge --ff-only <working branch>` (or `git reset --hard <working branch>` while it has no work yet).
- Subagents share one target directory, `CARGO_TARGET_DIR=/home/user/noteapp/target`, to save disk and build time. Subagents must **not** run `cargo clean`. When done, they remove any app binary copies they made, and keep only one.
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
  - Assets: `noteapp-macos-arm64.zip` (ad-hoc signed `.app`), `noteapp-macos-arm64.tar.gz`, `noteapp-linux-x86_64.tar.gz`, plus `SHA256SUMS` checksums and auto-generated release notes.
- `.github/workflows/md_core.yml` runs fmt, clippy and tests for `md_core` on every push.
- Pushing changes under `.github/workflows/` needs a GitHub credential with **workflow** scope. If a push is rejected for that reason, park the file under `docs/ci/` and add a line to Blockers below.

## CI

- **`release.yml`** runs on push to `main`, `claude/**` and `v*` tags, and on manual dispatch. The `macos` job (`macos-14`) and the `linux` job (`ubuntu-22.04`) each run `cargo build --release -p app` and package the assets. The `publish` job then replaces the `nightly` pre-release, or creates a versioned release for a tag. Both build jobs use `Swatinem/rust-cache@v2`.
- **`md_core.yml`** runs on every push and pull request (`ubuntu-latest`): `cargo fmt --all --check`, `cargo clippy -p md_core --all-targets -- -D warnings` and `cargo test -p md_core`, with `Swatinem/rust-cache@v2`.
- **`workspace.yml`** runs on every push and pull request (`ubuntu-22.04`): it installs the apt packages from the `linux` job in `release.yml`, then runs `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`, with `Swatinem/rust-cache@v2`.
- **macOS code is compiled only in CI.** After each push, check that the release run for the branch is green on the GitHub Actions page for that branch. Do not call a change that touches macOS code done until it is.
- **New workflows have not run yet.** `workspace.yml` was added on a branch that has not been pushed, so neither it nor the affected `release.yml` builds have run. Confirm both are green after the push.

## Status

### Iteration 1 (P0 to P3)

| Area | State |
|------|-------|
| P0 scaffold | Done. CI workflows enabled in `.github/workflows/`. |
| P1 spikes | S4 done (provisional GO). S3 GO (conditional; IME check open, macOS-only). S1 GO (split host; math path on hold). S2 runs on Linux (X11/Xvfb, `docs/spikes/S2-linux-hello.png`); macOS run and go/no-go still PENDING (blocker 1). |
| P2 md_core | 2.1–2.8 done (156 unit tests + 5 proptests). |
| P3 editor basic | 3.1 to 3.7 done. Linux gate passed (`docs/P3-gate-linux.md`, 11 of 11 checks). macOS gate run (screenshot, IME composition) still PENDING (blockers 1 and 2b). Save is `cmd-s` on macOS and `ctrl-s` on Linux. |

Iteration 1 is complete on Linux (gate passed). The macOS gate run (screenshot, IME composition) is still pending and needs a Mac: blockers 1 and 2(b).

### Iteration 2 (P4 to P10)

Screenshots are in `docs/screenshots/`. All rows are verified on Linux (Xvfb) unless noted.

| Area | State | Screenshots |
|------|-------|-------------|
| P4 live preview | 4.1 to 4.5 done: styled basic blocks, lists and task items, read-only tables and images, inline reveal, input rules. Follow-ups landed: table cells render inline markdown, `%20` image paths decode (`md_editor/src/blocks/image.rs`). | `4.1.png`, `4.2.png`, `4.3.png`, `4.4.png`, `4.4-caret-in-bold.png`, `4.5.png` |
| P5 code fences | 5.1 to 5.3 done (`md_render::highlight`, `lang`, fence element with fence reveal). | `5.3.png`, `5.3-caret-inside.png` |
| P6 render host, ON HOLD | 6.2 done (`cache`, `queue`, render traits); 6.4 core done (`md_render::raster`). 6.1, 6.3, 6.5, 6.6 are on hold and must not be touched. | none |
| P7 Mermaid, ON HOLD | 7.3 core done (`queue` debounce 300 ms, `fallback` last-good). 7.1, 7.2 and the 7.3 UI part are on hold. | none |
| P8 modes | 8.1 Source Mode, 8.2 caret and scroll preservation, 8.3 Focus Mode, 8.4 Typewriter Mode, 8.5 mode keys done (see below). | `8.1.png` (also 8.2), `8.3.png`, `8.3-moved.png`, `8.4.png` |
| Soft word wrap | Done (not a plan task). Up and Down move by visual row with an x goal. | `wrap.png`, `wrap-selection.png` |
| P9 workspace | 9.1 sidebar, 9.2 watcher applied in the app, 9.3 create, rename and delete UI, 9.4 search panel, 9.5 image paste all done. Follow-ups landed: sidebar delete moves to the system Trash (`trash` crate), empty folders show in the tree, a Folder button creates folders, search on a dirty open note uses its buffer (`workspace::search` `BufferOverride`), and selection highlights over whole table and image blocks (painted over the content, whole block bounds). | `9.1.png`, `9.2.png`, `9.3-ui.png`, `9.4.png`, `9.5.png` |
| P10 theming | 10.1 colour tokens (Light and Dark, `md_editor/src/theme.rs`) and 10.2 Auto (`app/src/theme.rs`, follows the window appearance) done. Cycle the theme with Ctrl+K Ctrl+T (Cmd+K Cmd+T). 10.3 settings persistence (`app/src/settings.rs`): a JSON file in the config dir (provisional, DESIGN §16 item 9). Theme mode, last vault, Focus and Typewriter are saved and restored; Source Mode is not. `NOTEAPP_CONFIG_DIR` overrides the dir. 10.4 perf part done. Keystroke-to-paint on Xvfb/llvmpipe, 4 vCPU, 10k-line document: median ~9.4 ms, p90 ~16.7 ms. The 16 ms p90 target is NOT met on this rig. Reparse touches only dirty blocks: pass. Only visible blocks are laid out: pass. Results in `docs/spikes/perf-pass.md`. The measure+prepaint double layout is removed (`MeasuredRows` cache in `md_editor/src/element.rs`); the Xvfb re-measure is still pending, plus a real GPU and macOS. The math and diagram part of 10.4 is on hold with P6/P7. | `10.1-light.png`, `10.1-dark.png`, `10.3-restart.png`, `10.3-vault-restart.png`, `10.4-perf.png`, `10.4-followup-typed.png` |

8.5 (mode keys) is done: Source on Ctrl+/ (Linux) or Cmd+/ (macOS), Focus on F8, Typewriter on F9, and the Cycle Theme chord, all in one table in `crates/app/src/keymap.rs`. The `NOTEAPP_SOURCE`, `NOTEAPP_FOCUS` and `NOTEAPP_TYPEWRITER` env vars were removed. Focus and Typewriter can be on together, and both work in Source Mode.

### Linux environment notes

- **Body font.** On Linux the body text uses "Liberation Sans" (`BODY_FAMILY` in `md_editor/src/element.rs`). Install `fonts-liberation`. GPUI maps the system font to IBM Plex Sans, which is usually missing, and then there are no bold or italic faces.
- **X auth errors.** `xvfb-run` can give X auth errors for `import` and `xdotool`. Either set `XAUTHORITY` to the auth file that `xvfb-run` created, or start `Xvfb :N -ac -nolisten tcp -screen 0 1280x800x24 &` directly and use `DISPLAY=:N`.
- **Shared target directory.** When agents share one `CARGO_TARGET_DIR`, another build overwrites `target/debug/app`. Build with `cargo build -p app`, copy the binary at once, and keep only one copy, because disk is limited.
- **Theme under Xvfb.** The window appearance on Xvfb is Light, so Auto always resolves to Light there. Test Dark with the Cycle Theme chord (Ctrl+K Ctrl+T) instead. Auto's resolution is covered by unit tests in `app/src/theme.rs`.
- **Dev Mac has only Command Line Tools.** `cargo build -p app` fails locally: the gpui_apple build script needs Xcode's `metal` utility (`xcrun: unable to find utility "metal"`). The library crates build and test fine. Verify app-crate changes in CI. GPUI has a `runtime_shaders` feature as a possible local workaround (not adopted).
- **Math input rule is off.** `$$` is disabled in the editor by `HELD_RULES` in `md_editor/src/input_rules.rs` (`math_block`), while math is on hold. Keep it off until the owner confirms.

## Blockers and open items

Each item names what is blocked and what unblocks it. Remove an item when it is resolved.

1. **S2 go/no-go pending (iteration 1).** `docs/spikes/S2-gpui-hello.md` needs a macOS run and a screenshot. CI cannot take an interactive screenshot.
   Unblock: run the `nightly` macOS build on a Mac (`xattr -dr com.apple.quarantine noteapp.app`) and attach a screenshot. Alternatively, accept the Linux run plus a green macOS CI build as the gate.
2. **macOS-only checks from S1/S3.** (a) The hidden WKWebView host that Mermaid needs (DESIGN §14 item 1) is unverified. Held with P6/P7 (iteration 2, on hold). (b) IME composition across hidden markup ranges is not evaluated, since md_core has no UTF-16 or marked-text API. This blocks the macOS P3 gate (3.4).
   Unblock for (b): a macOS run of the P3 gate. Unblock for (a): a macOS run, only when the owner lifts the hold. See `docs/spikes/S1-render-host.md` and `S3-display-map.md`.
3. **macOS signing.** The `nightly` release publishes macOS and Linux builds (macOS compiles in CI since the 1.98.1 toolchain bump). Release builds are ad-hoc signed and not notarized, so users must clear quarantine.
   Unblock: an Apple Developer ID certificate and notarization credentials stored as repo secrets. This needs the repo owner.
4. **Dev Mac cannot build the `app` crate locally.** Only Command Line Tools are installed, so the gpui_apple build script fails on the `metal` shader step. The library crates build and test fine.
   Unblock: install Xcode, or confirm the macOS CI build is green after each push, or adopt GPUI's `runtime_shaders` feature as a local workaround (not adopted).

### Iteration 2 open items (no blocker)

- **IME bounds across a wrap row** cover only the first row.
- **Remote images are not loaded.**
- **A reload after an external change is one undo step.**
- **Typewriter Mode caret far-jump** may show one blank frame.
- **Two workspace watcher tests fail on macOS** (FSEvents timing; they pass on Linux): `external_edit_reloads_clean_buffer_and_keeps_dirty_one` and `removed_file_is_reported_and_marks_buffer_missing`.
- **New Folder has no key binding** (owner decision; the `NewFolder` action exists unbound).

## Next tasks to dispatch (in order)

1. **Push `claude/pending-work` and confirm CI is green.** `release.yml` (macOS + Linux builds) and the new `workspace.yml` have not run on this branch (not pushed). The app-crate changes (`search_panel.rs`, `workspace_view.rs`, `file_ops_ui.rs`) were never compiled locally, so do not call them done until CI passes.
2. **macOS verification on a real Mac (owner).** Run the `nightly` build (`xattr -dr com.apple.quarantine noteapp.app`). Record the S2 screenshot and the P3 gate (blockers 1 and 2b). Check iteration 2 on macOS:
   - system font bold and italic (see the Linux font note);
   - Cmd keys: Cmd+/, Cmd+K Cmd+T, Cmd+N, Cmd+Shift+F;
   - Auto theme following the macOS appearance live (10.2 acceptance: the window updates within one frame);
   - image paste from the macOS clipboard (9.5);
   - settings file at `~/Library/Application Support/noteapp/settings.json` (10.3, `app/src/settings.rs`).
3. **Performance: re-measure keystroke-to-paint.** The double layout is removed. Re-measure the same way as in `docs/spikes/perf-pass.md` (same generated 10k-line document, same `xdotool` method) and record the result there. Then re-measure on a real GPU and on macOS.
4. **Open items worth doing (from the list above).** Remote images; reload as one undo step; watcher FSEvents timing tests; IME bounds across wrap rows (needs the macOS check in 2); Typewriter far-jump blank frame.
5. **Owner decisions pending.** Mode keys (DESIGN §16 item 1; implemented as Ctrl/Cmd+/, F8, F9 in `crates/app/src/keymap.rs`); settings location (DESIGN §16 item 9, provisional); New Folder key binding (the `NewFolder` action exists unbound); Linux scope (DESIGN §1.2); macOS signing (blocker 3); lifting the math and Mermaid hold.
6. **Do NOT start** math (P6 6.1, 6.3, 6.5, 6.6) or Mermaid (P7), including the existing code. These are on hold until the repo owner explicitly confirms.
