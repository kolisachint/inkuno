# noteapp

noteapp is a Markdown notes editor for plain `.md` files kept in folders. It is written in Rust on GPUI, the UI framework from Zed. The design goal is a Typora-style live preview, with Markdown kept as plain text on disk.

**Current state: iteration 1 is done on Linux, with macOS verification pending.** Iteration 1 is a plain-markdown editor: it opens, edits, selects, navigates, copies and pastes, undoes and redoes, scrolls and saves one `.md` file. IME input is implemented; its macOS check is pending. Live preview, code fences, math, Mermaid, modes and the workspace sidebar come later (iteration 2; math and Mermaid are on hold).

**Download:** nightly builds for macOS (arm64) and Linux (x86_64) are on the [`nightly` GitHub release](https://github.com/kolisachint/noteapp/releases/tag/nightly). The macOS build is ad-hoc signed, so run `xattr -dr com.apple.quarantine noteapp.app` after unzipping.

**Build:** `cargo run -p app [file.md]`. The toolchain is pinned in `rust-toolchain.toml`. On Linux, first install the apt packages listed in `.github/workflows/release.yml`.

**Plans:** [iteration 1](docs/IMPLEMENTATION_PLAN.md), [iteration 2](docs/IMPLEMENTATION_PLAN_ITERATION_2.md), [design](docs/DESIGN.md), and [agent notes](AGENTS.md).

Licence: GPL-3.0-or-later (see [LICENSE](LICENSE)).
