//! Reading and writing notes. The rules are the app's: a taken name is refused, a write is
//! atomic, a rename rewrites links unless another note still has the old name, and nothing
//! is deleted.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::{
    lang::{date_name, time_name},
    logic::wiki::Anchor,
    logic::{assistant, daily, markdown, markdown::Link, pending, rank, revision},
    platform::{clock, link, revisions, vault as files},
};

use super::{
    outcome::{Outcome, arg_opt, arg_str, arg_usize, disk},
    store::{self as vault, Vault},
};

pub fn vault_info(vault: &Vault) -> Outcome {
    let mut tags: Vec<(String, usize)> = Vec::new();
    for note in vault.notes.iter().filter(|n| !vault.is_template(n)) {
        for tag in note.tags.iter() {
            if let Some((_, count)) = tags.iter_mut().find(|(name, _)| name == tag) {
                *count += 1;
            } else {
                tags.push((tag.clone(), 1));
            }
        }
    }
    tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    tags.truncate(12);
    let folders: Vec<String> = {
        let mut dirs: Vec<String> = vault
            .dirs
            .iter()
            .map(|dir| vault::rel(&vault.root, dir))
            .filter(|dir| !dir.is_empty())
            .collect();
        dirs.sort();
        dirs
    };
    Outcome::ok(json!({
        "folder": vault.root.display().to_string(),
        "notes": vault.notes.iter().filter(|n| !vault.is_template(n)).count(),
        "templates": vault.notes.iter().filter(|n| vault.is_template(n)).count(),
        "diagrams": vault.diagrams.len(),
        "tables": vault.tables.len(),
        "pictures": vault.pictures.len(),
        "attachments": vault.attachments.len(),
        "folders": folders,
        "tags": tags.into_iter().map(|(tag, count)| json!({"tag": tag, "count": count})).collect::<Vec<_>>(),
        "daily": date_name(clock::today()),
        "version": env!("CARGO_PKG_VERSION"),
        "unreadable": vault.unreadable.iter().map(|(path, kind)| json!({
            "path": vault::rel(&vault.root, path),
            "error": disk(std::io::Error::new(*kind, "")),
        })).collect::<Vec<_>>(),
    }))
}

pub fn list_notes(vault: &Vault, args: &Value) -> Outcome {
    let kind = arg_opt(args, "kind").unwrap_or("all");
    let sort = arg_opt(args, "sort").unwrap_or("name");
    if !matches!(
        kind,
        "note" | "diagram" | "table" | "picture" | "attachment" | "all"
    ) {
        return Outcome::err(format!("unknown kind `{kind}`"));
    }
    if !matches!(sort, "name" | "recent") {
        return Outcome::err(format!("unknown sort `{sort}`"));
    }
    let scope = match scope(vault, args) {
        Ok(scope) => scope,
        Err(message) => return Outcome::err(message),
    };
    let limit = arg_usize(args, "limit").unwrap_or(100).clamp(1, 200);
    let cursor: usize = arg_opt(args, "cursor")
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);
    let mut rows = Vec::new();
    if matches!(kind, "note" | "all") {
        for note in &vault.notes {
            if !kept(vault, &note.path, &note.tags, scope.as_deref(), args) {
                continue;
            }
            rows.push((
                note.name.clone(),
                vault::unix(note.mtime),
                json!({
                    "path": vault::rel(&vault.root, &note.path),
                    "title": note.name,
                    "folder": vault::folder(&vault.root, &note.path),
                    "tags": note.tags,
                    "modified": vault::unix(note.mtime),
                    "kind": "note",
                }),
            ));
        }
    }
    let files: [(&str, &[std::path::PathBuf]); 4] = [
        ("diagram", &vault.diagrams),
        ("table", &vault.tables),
        ("picture", &vault.pictures),
        ("attachment", &vault.attachments),
    ];
    for (file_kind, paths) in files {
        if kind != "all" && kind != file_kind {
            continue;
        }
        for path in paths {
            if !kept(vault, path, &[], scope.as_deref(), args) {
                continue;
            }
            let modified = path
                .metadata()
                .and_then(|m| m.modified())
                .map(vault::unix)
                .unwrap_or(0);
            let title = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            rows.push((
                title.clone(),
                modified,
                json!({
                    "path": vault::rel(&vault.root, path),
                    "title": title,
                    "folder": vault::folder(&vault.root, path),
                    "tags": [],
                    "modified": modified,
                    "kind": file_kind,
                }),
            ));
        }
    }
    if sort == "recent" {
        rows.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| files::natural(&a.0).cmp(&files::natural(&b.0)))
        });
    } else {
        rows.sort_by_key(|a| files::natural(&a.0));
    }
    let next = (cursor + limit < rows.len()).then(|| (cursor + limit).to_string());
    let notes: Vec<Value> = rows
        .into_iter()
        .skip(cursor)
        .take(limit)
        .map(|(_, _, row)| row)
        .collect();
    Outcome::ok(json!({"notes": notes, "next": next}))
}

/// The folder a listing keeps, as a path relative to the vault (`None` for the whole vault). The
/// folder is read the way `new` and `move` read theirs, so `.`, `./ideas` and `ideas/` agree.
fn scope(vault: &Vault, args: &Value) -> Result<Option<String>, String> {
    let Some(folder) = args.get("folder").and_then(Value::as_str) else {
        return Ok(None);
    };
    let dir = vault.inside(folder, false)?;
    if !dir.is_dir() {
        return Err(format!("no folder `{folder}`"));
    }
    let rel = vault::rel(&vault.root, &dir);
    Ok((!rel.is_empty()).then_some(rel))
}

fn kept(vault: &Vault, path: &Path, tags: &[String], scope: Option<&str>, args: &Value) -> bool {
    if let Some(folder) = scope {
        let dir = vault::folder(&vault.root, path);
        if dir != folder && !dir.starts_with(&format!("{folder}/")) {
            return false;
        }
    }
    if let Some(tag) = arg_opt(args, "tag") {
        let tag = tag.trim_start_matches('#');
        if !tags.iter().any(|t| t == tag) {
            return false;
        }
    }
    true
}

pub fn search_notes(vault: &Vault, args: &Value) -> Outcome {
    let query = match arg_str(args, "query") {
        Ok(query) => query,
        Err(message) => return Outcome::err(message),
    };
    let scope = match scope(vault, args) {
        Ok(scope) => scope,
        Err(message) => return Outcome::err(message),
    };
    let limit = arg_usize(args, "limit").unwrap_or(20).clamp(1, 50);
    let q = query.to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();
    let mut scored = Vec::new();
    let mut named = vec![false; vault.notes.len()];
    for (i, note) in vault.notes.iter().enumerate() {
        if vault.is_template(note) || !kept(vault, &note.path, &note.tags, scope.as_deref(), args) {
            continue;
        }
        if let Some(score) = rank::fuzzy(&q, &note.name.to_lowercase()) {
            scored.push((score, i));
        }
    }
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for &(_, i) in &scored {
        named[i] = true;
    }
    let mut hits: Vec<Value> = scored
        .into_iter()
        .take(limit)
        .map(|(_, i)| hit(vault, &vault.notes[i], "name", ""))
        .collect();
    if hits.len() < limit && q.chars().count() >= 2 {
        let word = words.first().copied().unwrap_or(q.as_str());
        for (i, note) in vault.notes.iter().enumerate() {
            if named[i]
                || vault.is_template(note)
                || !kept(vault, &note.path, &note.tags, scope.as_deref(), args)
            {
                continue;
            }
            if rank::contains_all(&note.lower, &words) {
                hits.push(hit(
                    vault,
                    note,
                    "text",
                    &rank::snippet(&note.body, &note.lower, word),
                ));
                if hits.len() == limit {
                    break;
                }
            }
        }
    }
    Outcome::ok(json!({"hits": hits}))
}

fn hit(vault: &Vault, note: &crate::logic::note::Note, where_: &str, snippet: &str) -> Value {
    json!({
        "path": vault::rel(&vault.root, &note.path),
        "title": note.name,
        "folder": vault::folder(&vault.root, &note.path),
        "where": where_,
        "snippet": snippet,
    })
}

pub fn read_note(vault: &Vault, args: &Value) -> Outcome {
    let note = match open(vault, args) {
        Ok(note) => note,
        Err(outcome) => return outcome,
    };
    let (text, token) = match vault::read_text(&note.path) {
        Ok(read) => read,
        Err(message) => return Outcome::err(message),
    };
    let shown = match arg_opt(args, "section") {
        Some(section) => match section_text(&text, section) {
            Ok(part) => part,
            Err(message) => return Outcome::err(message),
        },
        None => text.clone(),
    };
    let meta = markdown::front_matter(&text);
    Outcome::ok(json!({
        "path": vault::rel(&vault.root, &note.path),
        "title": note.name,
        "front_matter": &text[..meta],
        "tags": note.tags,
        "aliases": note.aliases,
        "text": shown,
        "links": outgoing(vault, &text),
        "embeds": embeds(vault, &text),
        "comments": comments_of(&text),
        "token": token,
    }))
}

fn section_text(text: &str, section: &str) -> Result<String, String> {
    let anchor = if let Some(id) = section.strip_prefix('^') {
        if id.is_empty() {
            return Err("the block id is empty".into());
        }
        Anchor::Block(id.to_string())
    } else {
        Anchor::Heading(section.to_string())
    };
    crate::logic::wiki::section(text, &anchor).ok_or_else(|| match &anchor {
        Anchor::Block(id) => format!("no block `^{id}`"),
        _ => format!("no heading `{section}`"),
    })
}

fn outgoing(vault: &Vault, text: &str) -> Vec<Value> {
    let mut found = Vec::new();
    let mut seen = Vec::new();
    let mut in_code = false;
    for (row, line) in text.lines().enumerate() {
        if markdown::is_fence(line) {
            in_code = !in_code;
            continue;
        }
        if in_code {
            continue;
        }
        for (_, link) in markdown::links(line) {
            let Link::Wiki(name) = link else { continue };
            let target = name
                .split(['#', '|'])
                .next()
                .unwrap_or(&name)
                .trim()
                .to_string();
            if target.is_empty() || seen.contains(&target.to_lowercase()) {
                continue;
            }
            seen.push(target.to_lowercase());
            let resolved = wiki_path(vault, &target);
            found.push(json!({"target": target, "path": resolved, "line": row + 1}));
        }
    }
    found
}

fn wiki_path(vault: &Vault, name: &str) -> Option<String> {
    vault
        .find_note(name)
        .ok()
        .map(|note| vault::rel(&vault.root, &note.path))
}

fn embeds(vault: &Vault, text: &str) -> Vec<Value> {
    text.lines()
        .filter_map(crate::logic::wiki::parse)
        .map(|target| {
            let resolved = wiki_path(vault, &target.note);
            json!({
                "note": target.note,
                "heading": match &target.anchor {
                    Anchor::Heading(h) => Some(h.clone()),
                    Anchor::Block(id) => Some(format!("^{id}")),
                    Anchor::Whole => None,
                },
                "path": resolved,
            })
        })
        .collect()
}

fn comments_of(text: &str) -> Vec<Value> {
    markdown::all_comments(text)
        .into_iter()
        .map(|note| json!({"passage": note.noted, "comment": note.said, "scope": note.scope}))
        .collect()
}

pub fn note_links(vault: &Vault, args: &Value) -> Outcome {
    let note = match open(vault, args) {
        Ok(note) => note,
        Err(outcome) => return outcome,
    };
    let path = note.path.clone();
    let back: Vec<Value> = wiki::backlinks_of(vault, &path);
    let text = match fs::read_to_string(&path).map_err(disk) {
        Ok(text) => text,
        Err(message) => return Outcome::err(message),
    };
    let pictures: Vec<Value> = text
        .lines()
        .filter_map(|line| markdown::image(line).map(|(_, name)| name))
        .map(|name| json!({"name": name, "path": vault.find_file(&name).ok().map(|p| vault::rel(&vault.root, &p))}))
        .collect();
    Outcome::ok(json!({
        "outgoing": outgoing(vault, &text),
        "backlinks": back,
        "tags": note.tags,
        "embeds": embeds(vault, &text),
        "pictures": pictures,
    }))
}

pub fn list_tags(vault: &Vault) -> Outcome {
    let mut tags: Vec<(String, usize)> = Vec::new();
    for note in vault.notes.iter().filter(|n| !vault.is_template(n)) {
        for tag in note.tags.iter() {
            if let Some((_, count)) = tags.iter_mut().find(|(name, _)| name == tag) {
                *count += 1;
            } else {
                tags.push((tag.clone(), 1));
            }
        }
    }
    tags.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Outcome::ok(
        json!({"tags": tags.into_iter().map(|(tag, count)| json!({"tag": tag, "count": count})).collect::<Vec<_>>()}),
    )
}

fn open<'a>(vault: &'a Vault, args: &Value) -> Result<&'a crate::logic::note::Note, Outcome> {
    let name = arg_str(args, "note").map_err(Outcome::err)?;
    vault.find_note(name).map_err(Outcome::err)
}

pub fn create_note(vault: &mut Vault, args: &Value) -> Outcome {
    let title = match arg_str(args, "title") {
        Ok(title) => title,
        Err(message) => return Outcome::err(message),
    };
    let Some(name) = files::clean_name(title) else {
        return Outcome::err("invalid name");
    };
    let dir = match place(vault, arg_opt(args, "folder")) {
        Ok(dir) => dir,
        Err(outcome) => return outcome,
    };
    if let Some(taken) = vault::name_taken(&dir, &name, "md") {
        return Outcome::err(format!(
            "a note already has the name `{name}` ({})",
            vault::rel(&vault.root, &taken)
        ));
    }
    let body = if let Some(template) = arg_opt(args, "template") {
        let Some(path) = template_path(vault, template) else {
            return Outcome::err(format!("no template named `{template}`"));
        };
        let text = match fs::read_to_string(&path).map_err(disk) {
            Ok(text) => text,
            Err(message) => return Outcome::err(message),
        };
        markdown::fill(
            &text,
            &name,
            &date_name(clock::today()),
            &time_name(clock::clock()),
        )
        .0
    } else {
        let mut body = format!("# {name}\n\n");
        if let Some(text) = arg_opt(args, "text") {
            body.push_str(text.trim_start_matches('\n'));
            if !body.ends_with('\n') {
                body.push('\n');
            }
        }
        body
    };
    // The folder is made last: a refused note leaves nothing behind.
    if let Err(message) = fs::create_dir_all(&dir).map_err(disk) {
        return Outcome::err(message);
    }
    let path = dir.join(format!("{name}.md"));
    if let Err(message) = files::write(&path, &body).map_err(disk) {
        return Outcome::err(message);
    }
    vault.rescan();
    Outcome::ok(json!({"path": vault::rel(&vault.root, &path)}))
}

fn place(vault: &Vault, folder: Option<&str>) -> Result<std::path::PathBuf, Outcome> {
    match folder {
        None => Ok(vault.root.clone()),
        Some(folder) => vault.inside(folder, false).map_err(Outcome::err),
    }
}

fn template_path(vault: &Vault, name: &str) -> Option<std::path::PathBuf> {
    let want = name.to_lowercase();
    vault
        .notes
        .iter()
        .find(|n| vault.is_template(n) && n.name.to_lowercase() == want)
        .map(|n| n.path.clone())
}

pub fn edit_note(vault: &mut Vault, args: &Value) -> Outcome {
    let path = match vault_note(vault, args) {
        Ok(path) => path,
        Err(outcome) => return outcome,
    };
    let (text, token) = match vault::read_text(&path).map_err(Outcome::err) {
        Ok(read) => read,
        Err(outcome) => return outcome,
    };
    let expected = match given_token(args) {
        Ok(expected) => expected,
        Err(message) => return Outcome::err(message),
    };
    if let Some(expected) = expected
        && expected != token
    {
        return Outcome::err("the note changed since it was read; read it again and retry");
    }
    let Some(edits) = args.get("edits").and_then(|v| v.as_array()) else {
        return Outcome::err("`edits` must be a list");
    };
    let mut next = text.clone();
    for (i, edit) in edits.iter().enumerate() {
        let (Some(old), Some(new)) = (
            edit.get("old").and_then(Value::as_str),
            edit.get("new").and_then(Value::as_str),
        ) else {
            return Outcome::err(format!(
                "edit {} needs a text `old` and a text `new`",
                i + 1
            ));
        };
        if old.is_empty() {
            return Outcome::err("an empty passage cannot be replaced");
        }
        match next.matches(old).count() {
            0 => return Outcome::err(format!("`{old}` was not found")),
            1 => next = next.replacen(old, new, 1),
            n => return Outcome::err(format!("`{old}` was found {n} times; it must occur once")),
        }
    }
    if let Err(message) = guard_pending(&text, &next) {
        return Outcome::err(message);
    }
    // A title change renames the note and rewrites its links: refused before anything is written
    // when a linking note has a pending change in the lines a link rewrite would change.
    if let Some(title) = planned_title(&path, &text, &next) {
        let old = files::stem(&path);
        if let Err(message) =
            vault::check_links_before_move(vault, &path, &old, &title, Some(next.as_str()))
        {
            return Outcome::err(message);
        }
    }
    if let Err(message) = files::write(&path, &next).map_err(disk) {
        return Outcome::err(message);
    }
    let (path, renamed, reason, rewritten) = follow_title(vault, &path, &text, &next);
    let token = vault::token(next.as_bytes());
    Outcome::ok(json!({
        "path": vault::rel(&vault.root, &path),
        "token": token,
        "renamed": renamed,
        "name_kept": reason,
        "rewritten": rewritten,
    }))
}

/// The guard of `edit`, `append` and `comment`: the one splice that turns `text` into `next` may
/// not touch a pending change (`pending::refuses`). `Err` names the reason in one sentence.
fn guard_pending(text: &str, next: &str) -> Result<(), String> {
    let Some((range, inserted)) = assistant::edit_between(text, next) else {
        return Ok(());
    };
    if pending::refuses(text, range, &inserted) {
        return Err(
            "the change touches a pending change: accept or reject it in Inkuno first".into(),
        );
    }
    Ok(())
}

/// The token an edit or an append was given, when it was given. An empty token is no token.
fn given_token(args: &Value) -> Result<Option<&str>, String> {
    match args.get("token").and_then(Value::as_str) {
        None => Ok(None),
        Some("") => Err("`token` is empty; pass the token that `read --json` gives".into()),
        Some(token) => Ok(Some(token)),
    }
}

/// The title `follow_title` renames the note to, when the change of `before` into `after` renames it:
/// the same test as `follow_title`, read first so the check can run before anything is written.
fn planned_title(path: &Path, before: &str, after: &str) -> Option<String> {
    let was = files::h1_of(before);
    let title = files::h1_of(after).filter(|title| was.as_ref() != Some(title))?;
    if title == files::stem(path) || path.with_file_name(format!("{title}.md")).exists() {
        return None;
    }
    Some(title)
}

/// A changed `# title` renames the file and the links, unless the new name is taken:
/// the text is kept and the file keeps its name, as the app does.
fn follow_title(
    vault: &mut Vault,
    path: &Path,
    before: &str,
    after: &str,
) -> (std::path::PathBuf, bool, Option<String>, usize) {
    let was = files::h1_of(before);
    let now = files::h1_of(after);
    let Some(title) = now.filter(|title| was.as_ref() != Some(title)) else {
        vault.rescan();
        return (path.to_path_buf(), false, None, 0);
    };
    let old = files::stem(path);
    if title == old {
        vault.rescan();
        return (path.to_path_buf(), false, None, 0);
    }
    let dest = path.with_file_name(format!("{title}.md"));
    if dest.exists() {
        vault.rescan();
        return (
            path.to_path_buf(),
            false,
            Some("this name is already taken".into()),
            0,
        );
    }
    // The history follows the note: first, so a refusal moves nothing; back, if the note cannot move.
    let moved = revisions::move_along(Some(&vault.root), path, &dest)
        .and_then(|()| files::rename(path, &dest));
    if let Err(error) = moved {
        let _ = revisions::move_along(Some(&vault.root), &dest, path);
        vault.rescan();
        return (path.to_path_buf(), false, Some(disk(error)), 0);
    }
    let rewritten = vault::relink(vault, &old, &title);
    (dest, true, None, rewritten)
}

pub fn append_to_note(vault: &mut Vault, args: &Value) -> Outcome {
    let path = match vault_note(vault, args) {
        Ok(path) => path,
        Err(outcome) => return outcome,
    };
    let text = match arg_str(args, "text") {
        Ok(text) => text,
        Err(message) => return Outcome::err(message),
    };
    let (body, token) = match vault::read_text(&path).map_err(Outcome::err) {
        Ok(read) => read,
        Err(outcome) => return outcome,
    };
    let expected = match given_token(args) {
        Ok(expected) => expected,
        Err(message) => return Outcome::err(message),
    };
    if let Some(expected) = expected
        && expected != token
    {
        return Outcome::err("the note changed since it was read; read it again and retry");
    }
    let next = match arg_opt(args, "under") {
        Some(heading) => match append_under(&body, heading, text) {
            Ok(next) => next,
            Err(message) => return Outcome::err(message),
        },
        None => {
            let mut next = body.trim_end().to_string();
            next.push_str("\n\n");
            next.push_str(text.trim());
            next.push('\n');
            next
        }
    };
    if let Err(message) = guard_pending(&body, &next) {
        return Outcome::err(message);
    }
    if let Err(message) = files::write(&path, &next).map_err(disk) {
        return Outcome::err(message);
    }
    vault.rescan();
    Outcome::ok(
        json!({"path": vault::rel(&vault.root, &path), "token": vault::token(next.as_bytes())}),
    )
}

fn append_under(text: &str, heading: &str, extra: &str) -> Result<String, String> {
    let heads = markdown::headings(text);
    let at = heads
        .iter()
        .position(|(_, label, _)| *label == heading)
        .or_else(|| {
            let want = heading.to_lowercase();
            let found: Vec<usize> = heads
                .iter()
                .enumerate()
                .filter(|(_, (_, label, _))| label.to_lowercase() == want)
                .map(|(i, _)| i)
                .collect();
            (found.len() == 1).then(|| found[0])
        });
    let Some(at) = at else {
        return Err(format!("no heading `{heading}`"));
    };
    let level = heads[at].0;
    let end = heads[at + 1..]
        .iter()
        .find(|(next, _, _)| *next <= level)
        .map(|(_, _, start)| *start)
        .unwrap_or(text.len());
    let mut out = text[..end].trim_end().to_string();
    out.push_str("\n\n");
    out.push_str(extra.trim());
    out.push('\n');
    if end < text.len() {
        if !text[end..].starts_with('\n') {
            out.push('\n');
        }
        out.push_str(&text[end..]);
    }
    Ok(out)
}

/// The text of the saved version of `path` whose token is `token`, the newest one first.
fn saved_text_with(
    home: &revisions::Home,
    path: &Path,
    token: &str,
) -> Result<Option<String>, String> {
    let saved = revisions::all(home, path).map_err(disk)?;
    Ok(saved
        .into_iter()
        .rev()
        .find(|rev| vault::token(rev.text.as_bytes()) == token)
        .map(|rev| rev.text))
}

/// Saves an automatic revision of `text`, by `author`. `None` when the text is the latest revision's.
fn snapshot(
    home: &revisions::Home,
    path: &Path,
    author: &str,
    memo: &str,
    text: &str,
) -> Result<Option<u32>, String> {
    match revisions::save_auto(home, path, author, memo, text) {
        Ok(revisions::Saved::Made { number, .. }) => Ok(Some(number)),
        Ok(revisions::Saved::Unchanged { .. }) => Ok(None),
        Err(e) => Err(format!("{}: {}", path.display(), disk(e))),
    }
}

/// Saves one automatic revision: `snapshot`, or a stand-in in tests.
type Save = fn(&revisions::Home, &Path, &str, &str, &str) -> Result<Option<u32>, String>;

/// The memo of the automatic revision of a proposed text: `own_last_proposal` looks for it.
const PROPOSED_MEMO: &str = "proposed, pending";

/// The sentence the command line adds to a refused `write` with pending changes.
const REPLACE_HINT: &str = "An agent replaces its own last proposal with --replace.";

/// `write`: proposes `text` as the note's new full text. The changes wait in the note as pending
/// marks until a person accepts or rejects them. Around the write, two automatic revisions are
/// saved: the text before (by the person's default author) and the proposed text (by `author`).
/// Nothing is written when the proposal is refused or the first snapshot fails. A failed save of
/// the proposed text does not undo the write: it is a warning, and its revision is null.
/// With `replace` (JSON `true`), a note whose pending changes are the agent's last proposal, as it
/// left it, gets the new proposal in their place: see `write_note_saving`.
pub fn write_note(vault: &mut Vault, args: &Value) -> Outcome {
    write_note_saving(vault, args, snapshot)
}

/// `write_note` with the save of a revision given (see `Save`).
fn write_note_saving(vault: &mut Vault, args: &Value, save: Save) -> Outcome {
    let path = match vault_note(vault, args) {
        Ok(path) => path,
        Err(outcome) => return outcome,
    };
    let author = match arg_str(args, "author") {
        Ok(author) => author.to_string(),
        Err(message) => return Outcome::err(message),
    };
    let Some(proposed) = args.get("text").and_then(Value::as_str) else {
        return Outcome::err("`text` is missing");
    };
    if proposed.trim().is_empty() {
        return Outcome::err("the text is empty");
    }
    let replace = args
        .get("replace")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let (current, token) = match vault::read_text(&path).map_err(Outcome::err) {
        Ok(read) => read,
        Err(outcome) => return outcome,
    };
    let home = revisions::home_of(&path, Some(&vault.root))
        .unwrap_or_else(|| revisions::Home::Vault(vault.root.clone()));
    // The text the agent read: the file as it is, or the saved version its token names. The token
    // is required: without it, a change by the person could not be told from the agent's base.
    let base = match given_token(args) {
        Err(message) => return Outcome::err(message),
        Ok(None) => {
            return Outcome::err(
                "write needs the token of `read --json`: it says which text the agent read",
            );
        }
        Ok(Some(expected)) if expected == token => current.clone(),
        Ok(Some(expected)) => match saved_text_with(&home, &path, expected) {
            Ok(Some(text)) => text,
            Ok(None) => {
                return Outcome::err("the note changed since it was read; read it again and retry");
            }
            Err(message) => return Outcome::err(message),
        },
    };
    // `--replace` takes back the agent's own last proposal, and writes the new one on the person's
    // text. Without pending changes it is a plain write.
    let replacing = replace && pending::has_pending(&current);
    let (kept, marked) = if replacing {
        if base != current {
            return Outcome::err("the note changed since it was read; read it again and retry");
        }
        match own_last_proposal(&home, &path, &author, &current) {
            Ok(true) => {}
            Ok(false) => {
                return Outcome::err(
                    "the pending changes are not your last proposal as you left it: a person decides them first",
                );
            }
            Err(message) => return Outcome::err(message),
        }
        let kept = pending::reject_all(&current);
        match pending::propose(&kept, &kept, proposed) {
            Ok(marked) => (kept, marked),
            Err(refusal) => return Outcome::err(refusal.to_string()),
        }
    } else {
        match pending::propose(&base, &current, proposed) {
            Ok(marked) => (current.clone(), marked),
            Err(refusal @ pending::Refusal::AlreadyPending) => {
                return Outcome::err(format!("{refusal}. {REPLACE_HINT}"));
            }
            Err(refusal) => return Outcome::err(refusal.to_string()),
        }
    };
    let person = match super::rev::default_author() {
        Ok(person) => person,
        Err(message) => return Outcome::err(message),
    };
    let before = match save(
        &home,
        &path,
        &person,
        &format!("before a change by {author}"),
        &kept,
    ) {
        Ok(number) => number,
        Err(message) => return Outcome::err(message),
    };
    if let Err(message) = files::write(&path, &marked).map_err(disk) {
        return Outcome::err(message);
    }
    // The write stands even when the proposed version cannot be saved: that is a warning, and the
    // proposed revision is null.
    let (proposed_number, warning) = match save(
        &home,
        &path,
        &author,
        PROPOSED_MEMO,
        &pending::accept_all(&marked),
    ) {
        Ok(number) => (number, None),
        Err(message) => (
            None,
            Some(format!(
                "the version of the proposed text could not be saved: {message}"
            )),
        ),
    };
    vault.rescan();
    let mut result = json!({
        "path": vault::rel(&vault.root, &path),
        "changes": pending::areas(&marked).len(),
        "before": before,
        "proposed": proposed_number,
    });
    if let Some(warning) = warning {
        result["warning"] = json!(warning);
    }
    if replacing {
        result["replaced"] = json!(true);
    }
    Outcome::ok(result)
}

/// Whether the pending changes of `current` are the agent's last proposal as it left it: the latest
/// revision is its automatic proposed version, by `author`, and the text is what accepting every
/// change gives. A change the person rejected, typed or commented makes it false; accepted ones do not.
fn own_last_proposal(
    home: &revisions::Home,
    path: &Path,
    author: &str,
    current: &str,
) -> Result<bool, String> {
    let latest = revisions::latest(home, path).map_err(disk)?;
    Ok(latest.is_some_and(|rev| {
        rev.kind == revision::Kind::Auto
            && rev.author == author
            && rev.note.as_deref() == Some(PROPOSED_MEMO)
            && rev.text == pending::accept_all(current)
    }))
}

pub fn rename_note(vault: &mut Vault, args: &Value) -> Outcome {
    let path = match vault.find_file(&match arg_str(args, "note") {
        Ok(note) => note.to_string(),
        Err(message) => return Outcome::err(message),
    }) {
        Ok(path) => path,
        Err(message) => return Outcome::err(message),
    };
    // find_file borrows vault: `path` is owned, so the vault stays mutable below.
    let old_file = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let old_stem = files::stem(&path);
    let image = files::is_image(&path)
        || files::is_attachment(&path)
        || vault.diagrams.iter().any(|p| p == &path);
    let name = arg_opt(args, "name").map(|n| n.to_string());
    // `--folder ""` is the vault root, so the key counts as given even when it is empty.
    let folder = args
        .get("folder")
        .and_then(Value::as_str)
        .map(str::to_string);
    if name.is_none() && folder.is_none() {
        return Outcome::err("`name` or `folder` is missing");
    }
    // A new name alone keeps the note in its folder: only --folder moves it.
    let dir = match folder.as_deref() {
        None => path
            .parent()
            .map_or_else(|| vault.root.clone(), Path::to_path_buf),
        Some(_) => match place(vault, folder.as_deref()) {
            Ok(dir) => dir,
            Err(outcome) => return outcome,
        },
    };
    // Everything is checked before anything is made or moved: a refusal leaves the disk as it was.
    let file_name = if let Some(name) = &name {
        if image {
            let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("svg");
            let Some(clean) = files::clean_name(name) else {
                return Outcome::err("invalid name");
            };
            format!("{clean}.{extension}")
        } else if path.extension().is_some_and(|e| e == "md") {
            let Some(clean) = files::clean_name(name) else {
                return Outcome::err("invalid name");
            };
            format!("{clean}.md")
        } else {
            return Outcome::err("this file cannot be renamed");
        }
    } else {
        old_file.clone()
    };
    let dest = dir.join(&file_name);
    // A case-only rename finds its own file at the new name on a case-insensitive disk: no clash.
    if dest != path && dest.exists() && !files::same_file(&dest, &path) {
        return Outcome::err(format!(
            "a file already has the name `{}` ({})",
            files::stem(&dest),
            vault::rel(&vault.root, &dest)
        ));
    }
    let new_stem = files::stem(&dest);
    let (old_link, new_link) = if image || !dest.extension().is_some_and(|e| e == "md") {
        (
            old_file,
            dest.file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or(new_stem.clone()),
        )
    } else {
        (old_stem.clone(), new_stem.clone())
    };
    // Links are rewritten after the move, so a refusal is decided before it: no linking note may
    // have a pending change in the lines a rewrite would change.
    if old_link != new_link
        && let Err(message) =
            vault::check_links_before_move(vault, &path, &old_link, &new_link, None)
    {
        return Outcome::err(message);
    }
    if dest != path {
        if let Err(message) = fs::create_dir_all(&dir).map_err(disk) {
            return Outcome::err(message);
        }
        // The history follows the note: first, so a refusal moves nothing; back, if the note cannot.
        let moved = revisions::move_along(Some(&vault.root), &path, &dest)
            .and_then(|()| files::rename(&path, &dest));
        if let Err(error) = moved {
            let _ = revisions::move_along(Some(&vault.root), &dest, &path);
            return Outcome::err(disk(error));
        }
    }
    // A note whose title was its name keeps the title in step.
    if dest.extension().is_some_and(|e| e == "md")
        && old_stem != new_stem
        && let Ok(text) = fs::read_to_string(&dest)
        && files::h1_of(&text).as_deref() == Some(old_stem.as_str())
    {
        let meta = markdown::front_matter(&text);
        let (head, body) = text.split_at(meta);
        if let Some(first) = body.lines().find(|l| !l.trim().is_empty()) {
            let hashes = first.trim_start().chars().take_while(|c| *c == '#').count();
            let title = format!("{} {new_stem}", "#".repeat(hashes.max(1)));
            let _ = files::write(&dest, format!("{head}{}", body.replacen(first, &title, 1)));
        }
    }
    let rewritten = if old_link != new_link {
        vault::relink(vault, &old_link, &new_link)
    } else {
        vault.rescan();
        0
    };
    // The moved note itself may still answer to the old name in another case: it is not another note.
    let left = old_link != new_link
        && rewritten == 0
        && vault
            .notes
            .iter()
            .any(|n| n.name.eq_ignore_ascii_case(&old_link) && !files::same_file(&n.path, &dest));
    Outcome::ok(json!({
        "path": vault::rel(&vault.root, &dest),
        "rewritten": rewritten,
        "links_left": left,
    }))
}

/// Asks the open Inkuno window to show a note or file: the user sees at once what the agent did.
pub fn show(vault: &mut Vault, args: &Value) -> Outcome {
    let path = match arg_str(args, "note")
        .map(str::to_string)
        .and_then(|note| vault.find_file(&note))
    {
        Ok(path) => path,
        Err(message) => return Outcome::err(message),
    };
    // The window compares absolute paths; `--vault` may have been given relative.
    let absolute = std::path::absolute(&path).unwrap_or(path.clone());
    match link::show(&absolute) {
        Ok(()) => Outcome::ok(json!({"shown": vault::rel(&vault.root, &path)})),
        Err(message) => Outcome::err(message),
    }
}

pub fn trash_note(vault: &mut Vault, args: &Value) -> Outcome {
    let path = match vault.find_file(&match arg_str(args, "note") {
        Ok(note) => note.to_string(),
        Err(message) => return Outcome::err(message),
    }) {
        Ok(path) => path,
        Err(message) => return Outcome::err(message),
    };
    match files::trash(&vault.root, &path).map_err(disk) {
        Ok(trashed) => {
            // Its history goes to the bin with it, under the name the undo looks for.
            let history = revisions::trash_along(Some(&vault.root), &path, &trashed);
            let path = vault::rel(&vault.root, &trashed);
            vault.rescan();
            match history {
                Ok(()) => Outcome::ok(json!({"path": path})),
                Err(error) => Outcome::err(format!(
                    "moved to .trash, but its history was not: {}",
                    disk(error)
                )),
            }
        }
        Err(message) => Outcome::err(message),
    }
}

pub fn restore_note(vault: &mut Vault, args: &Value) -> Outcome {
    let raw = match arg_str(args, "path") {
        Ok(path) => path,
        Err(message) => return Outcome::err(message),
    };
    let path = match vault.inside(raw, true) {
        Ok(path) => path,
        Err(message) => return Outcome::err(message),
    };
    let trash = vault.root.join(".trash");
    if !path.starts_with(&trash) {
        return Outcome::err("restore only brings a file back from .trash");
    }
    match files::restore(&vault.root, &path).map_err(disk) {
        Ok(restored) => {
            let history = revisions::restore_along(Some(&vault.root), &path, &restored);
            let path = vault::rel(&vault.root, &restored);
            vault.rescan();
            match history {
                Ok(()) => Outcome::ok(json!({"path": path})),
                Err(error) => Outcome::err(format!(
                    "put back, but its history was not: {}",
                    disk(error)
                )),
            }
        }
        Err(message) => Outcome::err(message),
    }
}

pub fn daily_note(vault: &mut Vault, args: &Value) -> Outcome {
    let date = match arg_opt(args, "date") {
        Some(date) => match parse_date(date) {
            Ok(date) => date,
            Err(message) => return Outcome::err(message),
        },
        None => clock::today(),
    };
    let name = date_name(date);
    if vault.find_note(&name).is_err() {
        let template = daily_template(vault);
        let body = match template {
            Some(path) => {
                let text = match fs::read_to_string(&path).map_err(disk) {
                    Ok(text) => text,
                    Err(message) => return Outcome::err(message),
                };
                markdown::fill(&text, &name, &name, &time_name(clock::clock())).0
            }
            None => format!("# {name}\n\n"),
        };
        let path = vault.root.join(format!("{name}.md"));
        if path.exists() {
            return Outcome::err(format!(
                "a note already has the name `{name}` ({})",
                vault::rel(&vault.root, &path)
            ));
        }
        if let Err(message) = files::write(&path, &body).map_err(disk) {
            return Outcome::err(message);
        }
        vault.rescan();
    }
    if let Some(text) = arg_opt(args, "append") {
        return append_to_note(vault, &json!({"note": name, "text": text}));
    }
    read_note(vault, &json!({"note": name}))
}

pub fn parse_date(text: &str) -> Result<(i32, u32, u32), String> {
    let mut parts = text.split('-');
    let year = parts.next().and_then(|p| p.parse().ok());
    let month = parts.next().and_then(|p| p.parse().ok());
    let day = parts.next().and_then(|p| p.parse().ok());
    match (year, month, day, parts.next()) {
        // A day the calendar has comes back unchanged from a day number: 2026-02-30 does not.
        (Some(year), Some(month), Some(day), None)
            if (1..=12).contains(&month)
                && markdown::day_of(markdown::day_number((year, month, day)))
                    == (year, month, day) =>
        {
            Ok((year, month, day))
        }
        _ => Err(format!("`{text}` is not a date (YYYY-MM-DD)")),
    }
}

fn daily_template(vault: &Vault) -> Option<std::path::PathBuf> {
    let named: Vec<(std::path::PathBuf, String)> = vault
        .notes
        .iter()
        .filter(|n| vault.is_template(n))
        .map(|n| (n.path.clone(), n.name.clone()))
        .collect();
    daily::pick(
        named
            .iter()
            .map(|(path, stem)| (path.clone(), stem.as_str())),
    )
}

/// `inkuno comment <note>`: a note of the vault.
pub fn add_comment(vault: &mut Vault, args: &Value) -> Outcome {
    let path = match vault_note(vault, args) {
        Ok(path) => path,
        Err(outcome) => return outcome,
    };
    let (passage, comment) = match comment_args(args) {
        Ok(pair) => pair,
        Err(message) => return Outcome::err(message),
    };
    let token = match annotate(&path, &passage, &comment) {
        Ok(token) => token,
        Err(message) => return Outcome::err(message),
    };
    vault.rescan();
    Outcome::ok(json!({"path": vault::rel(&vault.root, &path), "token": token}))
}

/// `inkuno comment --file PATH`: a Markdown file by its path. No vault is opened.
pub fn add_comment_file(args: &Value) -> Outcome {
    let given = match arg_str(args, "file") {
        Ok(given) => given,
        Err(message) => return Outcome::err(message),
    };
    let path = match markdown_file(given) {
        Ok(path) => path,
        Err(message) => return Outcome::err(message),
    };
    let (passage, comment) = match comment_args(args) {
        Ok(pair) => pair,
        Err(message) => return Outcome::err(message),
    };
    match annotate(&path, &passage, &comment) {
        Ok(token) => Outcome::ok(json!({"path": path.display().to_string(), "token": token})),
        Err(message) => Outcome::err(message),
    }
}

/// The passage and the comment of a `comment` command. An empty comment is left to `comment_on`,
/// which refuses it with its own sentence.
fn comment_args(args: &Value) -> Result<(String, String), String> {
    let passage = arg_str(args, "passage")?.to_string();
    let comment = arg_opt(args, "comment").unwrap_or_default().to_string();
    Ok((passage, comment))
}

/// The read–annotate–write step both `comment` paths share: the text is read, the passage is
/// wrapped, and the result is written atomically. Returns the token of the new text.
fn annotate(path: &Path, passage: &str, comment: &str) -> Result<String, String> {
    let (text, _) = vault::read_text(path)?;
    let next = markdown::comment_on(&text, passage, comment)?;
    guard_pending(&text, &next)?;
    files::write(path, &next).map_err(disk)?;
    Ok(vault::token(next.as_bytes()))
}

/// A `--file` path: it must end in `md` (any case) and name an existing regular file. The absolute
/// path is returned.
fn markdown_file(given: &str) -> Result<PathBuf, String> {
    let is_markdown = Path::new(given)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
    if !is_markdown {
        return Err(format!("`{given}` is not a Markdown file"));
    }
    let path = fs::canonicalize(given).map_err(|why| match why.kind() {
        std::io::ErrorKind::NotFound => format!("`{given}` was not found"),
        _ => format!("`{given}`: {}", disk(why)),
    })?;
    if path.is_dir() {
        return Err(format!("`{given}` is a folder, not a Markdown file"));
    }
    if !path.is_file() {
        return Err(format!("`{given}` is not a regular file"));
    }
    Ok(path)
}

fn vault_note(vault: &Vault, args: &Value) -> Result<std::path::PathBuf, Outcome> {
    let name = arg_str(args, "note").map_err(Outcome::err)?;
    vault
        .find_note(name)
        .map(|n| n.path.clone())
        .map_err(Outcome::err)
}

/// Backlinks with the line each one is written on. Lives next to the tool that shows them.
pub mod wiki {
    use super::*;

    pub fn backlinks_of(vault: &Vault, path: &Path) -> Vec<Value> {
        crate::logic::wiki::backlinks(&vault.notes, path)
            .into_iter()
            .map(|note| {
                let aimed = vault.notes.iter().find(|n| n.path == path);
                let line = aimed.and_then(|aimed| crate::logic::wiki::link_line(&note.body, aimed)).unwrap_or("").trim().to_string();
                json!({"path": vault::rel(&vault.root, &note.path), "title": note.name, "line": line})
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A save that fails for the proposed version only: the revision before is saved as usual.
    fn fail_the_proposed(
        home: &revisions::Home,
        path: &Path,
        author: &str,
        memo: &str,
        text: &str,
    ) -> Result<Option<u32>, String> {
        if memo.starts_with("proposed") {
            Err(format!("{}: the disk is full", path.display()))
        } else {
            snapshot(home, path, author, memo, text)
        }
    }

    #[test]
    fn a_proposed_version_that_cannot_be_saved_is_a_warning_and_the_write_stands() {
        let dir = std::env::temp_dir().join(format!("inkuno-ops-warn-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("Plan.md"), "# Plan\n").unwrap();
        let mut vault = Vault::open(dir.clone()).unwrap();
        let token = vault::token("# Plan\n".as_bytes());
        let args = json!({
            "note": "Plan",
            "author": "claude",
            "text": "# Plan\n\nTwo.\n",
            "token": token,
        });
        let outcome = write_note_saving(&mut vault, &args, fail_the_proposed);
        assert!(!outcome.is_error, "{}", outcome.sentence());
        assert_eq!(outcome.structured["changes"], 1);
        assert_eq!(outcome.structured["proposed"], Value::Null);
        assert!(
            outcome.structured["before"].is_u64(),
            "{}",
            outcome.structured
        );
        let warning = outcome.structured["warning"].as_str().unwrap();
        assert!(
            warning.starts_with("the version of the proposed text could not be saved: "),
            "{warning}"
        );
        assert!(!warning.contains('\n'), "{warning}");
        let text = fs::read_to_string(dir.join("Plan.md")).unwrap();
        assert!(text.contains("{++\n\nTwo.\n++}"), "{text}");
        let _ = fs::remove_dir_all(dir);
    }
}
