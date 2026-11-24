# iOS app: handoff note

Status: **design agreed, nothing built yet** (2026-11-24). Pick up here.

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

## Next steps

1. Write the iOS decisions into `app-ui.md` as their own section, doc-first as the process asks.
2. Open an issue for the iOS app. New dependencies (UniFFI) need agreement in the issue
   (CONTRIBUTING.md). Decide where the Xcode project lives: an `ios/` folder here, or its own repository.
3. Add a small `inkuno-ffi` crate (UniFFI) over the editor slice of `inkuno-logic`.
4. Editor spike in Xcode. It needs a Mac: attach it to the session.
5. Vault spike: pick a folder, keep the bookmark, then read, write and watch with file coordination.
