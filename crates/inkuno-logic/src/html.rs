//! The title of an HTML page: its `<title>`, read from the head of the file.

/// The longest title kept, in characters: a tab's label stays short.
const MAX_CHARS: usize = 80;

/// The title of a page, from the text of its head: the first `<title>` element outside comments and
/// `<script>`, `<style>` and `<svg>` elements (its tag in any case, with attributes allowed). Its
/// entities are decoded and its whitespace collapsed. None when there is no closed `<title>`, or when
/// it is empty. A title longer than 80 characters is cut to 80.
pub fn html_title(head: &str) -> Option<String> {
    let (start, end) = title_span(head)?;
    let text = decode(&head[start..end]);
    let title: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_CHARS)
        .collect();
    (!title.is_empty()).then_some(title)
}

/// The byte range of the text of the first `<title>` element, found by a scan that skips comments and
/// the elements whose text is not a title (`script`, `style`, `svg`). None when there is none closed.
fn title_span(head: &str) -> Option<(usize, usize)> {
    // ASCII lowercasing keeps every byte index of `head`, so the positions found in `lower` cut `head`.
    let lower = head.to_ascii_lowercase();
    let mut at = 0;
    loop {
        let open = at + lower[at..].find('<')?;
        let rest = &lower[open..];
        if let Some(body) = rest.strip_prefix("<!--") {
            // A comment runs to its end; an unclosed one hides the rest of the head.
            at = open + 4 + body.find("-->")? + 3;
        } else if let Some(name) = ["script", "style", "svg"]
            .into_iter()
            .find(|name| is_tag(rest, name))
        {
            let tag_end = open + rest.find('>')?;
            if lower[open..tag_end].ends_with('/') {
                // A self-closing element (`<svg/>`) has no text: its tag is the whole element.
                at = tag_end + 1;
            } else {
                // The element's text is skipped up to its closing tag.
                let close = open + lower[open..].find(&format!("</{name}"))?;
                at = close + lower[close..].find('>')? + 1;
            }
        } else if is_tag(rest, "title") {
            let start = open + rest.find('>')? + 1;
            let end = start + lower[start..].find("</title")?;
            return Some((start, end));
        } else {
            at = open + 1;
        }
    }
}

/// Whether `text` (lowercase, starting with `<`) opens the element `name`: the tag name ends where the
/// name ends, so `<titles>` is another element.
fn is_tag(text: &str, name: &str) -> bool {
    text.strip_prefix('<')
        .and_then(|tag| tag.strip_prefix(name))
        .and_then(|after| after.chars().next())
        .is_some_and(|c| c.is_whitespace() || c == '>' || c == '/')
}

/// The text with its entities decoded in one pass, so `&amp;lt;` reads as the text `&lt;`, not `<`.
/// An unknown or unterminated entity stays as it is.
fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        match entity(rest) {
            Some((ch, len)) => {
                out.push(ch);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The character of the entity at the start of `text` (`&name;`, `&#NN;` or `&#xHH;`) and its length
/// in bytes. None for any other text.
fn entity(text: &str) -> Option<(char, usize)> {
    // A name is short: a `;` further away is no entity.
    let end = text.find(';').filter(|&end| end <= 10)?;
    let name = &text[1..end];
    let ch = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "quot" => '"',
        "apos" => '\'',
        "nbsp" => ' ',
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse::<u32>().ok()?,
            };
            char::from_u32(code)?
        }
    };
    Some((ch, end + 1))
}

#[cfg(test)]
mod tests {
    use super::html_title;

    #[test]
    fn the_title_is_the_text_of_the_title_element() {
        assert_eq!(
            html_title("<html><head><title>My page</title></head>").as_deref(),
            Some("My page")
        );
    }

    #[test]
    fn the_tag_is_found_in_any_case_and_with_attributes() {
        assert_eq!(
            html_title("<HEAD><TITLE lang=\"en\">Notes</TITLE>").as_deref(),
            Some("Notes")
        );
        assert_eq!(
            html_title("<title\n>Spread</Title>").as_deref(),
            Some("Spread")
        );
    }

    #[test]
    fn another_element_is_not_a_title() {
        assert_eq!(html_title("<titles>No</titles>"), None);
        assert_eq!(
            html_title("<titlex>No</titlex><title>Yes</title>").as_deref(),
            Some("Yes")
        );
    }

    #[test]
    fn a_self_closing_svg_before_the_title_does_not_hide_it() {
        assert_eq!(html_title("<svg/><title>X</title>").as_deref(), Some("X"));
        assert_eq!(
            html_title("<svg xmlns=\"http://www.w3.org/2000/svg\" /><title>X</title>").as_deref(),
            Some("X")
        );
        // An svg with its text, and no close, still hides the rest of the head.
        assert_eq!(html_title("<svg><title>X</title>"), None);
    }

    #[test]
    fn a_title_in_a_comment_or_a_script_is_not_the_title() {
        assert_eq!(
            html_title("<!-- <title>Old</title> --><title>New</title>").as_deref(),
            Some("New")
        );
        assert_eq!(
            html_title("<script>var s = '<title>No</title>';</script><title>Yes</title>")
                .as_deref(),
            Some("Yes")
        );
        assert_eq!(
            html_title("<style>/* <title>No</title> */</style><title>Yes</title>").as_deref(),
            Some("Yes")
        );
        assert_eq!(
            html_title("<svg><title>Icon</title></svg><title>Page</title>").as_deref(),
            Some("Page")
        );
        // An unclosed comment hides the rest of the head.
        assert_eq!(html_title("<!-- <title>No</title>"), None);
    }

    #[test]
    fn entities_are_decoded_once() {
        assert_eq!(
            html_title("<title>A &amp; B &lt;3 &quot;x&quot; &#39;y&#39; &gt;</title>").as_deref(),
            Some("A & B <3 \"x\" 'y' >")
        );
        assert_eq!(
            html_title("<title>&amp;lt;</title>").as_deref(),
            Some("&lt;")
        );
    }

    #[test]
    fn apostrophe_nbsp_and_numeric_entities_are_decoded() {
        assert_eq!(
            html_title("<title>Ann&apos;s&nbsp;page</title>").as_deref(),
            Some("Ann's page")
        );
        assert_eq!(
            html_title("<title>&#233;t&#xE9; &#X41;</title>").as_deref(),
            Some("été A")
        );
        // Not an entity: kept as it is, and an invalid code point too.
        assert_eq!(
            html_title("<title>Tom &amp Jerry &#xZZ; &#1114112;</title>").as_deref(),
            Some("Tom &amp Jerry &#xZZ; &#1114112;")
        );
    }

    #[test]
    fn whitespace_is_collapsed_and_trimmed() {
        assert_eq!(
            html_title("<title>\n  Two\t\nlines  </title>").as_deref(),
            Some("Two lines")
        );
    }

    #[test]
    fn no_title_is_none() {
        assert_eq!(html_title(""), None);
        assert_eq!(html_title("<title>   </title>"), None);
        assert_eq!(html_title("<title></title>"), None);
        assert_eq!(html_title("<p>no title here</p>"), None);
        assert_eq!(html_title("<title>never closed"), None);
        assert_eq!(html_title("<title"), None);
    }

    #[test]
    fn a_long_title_is_cut_to_eighty_characters() {
        let long = "é".repeat(100);
        let title = html_title(&format!("<title>{long}</title>")).unwrap();
        assert_eq!(title.chars().count(), 80);
        assert!(title.chars().all(|c| c == 'é'));
    }

    #[test]
    fn the_first_title_wins() {
        assert_eq!(
            html_title("<title>One</title><title>Two</title>").as_deref(),
            Some("One")
        );
    }
}
