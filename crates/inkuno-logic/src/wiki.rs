//! What a `[[link]]` means in a vault: which note it names, which notes point back at one,
//! and when a rename must leave the links alone. Pure: the shell does the writing.

use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use super::markdown::{self, Link};
use super::note::Note;

/// The note a `[[link]]` points to: by its name first, otherwise by an alias.
/// A template answers no link (`template` says which notes are templates).
pub fn resolve<'a>(
    notes: &'a [Note],
    name: &str,
    template: impl Fn(&Note) -> bool,
) -> Option<&'a Note> {
    let wanted = name.to_lowercase();
    let live = |n: &&Note| !template(n);
    notes
        .iter()
        .filter(live)
        .find(|n| n.name.to_lowercase() == wanted)
        .or_else(|| notes.iter().filter(live).find(|n| n.answers(&wanted)))
}

/// Backlinks: the notes whose `[[wikilink]]` targets note `target`, by name order. When two
/// notes have the same name, the link targets the first, and the other has no backlinks.
pub fn backlinks<'a>(notes: &'a [Note], target: &Path) -> Vec<&'a Note> {
    let Some(aimed) = notes.iter().find(|note| note.path == target) else {
        return Vec::new();
    };
    let key = aimed.name.to_lowercase();
    if notes
        .iter()
        .find(|note| note.name.to_lowercase() == key)
        .is_some_and(|first| first.path != target)
    {
        return Vec::new();
    }
    let reaches =
        |note: &&Note| note.path != target && note.links.iter().any(|link| aimed.answers(link));
    let mut found: Vec<&Note> = notes.iter().filter(reaches).collect();
    found.sort_by_cached_key(|note| note.name.to_lowercase());
    found
}

/// First line of `body` with a wikilink to note `aimed` (its name or an alias).
pub fn link_line<'a>(body: &'a str, aimed: &Note) -> Option<&'a str> {
    body.lines().find(|line| {
        let links = markdown::links(line);
        links.iter().any(
            |(_, link)| matches!(link, Link::Wiki(name) if aimed.answers(&name.to_lowercase())),
        )
    })
}

/// The notes whose links must be rewritten when `old` is renamed `new`.
/// `None` when the links stay as they are: the two names match ignoring case, or another
/// note is still called `old` (the links may be meant for it). The renamed note itself must
/// already bear `new` in `notes`.
pub fn relink_set<'a>(notes: &'a [Note], old: &str, new: &str) -> Option<Vec<&'a Note>> {
    let key = old.to_lowercase();
    if key == new.to_lowercase() || notes.iter().any(|n| n.name.to_lowercase() == key) {
        return None;
    }
    Some(notes.iter().filter(|n| n.links.contains(&key)).collect())
}

/// The note a relative address names, from note `from` of the vault `root`: a leading `/` counts
/// from the root, anything else from the folder of `from`. `.` and `..` are resolved, and `.md` is
/// added when the address has none.
pub fn note_at(root: &Path, from: &Path, address: &str) -> PathBuf {
    let base = match address.starts_with('/') {
        true => root,
        false => from.parent().unwrap_or(root),
    };
    let mut out = base.to_path_buf();
    for part in address.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            part => out.push(part),
        }
    }
    if !out.to_string_lossy().to_lowercase().ends_with(".md") {
        let mut name = out.into_os_string();
        name.push(".md");
        return name.into();
    }
    out
}

/// The components of a path, as strings (`/` first for an absolute one).
fn names(p: &Path) -> Vec<String> {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect()
}

/// Percent-encodes what would end an address early or change its meaning: spaces, `%`, `#`, brackets,
/// parentheses, `<`, `>` and `?`. With `full` (the address already writes its names escaped), every
/// byte but the unreserved ones is encoded, so the name keeps the style of the address.
fn encode(text: &str, full: bool) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let plain = match full {
            true => c.is_ascii_alphanumeric() || "-._~".contains(c),
            false => !" %#()[]<>?".contains(c),
        };
        if plain {
            out.push(c);
        } else {
            for byte in c.to_string().bytes() {
                out += &format!("%{byte:02X}");
            }
        }
    }
    out
}

/// The address that names note `target` from note `from` of the vault `root`, as `note_at` reads it:
/// from the root with a leading `/` when `absolute`, else relative to the folder of `from`. Names are
/// percent-encoded (see `encode`).
fn address_of(root: &Path, from: &Path, target: &Path, absolute: bool) -> String {
    let address = if absolute {
        format!(
            "/{}",
            names(target.strip_prefix(root).unwrap_or(target)).join("/")
        )
    } else {
        // Up from the folder of `from` to the common folder, then down to `target`.
        let dir = names(from.parent().unwrap_or(root));
        let there = names(target);
        let common = dir.iter().zip(&there).take_while(|(a, b)| a == b).count();
        let ups = vec![".."; dir.len() - common];
        [ups, there[common..].iter().map(String::as_str).collect()]
            .concat()
            .join("/")
    };
    encode(&address, false)
}

/// The Markdown link `[title](address)` that a picker writes: the address to note `target` from the
/// folder `folder` of the vault `root`, as `address_of` writes it (relative, names encoded). The
/// brackets of the title are escaped, so that the label holds them.
pub fn link_to(title: &str, root: &Path, folder: &Path, target: &Path) -> String {
    let from = folder.join("_");
    let label = title.replace('[', "\\[").replace(']', "\\]");
    format!("[{label}]({})", address_of(root, &from, target, false))
}

/// The address `written` (a path part of a link, anchor excluded) with the note it names moved to
/// `new`, seen from note `linker` of the vault `root`. Each segment that still names the same folder or
/// file keeps its spelling (`./`, a doubled `.`, `%20`); the changed ones take the new names, written
/// in the style of the address.
fn respell(root: &Path, linker: &Path, written: &str, new: &Path) -> String {
    let absolute = written.starts_with('/');
    let base = match absolute {
        true => root,
        false => linker.parent().unwrap_or(root),
    };
    let parts: Vec<&str> = written.split('/').collect();
    // The walk of `written` from the base, each entry with the index of its part when it has one.
    let mut walk: Vec<(String, Option<usize>)> =
        names(base).into_iter().map(|name| (name, None)).collect();
    let base_len = walk.len();
    for (i, part) in parts.iter().enumerate() {
        match *part {
            "" | "." => {}
            ".." => {
                walk.pop();
            }
            part => walk.push((markdown::percent_decode(part), Some(i))),
        }
    }
    // As `note_at` reads it: a name without `.md` is a note all the same.
    let mut walked: Vec<String> = walk.iter().map(|(name, _)| name.clone()).collect();
    if let Some(last) = walked
        .last_mut()
        .filter(|name| !name.to_lowercase().ends_with(".md"))
    {
        last.push_str(".md");
    }
    let target = names(new);
    let common = walked
        .iter()
        .zip(&target)
        .take_while(|(a, b)| a == b)
        .count();
    if common >= walked.len() {
        return written.to_string();
    }
    let full = written.contains('%');
    let (mut out, from) = match common < base_len {
        // Up from the base to the common folder, then down to the note.
        true => (vec![String::from(".."); base_len - common], common),
        // The folders above the first changed part keep their spelling.
        false => (
            parts[..walk[common].1.unwrap_or(0)]
                .iter()
                .map(|part| part.to_string())
                .collect(),
            common,
        ),
    };
    let last_written = parts
        .last()
        .is_some_and(|part| part.to_lowercase().ends_with(".md"));
    for (p, name) in target.iter().enumerate().skip(from) {
        let kept = match walked.get(p) == Some(name) {
            true => walk[p].1,
            false => None,
        };
        out.push(match kept {
            Some(i) => parts[i].to_string(),
            None => {
                let name = match p + 1 == target.len() && !last_written {
                    true => name.strip_suffix(".md").unwrap_or(name),
                    false => name.as_str(),
                };
                encode(name, full)
            }
        });
    }
    out.join("/")
}

/// The text where the relative links of note `linker` (in the vault `root`) that name the file `old`
/// name `new` instead: a note moved or renamed. Each address keeps its spelling except the parts that
/// changed (see `respell`), and its anchor. `None` if no link changes.
// ponytail: only Markdown links are rewritten here; the `[[name]]` links follow `markdown::relink`.
pub fn relink_file(
    text: &str,
    root: &Path,
    linker: &Path,
    old: &Path,
    new: &Path,
) -> Option<String> {
    if old == new {
        return None;
    }
    let meta = markdown::front_matter(text);
    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    for (at, line) in markdown::prose_lines(&text[meta..]) {
        for (r, link) in markdown::links(line) {
            let Link::Note { path, .. } = link else {
                continue;
            };
            if note_at(root, linker, &path) != old {
                continue;
            }
            let Some((_, url)) = markdown::web_link(&line[r.clone()]) else {
                continue;
            };
            let whole = meta + at + r.start;
            let address = &text[whole + url.start..whole + url.end];
            let written = address.split('#').next().unwrap_or(address);
            let fresh = respell(root, linker, written, new);
            if fresh != written {
                let start = whole + url.start;
                edits.push((start..start + written.len(), fresh));
            }
        }
    }
    if edits.is_empty() {
        return None;
    }
    let mut out = text.to_string();
    for (range, fresh) in edits.into_iter().rev() {
        out.replace_range(range, &fresh);
    }
    Some(out)
}

/// The text of note `linker` with its links to the renamed note `old` rewritten to `new`: the
/// `[[links]]` when `wiki` (see `markdown::relink`), and the relative addresses when a note moved
/// from `moved.0` to `moved.1` (see `relink_file`). `None` if nothing changes. Shared by the app
/// and the command line, so that both rename alike.
pub fn rewrite_links(
    text: &str,
    root: Option<&Path>,
    linker: &Path,
    old: &str,
    new: &str,
    wiki: bool,
    moved: Option<(&Path, &Path)>,
) -> Option<String> {
    let mut fresh = text.to_string();
    if wiki && let Some(rewritten) = markdown::relink(&fresh, old, new) {
        fresh = rewritten;
    }
    if let (Some((from, to)), Some(root)) = (moved, root)
        && let Some(rewritten) = relink_file(&fresh, root, linker, from, to)
    {
        fresh = rewritten;
    }
    (fresh != text).then_some(fresh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::Arc, time::SystemTime};

    fn note(name: &str, links: &[&str]) -> Note {
        Note {
            name: name.into(),
            path: PathBuf::from(format!("/v/{name}.md")),
            tags: Arc::from([]),
            links: links.iter().map(|l| (*l).to_string()).collect(),
            aliases: Arc::from([]),
            mtime: SystemTime::UNIX_EPOCH,
            body: "".into(),
            lower: "".into(),
        }
    }

    #[test]
    fn lists_backlinks() {
        let mut notes = vec![
            note("Cible", &[]),
            note("b", &["cible"]),
            note("A", &["cible", "b"]),
            note("Seule", &["cible"]),
        ];
        notes[3].path = PathBuf::from("/v/sous/Cible.md");
        notes[3].name = "Cible".into();
        let names = |target: &str| {
            backlinks(&notes, Path::new(target))
                .iter()
                .map(|n| n.name.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(names("/v/Cible.md"), ["A", "b", "Cible"]);
        assert_eq!(names("/v/b.md"), ["A"]);
        // A second note with the same name: links target the first.
        assert!(names("/v/sous/Cible.md").is_empty() && names("/v/absente.md").is_empty());
        let body = "# A\n[[autre]] et [[Cible]]x\n| t | [[cible|alias]] |\n";
        assert_eq!(link_line(body, &notes[0]), Some("[[autre]] et [[Cible]]x"));
        assert_eq!(
            link_line("un [[Cible|alias]] là", &notes[0]),
            Some("un [[Cible|alias]] là")
        );
        assert_eq!(link_line(body, &notes[1]), None);
        // A front matter alias counts as the name.
        let mut notes = vec![note("Appli", &[]), note("Journal", &["inkuno app"])];
        notes[0].aliases = vec!["Inkuno App".into()].into();
        assert_eq!(backlinks(&notes, Path::new("/v/Appli.md")).len(), 1);
        assert_eq!(
            link_line("vu [[inkuno app]]", &notes[0]),
            Some("vu [[inkuno app]]")
        );
    }

    #[test]
    fn resolves_name_then_alias_and_skips_templates() {
        let mut notes = vec![
            note("Réunion", &[]),
            note("Journal", &[]),
            note("Modèle", &[]),
        ];
        notes[2].path = PathBuf::from("/v/Templates/Réunion.md");
        notes[2].name = "Réunion".into();
        notes[2].aliases = vec!["modele".into()].into();
        let template = |n: &Note| n.path.starts_with("/v/Templates");
        // The note wins over a template of the same name. An alias of the template answers nothing.
        assert_eq!(
            resolve(&notes, "réunion", template).map(|n| n.path.clone()),
            Some(PathBuf::from("/v/Réunion.md"))
        );
        assert!(resolve(&notes, "modele", template).is_none());
        notes[1].aliases = vec!["carnet".into()].into();
        assert_eq!(resolve(&notes, "carnet", template).unwrap().name, "Journal");
        // The name wins over an alias of another note.
        notes[0].aliases = vec!["autre".into()].into();
        assert_eq!(
            resolve(&notes, "Réunion", template).unwrap().name,
            "Réunion"
        );
        assert_eq!(resolve(&notes, "autre", |_| false).unwrap().name, "Réunion");
        assert!(resolve(&notes, "absente", template).is_none());
        // Alone, a template of that name answers no link.
        let only = vec![notes.pop().unwrap()];
        assert!(resolve(&only, "Réunion", template).is_none());
    }

    #[test]
    fn leaves_links_when_the_old_name_remains() {
        let notes = vec![
            note("Nouveau", &[]),
            note("Ancien", &[]),
            note("Autre", &["ancien"]),
        ];
        assert!(relink_set(&notes, "Ancien", "Nouveau").is_none());
        assert!(relink_set(&notes, "Nouveau", "nouveau").is_none());
        let notes = vec![note("Nouveau", &[]), note("Autre", &["ancien"])];
        let set = relink_set(&notes, "Ancien", "Nouveau").unwrap();
        assert_eq!(set.len(), 1);
        assert_eq!(set[0].name, "Autre");
    }
}

/// What `![[…]]` points at in its note: the note's name and what of it. Moved here from the
/// removed `logic/embed.rs`: the editor no longer renders embedded sections (a `![[Note]]` line
/// reads as plain link text), but renaming a heading and the command line (`read --section`,
/// `links`) still resolve them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Anchor {
    /// The whole note, without its front matter.
    Whole,
    /// A heading, and what is under it up to the next heading of the same level or above.
    Heading(String),
    /// A block marked with `^id` at its end.
    Block(String),
}

/// An embed: the note's name and what of it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub note: String,
    pub anchor: Anchor,
}

impl Target {
    /// How the frame names its source: `Note`, `Note › Heading`, `Note › ^id`.
    pub fn label(&self) -> String {
        match &self.anchor {
            Anchor::Whole => self.note.clone(),
            Anchor::Heading(h) => format!("{} › {h}", self.note),
            Anchor::Block(id) => format!("{} › ^{id}", self.note),
        }
    }

    /// The target as it is written between the brackets.
    #[cfg(test)]
    pub fn written(&self) -> String {
        match &self.anchor {
            Anchor::Whole => self.note.clone(),
            Anchor::Heading(h) => format!("{}#{h}", self.note),
            Anchor::Block(id) => format!("{}#^{id}", self.note),
        }
    }
}

/// The embed a line is made of, if it is only that: `![[Note#Heading]]`, spaces around it allowed.
/// An alias after `|` is left out. Whether `Note` is a note or a picture is for the caller to say.
pub fn parse(line: &str) -> Option<Target> {
    let inner = line.trim().strip_prefix("![[")?.strip_suffix("]]")?;
    link_target(inner)
}

/// What `[[…]]` points at, from what is between the brackets: `Note`, `Note#Heading`, `Note#^id`, an alias
/// after `|` left out. `None` for an empty name or a nested bracket.
pub fn link_target(inner: &str) -> Option<Target> {
    if inner.contains(['[', ']']) {
        return None;
    }
    let inner = inner.split('|').next()?.trim();
    let (note, anchor) = match inner.split_once('#') {
        Some((note, rest)) => (note.trim(), rest.trim()),
        None => (inner, ""),
    };
    if note.is_empty() {
        return None;
    }
    let anchor = match anchor.strip_prefix('^') {
        Some(id) if !id.is_empty() => Anchor::Block(id.to_string()),
        _ if anchor.is_empty() => Anchor::Whole,
        _ => Anchor::Heading(anchor.to_string()),
    };
    Some(Target {
        note: note.to_string(),
        anchor,
    })
}

/// The id a line ends with, `text ^id`: its range in the line. The id is letters, digits and `-`, and
/// stands alone at the end.
fn block_mark(line: &str) -> Option<Range<usize>> {
    let end = line.trim_end().len();
    let caret = line[..end].rfind('^')?;
    let id = &line[caret + 1..end];
    let alone = caret == 0 || line[..caret].ends_with([' ', '\t']);
    (alone && !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .then_some(caret..end)
}

/// The ids of the blocks of the text, in order, to offer after `^`.
pub fn block_ids(text: &str) -> Vec<&str> {
    markdown::prose_lines(&text[markdown::front_matter(text)..])
        .into_iter()
        .filter_map(|(_, l)| block_mark(l).map(|r| &l[r.start + 1..r.end]))
        .collect()
}

/// Byte range of what `anchor` shows in `text`, and whether the last line carries the `^id`
/// to leave out. `None` if the heading or the block is not there.
fn find(text: &str, anchor: &Anchor) -> Option<Range<usize>> {
    let meta = markdown::front_matter(text);
    match anchor {
        Anchor::Whole => Some(meta..text.len()),
        Anchor::Heading(wanted) => {
            let heads = markdown::headings(text);
            let k = heads.iter().position(|(_, label, _)| {
                label.trim().eq_ignore_ascii_case(wanted.trim())
                    || label.trim().to_lowercase() == wanted.trim().to_lowercase()
            })?;
            let (level, _, start) = heads[k];
            let end = heads[k + 1..]
                .iter()
                .find(|(l, ..)| *l <= level)
                .map_or(text.len(), |(.., at)| *at);
            Some(start..end)
        }
        Anchor::Block(id) => {
            let all = markdown::prose_lines(&text[meta..]);
            let at = all
                .iter()
                .position(|(_, l)| block_mark(l).is_some_and(|r| &l[r.start + 1..r.end] == id))?;
            // A mark alone on its line is that of the block just above.
            let alone =
                block_mark(all[at].1).is_some_and(|r| all[at].1[..r.start].trim().is_empty());
            let last = if alone { at.checked_sub(1)? } else { at };
            let mut first = last;
            while first > 0
                && !all[first - 1].1.trim().is_empty()
                && all[first - 1].0 + all[first - 1].1.len() + 1 == all[first].0
            {
                first -= 1;
            }
            if all[last].1.trim().is_empty() {
                return None;
            }
            Some(meta + all[first].0..meta + all[at].0 + all[at].1.len())
        }
    }
}

/// What the embed shows of `text`: the section, the front matter and the blank lines around it
/// and the `^id` of a block left out. `None` if the heading or the block is not there.
pub fn section(text: &str, anchor: &Anchor) -> Option<String> {
    let range = find(text, anchor)?;
    let mut shown = text[range].trim_matches('\n').trim_end().to_string();
    if let Anchor::Block(_) = anchor {
        // The mark goes, with the line it was alone on.
        let last_start = shown.rfind('\n').map_or(0, |i| i + 1);
        if let Some(mark) = block_mark(&shown[last_start..]) {
            shown.truncate(last_start + mark.start);
            shown = shown.trim_end().to_string();
        }
    }
    (!shown.trim().is_empty()).then_some(shown)
}

/// The byte where the embed's target starts in `text`: the heading, the block, or the top of the note
/// after its front matter. To open the note there.
pub fn position(text: &str, anchor: &Anchor) -> Option<usize> {
    find(text, anchor).map(|r| r.start)
}

/// Writes `![[note#old]]` and `[[note#old]]` as `…#new`, for every link of `text` that points at heading
/// `old` of note `note`: the new text, and how many. (A renamed note is `markdown::relink`.)
pub fn rename_heading(text: &str, note: &str, old: &str, new: &str) -> (String, usize) {
    rewrite(text, |name, anchor| {
        (name.eq_ignore_ascii_case(note)
            && anchor.is_some_and(|a| a.eq_ignore_ascii_case(old) && !a.starts_with('^')))
        .then(|| (name.to_string(), Some(new.to_string())))
    })
}

/// Rewrites the wikilinks of `text` (`[[…]]`, `![[…]]`) by what `change` says for their note and anchor:
/// a new note name and a new anchor; `None` leaves the link alone. Links in code blocks and inline code
/// are left alone.
fn rewrite(
    text: &str,
    change: impl Fn(&str, Option<&str>) -> Option<(String, Option<String>)>,
) -> (String, usize) {
    let (mut out, mut count) = (String::with_capacity(text.len()), 0);
    let blocks = markdown::blocks(text);
    let mut at = 0;
    for (n, line) in text.split('\n').enumerate() {
        if n > 0 {
            out.push('\n');
        }
        let start = at;
        at += line.len() + 1;
        if markdown::is_fence(line) || markdown::open_at(&blocks, start).is_some() {
            out.push_str(line);
            continue;
        }
        let mut flags = vec![0u32; line.len()];
        markdown::inline(line, 0, &mut flags);
        let mut done = 0;
        let mut from = 0;
        while let Some(open) = line[from..].find("[[").map(|i| i + from) {
            let Some(close) = line[open..].find("]]").map(|i| i + open) else {
                break;
            };
            from = close + 2;
            if flags[open] & markdown::CODE != 0 {
                continue;
            }
            let inner = &line[open + 2..close];
            let (target, alias) = inner
                .split_once('|')
                .map_or((inner, None), |(t, a)| (t, Some(a)));
            let (name, anchor) = target
                .split_once('#')
                .map_or((target, None), |(n, a)| (n, Some(a)));
            if let Some((new_name, new_anchor)) = change(name.trim(), anchor.map(str::trim)) {
                // Spaces around the name are kept as they were written.
                let lead = &name[..name.len() - name.trim_start().len()];
                let tail = &name[name.trim_end().len()..];
                let mut rewritten = format!("{lead}{new_name}{tail}");
                if let Some(a) = new_anchor {
                    rewritten.push('#');
                    rewritten.push_str(&a);
                }
                if let Some(alias) = alias {
                    rewritten.push('|');
                    rewritten.push_str(alias);
                }
                if rewritten != inner {
                    out.push_str(&line[done..open + 2]);
                    out.push_str(&rewritten);
                    done = close;
                    count += 1;
                }
            }
        }
        out.push_str(&line[done..]);
    }
    (out, count)
}

#[cfg(test)]
mod embed_tests {
    use super::*;

    fn target(note: &str, anchor: Anchor) -> Option<Target> {
        Some(Target {
            note: note.into(),
            anchor,
        })
    }

    #[test]
    fn reads_an_embed_line() {
        assert_eq!(parse("![[Note]]"), target("Note", Anchor::Whole));
        assert_eq!(
            parse("  ![[Note#Un titre]]  "),
            target("Note", Anchor::Heading("Un titre".into()))
        );
        assert_eq!(
            parse("![[ Note # Un titre | alias ]]"),
            target("Note", Anchor::Heading("Un titre".into()))
        );
        assert_eq!(
            parse("![[Note#^abc-1]]"),
            target("Note", Anchor::Block("abc-1".into()))
        );
        assert_eq!(
            parse("![[Note#^]]"),
            target("Note", Anchor::Heading("^".into()))
        );
        assert_eq!(
            parse("![[Dossier/Note]]"),
            target("Dossier/Note", Anchor::Whole)
        );
        for line in [
            "[[Note]]",
            "texte ![[Note]]",
            "![[Note]] texte",
            "![[]]",
            "![[#Titre]]",
            "![[a]] ![[b]]",
            "![](a.png)",
            "",
            "![[a[b]]",
            "- ![[Note]]",
        ] {
            assert_eq!(parse(line), None, "{line:?}");
        }
        let t = parse("![[Note#Titre]]").unwrap();
        assert_eq!(
            (t.label(), t.written()),
            ("Note › Titre".into(), "Note#Titre".into())
        );
    }

    const NOTE: &str = "---\ntags: [a]\n---\n# Titre\n\nIntro.\n\n## Un\n\ntexte un ^u1\n\n### Sous-un\n\nsous.\n\n## Deux\n\n```\n# pas un titre\n```\n\ndeux.\n\n# Autre\n\nfin ^fin-1\n";

    #[test]
    fn shows_a_section() {
        let s = |a: Anchor| section(NOTE, &a);
        // The heading and what is under it, up to the next heading of the same level or above.
        assert_eq!(
            s(Anchor::Heading("Un".into())).unwrap(),
            "## Un\n\ntexte un ^u1\n\n### Sous-un\n\nsous."
        );
        assert_eq!(
            s(Anchor::Heading("un".into())).unwrap(),
            s(Anchor::Heading(" UN ".into())).unwrap()
        );
        assert_eq!(
            s(Anchor::Heading("Sous-un".into())).unwrap(),
            "### Sous-un\n\nsous."
        );
        assert_eq!(
            s(Anchor::Heading("Deux".into())).unwrap(),
            "## Deux\n\n```\n# pas un titre\n```\n\ndeux."
        );
        assert_eq!(
            s(Anchor::Heading("Titre".into())).unwrap().lines().last(),
            Some("deux.")
        );
        assert_eq!(
            s(Anchor::Heading("Autre".into())).unwrap(),
            "# Autre\n\nfin ^fin-1"
        );
        // Without its front matter, the whole note.
        assert!(
            s(Anchor::Whole).unwrap().starts_with("# Titre")
                && s(Anchor::Whole).unwrap().ends_with("fin ^fin-1")
        );
        // A block, without its mark.
        assert_eq!(s(Anchor::Block("u1".into())).unwrap(), "texte un");
        assert_eq!(s(Anchor::Block("fin-1".into())).unwrap(), "fin");
        for missing in [
            Anchor::Heading("Absent".into()),
            Anchor::Heading("pas un titre".into()),
            Anchor::Block("zzz".into()),
        ] {
            assert_eq!(s(missing), None);
        }
        assert_eq!(section("", &Anchor::Whole), None);
        assert_eq!(section("---\na: b\n---\n", &Anchor::Whole), None);
        assert_eq!(block_ids(NOTE), ["u1", "fin-1"]);
        assert_eq!(
            position(NOTE, &Anchor::Heading("Un".into())),
            Some(NOTE.find("## Un").unwrap())
        );
        assert_eq!(
            position(NOTE, &Anchor::Whole),
            Some(NOTE.find("# Titre").unwrap())
        );
    }

    #[test]
    fn finds_blocks() {
        // A block is the lines around its mark; a mark alone on its line is that of the block above.
        let text = "a\nb ^x\nc\n\nlist\n- un\n- deux\n^liste\n\nfin ^y";
        assert_eq!(section(text, &Anchor::Block("x".into())).unwrap(), "a\nb");
        assert_eq!(
            section(text, &Anchor::Block("liste".into())).unwrap(),
            "list\n- un\n- deux"
        );
        assert_eq!(section(text, &Anchor::Block("y".into())).unwrap(), "fin");
        for text in ["a^b", "a ^", "a ^b c", "^a\n", "x ^é"] {
            assert_eq!(section(text, &Anchor::Block("b".into())), None, "{text:?}");
        }
        assert_eq!(
            section("```\ncode ^x\n```\n", &Anchor::Block("x".into())),
            None
        );
        assert_eq!(block_ids("un ^a\n```\nx ^b\n```\n^c"), ["a", "c"]);
        // A tilde line in a backtick block is code, and a backtick line in a tilde block too.
        assert_eq!(block_ids("```md\n~~~\nx ^a\n~~~\n```\nz ^b"), ["b"]);
        assert_eq!(block_ids("~~~\nx ^a\n```\n~~~\nz ^b"), ["b"]);
        // Line endings and accents.
        assert_eq!(
            section("# É\r\n\r\ntexte ^é1\r\n", &Anchor::Heading("é".into())).unwrap(),
            "# É\r\n\r\ntexte ^é1"
        );
    }

    #[test]
    fn rewrites_links_when_a_heading_is_renamed() {
        let (out, n) = rename_heading(
            "![[N#Un]] [[N#un|a]] [[N#Deux]] [[M#Un]] ![[N#^Un]] [[N]]",
            "N",
            "Un",
            "Premier",
        );
        assert_eq!(
            (out.as_str(), n),
            (
                "![[N#Premier]] [[N#Premier|a]] [[N#Deux]] [[M#Un]] ![[N#^Un]] [[N]]",
                2
            )
        );
        assert_eq!(
            rename_heading("rien", "N", "a", "b"),
            ("rien".to_string(), 0)
        );
        // Code is left alone, spaces around the name are kept, the alias too.
        let text = "`[[N#Un]]` ![[ N #Un|x]]\n```\n![[N#Un]]\n```\n";
        assert_eq!(
            rename_heading(text, "N", "Un", "Z"),
            (
                "`[[N#Un]]` ![[ N #Z|x]]\n```\n![[N#Un]]\n```\n".to_string(),
                1
            )
        );
        // A tilde line inside a backtick block is code, and the block goes on to its own closing fence.
        let mixed = "```md\n~~~\n![[N#Un]]\n~~~\n```\n![[N#Un]]\n";
        assert_eq!(
            rename_heading(mixed, "N", "Un", "Z"),
            ("```md\n~~~\n![[N#Un]]\n~~~\n```\n![[N#Z]]\n".to_string(), 1)
        );
        let tilde = "~~~\n![[N#Un]]\n```\n~~~\n![[N#Un]]";
        assert_eq!(
            rename_heading(tilde, "N", "Un", "Z"),
            ("~~~\n![[N#Un]]\n```\n~~~\n![[N#Z]]".to_string(), 1)
        );
    }

    #[test]
    fn a_picker_link_is_relative_to_the_folder_of_the_note() {
        let root = Path::new("/v");
        assert_eq!(
            link_to("Plan", root, root, &root.join("Plan.md")),
            "[Plan](Plan.md)"
        );
        assert_eq!(
            link_to("Plan", root, &root.join("Idées"), &root.join("Plan.md")),
            "[Plan](../Plan.md)"
        );
        assert_eq!(
            link_to("Note x", root, &root.join("A"), &root.join("B c/Note x.md")),
            "[Note x](../B%20c/Note%20x.md)"
        );
    }

    #[test]
    fn resolves_relative_addresses_and_follows_renames() {
        let root = Path::new("/v");
        let from = Path::new("/v/sous/Lien.md");
        assert_eq!(note_at(root, from, "Note.md"), Path::new("/v/sous/Note.md"));
        assert_eq!(
            note_at(root, from, "../Note%20A.md"),
            Path::new("/v/Note%20A.md")
        );
        assert_eq!(
            note_at(root, from, "./a/../b.md"),
            Path::new("/v/sous/b.md")
        );
        assert_eq!(note_at(root, from, "/x/y"), Path::new("/v/x/y.md"));
        assert_eq!(
            note_at(root, from, "Note.v2"),
            Path::new("/v/sous/Note.v2.md")
        );
        // A rename in the same folder keeps the form of each address, its anchor too.
        let (old, new) = (Path::new("/v/sous/Vieux.md"), Path::new("/v/sous/Neuf.md"));
        let text = "# A\n[x](Vieux.md) [y](/sous/Vieux.md#Titre) [z](../sous/Vieux.md) [w](Autre.md)\n\
                    ```\n[c](Vieux.md)\n```\n";
        let want = "# A\n[x](Neuf.md) [y](/sous/Neuf.md#Titre) [z](../sous/Neuf.md) [w](Autre.md)\n\
                    ```\n[c](Vieux.md)\n```\n";
        assert_eq!(
            relink_file(text, root, from, old, new).as_deref(),
            Some(want)
        );
        // A move to another folder: the address is written from the linking note's folder.
        let moved = Path::new("/v/Dossier/Neuf.md");
        let got = relink_file("[x](Vieux.md) [y](Vieux.md#a%20b)", root, from, old, moved).unwrap();
        assert_eq!(got, "[x](../Dossier/Neuf.md) [y](../Dossier/Neuf.md#a%20b)");
        assert_eq!(relink_file("[w](Autre.md)", root, from, old, new), None);
        // The same name on both sides changes nothing, whatever the links say.
        assert_eq!(relink_file("[x](Vieux.md)", root, from, old, old), None);
    }

    #[test]
    fn a_rename_keeps_the_spelling_of_the_address() {
        let root = Path::new("/v");
        let from = Path::new("/v/Lien.md");
        let (old, new) = (
            Path::new("/v/sub/Target.md"),
            Path::new("/v/sub/Renamed.md"),
        );
        let text = "[t](./sub/Target.md) [u](sub/./Target.md) [a](<sub/Target.md>)";
        assert_eq!(
            relink_file(text, root, from, old, new).as_deref(),
            Some("[t](./sub/Renamed.md) [u](sub/./Renamed.md) [a](<sub/Target.md>)")
        );
        // Percent-encoded names stay encoded; the other segments keep their spelling.
        let (old, new) = (Path::new("/v/Café.md"), Path::new("/v/Café 2.md"));
        assert_eq!(
            relink_file("[a](./Caf%C3%A9.md#Un%20titre)", root, from, old, new).as_deref(),
            Some("[a](./Caf%C3%A9%202.md#Un%20titre)")
        );
        // A folder that moves keeps the spelling of the folders that stay.
        let (old, new) = (
            Path::new("/v/sub/Target.md"),
            Path::new("/v/other/Target.md"),
        );
        assert_eq!(
            relink_file("[t](./sub/Target)", root, from, old, new).as_deref(),
            Some("[t](./other/Target)")
        );
    }

    #[test]
    fn a_picker_link_resolves_to_its_note() {
        let root = Path::new("/v");
        let names = [
            "C# tips",
            "Plan (v2",
            "Plan (v2)",
            "a]b[c",
            "100% done",
            "x y",
            "é?",
        ];
        for name in names {
            let target = root.join("sub").join(format!("{name}.md"));
            for folder in [root.join("sub"), root.join("sub/deep"), root.join("other")] {
                let line = link_to(name, root, &folder, &target);
                let linker = folder.join("Lien.md");
                let found = markdown::links(&line)
                    .into_iter()
                    .find_map(|(_, link)| match link {
                        Link::Note { path, .. } => Some(note_at(root, &linker, &path)),
                        _ => None,
                    });
                assert_eq!(found.as_deref(), Some(target.as_path()), "{line}");
            }
        }
        assert_eq!(
            link_to("C# tips", root, root, &root.join("C# tips.md")),
            "[C# tips](C%23%20tips.md)"
        );
    }

    #[test]
    fn a_rename_writes_a_link_that_resolves_to_the_new_note() {
        let root = Path::new("/v");
        let from = Path::new("/v/Lien.md");
        let (old, new) = (
            Path::new("/v/Vieux.md"),
            Path::new("/v/Plan (v2) [x] 100%.md"),
        );
        let text = "[v](Vieux.md)";
        let fresh = relink_file(text, root, from, old, new).unwrap();
        let found = markdown::links(&fresh)
            .into_iter()
            .find_map(|(_, link)| match link {
                Link::Note { path, .. } => Some(note_at(root, from, &path)),
                _ => None,
            });
        assert_eq!(found.as_deref(), Some(new));
    }
}
