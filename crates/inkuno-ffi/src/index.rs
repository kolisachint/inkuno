//! The vault as the app's index keeps it: the notes by name, their links and tags, backlinks, the
//! empty page's recall, and the search of the palette. The Swift side reads the files, gives each
//! note as `NoteInput` (vault-relative path, text, modification time) and updates the index as the
//! files change. Pure: the index never touches the disk.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use inkuno_logic::{
    markdown,
    note::{Note, is_template, lowered},
    rank, recall, wiki,
};

use crate::{
    notes::Ymd,
    search::SnippetLine,
    text::{offset_utf16, utf16_len},
};

/// Most text matches the palette lists under the names (`palette.rs`, the `take(8)` of the text).
const TEXT_HITS: usize = 8;

/// A note as the Swift side gives it to the index.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct NoteInput {
    /// Vault-relative path, `/` separated (`ideas/plan.md`).
    pub path: String,
    /// The whole text of the note.
    pub body: String,
    /// Last modification, in seconds since the Unix epoch.
    pub mtime_unix: i64,
}

/// A note the index names: its path, its name (the file name without its extension) and its
/// modification time.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct NoteRef {
    pub path: String,
    pub name: String,
    pub mtime_unix: i64,
}

/// A note to reread (see `inkuno_logic::recall::recall`).
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct RecalledNote {
    pub path: String,
    pub name: String,
    /// The first line of text of the note, Markdown markers taken out.
    pub line: String,
    /// Whole days since the note's date (a daily note) or since it was last written.
    pub days: i64,
}

/// Why a search result is listed (the palette's three groups).
#[derive(uniffi::Enum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchKind {
    /// The name matches the query: a substring first, otherwise a subsequence.
    ByName,
    /// The text holds every word of the query; `snippet` shows the line.
    ByText,
    /// The query is `#tag`: the note has a tag that starts with it.
    ByTag,
}

/// A search result, in the order the palette lists them.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    pub path: String,
    pub name: String,
    pub kind: SearchKind,
    /// For a text match: the line where the first word is, and where it is in that line.
    pub snippet: Option<SnippetLine>,
}

/// The vault's notes, kept in the order the desktop keeps them: most recently modified first, then
/// by path. Interior mutability: the Swift side may update it while a search runs elsewhere.
#[derive(uniffi::Object)]
pub struct VaultIndex {
    notes: Mutex<Vec<Note>>,
}

#[uniffi::export]
impl VaultIndex {
    /// An index of `notes`. A later path wins over an earlier one of the same path.
    #[uniffi::constructor]
    pub fn new(notes: Vec<NoteInput>) -> Arc<Self> {
        let index = Self {
            notes: Mutex::new(Vec::new()),
        };
        for note in notes {
            index.update(note);
        }
        Arc::new(index)
    }

    /// Adds the note, or replaces the one of its path.
    pub fn update(&self, note: NoteInput) {
        let note = note_of(note);
        let mut notes = self.lock();
        notes.retain(|n| n.path != note.path);
        let at = notes.partition_point(|n| sort_key(n) < sort_key(&note));
        notes.insert(at, note);
    }

    /// Takes the note of `path` out of the index (nothing when there is none).
    pub fn remove(&self, path: String) {
        let path = PathBuf::from(path);
        self.lock().retain(|n| n.path != path);
    }

    /// How many notes the index holds.
    pub fn count(&self) -> u64 {
        self.lock().len() as u64
    }

    /// The note a `[[link]]` named `name` points to: by its name first, otherwise by an alias. A
    /// template answers no link (see `inkuno_logic::wiki::resolve`).
    pub fn resolve(&self, name: String) -> Option<NoteRef> {
        let notes = self.lock();
        wiki::resolve(&notes, &name, |n| is_template(&n.path)).map(note_ref)
    }

    /// The notes whose wikilinks point at the note of `path`, by name (see
    /// `inkuno_logic::wiki::backlinks`).
    pub fn backlinks(&self, path: String) -> Vec<NoteRef> {
        let notes = self.lock();
        wiki::backlinks(&notes, Path::new(&path))
            .into_iter()
            .map(note_ref)
            .collect()
    }

    /// The notes to reread today, at most three (see `inkuno_logic::recall::recall`). `now_unix` is
    /// the time of the day, in seconds since the Unix epoch.
    pub fn recall(&self, today: Ymd, now_unix: i64) -> Vec<RecalledNote> {
        let notes = self.lock();
        recall::recall(&notes, today.date(), time_of(now_unix))
            .into_iter()
            .map(|found| RecalledNote {
                path: found.path.to_string_lossy().into_owned(),
                name: found.name,
                line: found.line,
                days: found.days,
            })
            .collect()
    }

    /// The palette's search for `query`, at most `limit` results: the notes whose name matches,
    /// best first; then (for two characters or more) the notes whose text holds every word. A
    /// `#tag` query lists the notes with that tag instead. See `inkuno_logic::rank`.
    pub fn search(&self, query: String, limit: u32) -> Vec<SearchHit> {
        let notes = self.lock();
        let limit = limit as usize;
        // The palette's query: trimmed and lowercase, names and tags are compared as they are.
        let q = query.trim().to_lowercase();
        if let Some(tag) = q.strip_prefix('#') {
            return notes
                .iter()
                .filter(|n| n.tags.iter().any(|t| t.starts_with(tag)))
                .take(limit)
                .map(|n| hit(n, SearchKind::ByTag, None))
                .collect();
        }
        let mut scored: Vec<(i32, usize)> = notes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| Some((rank::fuzzy(&q, &n.name.to_lowercase())?, i)))
            .collect();
        // Stable: at an equal score, the most recent first stays first.
        scored.sort_by_key(|(score, _)| -score);
        let mut named = vec![false; notes.len()];
        for &(_, i) in &scored {
            named[i] = true;
        }
        let mut hits: Vec<SearchHit> = scored
            .into_iter()
            .map(|(_, i)| hit(&notes[i], SearchKind::ByName, None))
            .collect();
        if q.chars().count() >= 2 {
            let words: Vec<&str> = q.split_whitespace().collect();
            let word = words.first().copied().unwrap_or_default();
            hits.extend(
                notes
                    .iter()
                    .enumerate()
                    .filter(|(i, n)| !named[*i] && rank::contains_all(&n.lower, &words))
                    .take(TEXT_HITS)
                    .map(|(_, n)| {
                        let snippet = snippet_of(n, word);
                        hit(n, SearchKind::ByText, snippet)
                    }),
            );
        }
        hits.truncate(limit);
        hits
    }
}

impl VaultIndex {
    fn lock(&self) -> MutexGuard<'_, Vec<Note>> {
        // A panic while the lock is held leaves the list as it was: it is still usable.
        self.notes.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The index order: most recently modified first, then by path.
fn sort_key(note: &Note) -> (std::cmp::Reverse<SystemTime>, &PathBuf) {
    (std::cmp::Reverse(note.mtime), &note.path)
}

/// A note of the index from what the Swift side gives. Templates have no tags, links or aliases of
/// their own (see `inkuno_logic::note::is_template`), as on the desktop.
fn note_of(input: NoteInput) -> Note {
    let path = PathBuf::from(&input.path);
    let template = is_template(&path);
    let (tags, links, aliases) = if template {
        (Vec::new().into(), Vec::new().into(), Vec::new().into())
    } else {
        let (tags, links) = Note::index(&input.body);
        (tags, links, markdown::aliases(&input.body).into())
    };
    let body: Arc<str> = input.body.into();
    let lower = lowered(&body);
    Note {
        name: path
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path,
        tags,
        links,
        aliases,
        mtime: time_of(input.mtime_unix),
        body,
        lower,
    }
}

fn note_ref(note: &Note) -> NoteRef {
    NoteRef {
        path: note.path.to_string_lossy().into_owned(),
        name: note.name.clone(),
        mtime_unix: unix_of(note.mtime),
    }
}

fn hit(note: &Note, kind: SearchKind, snippet: Option<SnippetLine>) -> SearchHit {
    SearchHit {
        path: note.path.to_string_lossy().into_owned(),
        name: note.name.clone(),
        kind,
        snippet,
    }
}

/// The line of the note that holds `word` (see `inkuno_logic::rank::snippet_line`), with the word's
/// place in it in UTF-16.
fn snippet_of(note: &Note, word: &str) -> Option<SnippetLine> {
    let (text, range) = rank::snippet_line(&note.body, &note.lower, word)?;
    Some(SnippetLine {
        match_start_utf16: offset_utf16(&text, range.start),
        match_len_utf16: utf16_len(&text[range.start..range.end]),
        text,
    })
}

/// A Unix time (seconds) as a system time; the epoch when it is out of range.
fn time_of(unix: i64) -> SystemTime {
    match u64::try_from(unix) {
        Ok(secs) => UNIX_EPOCH
            .checked_add(Duration::from_secs(secs))
            .unwrap_or(UNIX_EPOCH),
        Err(_) => UNIX_EPOCH
            .checked_sub(Duration::from_secs(unix.unsigned_abs()))
            .unwrap_or(UNIX_EPOCH),
    }
}

/// A system time as Unix seconds (negative before the epoch).
fn unix_of(time: SystemTime) -> i64 {
    match time.duration_since(UNIX_EPOCH) {
        Ok(after) => after.as_secs() as i64,
        Err(before) => -(before.duration().as_secs() as i64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;
    const DAY: i64 = 86_400;

    fn input(path: &str, body: &str, mtime_unix: i64) -> NoteInput {
        NoteInput {
            path: path.into(),
            body: body.into(),
            mtime_unix,
        }
    }

    fn names(hits: &[SearchHit]) -> Vec<String> {
        hits.iter().map(|h| h.name.clone()).collect()
    }

    #[test]
    fn a_name_resolves_then_an_alias_and_a_template_answers_nothing() {
        let index = VaultIndex::new(vec![
            input("Réunion.md", "# Réunion", NOW),
            input(
                "Journal.md",
                "---\naliases:\n  - carnet\n---\nbody",
                NOW - DAY,
            ),
            input("Templates/Réunion.md", "{{date}}", NOW - 2 * DAY),
        ]);
        assert_eq!(
            index.resolve("réunion".into()),
            Some(NoteRef {
                path: "Réunion.md".into(),
                name: "Réunion".into(),
                mtime_unix: NOW
            })
        );
        assert_eq!(index.resolve("carnet".into()).unwrap().name, "Journal");
        assert_eq!(index.resolve("nothing".into()), None);
        // A template of a name alone answers no link.
        let only = VaultIndex::new(vec![input("Templates/Solo.md", "x", NOW)]);
        assert_eq!(only.resolve("Solo".into()), None);
    }

    #[test]
    fn backlinks_are_the_linking_notes_by_name() {
        let index = VaultIndex::new(vec![
            input("Plan.md", "no links", NOW),
            input("b.md", "see [[plan]]", NOW),
            input("A.md", "[[Plan]] and [[b]]", NOW),
        ]);
        let found: Vec<String> = index
            .backlinks("Plan.md".into())
            .into_iter()
            .map(|n| n.name)
            .collect();
        assert_eq!(found, ["A", "b"]);
        assert!(index.backlinks("absent.md".into()).is_empty());
    }

    #[test]
    fn updating_and_removing_keep_the_index_current() {
        let index = VaultIndex::new(vec![input("Plan.md", "[[Other]]", NOW)]);
        assert!(index.backlinks("Other.md".into()).is_empty());
        index.update(input("Plan.md", "# Plan", NOW));
        assert_eq!(index.count(), 1);
        index.update(input("Other.md", "", NOW));
        assert_eq!(index.count(), 2);
        index.remove("Plan.md".into());
        assert_eq!(index.count(), 1);
        assert_eq!(index.resolve("Plan".into()), None);
    }

    #[test]
    fn names_come_first_then_text_then_the_rest_is_cut() {
        let index = VaultIndex::new(vec![
            input("courses.md", "- lait", NOW - DAY),
            input("Plan.md", "rien", NOW),
            input("Notes.md", "les courses sont faites", NOW - 2 * DAY),
        ]);
        let hits = index.search("cou".into(), 10);
        assert_eq!(names(&hits), ["courses", "Notes"]);
        assert_eq!(hits[0].kind, SearchKind::ByName);
        assert_eq!(hits[1].kind, SearchKind::ByText);
        let snippet = hits[1].snippet.clone().unwrap();
        assert_eq!(snippet.text, "les courses sont faites");
        assert_eq!((snippet.match_start_utf16, snippet.match_len_utf16), (4, 3));
        // Beyond the limit, the list is cut.
        assert_eq!(index.search("cou".into(), 1).len(), 1);
    }

    #[test]
    fn one_letter_does_not_search_the_text_and_a_name_is_fuzzy() {
        let index = VaultIndex::new(vec![
            input("Plan budget.md", "a note about x", NOW),
            input("Other.md", "contains x", NOW),
        ]);
        assert!(index.search("x".into(), 10).is_empty());
        // A subsequence of the name matches: `pbu` finds `Plan budget`.
        assert_eq!(names(&index.search("pbu".into(), 10)), ["Plan budget"]);
    }

    #[test]
    fn a_text_match_needs_every_word() {
        let index = VaultIndex::new(vec![input("A.md", "lait et pâtes", NOW)]);
        assert_eq!(index.search("lait pâtes".into(), 10).len(), 1);
        assert!(index.search("lait beurre".into(), 10).is_empty());
    }

    #[test]
    fn a_tag_query_lists_the_tagged_notes() {
        let index = VaultIndex::new(vec![
            input("A.md", "#Work is here", NOW),
            input("B.md", "#home only", NOW),
        ]);
        let hits = index.search("#wo".into(), 10);
        assert_eq!(names(&hits), ["A"]);
        assert_eq!(hits[0].kind, SearchKind::ByTag);
    }

    #[test]
    fn recall_offers_the_past_daily_note_of_a_week_ago() {
        // 2026-11-24, and the daily note of 2026-11-17 written then.
        let today = Ymd {
            year: 2026,
            month: 11,
            day: 24,
        };
        let now = crate::history::revision_unix_of("2026-11-24T09:00:00Z".into()).unwrap();
        let week = crate::history::revision_unix_of("2026-11-17T09:00:00Z".into()).unwrap();
        let index = VaultIndex::new(vec![input("2026-11-17.md", "# Monday\nfirst line", week)]);
        let found = index.recall(today, now);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "2026-11-17");
        assert_eq!(found[0].line, "first line");
        assert_eq!(found[0].days, 7);
    }

    #[test]
    fn a_negative_time_round_trips() {
        assert_eq!(unix_of(time_of(-DAY)), -DAY);
        assert_eq!(unix_of(time_of(NOW)), NOW);
    }
}
