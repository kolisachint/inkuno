# S2: GPUI on macOS (hello window)

Plan task 1.2 (code part). DESIGN §13 S2, §10.

## Pinned revision

- Repository: https://github.com/zed-industries/zed
- Revision: `c7bea343838f42b4c14151393e41f0bbe033c95e`
- Commit date: 2026-11-04T19:25:28Z (committer date from a shallow fetch of that commit)
- Subject: `gpui: Fix cursor reverting when a drag starts (#66540)`
- Chosen as the then-current `main`, as the task asked. The gpui crate in this revision reports version `0.2.2`; the lock entry is a git source, not crates.io.
- Recorded in: root `Cargo.toml` (`[workspace.dependencies]`), `crates/app/Cargo.toml` and `crates/md_editor/Cargo.toml` (macOS-only).

## Verified on Linux

- Lock resolution: `cargo metadata` resolved `gpui` into `Cargo.lock` as
  `git+https://github.com/zed-industries/zed?rev=c7bea343...#c7bea343...`.
  No crates.io `gpui` entry exists in the lock (`registry+` count on the gpui entry is 0).
  `cargo update -p gpui` was not usable because gpui is only referenced under `cfg(target_os = "macos")`, so cargo does not know the package yet; `cargo metadata` resolved it.
- Lock changes: 323 packages were added (the lock covers all target platforms, so the gpui dependency tree is included). No existing (name, version) pair was removed or changed.
- `gpui_platform` is also pinned at the same rev and used from `crates/app` on macOS (see deviation below).
- `cargo build`, `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all --check` pass on Linux. GPUI itself is not compiled on Linux.

## Deviation from the brief

- The pinned example (`crates/gpui/examples/hello_world.rs`) gets its platform from `gpui_platform::application()`, not from `gpui` alone. `gpui_platform` is therefore pinned at the same rev and added to `crates/app` under the macOS target section. This is the only dependency beyond `gpui`.

## Hello window

`crates/app/src/main.rs` mirrors the pinned example's structure:

- macOS: `application().run(...)`, `Bounds::centered`, `cx.open_window` with a `TitlebarOptions` title of `noteapp`, a `Render` view that shows `Hello, noteapp`, and `cx.activate(true)`.
- Other platforms: prints `noteapp targets macOS; GPUI window disabled on this platform.` and exits 0.
- The example's `example_support::load_fonts` is omitted: in the pinned tree its native implementation is a no-op returning `true`.

## Not verified (needs a macOS run)

- The macOS branch of `main.rs` has not been compiled. It cannot be built on Linux because GPUI is not compiled there. Any API drift will show up in the first macOS build.
- `cargo run -p app` opens a window titled `noteapp` showing `Hello, noteapp`.
- A screenshot is attached to the note once captured on macOS.

Go/no-go: PENDING — needs macOS run.

## Linux

Verified on Linux (Ubuntu, Xvfb with llvmpipe Vulkan), same pinned rev.

- Dependencies: `crates/app` and `crates/md_editor` now depend on `gpui` under `cfg(any(target_os = "macos", target_os = "linux"))`. `crates/app/Cargo.toml` also enables `gpui_platform` features `x11` and `wayland` for `target_os = "linux"` only. The workspace builds `gpui_linux` with default features off, so without them the Linux platform has no windowing backend and the app runs headless, with no window.
- `main.rs`: the hello window path is compiled for macOS and Linux. The fallback notice is compiled only on other platforms.
- Build: `cargo build -p app` succeeds on Linux (GPUI compiled, no API drift against the pinned rev).
- Run: under `xvfb-run` (display `:99`), the process opens an X11 window titled `noteapp`, 500x500, and renders `Hello, noteapp`. Screenshot: `docs/spikes/S2-linux-hello.png`.
- Checks: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test --workspace` pass.
- Not verified: Wayland (only the X11 path was run), and any real GPU driver (only llvmpipe was available).
