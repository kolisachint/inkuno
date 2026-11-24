//! Finding notes: the palette's ranking and the one-line snippet of a full-text match.

use inkuno_logic::{lang, rank};

use crate::text::{offset_utf16, utf16_len};

/// The text as search compares it: lowercase, without its accents (`Thème` becomes `theme`).
#[uniffi::export]
pub fn fold_text(text: String) -> String {
    lang::fold(&text)
}

/// How well `name` matches `query`: a substring first (earlier is better), otherwise a
/// subsequence; `None` when it does not match. Both are expected folded (see `fold_text`).
#[uniffi::export]
pub fn fuzzy_score(query: String, name: String) -> Option<i32> {
    rank::fuzzy(&query, &name)
}

/// The line of a note where a word matches, as plain text, and where the word is in it.
#[derive(uniffi::Record, Clone, Debug, PartialEq, Eq)]
pub struct SnippetLine {
    pub text: String,
    pub match_start_utf16: u64,
    pub match_len_utf16: u64,
}

/// The first line of `body` that holds `word`, cut to about 80 characters around it. `lower` is
/// `body` lowercased and `word` is lowercase (see `inkuno_logic::rank::snippet_line`).
#[uniffi::export]
pub fn snippet_line(body: String, lower: String, word: String) -> Option<SnippetLine> {
    let (text, range) = rank::snippet_line(&body, &lower, &word)?;
    Some(SnippetLine {
        match_start_utf16: offset_utf16(&text, range.start),
        match_len_utf16: utf16_len(&text[range.start..range.end]),
        text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folding_drops_case_and_accents() {
        assert_eq!(fold_text("Thème Été".into()), "theme ete");
    }

    #[test]
    fn a_substring_beats_a_subsequence() {
        let sub = fuzzy_score("cou".into(), "courses".into());
        let subseq = fuzzy_score("crs".into(), "courses".into());
        assert!(sub > subseq);
        assert!(subseq.is_some());
        assert_eq!(fuzzy_score("xyz".into(), "courses".into()), None);
        assert!(fuzzy_score(String::new(), "courses".into()).is_some());
    }

    #[test]
    fn snippet_reports_the_match_in_utf16() {
        // The first line holding "venue" is the second one; "é😀 " is before the word in it.
        let body = "# Title\né😀 the venue is set";
        let lower = body.to_lowercase();
        let snippet = snippet_line(body.into(), lower, "venue".into()).unwrap();
        assert_eq!(snippet.text, "é😀 the venue is set");
        assert_eq!(snippet.match_start_utf16, 8);
        assert_eq!(snippet.match_len_utf16, 5);
    }

    #[test]
    fn snippet_of_a_word_that_is_absent_is_none() {
        let body = "nothing here";
        assert_eq!(
            snippet_line(body.into(), body.into(), "absent".into()),
            None
        );
    }
}
