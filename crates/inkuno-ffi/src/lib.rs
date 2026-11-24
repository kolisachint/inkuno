//! The editor slice of `inkuno-logic`, exported to Swift through UniFFI (proc-macro mode).
//!
//! The iOS app (`ios/`) links `InkunoCore.xcframework` built from this crate and imports the Swift
//! module `InkunoCore` generated into `ios/Generated/` (`scripts/build-ios-core.sh`).
//!
//! Positions: every offset that crosses the FFI is a `u64`. Records carry UTF-16 offsets
//! (`*_utf16`), the unit UITextView counts in; `utf8_to_utf16` and `utf16_to_utf8` convert the
//! rest. No function here touches the disk, a window or a clock: the dates are passed in.

mod changes;
mod editing;
mod files;
mod lines;
mod notes;
mod search;
mod text;

pub use editing::{EnterOutcome, TextEdit};
pub use lines::{
    DocumentLayout, HeadingInfo, LineInfo, LineKind, LineLayout, LinkInfo, LinkKind, NoteIndex,
    StyleBits, StyleRun,
};
pub use notes::{DateSuggestion, Hms, Ymd};
pub use search::SnippetLine;

uniffi::setup_scaffolding!();
