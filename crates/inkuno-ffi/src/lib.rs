//! The editor and vault logic of `inkuno-logic`, exported to Swift through UniFFI (proc-macro mode).
//!
//! The iOS app (`ios/`) links `InkunoCore.xcframework` built from this crate and imports the Swift
//! module `InkunoCore` generated into `ios/Generated/` (`scripts/build-ios-core.sh`).
//!
//! Positions: every offset that crosses the FFI is a `u64`. Records carry UTF-16 offsets
//! (`*_utf16`), the unit UITextView counts in; `utf8_to_utf16` and `utf16_to_utf8` convert the
//! rest. No function here touches the disk, a window or a clock: the dates are passed in, and the
//! Swift side reads and writes the vault's files itself (with `NSFileCoordinator`). The desktop's
//! file formats are here as pure functions: the pending-changes list (`changes`), the revisions
//! (`history`), the paths and trash names (`files`), the links (`wiki`), the comments (`comments`),
//! the merge of an outside change (`merging`), and the vault index with its search (`index`).

mod changes;
mod comments;
mod editing;
mod files;
mod history;
mod index;
mod lines;
mod merging;
mod notes;
mod search;
mod text;
mod wiki;

pub use changes::{PendingArea, PendingEntry, PendingSplit, ProposeError};
pub use comments::{CommentError, CommentInfo};
pub use editing::{EnterOutcome, TextEdit};
pub use history::{
    Revision, RevisionCompareError, RevisionError, RevisionFormat, RevisionHeader, RevisionKind,
};
pub use index::{NoteInput, NoteRef, RecalledNote, SearchHit, SearchKind, VaultIndex};
pub use lines::{
    DocumentLayout, HeadingInfo, LineInfo, LineKind, LineLayout, LinkInfo, LinkKind, NoteIndex,
    StyleBits, StyleRun,
};
pub use merging::MergeEdit;
pub use notes::{DateSuggestion, Hms, Ymd};
pub use search::SnippetLine;
pub use wiki::{LinkAnchor, LinkTarget};

uniffi::setup_scaffolding!();
