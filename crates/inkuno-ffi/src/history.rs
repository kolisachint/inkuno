//! Revisions: the saved versions of a note, their files, the diff between two of them, and the
//! Versions panel's labels. The file format is `inkuno_logic::revision`; the Swift side reads and
//! writes the files with `NSFileCoordinator`, and never saves a revision over another.

use inkuno_logic::revision::{self as rev, Format, Header, Kind};

/// Why a revision was made (see `inkuno_logic::revision::Kind`). `Auto` is an automatic version.
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionKind {
    Checkpoint,
    Auto,
}

impl From<Kind> for RevisionKind {
    fn from(kind: Kind) -> Self {
        match kind {
            Kind::Checkpoint => Self::Checkpoint,
            Kind::Auto => Self::Auto,
        }
    }
}

impl From<RevisionKind> for Kind {
    fn from(kind: RevisionKind) -> Self {
        match kind {
            RevisionKind::Checkpoint => Self::Checkpoint,
            RevisionKind::Auto => Self::Auto,
        }
    }
}

/// One saved version of a note: a complete copy of its text at that moment.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    /// 1-based.
    pub number: u32,
    /// The version it was saved after; none for the first.
    pub parent: Option<u32>,
    pub author: String,
    /// UTC, RFC 3339, to the second (`2026-11-14T10:32:00Z`).
    pub date: String,
    pub kind: RevisionKind,
    /// What changed or was decided.
    pub note: Option<String>,
    /// The complete note text, as it was.
    pub text: String,
}

/// The front matter of a revision file, without its text (what the Versions panel lists).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct RevisionHeader {
    pub number: u32,
    pub parent: Option<u32>,
    pub author: String,
    pub date: String,
    pub kind: RevisionKind,
    pub note: Option<String>,
}

impl From<Header> for RevisionHeader {
    fn from(head: Header) -> Self {
        Self {
            number: head.number,
            parent: head.parent,
            author: head.author,
            date: head.date,
            kind: head.kind.into(),
            note: head.note,
        }
    }
}

impl From<RevisionHeader> for Header {
    fn from(head: RevisionHeader) -> Self {
        Self {
            number: head.number,
            parent: head.parent,
            author: head.author,
            date: head.date,
            kind: head.kind.into(),
            note: head.note,
        }
    }
}

/// Why a revision file cannot be read (see `inkuno_logic::revision::Error`).
#[derive(uniffi::Error, Debug, Clone, PartialEq, Eq)]
pub enum RevisionError {
    /// The file does not open with a `---` line.
    NoHeader,
    /// The front matter is never closed by a `---` line.
    Unclosed,
    /// A field is missing or not valid.
    Field { name: String },
}

impl std::fmt::Display for RevisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoHeader => f.write_str("the file does not open with ---"),
            Self::Unclosed => f.write_str("the front matter is not closed by ---"),
            Self::Field { name } => write!(f, "the field `{name}` is missing or not valid"),
        }
    }
}

impl From<rev::Error> for RevisionError {
    fn from(error: rev::Error) -> Self {
        match error {
            rev::Error::NoHeader => Self::NoHeader,
            rev::Error::Unclosed => Self::Unclosed,
            rev::Error::Field(name) => Self::Field {
                name: name.to_string(),
            },
        }
    }
}

/// Why a version cannot be compared with the note's text (see `inkuno_logic::revision::compare`).
#[derive(uniffi::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevisionCompareError {
    /// The note's text is the version's: nothing to show.
    Unchanged,
    /// The version or the note holds pending changes: they are accepted or rejected first.
    Pending,
}

impl std::fmt::Display for RevisionCompareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unchanged => f.write_str("the note's text is the version's"),
            Self::Pending => {
                f.write_str("the note has pending changes: accept or reject them first")
            }
        }
    }
}

/// The way a diff is written (see `inkuno_logic::revision::Format`).
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevisionFormat {
    /// A unified diff, with three lines of context.
    Unified,
    /// CriticMarkup over the whole text: `{--removed--}` and `{++added++}`.
    Critic,
}

/// The file text of a revision: its front matter, then the text byte for byte.
#[uniffi::export]
pub fn revision_render(revision: Revision) -> String {
    rev::render(&rev::Revision {
        number: revision.number,
        parent: revision.parent,
        author: revision.author,
        date: revision.date,
        kind: revision.kind.into(),
        note: revision.note,
        text: revision.text,
    })
}

/// A revision from its file text (see `inkuno_logic::revision::parse`).
#[uniffi::export]
pub fn revision_parse(file: String) -> Result<Revision, RevisionError> {
    let parsed = rev::parse(&file)?;
    Ok(Revision {
        number: parsed.number,
        parent: parsed.parent,
        author: parsed.author,
        date: parsed.date,
        kind: parsed.kind.into(),
        note: parsed.note,
        text: parsed.text,
    })
}

/// The front matter of a revision file, without reading its text (see
/// `inkuno_logic::revision::parse_header`). A file cut after its front matter reads the same.
#[uniffi::export]
pub fn revision_parse_header(file: String) -> Result<RevisionHeader, RevisionError> {
    let (head, _) = rev::parse_header(&file)?;
    Ok(head.into())
}

/// The file name of revision `number`: `0001.md`, wider past 9999.
#[uniffi::export]
pub fn revision_file_name(number: u32) -> String {
    rev::file_name(number)
}

/// The revision number a file name stands for (`0003.md` is 3); `None` for any other name.
#[uniffi::export]
pub fn revision_number_of(name: String) -> Option<u32> {
    rev::number_of(&name)
}

/// The number of the next revision, after the numbers `saved` (`1` when there are none).
#[uniffi::export]
pub fn revision_next_number(saved: Vec<u32>) -> u32 {
    rev::next_number(saved)
}

/// What the Versions panel shows on a version's row (see `inkuno_logic::revision::row_name`):
/// `#12 · sachin`, then `· auto_word` for an automatic version, then `· note`.
#[uniffi::export]
pub fn revision_row_name(head: RevisionHeader, auto_word: String) -> String {
    rev::row_name(&head.into(), &auto_word)
}

/// A version's date as the Versions panel shows it, in the local time of `utc_offset_seconds`, to
/// the minute: `2026-11-14 11:32`. A date that is not RFC 3339 UTC is shown as it is.
#[uniffi::export]
pub fn revision_local_date(date: String, utc_offset_seconds: i64) -> String {
    rev::local_date(&date, |unix| rev::utc_parts(unix + utc_offset_seconds))
}

/// A Unix time as UTC RFC 3339 to the second: `2026-11-14T10:32:00Z`.
#[uniffi::export]
pub fn revision_date(unix: i64) -> String {
    rev::rfc3339(unix)
}

/// A date written as UTC RFC 3339 read back as Unix seconds; `None` for any other text.
#[uniffi::export]
pub fn revision_unix_of(date: String) -> Option<i64> {
    rev::unix_of(&date)
}

/// The diff from a version's text `old` to the note's text `new`, in `format` (see
/// `inkuno_logic::revision::diff`). `name`, `old_label` and `new_label` head a unified diff.
#[uniffi::export]
pub fn revision_diff(
    format: RevisionFormat,
    name: String,
    old_label: String,
    new_label: String,
    old: String,
    new: String,
) -> String {
    let format = match format {
        RevisionFormat::Unified => Format::Unified,
        RevisionFormat::Critic => Format::Critic,
    };
    rev::diff(format, &name, &old_label, &new_label, &old, &new)
}

/// The compare of a version's text with the note's text: the version's text, with each difference
/// written as a pending change, so that the editor draws it (see `inkuno_logic::revision::compare`).
#[uniffi::export]
pub fn revision_compare(version: String, now: String) -> Result<String, RevisionCompareError> {
    rev::compare(&version, &now).map_err(|error| match error {
        rev::CompareError::Unchanged => RevisionCompareError::Unchanged,
        rev::CompareError::Pending => RevisionCompareError::Pending,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Revision {
        Revision {
            number: 2,
            parent: Some(1),
            author: "sachin".into(),
            date: "2026-11-14T10:32:00Z".into(),
            kind: RevisionKind::Checkpoint,
            note: Some("dropped the plugin idea".into()),
            text: "# Plan\n\nbody\n".into(),
        }
    }

    #[test]
    fn a_revision_round_trips_through_its_file() {
        let file = revision_render(sample());
        assert!(file.starts_with("---\nrevision: 2\nparent: 1\nauthor: sachin\n"));
        assert!(file.contains("kind: checkpoint\nnote: dropped the plugin idea\n---\n# Plan"));
        assert_eq!(revision_parse(file.clone()).unwrap(), sample());
        let head = revision_parse_header(file).unwrap();
        assert_eq!(head.number, 2);
        assert_eq!(head.kind, RevisionKind::Checkpoint);
    }

    #[test]
    fn a_file_without_a_header_is_refused_with_why() {
        assert_eq!(
            revision_parse("# no header".into()),
            Err(RevisionError::NoHeader)
        );
        assert_eq!(
            revision_parse("---\nrevision: 1\n".into()),
            Err(RevisionError::Unclosed)
        );
        assert_eq!(
            revision_parse("---\nrevision: x\n---\n".into()),
            Err(RevisionError::Field {
                name: "revision".into()
            })
        );
        assert_eq!(
            RevisionError::Field {
                name: "kind".into()
            }
            .to_string(),
            "the field `kind` is missing or not valid"
        );
    }

    #[test]
    fn file_names_and_numbers() {
        assert_eq!(revision_file_name(3), "0003.md");
        assert_eq!(revision_file_name(12345), "12345.md");
        assert_eq!(revision_number_of("0003.md".into()), Some(3));
        assert_eq!(revision_number_of("notes.md".into()), None);
        assert_eq!(revision_number_of("0000.md".into()), None);
        assert_eq!(revision_next_number(vec![]), 1);
        assert_eq!(revision_next_number(vec![2, 5, 3]), 6);
    }

    #[test]
    fn the_row_and_the_local_date_read_like_the_panel() {
        let head = revision_parse_header(revision_render(Revision {
            kind: RevisionKind::Auto,
            note: None,
            ..sample()
        }))
        .unwrap();
        assert_eq!(
            revision_row_name(head.clone(), "auto".into()),
            "#2 · sachin · auto"
        );
        assert_eq!(
            revision_local_date(head.date.clone(), 3600),
            "2026-11-14 11:32"
        );
        assert_eq!(revision_local_date("2026-11-14".into(), 0), "2026-11-14");
    }

    #[test]
    fn dates_convert_both_ways() {
        let unix = revision_unix_of("2026-11-14T10:32:00Z".into()).unwrap();
        assert_eq!(revision_date(unix), "2026-11-14T10:32:00Z");
        assert_eq!(revision_unix_of("2026-02-30T00:00:00Z".into()), None);
        assert_eq!(revision_unix_of("yesterday".into()), None);
    }

    #[test]
    fn a_diff_is_unified_or_critic() {
        let unified = revision_diff(
            RevisionFormat::Unified,
            "plan.md".into(),
            "@2".into(),
            "@now".into(),
            "a\nb\n".into(),
            "a\nc\n".into(),
        );
        assert!(unified.contains("-b\n+c\n"));
        let critic = revision_diff(
            RevisionFormat::Critic,
            String::new(),
            String::new(),
            String::new(),
            "a\nb\n".into(),
            "a\nc\n".into(),
        );
        assert_eq!(critic, "a\n{--b--}\n{++c++}\n");
    }

    #[test]
    fn a_compare_says_why_it_has_nothing_to_show() {
        assert_eq!(
            revision_compare("same\n".into(), "same\n".into()),
            Err(RevisionCompareError::Unchanged)
        );
        assert!(revision_compare("old\n".into(), "new\n".into()).is_ok());
        assert_eq!(
            revision_compare("{++\nx\n++}\n".into(), "y\n".into()),
            Err(RevisionCompareError::Pending)
        );
    }
}
