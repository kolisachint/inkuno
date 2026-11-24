//! How the palette ranks a search: a name first (a substring, then a subsequence), then a note
//! whose text holds every word. Pure, so an agent searches the vault the way the palette does.

/// All the words appear in the text (already lowercase).
pub fn contains_all(lower: &str, words: &[&str]) -> bool {
    words.iter().all(|w| lower.contains(w))
}

/// The note's line where `word` (lowercase) appears, with its original case,
/// recentred on the word and shortened.
pub fn snippet(body: &str, lower: &str, word: &str) -> String {
    let Some(at) = lower.find(word) else {
        return String::new();
    };
    snippet_at(body, lower[..at].matches('\n').count(), word)
}

/// The same, for the line at index `row`.
pub fn snippet_at(body: &str, row: usize, word: &str) -> String {
    let Some(shown) = body.lines().nth(row) else {
        return String::new();
    };
    let (folded, at) = lowered(shown);
    let col = folded
        .find(word)
        .map_or(0, |i| shown[..at[i]].chars().count());
    let chars: Vec<char> = shown.chars().collect();
    let start = col.saturating_sub(24);
    let part: String = chars.iter().skip(start).take(80).collect();
    format!(
        "{}{}{}",
        if start > 0 { "…" } else { "" },
        part.trim(),
        if start + 80 < chars.len() { "…" } else { "" }
    )
}

/// The line of the note where `word` (lowercase) first appears, as plain text, and the byte range of
/// the word in it: the one-line snippet of a full-text match. Markdown markers are dropped, as
/// `plain_line` does; the text is cut to about 80 characters around the word.
pub fn snippet_line(
    body: &str,
    lower: &str,
    word: &str,
) -> Option<(String, std::ops::Range<usize>)> {
    let at = lower.find(word)?;
    let row = lower[..at].matches('\n').count();
    let plain = plain_line(body.lines().nth(row)?);
    // The match is found in the lowercase line and mapped back onto the plain one: lowercasing can
    // change the byte length of a character (`K`, `İ`), so offsets do not carry over.
    let (folded, map) = lowered(&plain);
    let found = folded.find(word)?;
    let hit = original_span(&plain, &map, found..found + word.len());
    let start = plain[..hit.start]
        .char_indices()
        .rev()
        .nth(23)
        .map_or(0, |(i, _)| i);
    let head = if start > 0 { "…" } else { "" };
    let text: String = plain.get(start..)?.chars().take(80).collect();
    let end = (hit.end - start).min(text.len());
    let from = (hit.start - start).min(end);
    let range = head.len() + from..head.len() + end;
    Some((format!("{head}{text}"), range))
}

/// The text in lowercase, and for each byte of that text the byte where the character it came from
/// starts in `text`, so a match in the lowercase text maps back onto the original.
fn lowered(text: &str) -> (String, Vec<usize>) {
    let mut lower = String::with_capacity(text.len());
    let mut at = Vec::with_capacity(text.len());
    for (i, c) in text.char_indices() {
        for l in c.to_lowercase() {
            lower.push(l);
            at.extend(std::iter::repeat_n(i, l.len_utf8()));
        }
    }
    (lower, at)
}

/// The span of `text` that the lowercase bytes `found` came from, whole characters only: a match
/// that ends inside a character (`i` of `İ`) takes the whole of it.
fn original_span(
    text: &str,
    at: &[usize],
    found: std::ops::Range<usize>,
) -> std::ops::Range<usize> {
    let start = at.get(found.start).copied().unwrap_or(text.len());
    if found.is_empty() {
        return start..start;
    }
    let last = at[found.end - 1];
    start..last + text[last..].chars().next().map_or(0, char::len_utf8)
}

/// `line` with every comment taken out and its commented text kept, as `vault::without_comments`
/// does: the comments are the ones the Markdown comments parser reads, in either form.
fn without_comments(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut at = 0;
    for (whole, noted, _) in crate::markdown::comments(line) {
        out.push_str(&line[at..noted.end]);
        at = whole.end;
    }
    out.push_str(&line[at..]);
    out
}

/// A line of Markdown as the text it shows: heading, quote, list and task marks, emphasis and code
/// markers, and links reduced to their text. A `_` stays, so that `my_name` reads as it is.
pub fn plain_line(line: &str) -> String {
    let shown = without_comments(line);
    let mut s = shown.trim();
    let hashes = s.len() - s.trim_start_matches('#').len();
    if (1..=6).contains(&hashes) && s[hashes..].starts_with(' ') {
        s = s[hashes..].trim_start();
    }
    while let Some(rest) = s.strip_prefix('>') {
        s = rest.trim_start();
    }
    for mark in ["- [ ] ", "- [x] ", "- [X] ", "- ", "* ", "+ "] {
        if let Some(rest) = s.strip_prefix(mark) {
            s = rest;
            break;
        }
    }
    let digits = s.chars().take_while(char::is_ascii_digit).count();
    if digits > 0 && s[digits..].starts_with(". ") {
        s = &s[digits + 2..];
    }
    let s = s
        .replace("**", "")
        .replace("~~", "")
        .replace("{==", "")
        .replace("==}", "")
        .replace("==", "")
        .replace("[[", "")
        .replace("]]", "")
        .replace('`', "");
    // `[text](target)` shows its text; the target is not shown.
    let mut out = String::with_capacity(s.len());
    let mut rest = s.as_str();
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find("](").map(|i| open + i) else {
            break;
        };
        let Some(end) = rest[close..].find(')').map(|i| close + i) else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&rest[open + 1..close]);
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// The text in lowercase, without its accents: « thème » is found by typing « theme ».
pub fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

/// Match score: substring first, otherwise subsequence. A higher score is a better name.
pub fn fuzzy(query: &str, name: &str) -> Option<i32> {
    if let Some(i) = name.find(query) {
        return Some(1000 - i as i32);
    }
    let mut chars = name.chars();
    for c in query.chars() {
        chars.find(|&x| x == c)?;
    }
    Some(-(name.len() as i32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_text_with_all_the_words() {
        let lower = "# courses\n\n- lait et pâtes au citron\n";
        assert!(contains_all(lower, &["pâtes", "citron"]));
        assert!(!contains_all(lower, &["pâtes", "beurre"]));
    }

    #[test]
    fn lists_each_line_once() {
        let body = "Lait, lait\n\nrien\nau LAIT cru\n";
        assert_eq!(snippet_at(body, 3, "lait"), "au LAIT cru");
    }

    #[test]
    fn snippet_keeps_the_case_and_trims() {
        let body = "# Courses\n\n- Lait et Pâtes au Citron\n";
        assert_eq!(
            snippet(body, &body.to_lowercase(), "pâtes"),
            "- Lait et Pâtes au Citron"
        );
        let long = format!("{}Citron{}", "a".repeat(100), "b".repeat(100));
        let cut = snippet(&long, &long.to_lowercase(), "citron");
        assert!(cut.starts_with('…') && cut.ends_with('…') && cut.contains("Citron"));
    }

    #[test]
    fn ranks_matches() {
        assert!(fuzzy("cou", "courses") > fuzzy("cou", "les courses"));
        assert!(fuzzy("crs", "courses").is_some());
        assert!(fuzzy("crs", "courses") < fuzzy("ours", "courses"));
        assert_eq!(fuzzy("xyz", "courses"), None);
        assert!(fuzzy("", "courses").is_some());
    }

    #[test]
    fn a_snippet_is_the_plain_line_with_the_word_marked() {
        let body = "# Plan\n\n- [ ] **Book** the [venue](https://x.y/z)\n";
        let lower = body.to_lowercase();
        let (text, range) = snippet_line(body, &lower, "venue").unwrap();
        assert_eq!(text, "Book the venue");
        assert_eq!(&text[range], "venue");
        assert_eq!(plain_line("> 1. `code` and my_name"), "code and my_name");
        // Comment markup goes with what is said; the passage stays, in either form.
        assert_eq!(plain_line("## Tasks<!-- comment: Split this? -->"), "Tasks");
        assert_eq!(
            plain_line("- [ ] Settings page<!-- comment: Add it. -->"),
            "Settings page"
        );
        assert_eq!(plain_line("- un {==mot==}{>>à revoir<<} ici"), "un mot ici");
        assert_eq!(plain_line("<!-- comment: alone -->"), "");
        // Every form of comment goes, replies included, and the commented text stays.
        assert_eq!(
            plain_line(
                "# Plan<!-- comment @Ana 2026-11-24T10:02Z: check ↳ @Bo 2026-11-24T10:05Z: reply -->"
            ),
            "Plan"
        );
        assert_eq!(
            plain_line("keep<!-- comment @Ana 2026-11-24T10:02Z: a --> this"),
            "keep this"
        );
        assert_eq!(
            plain_line("- a<!-- comment @todo fix -->"),
            "a<!-- comment @todo fix -->"
        );
        assert!(snippet_line(body, &lower, "absent").is_none());
    }

    /// The snippet and its marked word, for a line and a word; the marked text must be the word.
    fn marked(body: &str, word: &str) -> (String, String) {
        let (text, range) = snippet_line(body, &body.to_lowercase(), word).unwrap();
        let marked = text[range].to_string();
        (text, marked)
    }

    #[test]
    fn a_snippet_marks_the_word_when_lowercasing_changes_the_length() {
        // Kelvin sign K (3 bytes, lowercase k is 1): a word after it used to be found at the wrong byte.
        assert_eq!(marked("\u{212A}\u{212A}\u{212A} venue", "venue").1, "venue");
        assert_eq!(marked("Kelvin \u{212A}elvin venue", "venue").1, "venue");
        // The word itself with a Kelvin sign: `k` in the note matches the query `k`.
        let (text, range) = snippet_line(
            "a \u{212A}elvin\n",
            &"a \u{212A}elvin\n".to_lowercase(),
            "k",
        )
        .unwrap();
        assert_eq!(&text[range], "\u{212A}");
        // İ lowercases to two characters (i and a combining dot): the match after it is still exact.
        assert_eq!(marked("İstanbul venue", "venue").1, "venue");
        assert_eq!(marked("İ İ venue", "venue").1, "venue");
        // The word matched inside İ takes the whole character, not half of it.
        let (text, range) = snippet_line("İz\n", &"İz\n".to_lowercase(), "i").unwrap();
        assert_eq!(&text[range], "İ");
    }

    #[test]
    fn a_snippet_is_cut_on_character_boundaries() {
        // ß keeps its length, ẞ (capital sharp s) becomes ß: both are fine before the match.
        assert_eq!(marked("Straße venue", "venue").1, "venue");
        assert_eq!(marked("STRAẞE venue", "venue").1, "venue");
        assert_eq!(marked("Straße venue", "straße").1, "Straße");
        // Emoji are four bytes and unchanged by lowercasing.
        assert_eq!(marked("🎉🎉 party 🎉 Venue", "venue").1, "Venue");
        // A long line with multibyte text before the match is cut around it, on boundaries.
        let long = format!(
            "{}İ{}Venue{}",
            "é".repeat(40),
            "🎉".repeat(30),
            "ß".repeat(100)
        );
        let (text, range) = snippet_line(&long, &long.to_lowercase(), "venue").unwrap();
        assert!(text.starts_with('…') && text.chars().count() == 81);
        assert_eq!(&text[range], "Venue");
    }

    #[test]
    fn a_snippet_marks_a_decomposed_accent_in_full() {
        // Combining marks are separate characters: the word is found as typed, and the mark stays with its letter.
        let body = "Cafe\u{301} venue\n";
        assert_eq!(marked(body, "venue").1, "venue");
        assert_eq!(marked(body, "e\u{301}").1, "e\u{301}");
        assert_eq!(marked("x e\u{301}\u{301} y", "e\u{301}").1, "e\u{301}");
        assert_eq!(marked("Ça ÉTÉ", "été").1, "ÉTÉ");
    }

    #[test]
    fn the_snippet_helpers_do_not_panic_on_any_of_these() {
        let body = "K\u{212A} İ ß 🎉 e\u{301}\n\u{212A}\u{130}\u{df}\u{1f389}\n";
        let lower = body.to_lowercase();
        for word in ["k", "i", "i\u{307}", "ß", "🎉", "e\u{301}", "", "zzz"] {
            let _ = snippet(body, &lower, word);
            let _ = snippet_line(body, &lower, word);
        }
        assert_eq!(snippet_at(body, 0, "k"), "K\u{212A} İ ß 🎉 e\u{301}");
    }
}
