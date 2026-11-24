//! The file of pending changes beside a note, as its text reads and writes: `{"version": 2,
//! "changes": [{id, author, at, old, new, before, after, orphan_since}]}`. Which file holds it,
//! and when it is read or dropped, is the shell's (`platform/sidecar.rs`). Pure: no disk here.

use serde_json::{Value, json};

use crate::pending::Entry;

/// The pending changes of a list file's text. A text that is not a list reads as no changes, and so
/// does a change with a field missing or of another type. `before` and `after` are absent from a
/// version 1 list (found by their offset), and `orphan_since` from a list written before orphans
/// were kept.
pub fn parse(text: &str) -> Vec<Entry> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(list) = value.get("changes").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|change| {
            Some(Entry {
                id: u32::try_from(change.get("id")?.as_u64()?).ok()?,
                author: change.get("author")?.as_str()?.to_string(),
                at: usize::try_from(change.get("at")?.as_u64()?).ok()?,
                old: change.get("old")?.as_str()?.to_string(),
                new: change.get("new")?.as_str()?.to_string(),
                before: change
                    .get("before")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                after: change
                    .get("after")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                orphan_since: change.get("orphan_since").and_then(Value::as_u64),
            })
        })
        .collect()
}

/// The text of a list file (version 2), with the line break that ends it. The keys are written in
/// the order `serde_json` gives them, as the app always wrote them.
pub fn render(entries: &[Entry]) -> String {
    let changes: Vec<Value> = entries
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "author": e.author,
                "at": e.at,
                "old": e.old,
                "new": e.new,
                "before": e.before,
                "after": e.after,
                "orphan_since": e.orphan_since,
            })
        })
        .collect();
    // ponytail: a value of strings and numbers always renders, so the error case is not reachable.
    let text = serde_json::to_string_pretty(&json!({ "version": 2, "changes": changes }))
        .expect("a JSON value of strings and numbers renders");
    text + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: u32) -> Entry {
        Entry {
            id,
            author: "sachin".into(),
            at: 12,
            old: "old line".into(),
            new: "new line".into(),
            before: "before".into(),
            after: "after".into(),
            orphan_since: None,
        }
    }

    #[test]
    fn a_list_round_trips() {
        let mut orphan = entry(2);
        orphan.orphan_since = Some(1_700_000_000);
        let entries = vec![entry(1), orphan];
        assert_eq!(parse(&render(&entries)), entries);
    }

    #[test]
    fn the_file_is_version_2_with_a_line_break_at_the_end() {
        let text = render(&[entry(1)]);
        assert!(text.starts_with("{\n  \"changes\": ["));
        assert!(text.contains("\"version\": 2"));
        assert!(text.contains("\"orphan_since\": null"));
        assert!(text.ends_with("}\n"));
    }

    #[test]
    fn a_version_1_list_reads_without_before_and_after() {
        let text = r#"{"version":1,"changes":[{"id":3,"author":"a","at":4,"old":"x","new":"y"}]}"#;
        let found = parse(text);
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].before.as_str(), found[0].after.as_str()),
            ("", "")
        );
        assert_eq!(found[0].orphan_since, None);
    }

    #[test]
    fn a_change_with_a_missing_field_is_left_out() {
        let text = r#"{"changes":[{"id":1,"author":"a","at":0,"old":"x"},{"id":2,"author":"a","at":0,"old":"x","new":"y"}]}"#;
        let found = parse(text);
        assert_eq!(found.iter().map(|e| e.id).collect::<Vec<_>>(), [2]);
    }

    #[test]
    fn what_is_not_a_list_reads_as_none() {
        assert!(parse("not json").is_empty());
        assert!(parse(r#"{"version":2}"#).is_empty());
        assert!(parse(r#"{"changes":"no"}"#).is_empty());
    }
}
