//! The text a command prints without `--json`: a few lines a person reads and a shell can cut.
//! Each function reads the JSON result the command produces with `--json`, so the two never
//! disagree about what the work did.

use serde_json::Value;

/// What a command prints: its standard output, and the lines it adds on standard error.
pub struct Shown {
    pub out: String,
    pub err: String,
}

impl Shown {
    /// Standard output only.
    pub fn plain(out: String) -> Self {
        Self {
            out,
            err: String::new(),
        }
    }
}

/// A field as text: a string as it is, a number as it reads, nothing for a missing or null one.
fn field(value: &Value, key: &str) -> String {
    match &value[key] {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A string of the JSON, or nothing when it is not one.
fn text_of(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

/// The text of a list of items, one line each: `line` says what an item reads as.
fn lines_of(items: &Value, line: impl Fn(&Value) -> String) -> String {
    items
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| line(item) + "\n")
        .collect()
}

/// A block of the `links` command: its title and its items indented two spaces. An empty block is
/// left out entirely.
fn section(out: &mut String, title: &str, items: &Value, line: impl Fn(&Value) -> String) {
    let lines = lines_of(items, |item| format!("  {}", line(item)));
    if !lines.is_empty() {
        out.push_str(title);
        out.push_str(":\n");
        out.push_str(&lines);
    }
}

/// A snippet or a sentence on one line: its words, single-spaced.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// ` (missing)` when the JSON has no path for the item, else nothing.
fn missing(path: &Value) -> &'static str {
    if path.is_null() { " (missing)" } else { "" }
}

pub fn vault(v: &Value) -> Shown {
    let mut out = String::new();
    for key in [
        "folder",
        "notes",
        "templates",
        "diagrams",
        "tables",
        "pictures",
        "attachments",
    ] {
        out.push_str(&format!("{key}: {}\n", field(v, key)));
    }
    let folders: Vec<String> = items_of(&v["folders"], |f| text_of(f).to_string());
    out.push_str(&format!("folders: {}\n", joined(&folders)));
    let tags: Vec<String> = items_of(&v["tags"], |t| {
        format!("#{} ({})", text_of(&t["tag"]), field(t, "count"))
    });
    out.push_str(&format!("tags: {}\n", joined(&tags)));
    out.push_str(&format!("daily: {}\n", field(v, "daily")));
    out.push_str(&format!("version: {}\n", field(v, "version")));
    let unreadable: Vec<String> = items_of(&v["unreadable"], |u| {
        format!("{} ({})", text_of(&u["path"]), text_of(&u["error"]))
    });
    if !unreadable.is_empty() {
        out.push_str(&format!("unreadable: {}\n", joined(&unreadable)));
    }
    Shown::plain(out)
}

/// The items joined by `, `, or `none` when there are none.
fn joined(items: &[String]) -> String {
    if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    }
}

/// Each item of a JSON list, as `text` reads it.
fn items_of(items: &Value, text: impl Fn(&Value) -> String) -> Vec<String> {
    items.as_array().into_iter().flatten().map(text).collect()
}

pub fn list(v: &Value) -> Shown {
    let out = lines_of(&v["notes"], |row| text_of(&row["path"]).to_string());
    // The next page is named on standard error, so the listing itself stays one path per line.
    let err = v["next"]
        .as_str()
        .map(|next| format!("more: --cursor {next}\n"))
        .unwrap_or_default();
    Shown { out, err }
}

pub fn search(v: &Value) -> Shown {
    Shown::plain(lines_of(&v["hits"], |hit| {
        let path = text_of(&hit["path"]);
        let snippet = one_line(text_of(&hit["snippet"]));
        if snippet.is_empty() {
            path.to_string()
        } else {
            format!("{path}: {snippet}")
        }
    }))
}

pub fn tags(v: &Value) -> Shown {
    Shown::plain(lines_of(&v["tags"], |t| {
        format!("#{}\t{}", text_of(&t["tag"]), field(t, "count"))
    }))
}

pub fn links(v: &Value) -> Shown {
    let mut out = String::new();
    section(
        &mut out,
        "links",
        &v["outgoing"],
        |link| match link["path"].as_str() {
            Some(path) => format!("{path}:{}", field(link, "line")),
            None => format!(
                "{}:{} (missing)",
                text_of(&link["target"]),
                field(link, "line")
            ),
        },
    );
    section(&mut out, "backlinks", &v["backlinks"], |back| {
        format!(
            "{}: {}",
            text_of(&back["path"]),
            one_line(text_of(&back["line"]))
        )
    });
    section(&mut out, "tags", &v["tags"], |tag| {
        format!("#{}", text_of(tag))
    });
    section(&mut out, "embeds", &v["embeds"], |embed| {
        let heading = match embed["heading"].as_str() {
            Some(heading) => format!("#{heading}"),
            None => String::new(),
        };
        let target = match embed["path"].as_str() {
            Some(path) => path.to_string(),
            None => format!("{} (missing)", text_of(&embed["note"])),
        };
        format!("{target}{heading}")
    });
    section(&mut out, "pictures", &v["pictures"], |picture| {
        let shown = match picture["path"].as_str() {
            Some(path) => path,
            None => text_of(&picture["name"]),
        };
        format!("{shown}{}", missing(&picture["path"]))
    });
    Shown::plain(out)
}

/// `read`: the text of the note, or of its section.
pub fn read(v: &Value) -> Shown {
    Shown::plain(text_of(&v["text"]).to_string())
}

/// The note's path, on a line of its own: what `new`, `append`, `daily`, `comment`, `restore`
/// and `trash` print.
pub fn path(v: &Value) -> Shown {
    Shown::plain(format!("{}\n", text_of(&v["path"])))
}

/// `edit`: the path (the new one when the title renamed the note). A title that could not rename
/// the note is said on standard error: the text was saved all the same.
pub fn edit(v: &Value) -> Shown {
    let err = match v["name_kept"].as_str() {
        Some(reason) => format!("inkuno: the name is kept: {reason}\n"),
        None => String::new(),
    };
    Shown {
        out: format!("{}\n", text_of(&v["path"])),
        err,
    }
}

/// `move`: the new path, then how many links were rewritten, then whether links were left.
pub fn moved(v: &Value) -> Shown {
    let mut out = format!("{}\n", text_of(&v["path"]));
    let rewritten = v["rewritten"].as_u64().unwrap_or(0);
    if rewritten > 0 {
        out.push_str(&format!("links rewritten: {rewritten}\n"));
    }
    if v["links_left"].as_bool() == Some(true) {
        out.push_str("another note still has the old name, so the links were left alone\n");
    }
    Shown::plain(out)
}

/// `show`: the file the window was asked to open.
pub fn shown(v: &Value) -> Shown {
    Shown::plain(format!("shown: {}\n", text_of(&v["shown"])))
}

/// `render --check`: `ok` and the kind when the source draws, nothing otherwise (the reason is
/// said by the command, on standard error).
pub fn check(v: &Value) -> String {
    if v["readable"].as_bool() == Some(true) {
        format!("ok {}\n", text_of(&v["kind"]))
    } else {
        String::new()
    }
}

/// `render --out FILE`: the file written, its size, and its kind.
pub fn written(file: &str, v: &Value) -> Shown {
    Shown::plain(format!(
        "wrote {file} ({}x{}, {})\n",
        field(v, "width"),
        field(v, "height"),
        field(v, "kind")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn links_lists_each_section_and_marks_what_is_missing() {
        let shown = links(&json!({
            "outgoing": [
                {"target": "B", "path": "B.md", "line": 3},
                {"target": "Gone", "path": null, "line": 5}
            ],
            "backlinks": [{"path": "A.md", "title": "A", "line": "see [[B]]  now"}],
            "tags": ["work"],
            "embeds": [{"note": "C", "heading": "Next", "path": "C.md"}],
            "pictures": [{"name": "x.png", "path": null}]
        }));
        assert_eq!(
            shown.out,
            "links:\n  B.md:3\n  Gone:5 (missing)\nbacklinks:\n  A.md: see [[B]] now\ntags:\n  #work\nembeds:\n  C.md#Next\npictures:\n  x.png (missing)\n"
        );
    }

    #[test]
    fn an_empty_section_is_left_out() {
        let shown = links(&json!({"outgoing": [], "backlinks": [], "tags": ["a"]}));
        assert_eq!(shown.out, "tags:\n  #a\n");
        assert!(links(&json!({})).out.is_empty());
    }

    #[test]
    fn a_next_page_is_named_on_standard_error() {
        let shown = list(&json!({"notes": [{"path": "a.md"}, {"path": "b.md"}], "next": "2"}));
        assert_eq!(shown.out, "a.md\nb.md\n");
        assert_eq!(shown.err, "more: --cursor 2\n");
        assert!(list(&json!({"notes": [], "next": null})).err.is_empty());
    }

    #[test]
    fn a_hit_by_name_is_its_path_alone() {
        let shown = search(&json!({"hits": [
            {"path": "A.md", "snippet": ""},
            {"path": "B.md", "snippet": "hello\n  world"}
        ]}));
        assert_eq!(shown.out, "A.md\nB.md: hello world\n");
    }

    #[test]
    fn a_move_says_what_it_rewrote_and_what_it_left() {
        assert_eq!(
            moved(&json!({"path": "N.md", "rewritten": 2, "links_left": false})).out,
            "N.md\nlinks rewritten: 2\n"
        );
        let left = moved(&json!({"path": "N.md", "rewritten": 0, "links_left": true}));
        assert_eq!(
            left.out,
            "N.md\nanother note still has the old name, so the links were left alone\n"
        );
    }

    #[test]
    fn a_check_says_ok_with_its_kind_only_when_it_draws() {
        assert_eq!(
            check(&json!({"readable": true, "kind": "flowchart"})),
            "ok flowchart\n"
        );
        assert_eq!(check(&json!({"readable": false, "kind": "other"})), "");
    }
}
