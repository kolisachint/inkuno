//! `inkuno rev`: the saved versions of a note, from the command line.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use super::{Opts, failed, options, usage};
use crate::lang::tr;
use crate::logic::prefs::Prefs;
use crate::logic::revision::{self, Format, Revision};
use crate::platform::{
    revisions::{self, Home, Saved},
    scratch, vault,
};

/// The help of `inkuno rev` and `inkuno help rev`.
pub const HELP: &str = "\
inkuno rev save <note> [--author NAME] [-m TEXT]
    save the note's text as a new revision; prints its number
inkuno rev list <note> [--json]
    the revisions, newest first: number, date, author, kind, note
inkuno rev show <note> <n> [--json]
    the full text of revision n (--json: its header and text)
inkuno rev diff <note> [<a>] [<b>] [--format unified|critic]
    no number: the latest revision against the file as it is now
    one number: that revision against the file as it is now
    two numbers: revision a against revision b
    unified (default) is a patch `patch` and `git apply` read; critic marks the
    changes inline with {--removed--} and {++added++}

<note> is the path of a .md file. Its vault is the folder above it that holds
an .inkuno folder; failing that, the vault the app has open when the note is in
it; failing that, the note's own folder. A scratch note (in the app's scratch
folder) keeps its revisions there. Errors exit 1 with the reason on stderr.

--author defaults to the author setting of the app, else INKUNO_AUTHOR, else the
system user name, else `unknown`.

examples:
  inkuno rev save ideas/plan.md --author ana -m \"dropped the plugin idea\"
  inkuno rev diff ideas/plan.md 2 --format critic
";

/// Runs `rev` with its arguments (after the word `rev`).
pub fn run(args: &[String]) -> i32 {
    let Some(sub) = args.first().map(String::as_str) else {
        return usage(tr(
            "rev needs save, list, show or diff",
            "rev attend save, list, show ou diff",
        ));
    };
    let rest = &args[1..];
    let value_options: &[&str] = match sub {
        "save" => &["--author", "--message"],
        "list" | "show" => &[],
        "diff" => &["--format"],
        "--help" | "-h" => {
            print!("{HELP}");
            return 0;
        }
        other => return usage(&format!("unknown rev command `{other}`")),
    };
    let opts = match options(rest, value_options, &[]) {
        Ok(opts) => opts,
        Err(reason) => return usage(&format!("rev {sub}: {reason}")),
    };
    if opts.help {
        print!("{HELP}");
        return 0;
    }
    let result = match sub {
        "save" => save(&opts),
        "list" => list(&opts),
        "show" => show(&opts),
        _ => diff(&opts),
    };
    match result {
        Ok(out) => {
            print!("{out}");
            0
        }
        Err(reason) => failed(reason),
    }
}

/// The note's home and its path, as the app would know them. Errors name the note.
pub fn locate(note: &str) -> Result<(Home, PathBuf), String> {
    let path = std::path::absolute(note).map_err(|e| format!("{note}: {e}"))?;
    if path.extension().is_none_or(|e| e != "md") || !path.is_file() {
        return Err(format!("no such note: {note} (a .md file is expected)"));
    }
    let scratch_dir = scratch::dir();
    if path.starts_with(&scratch_dir) {
        return Ok((Home::Scratch(scratch_dir), path));
    }
    let folder = path.parent().unwrap_or(Path::new("/")).to_path_buf();
    Ok((Home::Vault(vault_root(&folder)), path))
}

/// The vault of a note in `folder`: the nearest folder above with an `.inkuno` folder, else the
/// vault the app has open when the note is in it, else `folder` itself.
fn vault_root(folder: &Path) -> PathBuf {
    if let Some(root) = folder.ancestors().find(|dir| dir.join(".inkuno").is_dir()) {
        return root.to_path_buf();
    }
    let (open, _) = vault::load_config();
    match open {
        Some(root) if folder.starts_with(&root) => root,
        _ => folder.to_path_buf(),
    }
}

/// The one note the command names, and where its revisions are.
fn note_of(opts: &Opts, count: usize) -> Result<(Home, PathBuf, Vec<String>), String> {
    let Some(note) = opts.words.first() else {
        return Err(tr(
            "a note is needed: the path of a .md file",
            "une note est nécessaire : le chemin d'un fichier .md",
        )
        .into());
    };
    if opts.words.len() > count {
        return Err(format!(
            "too many arguments: {}",
            opts.words[count..].join(" ")
        ));
    }
    let (home, path) = locate(note)?;
    Ok((home, path, opts.words[1..].to_vec()))
}

/// A revision number from the command line.
fn number_of_word(word: &str) -> Result<u32, String> {
    word.parse::<u32>()
        .ok()
        .filter(|n| *n >= 1)
        .ok_or_else(|| format!("`{word}` is not a revision number (1, 2, 3…)"))
}

/// The author of a new revision: `--author`, else the app's setting, else the environment.
fn author(opts: &Opts) -> Result<String, String> {
    let given = match opts.values.get("--author") {
        Some(name) => name.clone(),
        None => {
            let saved = Prefs::parse(&vault::load_settings()).author;
            if saved.is_empty() {
                revision::default_author(|key| std::env::var(key).ok())
            } else {
                saved
            }
        }
    };
    let name = revision::clean_author(&given);
    if name.is_empty() {
        return Err("the author is empty: give --author NAME".into());
    }
    Ok(name)
}

fn save(opts: &Opts) -> Result<String, String> {
    let (home, path, _) = note_of(opts, 1)?;
    let text = fs::read_to_string(&path)
        .map_err(|e| format!("{}: {}", path.display(), crate::lang::io_reason(&e)))?;
    let author = author(opts)?;
    let memo = opts
        .values
        .get("--message")
        .map(String::as_str)
        .filter(|m| !m.is_empty());
    match revisions::save(&home, &path, &author, memo, &text, revisions::now()) {
        Ok(Saved::Made { number, .. }) => Ok(format!("{number}\n")),
        Ok(Saved::Unchanged { latest }) => Err(format!("nothing changed since revision {latest}")),
        Err(e) => Err(format!(
            "{}: {}",
            path.display(),
            crate::lang::io_reason(&e)
        )),
    }
}

/// The revisions of a note, or an empty list when it has none yet.
fn history(home: &Home, path: &Path) -> Result<Vec<Revision>, String> {
    revisions::all(home, path)
        .map_err(|e| format!("{}: {}", path.display(), crate::lang::io_reason(&e)))
}

/// A revision's number, or the reason it is not there.
fn revision_at(home: &Home, path: &Path, number: u32) -> Result<Revision, String> {
    match revisions::get(home, path, number) {
        Ok(rev) => Ok(rev),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let saved = history(home, path)?.len();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            Err(if saved == 0 {
                format!("no revision {number} of {name}: none saved yet")
            } else {
                format!("no revision {number} of {name}: {saved} saved, numbered 1 to {saved}")
            })
        }
        Err(e) => Err(format!("revision {number}: {}", crate::lang::io_reason(&e))),
    }
}

fn list(opts: &Opts) -> Result<String, String> {
    let (home, path, _) = note_of(opts, 1)?;
    let mut revs = history(&home, &path)?;
    revs.reverse();
    if opts.json {
        let items: Vec<Value> = revs.iter().map(header_json).collect();
        return Ok(pretty(&Value::Array(items)));
    }
    let mut out = String::new();
    for rev in &revs {
        out += &format!(
            "{:>4}  {}  {}  {}  {}\n",
            rev.number,
            rev.date,
            rev.author,
            rev.kind.word(),
            rev.note.as_deref().unwrap_or("")
        );
    }
    Ok(out)
}

fn show(opts: &Opts) -> Result<String, String> {
    let (home, path, rest) = note_of(opts, 2)?;
    let Some(word) = rest.first() else {
        return Err("a revision number is needed: rev show <note> <n>".into());
    };
    let number = number_of_word(word)?;
    let rev = revision_at(&home, &path, number)?;
    if opts.json {
        let mut header = header_json(&rev);
        header["text"] = Value::String(rev.text.clone());
        return Ok(pretty(&header));
    }
    Ok(rev.text)
}

fn diff(opts: &Opts) -> Result<String, String> {
    let (home, path, rest) = note_of(opts, 3)?;
    let format = match opts.values.get("--format") {
        None => Format::Unified,
        Some(word) => Format::of_word(word)
            .ok_or_else(|| format!("--format {word}: use unified or critic"))?,
    };
    let current = fs::read_to_string(&path)
        .map_err(|e| format!("{}: {}", path.display(), crate::lang::io_reason(&e)))?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let numbers = rest
        .iter()
        .map(|w| number_of_word(w))
        .collect::<Result<Vec<u32>, String>>()?;
    let (old, old_label, new, new_label) = match numbers.as_slice() {
        [] => {
            let latest = history(&home, &path)?
                .pop()
                .ok_or_else(|| format!("no revision of {name} yet: save one with rev save"))?;
            (
                latest.text,
                latest.number.to_string(),
                current,
                "current".to_string(),
            )
        }
        [a] => (
            revision_at(&home, &path, *a)?.text,
            a.to_string(),
            current,
            "current".to_string(),
        ),
        [a, b] => (
            revision_at(&home, &path, *a)?.text,
            a.to_string(),
            revision_at(&home, &path, *b)?.text,
            b.to_string(),
        ),
        _ => return Err("at most two revision numbers: rev diff <note> [<a>] [<b>]".into()),
    };
    Ok(revision::diff(
        format, &name, &old_label, &new_label, &old, &new,
    ))
}

/// A revision's header as JSON: the number, parent, author, date, kind and note.
fn header_json(rev: &Revision) -> Value {
    json!({
        "revision": rev.number,
        "parent": rev.parent,
        "author": rev.author,
        "date": rev.date,
        "kind": rev.kind.word(),
        "note": rev.note,
    })
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default() + "\n"
}
