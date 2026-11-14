//! The command line of the same binary: the vault from a terminal or an agent (`inkuno read`,
//! `inkuno new`, …), the saved versions of a note (`inkuno rev …`), the keyboard shortcuts
//! (`inkuno keys …`) and `inkuno help`. No window: it works whether or not the app is running,
//! and the app sees its changes through its file watcher. Results go to standard output, errors
//! to standard error with exit status 1; a wrong command line exits with 2.

mod commands;
pub mod draw;
mod guide;
mod keys;
pub mod ops;
pub mod outcome;
mod rev;
pub mod store;
mod text;

#[cfg(test)]
mod check;

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

/// Exit status of a command that could not do its work.
const FAILED: i32 = 1;
/// Exit status of a command line that is not one of ours.
const USAGE: i32 = 2;

/// The help shown by `inkuno help` and `inkuno --help`: every command, grouped, with the rules
/// that hold for all of them.
pub const TOP: &str = "\
reading
  inkuno vault                       the vault: its folder, counts, folders, tags
  inkuno list                        notes and files, by name or most recent, a page at a time
  inkuno search <words…>             notes that contain every word: names first, then text
  inkuno read <note>                 a note's text, or one --section of it
  inkuno links <note>                links out, backlinks, tags and pictures of a note
  inkuno tags                        every tag, with how many notes carry it
  inkuno show <note>                 open a note in the Inkuno window
writing
  inkuno new <title…>                a new note; its body is --text, or standard input
  inkuno edit <note>                 exact replacements, refused when the note has changed
  inkuno append <note>               add text at the end, or at the end of a section
  inkuno move <note>                 rename or move a note; the [[links]] to it follow
  inkuno trash <note>                move a note to .trash (nothing is deleted)
  inkuno restore <path>              bring a file back from .trash
  inkuno daily                       today's daily note, or the one of --date
  inkuno comment <note>              comment a passage: {==passage==}{>>comment<<}
diagrams
  inkuno render [<file>]             a PNG of an SVG, of a note's Mermaid block, or of a source
  inkuno render --check              whether a Mermaid source draws
guide
  inkuno guide [page]                the user guide: its pages, or one page as Markdown
versions
  inkuno rev <save|list|show|diff>   the saved versions of a note
shortcuts
  inkuno keys <list|set|reset>       the keyboard shortcuts, as keybindings.json
help
  inkuno help [command]              this text, or the help of one command

Notes are named by their title, an alias, or their path in the vault. Link a note with [[Name]].
Nothing is deleted: trash moves a file to .trash. Pass the token from `inkuno read --json` to
`edit` or `append`, and a note changed since is refused. Every command takes --vault DIR; without
it, the vault is the folder above the current one that holds an .inkuno folder, else the vault the
app has open. Commands print text; --json prints the result as JSON. `inkuno <command> --help`
gives the options and an example.
";

/// Runs a command. `args` starts with the command word; the result is the exit status.
pub fn run(args: &[String]) -> i32 {
    let rest = args.get(1..).unwrap_or(&[]);
    match args.first().map(String::as_str) {
        Some("rev") => rev::run(rest),
        Some("keys") => keys::run(rest),
        Some("help") => help(rest),
        Some(word) => match commands::run(word, rest) {
            Some(code) => code,
            None => unknown(word),
        },
        None => usage("a command is needed: try `inkuno help`"),
    }
}

/// `inkuno help [command]`: the top help, or the help of one command.
fn help(rest: &[String]) -> i32 {
    let Some(word) = rest.first().map(String::as_str) else {
        print!("{TOP}");
        return 0;
    };
    let text = match word {
        "rev" => rev::HELP,
        "keys" => keys::HELP,
        other => match commands::help(other) {
            Some(text) => text,
            None => return unknown(other),
        },
    };
    print!("{text}");
    0
}

/// Every word that starts a command, the old ones included.
fn names() -> Vec<String> {
    ["rev", "keys", "help"]
        .into_iter()
        .chain(commands::names())
        .map(String::from)
        .collect()
}

/// A word that is no command: the closest names are suggested.
fn unknown(word: &str) -> i32 {
    let near = suggest(word, &names());
    let hint = match near.as_slice() {
        [] => String::new(),
        [one] => format!(", did you mean `{one}`?"),
        many => format!(
            ", did you mean {}?",
            many.iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(" or ")
        ),
    };
    usage(&format!("unknown command `{word}`{hint}"))
}

/// A usage error: the reason on standard error, and where the help is.
pub fn usage(reason: &str) -> i32 {
    eprintln!("inkuno: {reason}");
    eprintln!("try `inkuno help`");
    USAGE
}

/// A failed command: the reason on standard error.
fn failed(reason: impl std::fmt::Display) -> i32 {
    eprintln!("inkuno: {reason}");
    FAILED
}

/// A JSON value as the commands print it: indented, with a final newline.
pub fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default() + "\n"
}

/// What a command line holds once its options are read.
#[derive(Debug, Default, PartialEq)]
pub struct Opts {
    /// The words that are not options, in order.
    pub words: Vec<String>,
    /// `--json` was given.
    pub json: bool,
    /// `--help` or `-h` was given.
    pub help: bool,
    /// The options with a value, by their long name (`-m` is `--message`).
    pub values: BTreeMap<String, String>,
    /// The switches given, by their long name.
    pub switches: BTreeSet<String>,
}

/// Reads a command line: `value_options` take the next word or `=value`, `switches` stand alone.
/// `--json` and `--help` are always switches. An unknown option is a usage error. A lone `-` is
/// a word: standard input, by convention.
pub fn options(args: &[String], value_options: &[&str], switches: &[&str]) -> Result<Opts, String> {
    let mut opts = Opts::default();
    let mut words = args.iter();
    while let Some(arg) = words.next() {
        if arg == "--help" || arg == "-h" {
            opts.help = true;
            continue;
        }
        if arg == "--json" {
            opts.json = true;
            continue;
        }
        if !arg.starts_with('-') || arg == "-" {
            opts.words.push(arg.clone());
            continue;
        }
        let (name, inline) = match arg.split_once('=') {
            Some((name, value)) => (name, Some(value.to_string())),
            None => (arg.as_str(), None),
        };
        let long = match name {
            "-m" => "--message",
            other => other,
        };
        if value_options.contains(&long) {
            let value = match inline {
                Some(value) => value,
                None => words
                    .next()
                    .cloned()
                    .ok_or_else(|| format!("{name} needs a value"))?,
            };
            opts.values.insert(long.to_string(), value);
        } else if switches.contains(&long) {
            if inline.is_some() {
                return Err(format!("{name} takes no value"));
            }
            opts.switches.insert(long.to_string());
        } else {
            return Err(format!("unknown option {name}"));
        }
    }
    Ok(opts)
}

/// The closest of `known` to `word`, for "did you mean": the names within a few edits, nearest first.
pub fn suggest(word: &str, known: &[String]) -> Vec<String> {
    let mut near: Vec<(usize, &String)> = known
        .iter()
        .map(|name| (distance(word, name), name))
        .filter(|(d, _)| *d <= word.chars().count().max(4) / 3 + 1)
        .collect();
    near.sort();
    near.into_iter()
        .take(3)
        .map(|(_, name)| name.clone())
        .collect()
}

/// Edit distance between two words (insertions, deletions, substitutions).
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    #[test]
    fn reads_words_values_and_switches() {
        let opts = options(
            &args(&["plan.md", "--author", "ana", "-m=kept it", "--json", "3"]),
            &["--author", "--message"],
            &[],
        )
        .unwrap();
        assert_eq!(opts.words, ["plan.md", "3"]);
        assert_eq!(opts.values["--author"], "ana");
        assert_eq!(opts.values["--message"], "kept it");
        assert!(opts.json && !opts.help);
    }

    #[test]
    fn reads_a_switch_on_its_own() {
        let opts = options(&args(&["--check", "-"]), &[], &["--check"]).unwrap();
        assert!(opts.switches.contains("--check"));
        assert_eq!(opts.words, ["-"]);
        assert!(options(&args(&["--check=yes"]), &[], &["--check"]).is_err());
    }

    #[test]
    fn refuses_unknown_options_and_missing_values() {
        assert!(options(&args(&["--bogus"]), &[], &[]).is_err());
        assert!(options(&args(&["--author"]), &["--author"], &[]).is_err());
        assert!(options(&args(&["--json=yes"]), &[], &[]).is_err());
        assert!(options(&args(&["-h"]), &[], &[]).unwrap().help);
    }

    #[test]
    fn suggests_the_nearest_names() {
        let known: Vec<String> = ["app::OpenPalette", "editor::Bold", "editor::Italic"]
            .map(String::from)
            .to_vec();
        assert_eq!(suggest("editor::Bld", &known), ["editor::Bold"]);
        assert_eq!(suggest("app::OpenPalete", &known), ["app::OpenPalette"]);
        assert!(suggest("zzzzzzzzzz", &known).is_empty());
    }

    #[test]
    fn measures_edits() {
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "abc"), 3);
        assert_eq!(distance("same", "same"), 0);
    }
}
