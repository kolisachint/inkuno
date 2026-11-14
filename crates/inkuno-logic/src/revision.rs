//! Revisions of a note: explicit versions of its text, saved by a person or an agent. A revision is
//! a complete copy of the note at that moment, in a Markdown file that opens with a front matter
//! block Inkuno writes. Readable by people and by agents as plain files; the diff between two
//! revisions is derived from them, never stored.
//!
//! ```text
//! ---
//! revision: 3
//! parent: 2
//! author: sachin
//! date: 2026-11-14T10:32:00Z
//! kind: checkpoint
//! note: dropped the plugin idea, keep it local-first
//! ---
//! <the note text, byte for byte>
//! ```
//!
//! Pure: no disk here (`platform/revisions.rs` finds the files and their folders).

use std::fmt;
use std::path::{Path, PathBuf};

use crate::merge;

/// Where a vault keeps the revisions of its notes, under the vault root (hidden from the tree).
pub const VAULT_DIR: &str = ".inkuno/revisions";
/// Where the scratch folder keeps the revisions of its notes.
pub const SCRATCH_DIR: &str = ".revisions";
/// The line that opens and closes the front matter.
const FENCE: &str = "---";
/// Unchanged lines shown around each change in a unified diff.
pub const CONTEXT: usize = 3;

/// Why a revision was made. `auto` (taken around each agent write) is read but not yet written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Checkpoint,
    Auto,
}

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Checkpoint => "checkpoint",
            Kind::Auto => "auto",
        }
    }

    fn of_word(word: &str) -> Option<Kind> {
        match word {
            "checkpoint" => Some(Kind::Checkpoint),
            "auto" => Some(Kind::Auto),
            _ => None,
        }
    }
}

/// One saved version of a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Revision {
    /// 1-based.
    pub number: u32,
    /// The revision it was saved after; none for the first.
    pub parent: Option<u32>,
    pub author: String,
    /// UTC, RFC 3339, to the second.
    pub date: String,
    pub kind: Kind,
    /// What changed or was decided. An empty note is the same as none.
    pub note: Option<String>,
    /// The complete note text, as it was.
    pub text: String,
}

/// Why a revision file cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file does not open with a `---` line.
    NoHeader,
    /// The header is never closed by a `---` line.
    Unclosed,
    /// A field is missing or unreadable (`revision`, `parent`, `kind`).
    Field(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoHeader => write!(f, "no front matter: the file does not open with ---"),
            Error::Unclosed => write!(f, "the front matter is not closed by ---"),
            Error::Field(field) => write!(f, "the field `{field}` is missing or not valid"),
        }
    }
}

/// Writes a revision file: its front matter, then the text verbatim.
pub fn render(rev: &Revision) -> String {
    let mut out = format!("{FENCE}\nrevision: {}\n", rev.number);
    if let Some(parent) = rev.parent {
        out += &format!("parent: {parent}\n");
    }
    out += &format!("author: {}\n", encode(&rev.author));
    out += &format!("date: {}\n", encode(&rev.date));
    out += &format!("kind: {}\n", rev.kind.word());
    if let Some(note) = rev.note.as_deref().filter(|note| !note.is_empty()) {
        out += &format!("note: {}\n", encode(note));
    }
    out += FENCE;
    out += "\n";
    out += &rev.text;
    out
}

/// Reads a revision file. Unknown front matter keys are ignored; `author` and `date` default when
/// missing, `kind` to a checkpoint. The text is everything after the closing `---` line.
pub fn parse(file: &str) -> Result<Revision, Error> {
    let mut lines = file.split_inclusive('\n');
    let open = lines.next().ok_or(Error::NoHeader)?;
    if bare(open) != FENCE {
        return Err(Error::NoHeader);
    }
    let mut offset = open.len();
    let mut fields: Vec<(&str, &str)> = Vec::new();
    let mut closed = false;
    for line in lines {
        offset += line.len();
        let line = bare(line);
        if line == FENCE {
            closed = true;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            fields.push((key.trim(), value.trim()));
        }
    }
    if !closed {
        return Err(Error::Unclosed);
    }
    let get = |key: &str| fields.iter().find(|(k, _)| *k == key).map(|(_, v)| *v);
    let number = get("revision")
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n >= 1)
        .ok_or(Error::Field("revision"))?;
    let parent = match get("parent") {
        None => None,
        Some(v) => Some(
            v.parse::<u32>()
                .ok()
                .filter(|n| *n >= 1)
                .ok_or(Error::Field("parent"))?,
        ),
    };
    let kind = match get("kind") {
        None => Kind::Checkpoint,
        Some(v) => Kind::of_word(v).ok_or(Error::Field("kind"))?,
    };
    let author = get("author")
        .map(decode)
        .filter(|a| !a.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    let date = get("date").map(decode).unwrap_or_default();
    let note = get("note").map(decode).filter(|n| !n.is_empty());
    Ok(Revision {
        number,
        parent,
        author,
        date,
        kind,
        note,
        text: file[offset..].to_string(),
    })
}

/// A line without its line ending (`\n` or `\r\n`).
fn bare(line: &str) -> &str {
    line.strip_suffix('\n')
        .map_or(line, |l| l.strip_suffix('\r').unwrap_or(l))
}

/// A front matter value, written plain when it reads back as it is, else as a quoted string with
/// escapes, so that any text (colons, quotes, line breaks, Unicode) round-trips on one line.
pub fn encode(value: &str) -> String {
    let plain = !value.is_empty()
        && value.trim() == value
        && !value.starts_with('"')
        && !value.chars().any(char::is_control);
    if plain {
        return value.to_string();
    }
    let mut out = String::from('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The value of a front matter field: a quoted string is unescaped, anything else is taken as it
/// is. A quoted value that does not read as one is taken as it is too (a hand-edited file).
pub fn decode(raw: &str) -> String {
    raw.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .and_then(unescape)
        .unwrap_or_else(|| raw.to_string())
}

fn unescape(inner: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return None,
            '\\' => match chars.next()? {
                '\\' => out.push('\\'),
                '"' => out.push('"'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = chars.by_ref().take(4).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    Some(out)
}

/// The file name of revision `number`: `0001.md`, and wider past 9999.
pub fn file_name(number: u32) -> String {
    format!("{number:04}.md")
}

/// The revision number a file name stands for (`0003.md` is 3); `None` for any other name.
pub fn number_of(name: &str) -> Option<u32> {
    let digits = name.strip_suffix(".md")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok().filter(|n| *n >= 1)
}

/// The number of the next revision, after the ones already saved.
pub fn next_number(saved: impl IntoIterator<Item = u32>) -> u32 {
    saved.into_iter().max().map_or(1, |last| last + 1)
}

/// The folder of the revisions of `path` (a note, or a folder whose notes have revisions): the
/// same path under `root/dir`, so `ideas/plan.md` is `<vault>/.inkuno/revisions/ideas/plan.md`.
/// `None` when `path` is not inside `root`.
pub fn folder(root: &Path, dir: &str, path: &Path) -> Option<PathBuf> {
    let rel = path.strip_prefix(root).ok()?;
    if rel.as_os_str().is_empty() {
        return None;
    }
    Some(root.join(dir).join(rel))
}

/// The folder of a vault's trash for revisions: `<vault>/.inkuno/trash`.
pub fn vault_bin(vault: &Path) -> PathBuf {
    vault.join(".inkuno").join("trash")
}

/// The folder of a scratch folder's trash for revisions: `<scratch>/.revisions/.trash`.
pub fn scratch_bin(scratch: &Path) -> PathBuf {
    scratch.join(SCRATCH_DIR).join(".trash")
}

/// Which way the text of two revisions differs, by line, in the two renderings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Standard unified diff, with the revisions as `a/<name>@<n>` and `b/<name>@<m>`.
    Unified,
    /// CriticMarkup over the whole text: `{--removed--}` and `{++added++}`.
    Critic,
}

impl Format {
    pub fn of_word(word: &str) -> Option<Format> {
        match word {
            "unified" => Some(Format::Unified),
            "critic" => Some(Format::Critic),
            _ => None,
        }
    }
}

/// The unified diff from `old` to `new`, labelled `name@old_label` and `name@new_label`. Empty when
/// the texts are the same. Three lines of context, as `diff -u` (and `patch`) read it.
pub fn unified(name: &str, old_label: &str, new_label: &str, old: &str, new: &str) -> String {
    let changes = merge::hunks(&merge::lines(old), &merge::lines(new));
    if changes.is_empty() {
        return String::new();
    }
    let (a, b) = (merge::lines(old), merge::lines(new));
    // Each change with its context; neighbours closer than twice the context become one block.
    // The lines between two changes are the same in both texts, so the context is counted once.
    let mut blocks: Vec<(usize, usize, usize, usize, Vec<usize>)> = Vec::new();
    for (k, change) in changes.iter().enumerate() {
        let before = match k {
            0 => change.base.start,
            _ => change.base.start - changes[k - 1].base.end,
        }
        .min(CONTEXT);
        let after = match changes.get(k + 1) {
            Some(next) => next.base.start - change.base.end,
            None => a.len() - change.base.end,
        }
        .min(CONTEXT);
        let (os, oe) = (change.base.start - before, change.base.end + after);
        let (ns, ne) = (change.side.start - before, change.side.end + after);
        match blocks.last_mut() {
            Some(last) if os <= last.1 => {
                last.1 = oe;
                last.3 = ne;
                last.4.push(k);
            }
            _ => blocks.push((os, oe, ns, ne, vec![k])),
        }
    }
    let mut out = format!("--- a/{name}@{old_label}\n+++ b/{name}@{new_label}\n");
    for (os, oe, ns, ne, members) in blocks {
        // `start,count`; the count is left out when it is one, and an empty range names the
        // line before it (0 at the top), as `diff -u` does.
        let range = |start: usize, end: usize| {
            let count = end - start;
            match (count, start) {
                (1, _) => format!("{}", start + 1),
                (0, _) => format!("{start},0"),
                _ => format!("{},{count}", start + 1),
            }
        };
        out += &format!("@@ -{} +{} @@\n", range(os, oe), range(ns, ne));
        let mut i = os;
        for k in members {
            let change = &changes[k];
            for line in &a[i..change.base.start] {
                out += &marked(' ', line);
            }
            for line in &a[change.base.start..change.base.end] {
                out += &marked('-', line);
            }
            for line in &b[change.side.start..change.side.end] {
                out += &marked('+', line);
            }
            i = change.base.end;
        }
        for line in &a[i..oe] {
            out += &marked(' ', line);
        }
    }
    out
}

/// One line of a unified diff; a line without its end gets the marker `patch` expects.
fn marked(sign: char, line: &str) -> String {
    match line.strip_suffix('\n') {
        Some(body) => format!("{sign}{body}\n"),
        None => format!("{sign}{line}\n\\ No newline at end of file\n"),
    }
}

/// The text with its changes marked in CriticMarkup, line by line: a removed line is wrapped in
/// `{--…--}`, an added one in `{++…++}`, the others are kept. A line keeps its line ending outside
/// the markup. Empty change lists give the text unchanged.
pub fn critic(old: &str, new: &str) -> String {
    let changes = merge::hunks(&merge::lines(old), &merge::lines(new));
    let (a, b) = (merge::lines(old), merge::lines(new));
    let mut out = String::new();
    let mut i = 0;
    for change in &changes {
        for line in &a[i..change.base.start] {
            out += line;
        }
        for line in &a[change.base.start..change.base.end] {
            out += &wrapped("{--", "--}", line);
        }
        for line in &b[change.side.start..change.side.end] {
            out += &wrapped("{++", "++}", line);
        }
        i = change.base.end;
    }
    for line in &a[i..] {
        out += line;
    }
    out
}

/// A line inside CriticMarkup, its line ending (if any) after the closing mark.
fn wrapped(open: &str, close: &str, line: &str) -> String {
    match line.strip_suffix('\n') {
        Some(body) => format!("{open}{body}{close}\n"),
        None => format!("{open}{line}{close}"),
    }
}

/// Renders the difference between two texts in `format`.
pub fn diff(
    format: Format,
    name: &str,
    old_label: &str,
    new_label: &str,
    old: &str,
    new: &str,
) -> String {
    match format {
        Format::Unified => unified(name, old_label, new_label, old, new),
        Format::Critic => critic(old, new),
    }
}

/// The author of the revisions a person makes when none is chosen: `INKUNO_AUTHOR`, else the OS
/// user name (`USER`, then `USERNAME`), else `unknown`. `env` reads a variable.
pub fn default_author(env: impl Fn(&str) -> Option<String>) -> String {
    ["INKUNO_AUTHOR", "USER", "USERNAME"]
        .into_iter()
        .find_map(|key| {
            env(key)
                .map(|value| clean_author(&value))
                .filter(|a| !a.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

/// An author on one line: control characters become spaces, the ends are trimmed.
pub fn clean_author(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Days since 1970-01-01 to (year, month, day), in the proleptic Gregorian calendar.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// A Unix time as UTC RFC 3339 to the second: `2026-11-14T10:32:00Z`.
pub fn rfc3339(unix: i64) -> String {
    let (year, month, day) = civil(unix.div_euclid(86_400));
    let rest = unix.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rev(number: u32, text: &str) -> Revision {
        Revision {
            number,
            parent: number.checked_sub(1).filter(|n| *n >= 1),
            author: "sachin".into(),
            date: "2026-11-14T10:32:00Z".into(),
            kind: Kind::Checkpoint,
            note: None,
            text: text.into(),
        }
    }

    fn round_trip(text: &str, note: Option<&str>, author: &str) {
        let original = Revision {
            note: note.map(str::to_string),
            author: author.into(),
            ..rev(2, text)
        };
        let back = parse(&render(&original)).unwrap();
        assert_eq!(back, original, "text {text:?}, note {note:?}");
    }

    #[test]
    fn writes_the_documented_front_matter() {
        let text = render(&Revision {
            note: Some("dropped the plugin idea, keep it local-first".into()),
            ..rev(3, "# Plan\n")
        });
        assert_eq!(
            text,
            "---\nrevision: 3\nparent: 2\nauthor: sachin\ndate: 2026-11-14T10:32:00Z\n\
             kind: checkpoint\nnote: dropped the plugin idea, keep it local-first\n---\n# Plan\n"
        );
    }

    #[test]
    fn the_first_revision_has_no_parent() {
        let text = render(&rev(1, "a"));
        assert!(!text.contains("parent:"), "{text}");
        assert_eq!(parse(&text).unwrap().parent, None);
    }

    #[test]
    fn round_trips_the_text_byte_for_byte() {
        for text in [
            "",
            "\n",
            "plain\n",
            "no final newline",
            // The note starts with its own front matter: it is only body here.
            "---\ntitle: Old\n---\n# Note\n",
            // Lines that are the fence, inside the body.
            "a\n---\nb\n---\n",
            "---\n---\n",
            "CRLF\r\nline\r\n",
            "tabs\tand  spaces  \n",
            "Ünïcödé 🦀 日本語\n",
            "\"quoted\" and \\ backslash\n",
        ] {
            round_trip(text, None, "sachin");
            let back = parse(&render(&rev(4, text))).unwrap();
            assert_eq!(back.text, text);
        }
    }

    #[test]
    fn round_trips_tricky_notes_and_authors() {
        for note in [
            "colons: everywhere: here",
            "a \"quoted\" word",
            "two\nlines",
            "  spaces around  ",
            "\"starts with a quote",
            "---",
            "key: value",
            "tab\there",
            "back\\slash",
            "Ünïcödé 🦀",
            "control\u{1}char",
        ] {
            round_trip("body\n", Some(note), "sachin");
        }
        round_trip("body\n", None, "Sachin Kolisetty");
        round_trip("body\n", None, "name: with colon");
        round_trip("body\n", None, "multi\nline author");
    }

    #[test]
    fn an_empty_note_is_no_note() {
        let original = Revision {
            note: Some(String::new()),
            ..rev(1, "x")
        };
        let text = render(&original);
        assert!(!text.contains("note:"));
        assert_eq!(parse(&text).unwrap().note, None);
    }

    #[test]
    fn a_quoted_note_is_written_with_escapes_on_one_line() {
        let text = render(&Revision {
            note: Some("first\nsecond \"x\"".into()),
            ..rev(1, "x")
        });
        assert!(
            text.contains("note: \"first\\nsecond \\\"x\\\"\"\n"),
            "{text}"
        );
        assert_eq!(text.lines().filter(|l| l.starts_with("note:")).count(), 1);
    }

    #[test]
    fn reads_unknown_keys_and_missing_optional_fields() {
        let file = "---\nrevision: 5\nfuture: whatever: it is\nkind: checkpoint\n---\nbody";
        let rev = parse(file).unwrap();
        assert_eq!(rev.number, 5);
        assert_eq!(rev.parent, None);
        assert_eq!(rev.author, "unknown");
        assert_eq!(rev.date, "");
        assert_eq!(rev.note, None);
        assert_eq!(rev.text, "body");
    }

    #[test]
    fn reads_the_auto_kind_but_writes_only_checkpoints() {
        let file = "---\nrevision: 2\nkind: auto\n---\n";
        assert_eq!(parse(file).unwrap().kind, Kind::Auto);
        assert_eq!(Kind::Checkpoint.word(), "checkpoint");
        assert_eq!(Kind::Auto.word(), "auto");
    }

    #[test]
    fn reads_crlf_fences_and_keeps_the_body_as_it_is() {
        let file = "---\r\nrevision: 1\r\nauthor: x\r\n---\r\nbody\r\n";
        let rev = parse(file).unwrap();
        assert_eq!(rev.author, "x");
        assert_eq!(rev.text, "body\r\n");
    }

    #[test]
    fn refuses_files_that_are_not_revisions() {
        assert_eq!(parse("# just a note\n"), Err(Error::NoHeader));
        assert_eq!(parse(""), Err(Error::NoHeader));
        assert_eq!(parse("---\nrevision: 1\n"), Err(Error::Unclosed));
        assert_eq!(
            parse("---\nauthor: x\n---\n"),
            Err(Error::Field("revision"))
        );
        assert_eq!(
            parse("---\nrevision: 0\n---\n"),
            Err(Error::Field("revision"))
        );
        assert_eq!(
            parse("---\nrevision: 1\nparent: two\n---\n"),
            Err(Error::Field("parent"))
        );
        assert_eq!(
            parse("---\nrevision: 1\nkind: manual\n---\n"),
            Err(Error::Field("kind"))
        );
    }

    #[test]
    fn numbers_files_and_the_next_revision() {
        assert_eq!(file_name(1), "0001.md");
        assert_eq!(file_name(42), "0042.md");
        assert_eq!(file_name(12_345), "12345.md");
        assert_eq!(number_of("0003.md"), Some(3));
        assert_eq!(number_of("12345.md"), Some(12_345));
        assert_eq!(number_of("0000.md"), None);
        assert_eq!(number_of("notes.md"), None);
        assert_eq!(number_of("0003.txt"), None);
        assert_eq!(number_of(".0003.md.tmp"), None);
        assert_eq!(next_number([]), 1);
        assert_eq!(next_number([1, 2, 7]), 8);
    }

    #[test]
    fn maps_vault_and_scratch_notes_to_their_folders() {
        let vault = Path::new("/home/a/vault");
        assert_eq!(
            folder(vault, VAULT_DIR, &vault.join("ideas/plan.md")),
            Some(PathBuf::from(
                "/home/a/vault/.inkuno/revisions/ideas/plan.md"
            ))
        );
        assert_eq!(
            folder(vault, VAULT_DIR, &vault.join("Top.md")),
            Some(PathBuf::from("/home/a/vault/.inkuno/revisions/Top.md"))
        );
        let scratch = Path::new("/home/a/.config/inkuno/scratch");
        assert_eq!(
            folder(scratch, SCRATCH_DIR, &scratch.join("2026-11-14-103200.md")),
            Some(PathBuf::from(
                "/home/a/.config/inkuno/scratch/.revisions/2026-11-14-103200.md"
            ))
        );
        assert_eq!(folder(vault, VAULT_DIR, Path::new("/elsewhere/x.md")), None);
        assert_eq!(folder(vault, VAULT_DIR, vault), None);
        assert_eq!(
            vault_bin(vault),
            PathBuf::from("/home/a/vault/.inkuno/trash")
        );
        assert_eq!(
            scratch_bin(scratch),
            PathBuf::from("/home/a/.config/inkuno/scratch/.revisions/.trash")
        );
    }

    #[test]
    fn the_author_comes_from_the_environment() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |key: &str| {
                pairs
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, v)| v.to_string())
            }
        };
        assert_eq!(
            default_author(env(&[("INKUNO_AUTHOR", " ana "), ("USER", "root")])),
            "ana"
        );
        assert_eq!(default_author(env(&[("USER", "sachin")])), "sachin");
        assert_eq!(default_author(env(&[("USERNAME", "Kim")])), "Kim");
        // An empty or unreadable value is no value.
        assert_eq!(
            default_author(env(&[("INKUNO_AUTHOR", "  "), ("USER", "bo\nb")])),
            "bo b"
        );
        assert_eq!(default_author(env(&[])), "unknown");
        assert_eq!(clean_author("a\tb"), "a b");
    }

    #[test]
    fn formats_dates_in_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(-1), "1969-12-31T23:59:59Z");
    }

    #[test]
    fn unified_diff_has_headers_and_three_lines_of_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n";
        let new = "1\n2\n3\nfour\n5\n6\n7\n8\n9\n";
        assert_eq!(
            unified("plan.md", "1", "current", old, new),
            "--- a/plan.md@1\n+++ b/plan.md@current\n@@ -1,7 +1,7 @@\n 1\n 2\n 3\n-4\n+four\n 5\n 6\n 7\n"
        );
    }

    #[test]
    fn unified_diff_counts_insertions_and_deletions() {
        let old = "a\nb\n";
        assert_eq!(
            unified("n.md", "1", "2", old, "a\nnew\nb\n"),
            "--- a/n.md@1\n+++ b/n.md@2\n@@ -1,2 +1,3 @@\n a\n+new\n b\n"
        );
        assert_eq!(
            unified("n.md", "1", "2", "x\n", ""),
            "--- a/n.md@1\n+++ b/n.md@2\n@@ -1 +0,0 @@\n-x\n"
        );
        assert_eq!(
            unified("n.md", "1", "2", "", "x\n"),
            "--- a/n.md@1\n+++ b/n.md@2\n@@ -0,0 +1 @@\n+x\n"
        );
    }

    #[test]
    fn unified_diff_merges_changes_that_share_context() {
        let old: Vec<String> = (1..=20).map(|i| format!("{i}\n")).collect();
        let edit = |lines: &[(usize, &str)]| {
            let mut new = old.clone();
            for (at, text) in lines {
                new[*at - 1] = format!("{text}\n");
            }
            new.concat()
        };
        let old = old.concat();
        // Lines 3 and 9 leave five unchanged lines between them: the context meets, one block.
        let near = unified("n.md", "1", "2", &old, &edit(&[(3, "three"), (9, "nine")]));
        assert_eq!(near.matches("\n@@ ").count(), 1, "{near}");
        // Lines 2 and 19 are far apart: two blocks.
        let far = unified(
            "n.md",
            "1",
            "2",
            &old,
            &edit(&[(2, "two"), (19, "nineteen")]),
        );
        assert_eq!(far.matches("\n@@ ").count(), 2, "{far}");
    }

    #[test]
    fn unified_diff_is_empty_when_nothing_changed() {
        assert_eq!(unified("n.md", "1", "2", "same\n", "same\n"), "");
    }

    #[test]
    fn unified_diff_marks_a_missing_final_newline() {
        let out = unified("n.md", "1", "2", "a\nb", "a\nb\n");
        assert_eq!(
            out,
            "--- a/n.md@1\n+++ b/n.md@2\n@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+b\n"
        );
    }

    #[test]
    fn critic_marks_removed_and_added_lines() {
        assert_eq!(
            critic("keep\nold\nend\n", "keep\nnew\nend\n"),
            "keep\n{--old--}\n{++new++}\nend\n"
        );
        assert_eq!(critic("a\n", "a\n"), "a\n");
        assert_eq!(critic("", "x\n"), "{++x++}\n");
        assert_eq!(critic("x\n", ""), "{--x--}\n");
        assert_eq!(critic("a\n", "a\nb"), "a\n{++b++}");
    }

    #[test]
    fn the_format_words_are_unified_and_critic() {
        assert_eq!(Format::of_word("unified"), Some(Format::Unified));
        assert_eq!(Format::of_word("critic"), Some(Format::Critic));
        assert_eq!(Format::of_word("html"), None);
        assert_eq!(
            diff(Format::Critic, "n.md", "1", "2", "a\n", "b\n"),
            "{--a--}\n{++b++}\n"
        );
    }
}
