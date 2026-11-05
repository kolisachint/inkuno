//! Workspace search (task 9.4).
//!
//! Full-text search over the vault's `.md` files. Each search walks the vault
//! with `ignore` (respecting `.gitignore`, skipping hidden entries) and scans
//! files with `grep-searcher`. No index is written, so nothing is added to the vault.

use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use grep_matcher::Matcher;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{BinaryDetection, Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::WalkBuilder;

use crate::tree::{is_note_path, Vault};

/// One matching line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub path: PathBuf,
    /// 1-based line number.
    pub line_number: u64,
    /// The line without its trailing newline.
    pub line: String,
    /// Byte offset of the start of the line in the file.
    pub byte_offset: u64,
    /// Every match on the line, in order. Never empty for a hit from `search`.
    pub matches: Vec<MatchSpan>,
}

/// Where one match sits, precisely enough to place a caret on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSpan {
    /// Byte range of the match in the file. Use `byte_range.start` as the caret position.
    pub byte_range: Range<u64>,
    /// 1-based column of the match start, counted in characters from the start of the line.
    pub column: usize,
}

#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    pub case_insensitive: bool,
    /// `false` treats the query as literal text.
    pub regex: bool,
    pub max_results: Option<usize>,
}

/// Outcome of [`search_with_report`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchReport {
    pub hits: Vec<SearchHit>,
    /// Files that could not be read, with the error message. Each was skipped
    /// and the search carried on with the remaining files.
    pub skipped: Vec<(PathBuf, String)>,
}

/// Full-text search over vault `.md` files.
///
/// Each hit is one line that matches, with every match on that line in
/// `SearchHit::matches`. Hits are ordered by file path (case-insensitively, as the
/// sidebar sorts), then by line number. `max_results` caps the number of hits.
/// An empty query returns no hits.
///
/// Files that cannot be read are skipped silently; use [`search_with_report`]
/// to learn which ones were skipped.
pub fn search(vault: &Vault, query: &str, opts: &SearchOptions) -> io::Result<Vec<SearchHit>> {
    search_with_report(vault, query, opts).map(|report| report.hits)
}

/// Like [`search`], but also reports the files that were skipped because they
/// could not be read (permission denied, deleted mid-search, and so on).
pub fn search_with_report(
    vault: &Vault,
    query: &str,
    opts: &SearchOptions,
) -> io::Result<SearchReport> {
    if query.is_empty() || opts.max_results == Some(0) {
        return Ok(SearchReport::default());
    }
    let matcher = build_matcher(query, opts)?;
    let mut searcher = build_searcher();
    let files = note_files(vault.root())?;
    Ok(scan_files(
        &files,
        &matcher,
        &mut searcher,
        opts.max_results,
    ))
}

fn build_matcher(query: &str, opts: &SearchOptions) -> io::Result<RegexMatcher> {
    let pattern = if opts.regex {
        query.to_owned()
    } else {
        escape_literal(query)
    };
    RegexMatcherBuilder::new()
        .case_insensitive(opts.case_insensitive)
        .line_terminator(Some(b'\n'))
        .build(&pattern)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
}

fn build_searcher() -> Searcher {
    SearcherBuilder::new()
        .line_number(true)
        .binary_detection(BinaryDetection::quit(b'\x00'))
        .build()
}

/// Scans each file in order. A file that fails to open or read is recorded in
/// `skipped` and its partial hits are dropped, so each file contributes either
/// all of its hits or none.
fn scan_files(
    files: &[PathBuf],
    matcher: &RegexMatcher,
    searcher: &mut Searcher,
    max_results: Option<usize>,
) -> SearchReport {
    let mut report = SearchReport::default();
    for file in files {
        if max_results.is_some_and(|max| report.hits.len() >= max) {
            break;
        }
        let before = report.hits.len();
        let mut sink = Collector {
            path: file,
            matcher,
            hits: &mut report.hits,
            max: max_results,
        };
        if let Err(err) = searcher.search_path(matcher, file, &mut sink) {
            report.hits.truncate(before);
            report.skipped.push((file.clone(), err.to_string()));
        }
    }
    report
}

/// Every `.md` file below `root`, sorted case-insensitively by path.
/// Honours `.gitignore` and skips hidden entries.
fn note_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let walker = WalkBuilder::new(root)
        .hidden(true)
        .require_git(false)
        .git_global(false)
        .git_exclude(false)
        .ignore(false)
        .parents(false)
        .build();
    let mut files = Vec::new();
    for entry in walker {
        let entry = entry.map_err(io::Error::other)?;
        let is_file = entry.file_type().is_some_and(|t| t.is_file());
        if is_file && is_note_path(entry.path()) {
            files.push(entry.into_path());
        }
    }
    files.sort_by_cached_key(|p| (p.to_string_lossy().to_lowercase(), p.clone()));
    Ok(files)
}

/// Escapes regex metacharacters so the query matches literally.
fn escape_literal(query: &str) -> String {
    const META: &str = "\\.+*?()|[]{}^$#&-~";
    let mut out = String::with_capacity(query.len());
    for c in query.chars() {
        if META.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

struct Collector<'a> {
    path: &'a Path,
    matcher: &'a RegexMatcher,
    hits: &'a mut Vec<SearchHit>,
    max: Option<usize>,
}

impl Sink for Collector<'_> {
    type Error = io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, io::Error> {
        let raw = mat.bytes();
        let base = mat.absolute_byte_offset();
        let mut matches = Vec::new();
        self.matcher
            .find_iter(raw, |m| {
                let column = String::from_utf8_lossy(&raw[..m.start()]).chars().count() + 1;
                matches.push(MatchSpan {
                    byte_range: base + m.start() as u64..base + m.end() as u64,
                    column,
                });
                true
            })
            .map_err(io::Error::other)?;
        let line = String::from_utf8_lossy(raw)
            .trim_end_matches(['\n', '\r'])
            .to_owned();
        self.hits.push(SearchHit {
            path: self.path.to_path_buf(),
            line_number: mat.line_number().unwrap_or(0),
            line,
            byte_offset: base,
            matches,
        });
        Ok(self.max.is_none_or(|max| self.hits.len() < max))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn opts() -> SearchOptions {
        SearchOptions::default()
    }

    fn fixture() -> (tempfile::TempDir, Vault) {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a.md"), "alpha\nbeta gamma\nalpha again\n");
        write(&dir.path().join("sub/b.md"), "nothing\nbeta\n");
        write(&dir.path().join("c.txt"), "beta in a text file\n");
        let vault = Vault::open(dir.path()).unwrap();
        (dir, vault)
    }

    fn snapshot(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path.clone());
                }
                out.push(path);
            }
        }
        out.sort();
        out
    }

    #[test]
    fn returns_file_and_line_for_each_match() {
        let (dir, vault) = fixture();
        let hits = search(&vault, "beta", &opts()).unwrap();
        assert_eq!(
            hits,
            vec![
                SearchHit {
                    path: dir.path().join("a.md"),
                    line_number: 2,
                    line: "beta gamma".into(),
                    byte_offset: 6,
                    matches: vec![span(6..10, 1)],
                },
                SearchHit {
                    path: dir.path().join("sub/b.md"),
                    line_number: 2,
                    line: "beta".into(),
                    byte_offset: 8,
                    matches: vec![span(8..12, 1)],
                },
            ]
        );
    }

    fn span(byte_range: Range<u64>, column: usize) -> MatchSpan {
        MatchSpan { byte_range, column }
    }

    #[test]
    fn every_match_on_a_line_gets_a_span_into_the_file() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("n.md"), "one two one\nzzz\nxx one\n");
        let vault = Vault::open(dir.path()).unwrap();
        let hits = search(&vault, "one", &opts()).unwrap();
        assert_eq!(hits.len(), 2);
        // First line: two matches, columns 1 and 9.
        assert_eq!(hits[0].matches, vec![span(0..3, 1), span(8..11, 9)]);
        // Third line starts at byte 16; "one" begins at column 4.
        assert_eq!(hits[1].byte_offset, 16);
        assert_eq!(hits[1].matches, vec![span(19..22, 4)]);
        // Each span points at the matched text in the file.
        let bytes = fs::read(dir.path().join("n.md")).unwrap();
        for hit in &hits {
            for m in &hit.matches {
                let slice = &bytes[m.byte_range.start as usize..m.byte_range.end as usize];
                assert_eq!(slice, b"one");
            }
        }
    }

    #[test]
    fn column_counts_characters_while_span_counts_bytes() {
        let dir = tempfile::tempdir().unwrap();
        // "é" is two bytes, so the match is at byte 7 but column 7.
        write(&dir.path().join("u.md"), "héllo needle\n");
        let vault = Vault::open(dir.path()).unwrap();
        let hits = search(&vault, "needle", &opts()).unwrap();
        assert_eq!(hits[0].matches, vec![span(7..13, 7)]);
    }

    #[test]
    fn regex_match_spans_cover_the_matched_text_only() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("r.md"), "id=42 and id=7\n");
        let vault = Vault::open(dir.path()).unwrap();
        let o = SearchOptions {
            regex: true,
            ..opts()
        };
        let hits = search(&vault, r"id=\d+", &o).unwrap();
        // "id=42" is bytes 0..5; the second "id=7" starts at byte 10, column 11.
        assert_eq!(hits[0].matches, vec![span(0..5, 1), span(10..14, 11)]);
    }

    #[test]
    fn hits_are_ordered_by_path_case_insensitively_then_line() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("b.md"), "x\nx\n");
        write(&dir.path().join("A.md"), "x\n");
        write(&dir.path().join("c/z.md"), "x\n");
        write(&dir.path().join("c/a.md"), "x\n");
        let vault = Vault::open(dir.path()).unwrap();
        let hits = search(&vault, "x", &opts()).unwrap();
        let order: Vec<(String, u64)> = hits
            .iter()
            .map(|h| {
                let rel = h.path.strip_prefix(dir.path()).unwrap();
                (rel.to_string_lossy().into_owned(), h.line_number)
            })
            .collect();
        assert_eq!(
            order,
            vec![
                ("A.md".to_owned(), 1),
                ("b.md".to_owned(), 1),
                ("b.md".to_owned(), 2),
                ("c/a.md".to_owned(), 1),
                ("c/z.md".to_owned(), 1),
            ]
        );
    }

    #[test]
    fn max_results_stops_in_the_middle_of_a_file() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("a.md"), "x\nx\nx\n");
        write(&dir.path().join("b.md"), "x\n");
        let vault = Vault::open(dir.path()).unwrap();
        let o = SearchOptions {
            max_results: Some(2),
            ..opts()
        };
        let hits = search(&vault, "x", &o).unwrap();
        let lines: Vec<u64> = hits.iter().map(|h| h.line_number).collect();
        assert_eq!(lines, vec![1, 2]);
        assert!(hits.iter().all(|h| h.path.ends_with("a.md")));
    }

    #[test]
    fn multiple_matches_in_one_file_are_ordered_by_line() {
        let (dir, vault) = fixture();
        let hits = search(&vault, "alpha", &opts()).unwrap();
        let lines: Vec<u64> = hits.iter().map(|h| h.line_number).collect();
        assert_eq!(lines, vec![1, 3]);
        assert!(hits.iter().all(|h| h.path == dir.path().join("a.md")));
        assert_eq!(hits[1].line, "alpha again");
        assert_eq!(hits[1].byte_offset, 17);
    }

    #[test]
    fn literal_query_treats_regex_characters_literally() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("n.md"), "axb\na.b\n");
        let vault = Vault::open(dir.path()).unwrap();
        let hits = search(&vault, "a.b", &opts()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, "a.b");
        assert_eq!(hits[0].line_number, 2);
    }

    #[test]
    fn regex_option_uses_the_pattern() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("n.md"), "axb\na.b\n");
        let vault = Vault::open(dir.path()).unwrap();
        let o = SearchOptions {
            regex: true,
            ..opts()
        };
        let hits = search(&vault, "a.b", &o).unwrap();
        assert_eq!(hits.len(), 2);
        assert!(search(&vault, "a(", &o).is_err());
    }

    #[test]
    fn case_insensitive_option() {
        let (_dir, vault) = fixture();
        assert!(search(&vault, "ALPHA", &opts()).unwrap().is_empty());
        let o = SearchOptions {
            case_insensitive: true,
            ..opts()
        };
        assert_eq!(search(&vault, "ALPHA", &o).unwrap().len(), 2);
    }

    #[test]
    fn max_results_limits_hits() {
        let (_dir, vault) = fixture();
        let o = SearchOptions {
            max_results: Some(2),
            ..opts()
        };
        assert_eq!(search(&vault, "beta", &o).unwrap().len(), 2);
        let o = SearchOptions {
            max_results: Some(1),
            ..opts()
        };
        assert_eq!(search(&vault, "a", &o).unwrap().len(), 1);
        let o = SearchOptions {
            max_results: Some(0),
            ..opts()
        };
        assert!(search(&vault, "beta", &o).unwrap().is_empty());
    }

    #[test]
    fn gitignored_and_hidden_files_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join(".gitignore"), "skip.md\nignored/\n");
        write(&dir.path().join("skip.md"), "needle\n");
        write(&dir.path().join("ignored/x.md"), "needle\n");
        write(&dir.path().join(".hidden/y.md"), "needle\n");
        write(&dir.path().join(".dotnote.md"), "needle\n");
        write(&dir.path().join("keep.md"), "needle\n");
        let vault = Vault::open(dir.path()).unwrap();
        let hits = search(&vault, "needle", &opts()).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, dir.path().join("keep.md"));
    }

    #[test]
    fn empty_query_returns_nothing() {
        let (_dir, vault) = fixture();
        assert!(search(&vault, "", &opts()).unwrap().is_empty());
    }

    #[test]
    fn file_deleted_after_listing_is_skipped() {
        let (dir, _vault) = fixture();
        let gone = dir.path().join("gone.md");
        write(&gone, "beta\n");
        let listed = note_files(dir.path()).unwrap();
        assert!(listed.contains(&gone));
        fs::remove_file(&gone).unwrap();

        let matcher = build_matcher("beta", &opts()).unwrap();
        let mut searcher = build_searcher();
        let report = scan_files(&listed, &matcher, &mut searcher, None);

        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].0, gone);
        assert!(!report.skipped[0].1.is_empty());
        let paths: Vec<PathBuf> = report.hits.iter().map(|h| h.path.clone()).collect();
        assert_eq!(
            paths,
            vec![dir.path().join("a.md"), dir.path().join("sub/b.md")]
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_file_is_skipped_and_others_are_returned() {
        use std::os::unix::fs::PermissionsExt;

        let (dir, vault) = fixture();
        let locked = dir.path().join("locked.md");
        write(&locked, "beta behind a lock\n");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read(&locked).is_ok() {
            // Running as root (or with CAP_DAC_OVERRIDE): mode 0o000 does not
            // stop a read, so there is nothing to test.
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
            eprintln!("skipped: process can read mode 0o000 files");
            return;
        }

        let report = search_with_report(&vault, "beta", &opts()).unwrap();
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].0, locked);
        let paths: Vec<PathBuf> = report.hits.iter().map(|h| h.path.clone()).collect();
        assert_eq!(
            paths,
            vec![dir.path().join("a.md"), dir.path().join("sub/b.md")]
        );

        // The plain `search` API returns the same hits while the file is still locked.
        assert_eq!(search(&vault, "beta", &opts()).unwrap(), report.hits);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn search_writes_no_files() {
        let (dir, vault) = fixture();
        let before = snapshot(dir.path());
        search(&vault, "beta", &opts()).unwrap();
        let o = SearchOptions {
            case_insensitive: true,
            regex: true,
            max_results: Some(5),
        };
        search(&vault, "b.+a", &o).unwrap();
        assert_eq!(snapshot(dir.path()), before);
    }
}
