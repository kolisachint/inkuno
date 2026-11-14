//! The command line of the same binary: `inkuno rev …` (the revisions of a note), `inkuno keys …`
//! (the keyboard shortcuts) and `inkuno help`. No window: it works whether or not the app is
//! running, and the app sees its changes through its file watcher. Results go to standard output,
//! errors to standard error with exit status 1; a wrong command line exits with 2.

mod keys;
mod rev;

use std::collections::BTreeMap;

use crate::lang::tr;

/// Exit status of a command that could not do its work.
const FAILED: i32 = 1;
/// Exit status of a command line that is not one of ours.
const USAGE: i32 = 2;

/// Runs a command. `args` starts with the command word (`rev`, `keys` or `help`); the result is the
/// exit status.
pub fn run(args: &[String]) -> i32 {
    let rest = &args[1..];
    match args.first().map(String::as_str) {
        Some("rev") => rev::run(rest),
        Some("keys") => keys::run(rest),
        Some("help") => help(rest),
        _ => usage(tr("unknown command", "commande inconnue")),
    }
}

/// `inkuno help [rev|keys]`: the top help, or the help of one command.
fn help(rest: &[String]) -> i32 {
    match rest.first().map(String::as_str) {
        None => {
            print!("{}", TOP);
            0
        }
        Some("rev") => {
            print!("{}", rev::HELP);
            0
        }
        Some("keys") => {
            print!("{}", keys::HELP);
            0
        }
        Some(other) => usage(&format!("no help for `{other}`: try `rev` or `keys`")),
    }
}

/// The help shown by `inkuno --help` and `inkuno help`: the commands next to the app's own options.
pub const TOP: &str = "\
inkuno rev <save|list|show|diff>   the saved versions of a note, from the command line
inkuno keys <list|set|reset>       the keyboard shortcuts, as keybindings.json
inkuno help [rev|keys]             this text, or the help of one command

Run `inkuno rev --help` or `inkuno keys --help` for the details and examples.
";

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
}

/// Reads a command line: `value_options` take the next word or `=value`, `switches` stand alone.
/// `--json` and `--help` are always switches. An unknown option is a usage error.
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
            return Err(format!("{name} takes no value"));
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
