# iOS app: handoff note

Status: **v1 feature-complete in development builds** (2026-11-24): every v1 screen is built, the
Rust core is linked, and the app runs on the owner's iPhone. Branch `ios-v1`, draft PR #23. Pick up
at "Open items" below.

- Prototype: [`prototypes/v3-ios.html`](prototypes/v3-ios.html), published at
  https://claude.ai/artifact/68n8SgCXhVCDZft9GCEbgh. The decision and complexity table is in
  [`prototypes/README.md`](prototypes/README.md) under "v3-ios".
- The owner approved the limited v1 scope on 2026-11-24.

## Decisions (owner)

| Area | Decision |
|--|--|
| Devices | iPhone only. iPad later. |
| Stack | Native SwiftUI app. `inkuno-logic` is the shared core, exposed to Swift with UniFFI. |
| Vault | A folder the user picks in the Files app (iCloud Drive, Dropbox, Working Copy…), kept with a security-scoped bookmark. The same folder as the desktop app. |
| Editor | Styled as you type, as on the desktop: only the line being edited shows its Markdown. An accessory bar sits above the keyboard: `/ # [[`, task, indent, outdent, undo, hide. |
| Home | The desktop empty-page cards (Today, New note, Search, "From a month ago"). The file tree is push navigation. No tab bar. |
| Commands | Pull down, or tap search, to open the palette with notes and `>` commands. Long-press menus replace right-click. |
| Right panel | One bottom sheet with segments: Outline, Comments, Versions, Backlinks. |
| Assistant | Deferred. It needs a local process, which iOS does not allow. |

## Decisions (build, owner-approved 2026-11-24)

| Area | Decision |
|--|--|
| Xcode project | `ios/` folder in this repo, generated with **xcodegen** from `ios/project.yml`. The `.xcodeproj` is NOT committed (gitignored). |
| New dependency | UniFFI approved (no issue needed). |
| App identity | Bundle id `com.inkuno.Inkuno`, iOS 17.0 minimum, iPhone only. |
| First builds | Development builds on the owner's own iPhone (TestFlight later). Simulator builds unsigned. The owner's team is a free Personal Team: device builds expire after 7 days, and `release-ios.yml` cannot export until a paid account. |
| Fonts | System fonts in v1 (Inter and JetBrains Mono are not bundled). |
| Scratch | As on the desktop: scratch notes live outside the vault, in the app's Application Support/Scratch. |
| CI | `ci-ios.yml` (build + test) and `release-ios.yml` (packaging). |
| Rust crate | `crates/inkuno-ffi` (lib name `inkuno_ffi`), UniFFI wrapper over the editor slice of `inkuno-logic`. Swift module of the generated bindings: `InkunoCore`. |
| Build script | `scripts/build-ios-core.sh [debug\|release]` builds both iOS targets, generates Swift bindings into `ios/Generated/` and the framework `ios/Frameworks/InkunoCore.xcframework` (both gitignored). |
| Xcode targets | App target + scheme `Inkuno` (sources in `ios/Inkuno/`), unit test target `InkunoTests` (tests in `ios/InkunoTests/`). |

## v1 scope

**Keep:** the Files-folder vault, the live editor, home, tree, Scratch, Today, palette, accessory
bar and slash sheet, links, backlinks, tags and `@date`, Outline, view and resolve comments
(add from a long-press only), version list, save and restore, the pending-changes review sheet,
rendered tables (horizontal scroll), context menus, Settings, Light, Dark and System themes,
Trash restore from the palette.

**Out of v1:** the Compare screen (restore asks for confirmation instead), inline pending-change
washes (pill and sheet only), table cell editing (raw Markdown on the live line), Mermaid and math
rendering (shown as code), Focus mode, tar.gz backup, the other 15 themes, the Assistant.

## What we checked

- `cargo check --target aarch64-apple-ios` and `aarch64-apple-ios-sim` pass for `inkuno-logic`;
  `inkuno-platform` passes for `aarch64-apple-ios` with one unused-import warning
  (`crates/inkuno-platform/src/lib.rs:15`). This was a type check only: linking needs Xcode on a Mac.
- UniFFI wrapper: **medium** effort for the slice the editor needs (markdown classify and blocks,
  lists, tables, dates, search and rank, revision, pending, merge). Notes:
  - Positions are `usize` and `Range<usize>`, which UniFFI cannot export. They need u64 adapters.
  - `wiki::backlinks` and `resolve` return `&Note`. They need a Rust-side vault handle that
    returns owned values.
  - Functions that take closures (`impl Fn`, `&dyn Fn`) need concrete replacements.
- `inkuno-platform` parts that do not fit iOS: `assistant.rs` (child process), `link.rs` (localhost
  door), `notify` watching, and config paths from HOME/XDG. File access in a Files-app folder also
  needs `NSFileCoordinator`.

## Highest risks (do these first)

1. **The live editor**: TextKit 2 styling driven by the Rust parser, with IME and undo. Build a
   throwaway spike first.
2. **The Files-folder vault**: coordinated reads and writes, iCloud files that are not downloaded
   yet, and conflict copies (`Note (conflict DATE).md`).

## Done

- `app-ui.md` §24 records the iPhone UI.
- `crates/inkuno-ffi` exports the editor slice (layout, style runs, lists, indent, dates, search)
  and the vault formats (`VaultIndex`, pending list, revisions, comments, trash names, merge).
- The app: vault picker with bookmark and coordinated IO, home, tree, Scratch, Today, the TextKit 2
  live editor with accessory bar and slash sheet, palette with `>` commands, the panel sheet,
  pending review, block menu with comments, Trash, Settings, icon and launch screen.
- `ci-ios.yml` runs the core tests and the simulator tests on GitHub (green on PR #23).
- Local device install: `scripts/ios-device.sh` (needs `ios/Local.xcconfig` with `DEVELOPMENT_TEAM`).

## Open items

1. Hand tests on the device: IME (dictation, CJK keyboards), long notes, iCloud placeholders and
   conflict copies, bookmark restore after relaunch.
2. Comment resolve and the byline split stay in Swift (they live in the desktop GUI crate).
3. The core has no date style bit: `@date` is found by a Swift scan.
4. The vault index rebuilds when the palette opens or Home refreshes; no file watching yet.
5. A version's author is "unknown" until Settings has an author field.
6. TestFlight once there is a paid developer account.
