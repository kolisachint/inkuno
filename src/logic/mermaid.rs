//! Mermaid diagram kinds, for the Mermaid-in-wiki path: a ```mermaid block of
//! a note is rendered by `ui::figure::mermaid` and shown large by
//! `views::viewer::Source::Mermaid`. The canvas bridge (`to_diagram`, which
//! turned class/sequence/state diagrams into canvas diagrams) was removed
//! with the canvas editor.

/// The kind of Mermaid diagram a source is (kept so the viewer knows what
/// the block is; the canvas conversion is gone).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Class,
    Sequence,
    State,
}

/// The lines that count: no blank line, no comment, no front matter or directive.
fn body(source: &str) -> Vec<&str> {
    let mut lines = source.lines().map(str::trim);
    let mut out = Vec::new();
    let mut front = false;
    for (i, line) in lines.by_ref().enumerate() {
        if line == "---" && (i == 0 || front) {
            front = !front;
        } else if !front && !line.is_empty() && !line.starts_with("%%") {
            out.push(line);
        }
    }
    out
}

/// What kind of diagram this source is, if it is one of the three UML kinds.
pub fn kind(source: &str) -> Option<Kind> {
    let first = body(source).into_iter().next()?;
    let word = first
        .split(|c: char| c.is_whitespace() || c == ';')
        .next()?;
    match word {
        "classDiagram" | "classDiagram-v2" => Some(Kind::Class),
        "sequenceDiagram" => Some(Kind::Sequence),
        "stateDiagram" | "stateDiagram-v2" => Some(Kind::State),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kind_is_the_first_line_that_counts() {
        assert_eq!(kind("classDiagram\n A <|-- B"), Some(Kind::Class));
        assert_eq!(
            kind("%% a comment\n\n  sequenceDiagram"),
            Some(Kind::Sequence)
        );
        assert_eq!(
            kind("---\ntitle: x\n---\nstateDiagram-v2\n[*] --> A"),
            Some(Kind::State)
        );
        assert_eq!(kind("stateDiagram"), Some(Kind::State));
        // Not one of the three: a flowchart, a word, nothing.
        for source in [
            "graph TD\n A --> B",
            "flowchart LR",
            "pie title x",
            "",
            "   \n\n",
            "classDiagrams",
            "sequence",
        ] {
            assert!(kind(source).is_none(), "{source:?}");
        }
    }
}
