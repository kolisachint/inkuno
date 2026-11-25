//! Merging a change another program made into the text being typed (see `inkuno_logic::merge`).

use inkuno_logic::merge;

use crate::text::offset_utf16;

/// A replacement in the text being typed (`ours`): the UTF-16 range `start_utf16..end_utf16` of
/// `ours` (the text before any edit) takes `replacement`.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct MergeEdit {
    pub start_utf16: u64,
    pub end_utf16: u64,
    pub replacement: String,
}

/// What `theirs` changed since `base`, as replacements of `ours`, so the editor applies them
/// without losing a keystroke. The edits are in order and do not overlap; all their ranges are in
/// `ours` as it was, so apply them from the last to the first. `None` when both sides changed the
/// same lines differently: the caller keeps both texts. Lines are the unit, as in git.
#[uniffi::export]
pub fn merge_onto(base: String, ours: String, theirs: String) -> Option<Vec<MergeEdit>> {
    let edits = merge::onto(&base, &ours, &theirs)?;
    Some(
        edits
            .into_iter()
            .map(|(range, replacement)| MergeEdit {
                start_utf16: offset_utf16(&ours, range.start),
                end_utf16: offset_utf16(&ours, range.end),
                replacement,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Applies the edits from the last to the first, on UTF-16 offsets, as the Swift side does.
    fn apply(text: &str, edits: &[MergeEdit]) -> String {
        let mut units: Vec<u16> = text.encode_utf16().collect();
        for edit in edits.iter().rev() {
            let replacement: Vec<u16> = edit.replacement.encode_utf16().collect();
            units.splice(
                edit.start_utf16 as usize..edit.end_utf16 as usize,
                replacement,
            );
        }
        String::from_utf16(&units).unwrap()
    }

    #[test]
    fn a_change_on_another_line_is_applied_to_ours() {
        let base = "one\ntwo\nthree\n";
        let ours = "one\ntwo typed\nthree\n";
        let theirs = "one\ntwo\nthree agent\n";
        let edits = merge_onto(base.into(), ours.into(), theirs.into()).unwrap();
        assert_eq!(apply(ours, &edits), "one\ntwo typed\nthree agent\n");
    }

    #[test]
    fn the_same_line_changed_on_both_sides_is_a_clash() {
        let base = "one\ntwo\n";
        assert_eq!(
            merge_onto(base.into(), "one\nmine\n".into(), "one\ntheirs\n".into()),
            None
        );
    }

    #[test]
    fn a_change_after_a_multibyte_line_is_placed_in_utf16() {
        let base = "é 😀\nend\n";
        let ours = "é 😀 typed\nend\n";
        let theirs = "é 😀\nend agent\n";
        let edits = merge_onto(base.into(), ours.into(), theirs.into()).unwrap();
        assert_eq!(apply(ours, &edits), "é 😀 typed\nend agent\n");
    }
}
