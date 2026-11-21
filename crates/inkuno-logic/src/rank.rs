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
    snippet_at(body, lower, lower[..at].matches('\n').count(), word)
}

/// The same, for the line at index `row`.
pub fn snippet_at(body: &str, lower: &str, row: usize, word: &str) -> String {
    let (Some(shown), Some(low)) = (body.lines().nth(row), lower.lines().nth(row)) else {
        return String::new();
    };
    let col = low.find(word).map_or(0, |i| low[..i].chars().count());
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
    let hit = plain.to_lowercase().find(word)?;
    let start = plain[..hit]
        .char_indices()
        .rev()
        .nth(23)
        .map_or(0, |(i, _)| i);
    let head = if start > 0 { "…" } else { "" };
    let text: String = plain.get(start..)?.chars().take(80).collect();
    let from = head.len() + hit - start;
    let range = from..(from + word.len()).min(head.len() + text.len());
    Some((format!("{head}{text}"), range))
}

/// A line of Markdown as the text it shows: heading, quote, list and task marks, emphasis and code
/// markers, and links reduced to their text. A `_` stays, so that `my_name` reads as it is.
pub fn plain_line(line: &str) -> String {
    let shown = cut_between(&cut_between(line, "<!-- comment:", "-->"), "{>>", "<<}");
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

/// `text` with each `open`…`close` span taken out, the markers with it; an open without its close stays.
/// Both comment forms go this way: the passage stays, what is said does not.
fn cut_between(text: &str, open: &str, close: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let Some(end) = rest[start..].find(close).map(|i| start + i + close.len()) else {
            break;
        };
        out.push_str(&rest[..start]);
        rest = &rest[end..];
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
        let lower = body.to_lowercase();
        assert_eq!(snippet_at(body, &lower, 3, "lait"), "au LAIT cru");
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
        assert!(snippet_line(body, &lower, "absent").is_none());
    }
}
