//! The user guide, built into the binary: the pages `inkuno guide` prints, from `docs/user/src`.

/// The pages of the guide, by name, in the order of the book.
const PAGES: &[(&str, &str)] = &[
    (
        "introduction",
        include_str!("../../../../docs/user/src/introduction.md"),
    ),
    (
        "install",
        include_str!("../../../../docs/user/src/install.md"),
    ),
    (
        "first-steps",
        include_str!("../../../../docs/user/src/first-steps.md"),
    ),
    (
        "writing",
        include_str!("../../../../docs/user/src/writing.md"),
    ),
    ("links", include_str!("../../../../docs/user/src/links.md")),
    (
        "comments",
        include_str!("../../../../docs/user/src/comments.md"),
    ),
    (
        "daily-notes",
        include_str!("../../../../docs/user/src/daily-notes.md"),
    ),
    (
        "navigation",
        include_str!("../../../../docs/user/src/navigation.md"),
    ),
    (
        "tabs-and-panes",
        include_str!("../../../../docs/user/src/tabs-and-panes.md"),
    ),
    (
        "search",
        include_str!("../../../../docs/user/src/search.md"),
    ),
    ("vault", include_str!("../../../../docs/user/src/vault.md")),
    ("sync", include_str!("../../../../docs/user/src/sync.md")),
    (
        "settings",
        include_str!("../../../../docs/user/src/settings.md"),
    ),
    (
        "updates",
        include_str!("../../../../docs/user/src/updates.md"),
    ),
    (
        "shortcuts",
        include_str!("../../../../docs/user/src/shortcuts.md"),
    ),
    (
        "compatibility",
        include_str!("../../../../docs/user/src/compatibility.md"),
    ),
    (
        "troubleshooting",
        include_str!("../../../../docs/user/src/troubleshooting.md"),
    ),
    (
        "performance",
        include_str!("../../../../docs/user/src/performance.md"),
    ),
    (
        "agents",
        include_str!("../../../../docs/user/src/agents.md"),
    ),
];

/// The names of the pages, in the order of the book.
pub fn names() -> Vec<&'static str> {
    PAGES.iter().map(|(name, _)| *name).collect()
}

/// The Markdown of one page, by its name.
pub fn page(name: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, text)| *text)
}
