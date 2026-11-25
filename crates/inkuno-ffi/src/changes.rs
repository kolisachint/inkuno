//! Pending changes: an agent's proposed text, shown in the note as marked areas until a person
//! accepts or rejects each one, and the list file beside the note that keeps them on disk.

use inkuno_logic::{
    pending::{self, Entry, Refusal},
    sidecar,
};

use crate::text::{byte_offset, offset_utf16};

/// One change of a note, kept beside it in its list file (see `sidecar_parse`). `at` is a hint: the
/// UTF-8 byte offset of the change in the note's own text, as the file writes it (not UTF-16).
/// `orphan_since` (seconds since the Unix epoch) is set for a change the text no longer places.
/// `created` is the UTC time it was proposed, when known, and `run` the store call that proposed it.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct PendingEntry {
    pub id: u32,
    pub author: String,
    pub at: u64,
    pub old: String,
    pub new: String,
    pub before: String,
    pub after: String,
    pub orphan_since: Option<u64>,
    /// UTC time the change was proposed (`2026-11-24T10:02Z`); feed order only. None when unknown.
    #[uniffi(default = None)]
    pub created: Option<String>,
    /// The store call that proposed the change (Unix milliseconds): the changes of one call share it,
    /// and the feed shows them as one card. None for a change of an older list.
    #[uniffi(default = None)]
    pub run: Option<u64>,
}

impl From<Entry> for PendingEntry {
    fn from(e: Entry) -> Self {
        Self {
            id: e.id,
            author: e.author,
            at: e.at as u64,
            old: e.old,
            new: e.new,
            before: e.before,
            after: e.after,
            orphan_since: e.orphan_since,
            created: e.created,
            run: e.run,
        }
    }
}

impl From<PendingEntry> for Entry {
    fn from(e: PendingEntry) -> Self {
        Self {
            id: e.id,
            author: e.author,
            at: e.at as usize,
            old: e.old,
            new: e.new,
            before: e.before,
            after: e.after,
            orphan_since: e.orphan_since,
            created: e.created,
            run: e.run,
        }
    }
}

/// One area of pending changes of a text: its whole (marker lines included), its removed lines and
/// its added lines, in UTF-16 of the text. A side with no lines is empty (`start == end`).
#[derive(uniffi::Record, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingArea {
    pub start_utf16: u64,
    pub end_utf16: u64,
    pub removed_start_utf16: u64,
    pub removed_end_utf16: u64,
    pub added_start_utf16: u64,
    pub added_end_utf16: u64,
}

/// The note's own text, and its changes: what `split_pending` gives from the text the editor shows.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct PendingSplit {
    pub base: String,
    pub entries: Vec<PendingEntry>,
}

/// Why a proposed change is refused (see `inkuno_logic::pending::propose`).
#[derive(uniffi::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProposeError {
    NothingChanged,
    AlreadyPending,
    HoldsMarks,
    /// A person changed the lines the agent changed since it read them. `line` is 1-based.
    HumanChanged {
        line: u64,
    },
}

impl ProposeError {
    fn refusal(self) -> Refusal {
        match self {
            Self::NothingChanged => Refusal::NothingChanged,
            Self::AlreadyPending => Refusal::AlreadyPending,
            Self::HoldsMarks => Refusal::HoldsMarks,
            Self::HumanChanged { line } => Refusal::HumanChanged {
                line: line as usize,
            },
        }
    }
}

impl std::fmt::Display for ProposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.refusal(), f)
    }
}

/// The areas of pending changes of `text`, in order (see `inkuno_logic::pending::areas`).
#[uniffi::export]
pub fn pending_areas(text: String) -> Vec<PendingArea> {
    pending::areas(&text)
        .into_iter()
        .map(|area| {
            let utf16 = |range: std::ops::Range<usize>| {
                (
                    offset_utf16(&text, range.start),
                    offset_utf16(&text, range.end),
                )
            };
            let (start, end) = utf16(area.range);
            let (removed_start, removed_end) = utf16(area.removed);
            let (added_start, added_end) = utf16(area.added);
            PendingArea {
                start_utf16: start,
                end_utf16: end,
                removed_start_utf16: removed_start,
                removed_end_utf16: removed_end,
                added_start_utf16: added_start,
                added_end_utf16: added_end,
            }
        })
        .collect()
}

/// Whether `text` holds a pending change (see `inkuno_logic::pending::has_pending`).
#[uniffi::export]
pub fn has_pending_changes(text: String) -> bool {
    pending::has_pending(&text)
}

/// The index of the area that holds the UTF-16 offset `at_utf16` of `text`, if any.
#[uniffi::export]
pub fn pending_area_at(text: String, at_utf16: u64) -> Option<u64> {
    pending::area_at(&text, byte_offset(&text, at_utf16)).map(|i| i as u64)
}

/// Whether replacing the UTF-16 range `start_utf16..end_utf16` of `text` by `inserted` is refused
/// because it would change a pending change (see `inkuno_logic::pending::refuses`).
#[uniffi::export]
pub fn pending_edit_refused(
    text: String,
    start_utf16: u64,
    end_utf16: u64,
    inserted: String,
) -> bool {
    let range = byte_offset(&text, start_utf16)..byte_offset(&text, end_utf16);
    pending::refuses(&text, range, &inserted)
}

/// Area `index` accepted: its removed lines go, its added lines stay (see `pending::accept`).
#[uniffi::export]
pub fn accept_change(text: String, index: u64) -> String {
    pending::accept(&text, index as usize)
}

/// Area `index` rejected: its added lines go, its removed lines stay (see `pending::reject`).
#[uniffi::export]
pub fn reject_change(text: String, index: u64) -> String {
    pending::reject(&text, index as usize)
}

/// Every area accepted: the agent's text.
#[uniffi::export]
pub fn accept_all_changes(text: String) -> String {
    pending::accept_all(&text)
}

/// Every area rejected: the person's text.
#[uniffi::export]
pub fn reject_all_changes(text: String) -> String {
    pending::reject_all(&text)
}

/// The text the editor shows, split into the note's own text and its changes, as the list file
/// keeps them (see `inkuno_logic::pending::split`). `known`: the entries read from the list file.
#[uniffi::export]
pub fn split_pending(display: String, known: Vec<PendingEntry>) -> PendingSplit {
    let known: Vec<Entry> = known.into_iter().map(Entry::from).collect();
    let (base, entries) = pending::split(&display, &known);
    PendingSplit {
        base,
        entries: entries.into_iter().map(PendingEntry::from).collect(),
    }
}

/// The text the editor shows: `base` with the entries written as marked areas (see
/// `inkuno_logic::pending::compose`). An entry that cannot be placed is left out.
#[uniffi::export]
pub fn compose_pending(base: String, entries: Vec<PendingEntry>) -> String {
    pending::compose(&base, &entries_of(entries))
}

/// Whether each entry is placed in `base` (see `inkuno_logic::pending::placed`); an entry that is
/// not is an orphan.
#[uniffi::export]
pub fn placed_pending(base: String, entries: Vec<PendingEntry>) -> Vec<bool> {
    pending::placed(&base, &entries_of(entries))
}

/// A fragment as it is copied: the marks and the removed lines go, the added lines stay (see
/// `inkuno_logic::pending::plain`).
#[uniffi::export]
pub fn plain_pending_fragment(fragment: String) -> String {
    pending::plain(&fragment)
}

/// The text of a note with an agent's change written as pending areas on top of `current` (see
/// `inkuno_logic::pending::propose`). `base` is the text the agent read, `proposed` its new text.
#[uniffi::export]
pub fn propose_change(
    base: String,
    current: String,
    proposed: String,
) -> Result<String, ProposeError> {
    pending::propose(&base, &current, &proposed).map_err(|refusal| match refusal {
        Refusal::NothingChanged => ProposeError::NothingChanged,
        Refusal::AlreadyPending => ProposeError::AlreadyPending,
        Refusal::HoldsMarks => ProposeError::HoldsMarks,
        Refusal::HumanChanged { line } => ProposeError::HumanChanged { line: line as u64 },
    })
}

/// The changes of a list file's text (version 2, or 1). A text that is not a list reads as none
/// (see `inkuno_logic::sidecar::parse`).
#[uniffi::export]
pub fn sidecar_parse(json: String) -> Vec<PendingEntry> {
    sidecar::parse(&json)
        .into_iter()
        .map(PendingEntry::from)
        .collect()
}

/// The text of a list file for `entries`, with the line break that ends it (see
/// `inkuno_logic::sidecar::render`). The Swift side writes it, or removes the file when there are
/// no entries.
#[uniffi::export]
pub fn sidecar_render(entries: Vec<PendingEntry>) -> String {
    sidecar::render(&entries_of(entries))
}

fn entries_of(entries: Vec<PendingEntry>) -> Vec<Entry> {
    entries.into_iter().map(Entry::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const MARKED: &str = "keep\n{--\nold line\n--}\n{++\nnew line\n++}\nend\n";

    fn entry(id: u32, old: &str, new: &str) -> PendingEntry {
        PendingEntry {
            id,
            author: "agent".into(),
            at: 0,
            old: old.into(),
            new: new.into(),
            before: String::new(),
            after: String::new(),
            orphan_since: None,
            created: None,
            run: None,
        }
    }

    #[test]
    fn an_area_is_reported_in_utf16_with_its_sides() {
        let text = "é\n{--\nold\n--}\n{++\nnew\n++}\n";
        let found = pending_areas(text.into());
        assert_eq!(found.len(), 1);
        let area = found[0];
        assert_eq!(area.start_utf16, 2);
        assert_eq!(area.end_utf16, text.encode_utf16().count() as u64);
        // "é\n{--\n" is 6 units, "old\n" 4, "--}\n{++\n" 8, "new\n" 4, "++}\n" 4.
        assert_eq!((area.removed_start_utf16, area.removed_end_utf16), (6, 10));
        assert_eq!((area.added_start_utf16, area.added_end_utf16), (18, 22));
    }

    #[test]
    fn accept_and_reject_settle_one_area_or_all() {
        assert_eq!(accept_change(MARKED.into(), 0), "keep\nnew line\nend\n");
        assert_eq!(reject_change(MARKED.into(), 0), "keep\nold line\nend\n");
        assert_eq!(accept_all_changes(MARKED.into()), "keep\nnew line\nend\n");
        assert_eq!(reject_all_changes(MARKED.into()), "keep\nold line\nend\n");
        // An index past the areas changes nothing.
        assert_eq!(accept_change(MARKED.into(), 7), MARKED);
        assert!(has_pending_changes(MARKED.into()));
        assert!(!has_pending_changes("plain\n".into()));
    }

    #[test]
    fn an_edit_inside_an_area_is_refused_and_one_outside_is_not() {
        let at = |needle: &str| {
            let byte = MARKED.find(needle).unwrap();
            offset_utf16(MARKED, byte)
        };
        let start = at("old line");
        assert!(pending_edit_refused(
            MARKED.into(),
            start,
            start,
            "x".into()
        ));
        let keep = at("keep");
        assert!(!pending_edit_refused(MARKED.into(), keep, keep, "x".into()));
        assert_eq!(pending_area_at(MARKED.into(), start), Some(0));
        assert_eq!(pending_area_at(MARKED.into(), keep), None);
    }

    #[test]
    fn split_and_compose_are_inverse_for_a_text_with_changes() {
        let split = split_pending(MARKED.into(), vec![]);
        assert_eq!(split.base, "keep\nold line\nend\n");
        assert_eq!(split.entries.len(), 1);
        assert_eq!(split.entries[0].id, 1);
        assert_eq!(split.entries[0].old, "old line\n");
        assert_eq!(split.entries[0].new, "new line\n");
        assert_eq!(compose_pending(split.base, split.entries), MARKED);
    }

    #[test]
    fn a_known_entry_keeps_its_id_and_author() {
        let mut known = entry(7, "old line\n", "new line\n");
        known.author = "Claude".into();
        let split = split_pending(MARKED.into(), vec![known]);
        assert_eq!(split.entries[0].id, 7);
        assert_eq!(split.entries[0].author, "Claude");
    }

    #[test]
    fn an_entry_whose_text_is_gone_is_an_orphan() {
        let found = placed_pending("unrelated\n".into(), vec![entry(1, "gone\n", "x\n")]);
        assert_eq!(found, [false]);
        let found = placed_pending(
            "keep\nold line\nend\n".into(),
            vec![entry(1, "old line\n", "new line\n")],
        );
        assert_eq!(found, [true]);
    }

    #[test]
    fn a_proposal_is_written_as_areas_or_refused() {
        let base = "one\ntwo\n";
        let proposed = "one\n2\n";
        let written = propose_change(base.into(), base.into(), proposed.into()).unwrap();
        assert!(has_pending_changes(written.clone()));
        assert_eq!(accept_all_changes(written), proposed);
        assert_eq!(
            propose_change(base.into(), base.into(), base.into()),
            Err(ProposeError::NothingChanged)
        );
        assert_eq!(
            propose_change(base.into(), MARKED.into(), proposed.into()),
            Err(ProposeError::AlreadyPending)
        );
        assert_eq!(
            propose_change("one\n".into(), "one\n".into(), "{--\n".into()),
            Err(ProposeError::HoldsMarks)
        );
        let human = "uno\ntwo\n";
        let refused = propose_change(base.into(), human.into(), "one\n2\n".into());
        assert_eq!(refused, Err(ProposeError::HumanChanged { line: 1 }));
        assert!(refused.unwrap_err().to_string().contains("line 1"));
    }

    #[test]
    fn a_list_file_round_trips_through_the_bridge() {
        let entries = vec![entry(1, "a\n", "b\n")];
        let text = sidecar_render(entries.clone());
        assert_eq!(sidecar_parse(text), entries);
        assert!(sidecar_parse("nope".into()).is_empty());
    }

    #[test]
    fn a_fragment_copied_keeps_only_what_the_note_will_hold() {
        assert_eq!(
            plain_pending_fragment(MARKED.into()),
            "keep\nnew line\nend\n"
        );
    }
}
