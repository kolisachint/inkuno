//! Language: the interface text is written in English (`tr`), and how dates, times and
//! search keys are written. Used by every layer.

/// Interface text, in English. Every call still names both forms while the French wording
/// is being removed from the call sites; the second one is ignored.
pub fn tr(en: &'static str, _fr: &'static str) -> &'static str {
    en
}

/// English from now on, whatever the system's language: `inkuno mcp` talks to a model, and
/// the library's names and templates it returns are part of what the model reads.
pub fn speak_english() {}

/// Runs `f` unchanged: the interface is English everywhere, so there is nothing to force
/// anymore. Kept (ungated, unlike the old test-only helper) so the call sites that wrapped
/// their setup in it still compile: a dependency never builds with `cfg(test)`.
pub fn in_english<T>(f: impl FnOnce() -> T) -> T {
    f()
}

/// Why a disk operation failed, in English. An error we worded ourselves keeps its text; a system
/// one (`Permission denied (os error 13)`) is told by its kind, and the system's own words are only the last resort.
pub fn io_reason(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    if e.get_ref().is_some() {
        return e.to_string();
    }
    match e.kind() {
        PermissionDenied => "permission denied",
        NotFound => "not found",
        AlreadyExists => "this name is already taken",
        ReadOnlyFilesystem => "the disk is read-only",
        StorageFull => "the disk is full",
        InvalidFilename => "invalid name",
        NotADirectory => "not a folder",
        IsADirectory => "this is a folder",
        DirectoryNotEmpty => "the folder is not empty",
        _ => return e.to_string(),
    }
    .to_string()
}

/// A count with its thousands set apart: `1,000`.
pub fn number(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// How long ago, in the palette's short form: `today`, `3 d`, `2 wk`, `5 mo`, `1 y`.
/// `days` is the age in whole days.
pub fn age(days: u64) -> String {
    let (n, unit) = match days {
        0 => return "today".to_string(),
        1..=6 => (days, "d"),
        7..=29 => (days / 7, "wk"),
        30..=364 => (days / 30, "mo"),
        _ => (days / 365, "y"),
    };
    format!("{n} {unit}")
}

/// `09:05`: the time as written in a template.
pub fn time_name((hour, minute): (u32, u32)) -> String {
    format!("{hour:02}:{minute:02}")
}

/// `2026-10-09`: the date as written in a note and naming the daily note.
pub fn date_name((year, month, day): (i32, u32, u32)) -> String {
    format!("{year:04}-{month:02}-{day:02}")
}
/// Text reduced to what search compares: lowercase, without accents.
// ponytail: only French accents; switch to Unicode normalisation if another
// language arrives in `tr`.
pub fn fold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars().flat_map(char::to_lowercase) {
        match c {
            'à' | 'â' | 'ä' => out.push('a'),
            'é' | 'è' | 'ê' | 'ë' => out.push('e'),
            'î' | 'ï' => out.push('i'),
            'ô' | 'ö' => out.push('o'),
            'ù' | 'û' | 'ü' => out.push('u'),
            'ç' => out.push('c'),
            'œ' => out.push_str("oe"),
            '’' => out.push('\''),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_how_long_ago() {
        let said: Vec<String> = [0, 1, 6, 7, 29, 30, 364, 365, 729, 730, 100_000]
            .map(age)
            .into();
        assert_eq!(
            said,
            [
                "today", "1 d", "6 d", "1 wk", "4 wk", "1 mo", "12 mo", "1 y", "1 y", "2 y",
                "273 y"
            ]
        );
    }

    #[test]
    fn tells_why_a_disk_operation_failed() {
        use std::io::{Error, ErrorKind};
        // A real system error, whatever the system: a file that is not there.
        let os = std::fs::read(std::env::temp_dir().join("inkuno-absent-ici/rien")).unwrap_err();
        assert_eq!(io_reason(&os), "not found");
        assert!(!io_reason(&os).contains("os error"));
        for (kind, said) in [
            (ErrorKind::PermissionDenied, "permission denied"),
            (ErrorKind::NotFound, "not found"),
            (ErrorKind::StorageFull, "the disk is full"),
            (ErrorKind::ReadOnlyFilesystem, "the disk is read-only"),
        ] {
            assert_eq!(io_reason(&Error::from(kind)), said);
        }
        // Our own wording is kept; a kind we do not know falls back to the system's words.
        assert_eq!(
            io_reason(&Error::new(ErrorKind::AlreadyExists, "already taken")),
            "already taken"
        );
        assert_eq!(io_reason(&Error::other("")), "");
        assert!(!io_reason(&Error::from(ErrorKind::TimedOut)).is_empty());
    }
}
