//! Where the desktop keeps what goes with a note: its pending-changes list, its revision history
//! and the trash. Paths are relative to the vault folder (or to the scratch folder), `/` separated;
//! the Swift side does the file IO with `NSFileCoordinator`. `None` for a path that the desktop
//! would not map (empty, absolute, or climbing with `..`).

use inkuno_logic::trash;

/// The parts of a vault-relative path: `.` parts and empty parts are dropped, `..` refused.
fn parts_of(rel: &str) -> Option<Vec<&str>> {
    if rel.starts_with('/') {
        return None;
    }
    let mut parts = Vec::new();
    for part in rel.split('/') {
        match part {
            "" | "." => {}
            ".." => return None,
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then_some(parts)
}

/// A single file name of a trash folder: one part, neither empty nor `.`/`..`.
fn name_of(name: &str) -> Option<&str> {
    match parts_of(name)?.as_slice() {
        [one] => Some(one),
        _ => None,
    }
}

/// The folder of the revisions of a vault note (or of a folder of notes): the same path under
/// `.inkuno/revisions/`, so `ideas/plan.md` is `.inkuno/revisions/ideas/plan.md`.
#[uniffi::export]
pub fn revision_folder(note_path: String) -> Option<String> {
    Some(format!(
        ".inkuno/revisions/{}",
        parts_of(&note_path)?.join("/")
    ))
}

/// The folder of the revisions of a scratch note, named `name`: `.revisions/<name>`.
#[uniffi::export]
pub fn scratch_revision_folder(name: String) -> Option<String> {
    Some(format!(".revisions/{}", name_of(&name)?))
}

/// The pending-changes list of a vault note: `.inkuno/pending/<note path>.json`. A scratch note's
/// list is the same with its file name alone (its path is then just the name).
#[uniffi::export]
pub fn pending_list_path(note_path: String) -> Option<String> {
    Some(format!(
        ".inkuno/pending/{}.json",
        parts_of(&note_path)?.join("/")
    ))
}

/// The bin of a trashed note's pending-changes list: `.inkuno/pending-trash/<trashed name>.json`,
/// where `trashed name` is the name the item got in the trash (see `trash_name`).
#[uniffi::export]
pub fn trashed_pending_list_path(trashed_name: String) -> Option<String> {
    Some(format!(
        ".inkuno/pending-trash/{}.json",
        name_of(&trashed_name)?
    ))
}

/// The vault's trash, where deleted notes and folders go: `.trash` at the root.
#[uniffi::export]
pub fn vault_trash_folder() -> String {
    ".trash".to_string()
}

/// The trash of a vault's revision histories: `.inkuno/trash`, under the name the item got in
/// `.trash`.
#[uniffi::export]
pub fn vault_revision_bin() -> String {
    ".inkuno/trash".to_string()
}

/// The trash of a scratch folder: `.trash` in the scratch folder.
#[uniffi::export]
pub fn scratch_trash_folder() -> String {
    ".trash".to_string()
}

/// The trash of the scratch folder's revision histories: `.revisions/.trash`.
#[uniffi::export]
pub fn scratch_revision_bin() -> String {
    ".revisions/.trash".to_string()
}

/// The name an item gets in a trash folder: `name` if no item of that name is there, else `2 name`,
/// `3 name`… (the first free one). `taken` lists the names in the folder. Names are compared without
/// case, as macOS and Windows file systems do, so that nothing is replaced on either.
#[uniffi::export]
pub fn trash_name(name: String, taken: Vec<String>) -> String {
    let taken: Vec<String> = taken.iter().map(|t| t.to_lowercase()).collect();
    // ponytail: the names are read from a listing the caller made, not from the disk: a name that
    // appears between that listing and the move is not seen. The Swift side moves with
    // NSFileCoordinator, which makes the move itself refuse to replace an existing item.
    trash::name(&name, |candidate| taken.contains(&candidate.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_revision_folder_mirrors_the_note_path() {
        assert_eq!(
            revision_folder("ideas/plan.md".into()).as_deref(),
            Some(".inkuno/revisions/ideas/plan.md")
        );
        assert_eq!(
            revision_folder("./plan.md".into()).as_deref(),
            Some(".inkuno/revisions/plan.md")
        );
        assert_eq!(revision_folder("../plan.md".into()), None);
        assert_eq!(revision_folder("/plan.md".into()), None);
        assert_eq!(revision_folder(String::new()), None);
        assert_eq!(revision_folder(".".into()), None);
    }

    #[test]
    fn a_scratch_note_has_its_own_folder_and_list() {
        assert_eq!(
            scratch_revision_folder("2026-11-24-103200.md".into()).as_deref(),
            Some(".revisions/2026-11-24-103200.md")
        );
        assert_eq!(scratch_revision_folder("a/b.md".into()), None);
        assert_eq!(
            pending_list_path("2026-11-24-103200.md".into()).as_deref(),
            Some(".inkuno/pending/2026-11-24-103200.md.json")
        );
    }

    #[test]
    fn the_pending_list_mirrors_the_note_path() {
        assert_eq!(
            pending_list_path("ideas/plan.md".into()).as_deref(),
            Some(".inkuno/pending/ideas/plan.md.json")
        );
        assert_eq!(pending_list_path("../x.md".into()), None);
        assert_eq!(
            trashed_pending_list_path("2 plan.md".into()).as_deref(),
            Some(".inkuno/pending-trash/2 plan.md.json")
        );
        assert_eq!(trashed_pending_list_path("a/plan.md".into()), None);
    }

    #[test]
    fn the_bins_are_where_the_desktop_keeps_them() {
        assert_eq!(vault_trash_folder(), ".trash");
        assert_eq!(vault_revision_bin(), ".inkuno/trash");
        assert_eq!(scratch_trash_folder(), ".trash");
        assert_eq!(scratch_revision_bin(), ".revisions/.trash");
    }

    #[test]
    fn a_trashed_name_is_numbered_past_the_taken_ones() {
        assert_eq!(trash_name("plan.md".into(), vec![]), "plan.md");
        assert_eq!(
            trash_name("plan.md".into(), vec!["Plan.md".into()]),
            "2 plan.md"
        );
        assert_eq!(
            trash_name(
                "plan.md".into(),
                vec!["plan.md".into(), "2 plan.md".into(), "other".into()]
            ),
            "3 plan.md"
        );
    }
}
