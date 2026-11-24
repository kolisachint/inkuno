//! What a `[[link]]` means in the vault: its target, where a heading or a block starts in its
//! note, the block ids to offer after `^`, and the links a rename must rewrite.

use std::path::{Path, PathBuf};

use inkuno_logic::wiki::{self, Anchor};

use crate::text::offset_utf16;

/// What a link points at inside its note (see `inkuno_logic::wiki::Anchor`).
#[derive(uniffi::Enum, Clone, Debug, PartialEq, Eq)]
pub enum LinkAnchor {
    /// The whole note, without its front matter.
    Whole,
    /// A heading (`[[Note#Heading]]`), by its title.
    Heading { title: String },
    /// A block marked with `^id` (`[[Note#^id]]`).
    Block { id: String },
}

impl From<LinkAnchor> for Anchor {
    fn from(anchor: LinkAnchor) -> Self {
        match anchor {
            LinkAnchor::Whole => Anchor::Whole,
            LinkAnchor::Heading { title } => Anchor::Heading(title),
            LinkAnchor::Block { id } => Anchor::Block(id),
        }
    }
}

impl From<Anchor> for LinkAnchor {
    fn from(anchor: Anchor) -> Self {
        match anchor {
            Anchor::Whole => Self::Whole,
            Anchor::Heading(title) => Self::Heading { title },
            Anchor::Block(id) => Self::Block { id },
        }
    }
}

/// The note a `[[…]]` names, and what of it (see `inkuno_logic::wiki::Target`).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct LinkTarget {
    pub note: String,
    pub anchor: LinkAnchor,
}

/// What `[[…]]` points at, from what is between the brackets: `Note`, `Note#Heading`, `Note#^id`;
/// an alias after `|` is left out. `None` for an empty name or a nested bracket.
#[uniffi::export]
pub fn link_target(inner: String) -> Option<LinkTarget> {
    let target = wiki::link_target(&inner)?;
    Some(LinkTarget {
        note: target.note,
        anchor: target.anchor.into(),
    })
}

/// The UTF-16 offset where the anchor starts in `text`: the heading, the block, or the top of the
/// note after its front matter (see `inkuno_logic::wiki::position`).
#[uniffi::export]
pub fn link_position(text: String, anchor: LinkAnchor) -> Option<u64> {
    wiki::position(&text, &anchor.into()).map(|at| offset_utf16(&text, at))
}

/// What an embed of the anchor shows of `text`: the section, without the front matter, the blank
/// lines around it and the `^id` of a block (see `inkuno_logic::wiki::section`).
#[uniffi::export]
pub fn link_section(text: String, anchor: LinkAnchor) -> Option<String> {
    wiki::section(&text, &anchor.into())
}

/// The ids of the blocks of `text` (`^id` at the end of a line), in order, to offer after `^`.
#[uniffi::export]
pub fn block_ids(text: String) -> Vec<String> {
    wiki::block_ids(&text)
        .into_iter()
        .map(String::from)
        .collect()
}

/// The text of note `linker` with its links rewritten after a rename, or `None` when none changes
/// (see `inkuno_logic::wiki::rewrite_links`). `old` is renamed `new`: its `[[links]]` when `wiki`.
/// When the note moved from `moved_from` to `moved_to` (both vault-relative paths), its relative
/// Markdown addresses are rewritten too. Paths are vault-relative, `/` separated.
#[uniffi::export]
pub fn rewrite_links(
    text: String,
    linker: String,
    old: String,
    new: String,
    wiki: bool,
    moved_from: Option<String>,
    moved_to: Option<String>,
) -> Option<String> {
    // The vault root is the empty path: vault-relative paths resolve from it as the desktop's do
    // from its root.
    let root = Path::new("");
    let moved = match (moved_from, moved_to) {
        (Some(from), Some(to)) => Some((PathBuf::from(from), PathBuf::from(to))),
        _ => None,
    };
    wiki::rewrite_links(
        &text,
        Some(root),
        Path::new(&linker),
        &old,
        &new,
        wiki,
        moved
            .as_ref()
            .map(|(from, to)| (from.as_path(), to.as_path())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_target_names_its_note_and_anchor() {
        assert_eq!(
            link_target("Plan".into()),
            Some(LinkTarget {
                note: "Plan".into(),
                anchor: LinkAnchor::Whole
            })
        );
        assert_eq!(
            link_target("Plan#Budget|alias".into()),
            Some(LinkTarget {
                note: "Plan".into(),
                anchor: LinkAnchor::Heading {
                    title: "Budget".into()
                }
            })
        );
        assert_eq!(
            link_target("Plan#^b1".into()).unwrap().anchor,
            LinkAnchor::Block { id: "b1".into() }
        );
        assert_eq!(link_target("|alias".into()), None);
    }

    #[test]
    fn a_heading_position_is_in_utf16() {
        let text = "é 😀\n# Budget\nrest";
        let at = link_position(
            text.into(),
            LinkAnchor::Heading {
                title: "Budget".into(),
            },
        );
        // "é 😀\n" is 5 units.
        assert_eq!(at, Some(5));
        assert_eq!(
            link_position(text.into(), LinkAnchor::Heading { title: "No".into() }),
            None
        );
    }

    #[test]
    fn a_section_stops_at_the_next_heading_of_its_level() {
        let text = "# A\nbody\n## Sub\ninside\n# B\nafter";
        assert_eq!(
            link_section(text.into(), LinkAnchor::Heading { title: "A".into() }).as_deref(),
            Some("# A\nbody\n## Sub\ninside")
        );
    }

    #[test]
    fn block_ids_are_listed_in_order() {
        assert_eq!(
            block_ids("one ^first\ntwo\nthree ^second\n".into()),
            ["first", "second"]
        );
    }

    #[test]
    fn a_rename_rewrites_the_wikilinks_of_a_note() {
        let text = "See [[Old]] and [[old|alias]] and [[Other]].";
        assert_eq!(
            rewrite_links(
                text.into(),
                "Journal.md".into(),
                "Old".into(),
                "New".into(),
                true,
                None,
                None
            )
            .as_deref(),
            Some("See [[New]] and [[New|alias]] and [[Other]].")
        );
        assert_eq!(
            rewrite_links(
                text.into(),
                "Journal.md".into(),
                "Old".into(),
                "New".into(),
                false,
                None,
                None
            ),
            None
        );
    }

    #[test]
    fn a_moved_note_keeps_the_spelling_of_its_relative_links() {
        let text = "[plan](ideas/Plan.md) and [home](../Home.md)";
        // Written from the vault's root: `ideas/Plan.md` is the note that moves.
        let moved = rewrite_links(
            text.into(),
            "Journal.md".into(),
            String::new(),
            String::new(),
            false,
            Some("ideas/Plan.md".into()),
            Some("archive/Plan.md".into()),
        )
        .unwrap();
        assert_eq!(moved, "[plan](archive/Plan.md) and [home](../Home.md)");
    }
}
