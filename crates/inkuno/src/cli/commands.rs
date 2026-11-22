//! The vault's commands: `inkuno list`, `read`, `new`, `edit`… Each one reads its command line,
//! maps it to the JSON arguments of the tool it was (the functions the agent's server called),
//! runs it on the vault, and prints the result: the JSON, or for `read` the text of the note.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use super::{
    Opts, draw, failed, guide, ops, options,
    outcome::{Outcome, disk},
    pretty,
    store::Vault,
    suggest, text,
    text::Shown,
    usage,
};
use crate::platform::vault as files;

/// A command: its name, the help of `inkuno help <name>`, and what runs it.
struct Command {
    name: &'static str,
    help: &'static str,
    run: fn(&[String]) -> i32,
}

const COMMANDS: &[Command] = &[
    Command {
        name: "vault",
        help: VAULT_HELP,
        run: vault,
    },
    Command {
        name: "list",
        help: LIST_HELP,
        run: list,
    },
    Command {
        name: "search",
        help: SEARCH_HELP,
        run: search,
    },
    Command {
        name: "read",
        help: READ_HELP,
        run: read,
    },
    Command {
        name: "links",
        help: LINKS_HELP,
        run: links,
    },
    Command {
        name: "tags",
        help: TAGS_HELP,
        run: tags,
    },
    Command {
        name: "show",
        help: SHOW_HELP,
        run: show,
    },
    Command {
        name: "new",
        help: NEW_HELP,
        run: new,
    },
    Command {
        name: "edit",
        help: EDIT_HELP,
        run: edit,
    },
    Command {
        name: "append",
        help: APPEND_HELP,
        run: append,
    },
    Command {
        name: "write",
        help: WRITE_HELP,
        run: write,
    },
    Command {
        name: "move",
        help: MOVE_HELP,
        run: move_note,
    },
    Command {
        name: "trash",
        help: TRASH_HELP,
        run: trash,
    },
    Command {
        name: "restore",
        help: RESTORE_HELP,
        run: restore,
    },
    Command {
        name: "daily",
        help: DAILY_HELP,
        run: daily,
    },
    Command {
        name: "comment",
        help: COMMENT_HELP,
        run: comment,
    },
    Command {
        name: "render",
        help: RENDER_HELP,
        run: render,
    },
    Command {
        name: "guide",
        help: GUIDE_HELP,
        run: guide_cmd,
    },
];

/// The names of the vault's commands, in the order of the top help.
pub fn names() -> Vec<&'static str> {
    COMMANDS.iter().map(|command| command.name).collect()
}

/// The help of one command, when it is one of ours.
pub fn help(name: &str) -> Option<&'static str> {
    COMMANDS
        .iter()
        .find(|command| command.name == name)
        .map(|command| command.help)
}

/// Runs the command `name`, when it is one of ours: its exit status.
pub fn run(name: &str, rest: &[String]) -> Option<i32> {
    COMMANDS
        .iter()
        .find(|command| command.name == name)
        .map(|command| (command.run)(rest))
}

/// The words a command takes: at least `min`, at most `max`; `what` names them in the usage line.
type Words = (usize, usize, &'static str);

/// Reads the command line of `name`: its options (`--vault` is always one) and its words. The
/// `Err` is the exit status to stop with: 0 after `--help`, 2 for a wrong command line.
fn read_args(
    name: &str,
    args: &[String],
    values: &[&str],
    switches: &[&str],
    (min, max, what): Words,
) -> Result<Opts, i32> {
    let mut values = values.to_vec();
    values.push("--vault");
    let opts =
        options(args, &values, switches).map_err(|reason| usage(&format!("{name}: {reason}")))?;
    if opts.help {
        print!("{}", help(name).unwrap_or_default());
        return Err(0);
    }
    if opts.words.len() < min {
        return Err(usage(&format!("{name} needs {what}")));
    }
    if opts.words.len() > max {
        return Err(usage(&format!(
            "{name}: too many arguments: {}",
            opts.words[max..].join(" ")
        )));
    }
    Ok(opts)
}

/// The tool's arguments for the options given: each `(option, key)` pair that was given.
fn given(opts: &Opts, keys: &[(&str, &str)]) -> Value {
    let mut args = json!({});
    for (option, key) in keys {
        if let Some(value) = opts.values.get(*option) {
            args[*key] = json!(value);
        }
    }
    args
}

/// Adds a whole-number option to the tool's arguments, when it is given. Not a number, or a number
/// outside `min..=max`, is a usage error.
fn whole(
    name: &str,
    opts: &Opts,
    args: &mut Value,
    option: &str,
    key: &str,
    (min, max): (u64, u64),
) -> Result<(), i32> {
    let Some(text) = opts.values.get(option) else {
        return Ok(());
    };
    let number: u64 = text
        .parse()
        .map_err(|_| usage(&format!("{name}: `{text}` is not a whole number")))?;
    if !(min..=max).contains(&number) {
        let bound = if max == u64::MAX {
            format!("at least {min}")
        } else {
            format!("from {min} to {max}")
        };
        return Err(usage(&format!(
            "{name}: {option} takes a number {bound}, not `{text}`"
        )));
    }
    args[key] = json!(number);
    Ok(())
}

/// An option whose value is one of `allowed`; any other value is a usage error.
fn one_of(name: &str, opts: &Opts, option: &str, allowed: &[&str]) -> Result<(), i32> {
    match opts.values.get(option) {
        Some(value) if !allowed.contains(&value.as_str()) => Err(usage(&format!(
            "{name}: {option} takes one of {}, not `{value}`",
            allowed.join(", ")
        ))),
        _ => Ok(()),
    }
}

/// A `--token` that is given must not be empty: an empty one would check nothing.
fn token_given(name: &str, opts: &Opts) -> Result<(), i32> {
    match opts.values.get("--token") {
        Some(token) if token.is_empty() => Err(usage(&format!(
            "{name}: --token is empty; give the token that `read --json` gives"
        ))),
        _ => Ok(()),
    }
}

/// Standard input, read to its end. Only a `-` asks for it. A read that fails (bytes that are not
/// UTF-8, say) is a failure: an empty text would be taken for the person's words.
fn stdin_text() -> Result<String, String> {
    let mut text = String::new();
    io::stdin()
        .read_to_string(&mut text)
        .map_err(|_| "standard input could not be read as text".to_string())?;
    Ok(text)
}

/// The value of a text option: `-` is standard input, read now; any other value is the text itself.
/// `Err` when standard input cannot be read as text.
fn text_value(opts: &Opts, option: &str) -> Result<Option<String>, String> {
    match opts.values.get(option).map(String::as_str) {
        None => Ok(None),
        Some("-") => stdin_text().map(Some),
        Some(text) => Ok(Some(text.to_string())),
    }
}

/// The vault a command works on: `--vault DIR`; else the nearest folder above the current one (itself
/// included) that holds an `.inkuno` folder; else the vault the app has open.
fn vault_of(opts: &Opts) -> Result<Vault, String> {
    if let Some(dir) = opts.values.get("--vault") {
        return Vault::open(PathBuf::from(dir))
            .map_err(|why| format!("no vault at `{dir}` ({why})"));
    }
    let here = std::env::current_dir().ok();
    let found = here.as_deref().and_then(|dir| {
        dir.ancestors()
            .find(|folder| folder.join(".inkuno").is_dir())
            .map(Path::to_path_buf)
    });
    if let Some(root) = found {
        return Vault::open(root);
    }
    match files::load_config().0 {
        None => Err("no vault: open one in Inkuno, or pass `--vault`".into()),
        Some(path) if !path.is_dir() => Err(format!(
            "the vault in the config no longer exists (`{}`); pass `--vault`",
            path.display()
        )),
        Some(path) => Vault::open(path),
    }
}

/// Runs `work` on the vault of the command line.
fn on_vault(opts: &Opts, work: impl FnOnce(&mut Vault) -> Outcome) -> Result<Outcome, String> {
    let mut vault = vault_of(opts)?;
    Ok(work(&mut vault))
}

/// Prints the result: the reason on standard error when the work failed; else the JSON of the result
/// with `--json`, or else the text `text` makes of it.
fn finish(result: Result<Outcome, String>, json: bool, text: impl FnOnce(&Value) -> Shown) -> i32 {
    match result {
        Err(reason) => failed(reason),
        Ok(outcome) if outcome.is_error => failed(outcome.sentence()),
        Ok(outcome) => {
            let shown = if json {
                Shown::plain(pretty(&outcome.structured))
            } else {
                text(&outcome.structured)
            };
            print!("{}", shown.out);
            eprint!("{}", shown.err);
            0
        }
    }
}

fn vault(args: &[String]) -> i32 {
    let opts = match read_args("vault", args, &[], &[], (0, 0, "")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    finish(
        on_vault(&opts, |vault| ops::vault_info(vault)),
        opts.json,
        text::vault,
    )
}

fn list(args: &[String]) -> i32 {
    let opts = match read_args(
        "list",
        args,
        &[
            "--folder", "--tag", "--kind", "--sort", "--cursor", "--limit",
        ],
        &[],
        (0, 0, ""),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let kinds = ["note", "diagram", "table", "picture", "attachment", "all"];
    if let Err(code) = one_of("list", &opts, "--kind", &kinds).and(one_of(
        "list",
        &opts,
        "--sort",
        &["name", "recent"],
    )) {
        return code;
    }
    let mut tool = given(
        &opts,
        &[
            ("--folder", "folder"),
            ("--tag", "tag"),
            ("--kind", "kind"),
            ("--sort", "sort"),
            ("--cursor", "cursor"),
        ],
    );
    if let Err(code) = whole("list", &opts, &mut tool, "--limit", "limit", (1, 200)) {
        return code;
    }
    finish(
        on_vault(&opts, |vault| ops::list_notes(vault, &tool)),
        opts.json,
        text::list,
    )
}

fn search(args: &[String]) -> i32 {
    let opts = match read_args(
        "search",
        args,
        &["--tag", "--folder", "--limit"],
        &[],
        (1, usize::MAX, "the words to search for"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let mut tool = given(&opts, &[("--tag", "tag"), ("--folder", "folder")]);
    tool["query"] = json!(opts.words.join(" "));
    if let Err(code) = whole("search", &opts, &mut tool, "--limit", "limit", (1, 50)) {
        return code;
    }
    finish(
        on_vault(&opts, |vault| ops::search_notes(vault, &tool)),
        opts.json,
        text::search,
    )
}

fn read(args: &[String]) -> i32 {
    let opts = match read_args("read", args, &["--section"], &[], (1, 1, "a note")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let mut tool = given(&opts, &[("--section", "section")]);
    tool["note"] = json!(opts.words[0]);
    // The text alone is what a reader wants; --json adds the links, the comments and the token.
    finish(
        on_vault(&opts, |vault| ops::read_note(vault, &tool)),
        opts.json,
        text::read,
    )
}

fn links(args: &[String]) -> i32 {
    let opts = match read_args("links", args, &[], &[], (1, 1, "a note")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let tool = json!({"note": opts.words[0]});
    finish(
        on_vault(&opts, |vault| ops::note_links(vault, &tool)),
        opts.json,
        text::links,
    )
}

fn tags(args: &[String]) -> i32 {
    let opts = match read_args("tags", args, &[], &[], (0, 0, "")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    finish(
        on_vault(&opts, |vault| ops::list_tags(vault)),
        opts.json,
        text::tags,
    )
}

fn show(args: &[String]) -> i32 {
    let opts = match read_args("show", args, &[], &[], (1, 1, "a note")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let tool = json!({"note": opts.words[0]});
    finish(
        on_vault(&opts, |vault| ops::show(vault, &tool)),
        opts.json,
        text::shown,
    )
}

fn new(args: &[String]) -> i32 {
    let opts = match read_args(
        "new",
        args,
        &["--folder", "--template", "--text"],
        &[],
        (1, usize::MAX, "a title"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let mut tool = given(
        &opts,
        &[
            ("--folder", "folder"),
            ("--template", "template"),
            ("--text", "text"),
        ],
    );
    tool["title"] = json!(opts.words.join(" "));
    // The body is --text, or standard input with `--text -`; without --text the note has no body.
    match text_value(&opts, "--text") {
        Ok(Some(text)) => tool["text"] = json!(text),
        Ok(None) => {}
        Err(reason) => return failed(reason),
    }
    finish(
        on_vault(&opts, |vault| ops::create_note(vault, &tool)),
        opts.json,
        text::path,
    )
}

/// The `edits` of `inkuno edit`: one `--old` with its `--new`, or the JSON array of `--edits`.
fn edits_of(opts: &Opts) -> Result<Value, i32> {
    match (
        opts.values.get("--edits"),
        opts.values.get("--old"),
        opts.values.get("--new"),
    ) {
        (Some(text), None, None) => {
            let list = serde_json::from_str::<Value>(text)
                .ok()
                .filter(Value::is_array)
                .ok_or_else(|| {
                    usage("edit: --edits needs a JSON array of {\"old\": …, \"new\": …}")
                })?;
            for (i, item) in list.as_array().into_iter().flatten().enumerate() {
                let texts = item.get("old").is_some_and(Value::is_string)
                    && item.get("new").is_some_and(Value::is_string);
                if !texts {
                    return Err(usage(&format!(
                        "edit: --edits item {} needs a text \"old\" and a text \"new\"",
                        i + 1
                    )));
                }
            }
            Ok(list)
        }
        (None, Some(old), Some(new)) => Ok(json!([{"old": old, "new": new}])),
        _ => Err(usage("edit: give --old with --new, or --edits")),
    }
}

fn edit(args: &[String]) -> i32 {
    let opts = match read_args(
        "edit",
        args,
        &["--old", "--new", "--edits", "--token"],
        &[],
        (1, 1, "a note"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    if let Err(code) = token_given("edit", &opts) {
        return code;
    }
    let edits = match edits_of(&opts) {
        Ok(edits) => edits,
        Err(code) => return code,
    };
    let mut tool = given(&opts, &[("--token", "token")]);
    tool["note"] = json!(opts.words[0]);
    tool["edits"] = edits;
    finish(
        on_vault(&opts, |vault| ops::edit_note(vault, &tool)),
        opts.json,
        text::edit,
    )
}

fn append(args: &[String]) -> i32 {
    let opts = match read_args(
        "append",
        args,
        &["--under", "--token", "--text"],
        &[],
        (1, 1, "a note"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    if let Err(code) = token_given("append", &opts) {
        return code;
    }
    let text = match text_value(&opts, "--text") {
        Ok(Some(text)) => text,
        Ok(None) => return usage("append: give --text TEXT, or --text - to read standard input"),
        Err(reason) => return failed(reason),
    };
    let mut tool = given(&opts, &[("--under", "under"), ("--token", "token")]);
    tool["note"] = json!(opts.words[0]);
    tool["text"] = json!(text);
    finish(
        on_vault(&opts, |vault| ops::append_to_note(vault, &tool)),
        opts.json,
        text::path,
    )
}

/// `inkuno write`: a proposed full text of the note, as pending changes. `--author` is required.
fn write(args: &[String]) -> i32 {
    let opts = match read_args(
        "write",
        args,
        &["--author", "--text", "--token"],
        &["--replace"],
        (1, 1, "a note"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    if let Err(code) = token_given("write", &opts) {
        return code;
    }
    let Some(author) = opts.values.get("--author") else {
        return usage("write: give --author NAME, the agent that proposes the text");
    };
    let author = crate::logic::revision::clean_author(author);
    if author.is_empty() {
        return usage("write: --author is empty; give the name of the agent");
    }
    // The text is the whole new text: --text TEXT, or --text - for standard input.
    let proposed = match text_value(&opts, "--text") {
        Ok(Some(text)) => text,
        Ok(None) => return usage("write: give --text TEXT, or --text - to read standard input"),
        Err(reason) => return failed(reason),
    };
    // Without the token nothing says which text the agent read, so a change by the person could
    // not be told apart: the token is required.
    if !opts.values.contains_key("--token") {
        return usage(
            "write: give --token T, the token of `read --json`: it says which text the agent read",
        );
    }
    let mut tool = given(&opts, &[("--token", "token")]);
    tool["note"] = json!(opts.words[0]);
    tool["author"] = json!(author);
    tool["text"] = json!(proposed);
    if opts.switches.contains("--replace") {
        tool["replace"] = json!(true);
    }
    let result = on_vault(&opts, |vault| ops::write_note(vault, &tool));
    // The write stood, but its version could not be saved: said on standard error.
    if let Ok(outcome) = &result
        && let Some(warning) = outcome.structured["warning"].as_str()
    {
        eprintln!("inkuno: {warning}");
    }
    finish(result, opts.json, text::wrote)
}

fn move_note(args: &[String]) -> i32 {
    let opts = match read_args("move", args, &["--name", "--folder"], &[], (1, 1, "a note")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    if !opts.values.contains_key("--name") && !opts.values.contains_key("--folder") {
        return usage("move: give --name, --folder or both");
    }
    let mut tool = given(&opts, &[("--name", "name"), ("--folder", "folder")]);
    tool["note"] = json!(opts.words[0]);
    finish(
        on_vault(&opts, |vault| ops::rename_note(vault, &tool)),
        opts.json,
        text::moved,
    )
}

fn trash(args: &[String]) -> i32 {
    let opts = match read_args("trash", args, &[], &[], (1, 1, "a note")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let tool = json!({"note": opts.words[0]});
    finish(
        on_vault(&opts, |vault| ops::trash_note(vault, &tool)),
        opts.json,
        text::path,
    )
}

fn restore(args: &[String]) -> i32 {
    let opts = match read_args("restore", args, &[], &[], (1, 1, "a path in .trash")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let tool = json!({"path": opts.words[0]});
    finish(
        on_vault(&opts, |vault| ops::restore_note(vault, &tool)),
        opts.json,
        text::path,
    )
}

fn daily(args: &[String]) -> i32 {
    let opts = match read_args("daily", args, &["--date", "--append"], &[], (0, 0, "")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    if let Some(date) = opts.values.get("--date")
        && let Err(message) = ops::parse_date(date)
    {
        return usage(&format!("daily: {message}"));
    }
    let tool = given(&opts, &[("--date", "date"), ("--append", "append")]);
    finish(
        on_vault(&opts, |vault| ops::daily_note(vault, &tool)),
        opts.json,
        text::path,
    )
}

fn comment(args: &[String]) -> i32 {
    let opts = match read_args(
        "comment",
        args,
        &["--passage", "--text", "--file"],
        &[],
        (0, 1, "a note or --file PATH"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let file = opts.values.get("--file");
    if file.is_some_and(|path| path.is_empty()) {
        return usage("comment: --file needs a path");
    }
    if file.is_some() && opts.values.contains_key("--vault") {
        return usage("comment: --file names a file without a vault; drop --vault");
    }
    let mut tool = json!({});
    match (file, opts.words.first()) {
        (Some(_), Some(_)) => return usage("comment: give a note or --file, not both"),
        (None, None) => return usage("comment needs a note, or --file PATH"),
        (Some(path), None) => tool["file"] = json!(path),
        (None, Some(note)) => tool["note"] = json!(note),
    }
    let text = match text_value(&opts, "--text") {
        Ok(text) => text,
        Err(reason) => return failed(reason),
    };
    let (Some(passage), Some(text)) = (opts.values.get("--passage"), text) else {
        return usage("comment: give --passage and --text (--text - reads standard input)");
    };
    tool["passage"] = json!(passage);
    tool["comment"] = json!(text);
    let outcome = if file.is_some() {
        Ok(ops::add_comment_file(&tool))
    } else {
        on_vault(&opts, |vault| ops::add_comment(vault, &tool))
    };
    finish(outcome, opts.json, text::path)
}

fn render(args: &[String]) -> i32 {
    let opts = match read_args(
        "render",
        args,
        &[
            "--note",
            "--block",
            "--mermaid",
            "--theme",
            "--width",
            "--out",
        ],
        &["--check"],
        (0, 1, "a file"),
    ) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    // `--mermaid -` reads the source from standard input; nothing else does.
    let source = match text_value(&opts, "--mermaid") {
        Ok(source) => source,
        Err(reason) => return failed(reason),
    };
    if opts.switches.contains("--check") {
        let Some(source) = source else {
            return usage("render --check needs --mermaid SRC, or --mermaid - for standard input");
        };
        let outcome = draw::check_mermaid(&json!({"source": source}));
        let readable = outcome.structured["readable"].as_bool() == Some(true);
        let shown = if opts.json {
            pretty(&outcome.structured)
        } else {
            text::check(&outcome.structured)
        };
        print!("{shown}");
        if readable {
            return 0;
        }
        // A source that does not draw is a failure whatever the output: the reason on standard error.
        return failed(
            outcome.structured["error"]
                .as_str()
                .unwrap_or("the source does not draw"),
        );
    }
    if let Err(code) = one_of("render", &opts, "--theme", &["light", "dark"]) {
        return code;
    }
    if let Some(width) = opts.values.get("--width")
        && !width.parse::<f64>().is_ok_and(|w| w.is_finite() && w > 0.0)
    {
        return usage(&format!(
            "render: `{width}` is not a width in pixels: a number above 0"
        ));
    }
    let Some(out) = opts.values.get("--out") else {
        return usage("render needs --out FILE.png: the picture is written there");
    };
    let mut tool = given(
        &opts,
        &[
            ("--note", "note"),
            ("--theme", "theme"),
            ("--mermaid", "mermaid"),
        ],
    );
    if let Some(source) = &source {
        tool["mermaid"] = json!(source);
    }
    if let Some(path) = opts.words.first() {
        tool["path"] = json!(path);
    }
    if let Err(code) = whole(
        "render",
        &opts,
        &mut tool,
        "--block",
        "block",
        (1, u64::MAX),
    ) {
        return code;
    }
    if let Some(width) = opts
        .values
        .get("--width")
        .and_then(|w| w.parse::<f64>().ok())
    {
        tool["width"] = json!(width);
    }
    // A Mermaid source needs no vault; a picture of a note or a file does.
    let outcome = if source.is_some() {
        draw::render(None, "", &tool)
    } else {
        match vault_of(&opts) {
            Ok(vault) => draw::render(Some(&vault), "", &tool),
            Err(why) => draw::render(None, &why, &tool),
        }
    };
    if outcome.is_error {
        return failed(outcome.sentence());
    }
    let png = outcome.png.unwrap_or_default();
    if let Err(error) = files::write(Path::new(out), &png) {
        return failed(format!("{out}: {}", disk(error)));
    }
    let shown = if opts.json {
        Shown::plain(pretty(&outcome.structured))
    } else {
        text::written(out, &outcome.structured)
    };
    print!("{}", shown.out);
    0
}

fn guide_cmd(args: &[String]) -> i32 {
    let opts = match read_args("guide", args, &[], &[], (0, 1, "a page")) {
        Ok(opts) => opts,
        Err(code) => return code,
    };
    let Some(name) = opts.words.first() else {
        for page in guide::names() {
            println!("{page}");
        }
        return 0;
    };
    match guide::page(name) {
        Some(text) => {
            print!("{text}");
            0
        }
        None => {
            let names: Vec<String> = guide::names().into_iter().map(String::from).collect();
            let hint = suggest(name, &names)
                .first()
                .map(|near| format!("; did you mean `{near}`?"))
                .unwrap_or_default();
            failed(format!("no guide page `{name}`{hint}"))
        }
    }
}

const GUIDE_HELP: &str = "\
inkuno guide [page]
    the user guide, built into this program. With no page, the names of its pages, one per
    line; with a page, that page as Markdown (introduction, writing, links, comments, search…)

example:
  inkuno guide writing
";

const VAULT_HELP: &str = "\
inkuno vault [--vault DIR]
    the vault: its folder, its notes, templates, diagrams, tables, pictures and
    attachments, its folders, its most used tags, today's daily note and the version
    --json: the same facts as JSON

example:
  inkuno vault --vault ~/notes
";

const LIST_HELP: &str = "\
inkuno list [--folder F] [--tag T] [--kind K] [--sort S] [--limit N] [--cursor C]
    the notes and files of the vault, one path per line; a next page is named on standard
    error as `more: --cursor C`. --json: path, title, folder, tags, modified, kind, and next
    --folder keeps that folder and what is inside it; --tag matches a tag, with or without #
    --kind note, diagram, table, picture, attachment or all (the default)
    --sort name (the default) or recent; --limit 1 to 200 (default 100)
    --cursor: the next value of the previous page, while that page says there is one
    --json: the result as JSON

example:
  inkuno list --kind note --sort recent --limit 20
";

const SEARCH_HELP: &str = "\
inkuno search <words…> [--tag T] [--folder F] [--limit N]
    the notes whose name matches, then the notes whose text contains every word, most
    recent first, with a snippet around the match. Templates are not searched.
    one line per hit: `path: snippet` (a hit by name is its path alone). --json: the hits
    --limit 1 to 50 (default 20); a query of one letter matches names only

example:
  inkuno search meeting notes --tag work
";

const READ_HELP: &str = "\
inkuno read <note> [--section HEADING|^BLOCK] [--json]
    the text of a note; with --section, only that section or block
    --json: the path, title, tags, aliases, links, embeds, comments, the token and `unplaced`:
    the pending changes the text does not place (count and entries), shown nowhere until discarded.
    Pass the token to edit or append: a note changed since it was read is refused

example:
  inkuno read Plan --section \"Next steps\"
";

const LINKS_HELP: &str = "\
inkuno links <note>
    the notes this one links to, and where they are; the notes that link to it, with the
    line each link is on; its tags, embedded sections and pictures. Text: a section per
    kind (links, backlinks, tags, embeds, pictures), each item indented; --json: the lists

example:
  inkuno links Plan
";

const TAGS_HELP: &str = "\
inkuno tags
    every tag of the vault, with how many notes carry it, most used first: `#tag`, a tab, the
    count. --json: the tags as JSON

example:
  inkuno tags
";

const SHOW_HELP: &str = "\
inkuno show <note>
    opens the note, diagram, table or picture in the Inkuno window, in a tab of its own.
    Fails when Inkuno is not open. Changes nothing in the vault. Prints `shown: <path>`;
    --json: the same as JSON

example:
  inkuno show Plan
";

const NEW_HELP: &str = "\
inkuno new <title…> [--folder F] [--template T] [--text TEXT]
    a new note: its first line is the title and the file name. The body is --text TEXT, or
    --text - for standard input; without --text the note has only its title. A taken name is
    refused, nothing is overwritten. --template: a note of the template folder, its placeholders filled
    prints the note's path; --json: the result as JSON

example:
  echo \"First line of the body.\" | inkuno new Meeting notes --folder work --text -
";

const EDIT_HELP: &str = "\
inkuno edit <note> --old OLD --new NEW [--token T]
inkuno edit <note> --edits JSON [--token T]
    exact replacements, all or nothing: each old must occur once, else the file is kept.
    --edits takes a JSON array of {\"old\": …, \"new\": …}, applied together.
    A changed # title renames the note and rewrites its [[links]].
    --token: the token of read --json; a note changed since it was read is refused
    A change that touches a pending change is refused: accept or reject it in Inkuno first.
    prints the note's path, the new one when the title renamed it; --json: the result

example:
  inkuno edit Plan --old \"Draft\" --new \"Final\" --token 3f9a1c
";

const APPEND_HELP: &str = "\
inkuno append <note> [--under HEADING] [--text TEXT] [--token T]
    adds text at the end of the note, or at the end of the section under the heading
    (matched exactly, or as the only heading with that case). The text is --text TEXT, or
    --text - for standard input; without --text it is refused, and an empty text is refused.
    A text that touches a pending change is refused: accept or reject it in Inkuno first.
    Prints the note's path; --json: the result

example:
  inkuno append Plan --under \"Next steps\" --text \"- call Ana\"
";

const WRITE_HELP: &str = "\
inkuno write <note> --author NAME (--text TEXT | --text -) --token T [--replace]
    proposes the note's full new text. Each change is written into the note as a pending mark
    (removed lines, then added lines) until a person accepts or rejects it in Inkuno. The changes
    are kept beside the note, in .inkuno/pending: the note's file never holds a mark.
    Refused while the note has pending changes, and when the text is empty.
    --replace takes back the agent's own last proposal and writes the new one in its place. It
    is refused unless the pending changes are that proposal as it was left: the same author,
    nothing rejected, typed or commented. Accepted changes stay. Without pending changes it is a
    plain write. The text is the full note, without marks; the same text as the person's is refused.
    --token is required: the token of read --json, which says which text the agent read. A token
    of a saved version merges; a note changed in the same lines is refused. With --replace the
    token must be the note's current one.
    Each write saves two automatic revisions: the text before, and the proposed text. If the
    proposed version cannot be saved, the write stands and a warning goes to standard error.
    Prints `N changes pending in <note>`; --json: the path, the count and the two revisions,
    and `replaced: true` after a replacement

example:
  inkuno read Plan --json
  inkuno write Plan --author claude --text - --token 3f9a1c < new-plan.md
  inkuno write Plan --author claude --text - --token 7b2d4e --replace < revised-plan.md
";

const MOVE_HELP: &str = "\
inkuno move <note> [--name NEW] [--folder F]
    gives a note (or a diagram, picture or attachment) a new name, a new folder, or both.
    The [[links]] to it are rewritten, unless another note still has the old name: then
    links_left says so and nothing is rewritten. A taken name is refused. A link inside a
    pending change of another note is not rewritten: the move is refused, nothing is moved or
    written, and the message names that note. Accept or reject its pending changes first.
    prints the new path, then `links rewritten: N` when links were rewritten, and a line
    when another note kept the old name; --json: the result as JSON

example:
  inkuno move Draft --name Plan --folder projects
";

const TRASH_HELP: &str = "\
inkuno trash <note>
    moves the note or file to .trash, with its saved versions. Nothing is deleted.
    The path in .trash is printed; restore brings the file back. --json: the result as JSON

example:
  inkuno trash Old-draft
";

const RESTORE_HELP: &str = "\
inkuno restore <path>
    brings a file back from .trash to the vault root, with its saved versions. <path> is
    the path that trash printed. A taken name is numbered, never overwritten.
    prints the restored note's path; --json: the result as JSON

example:
  inkuno restore .trash/Old-draft.md
";

const DAILY_HELP: &str = "\
inkuno daily [--date YYYY-MM-DD] [--append TEXT]
    the daily note of the date, today by default. It is created from the Daily, Journal,
    Today, Note du jour or Aujourd'hui template when there is one. --append adds a paragraph.
    prints the note's path; --json: the note as read --json gives it

example:
  inkuno daily --date 2026-03-14 --append \"Shipped the release.\"
";

const COMMENT_HELP: &str = "\
inkuno comment <note> --passage P --text C [--json]
inkuno comment --file PATH --passage P --text C [--json]
    adds the comment C after the passage P: P<!-- comment: C -->, which GitHub hides. The passage
    must occur once, on one line; it is the text from the start of its line (or of its table cell,
    or of the comment before it) up to the comment.
    --file takes a .md file, named by its path, outside any vault: no vault is needed. A note and
    --file together, or --vault with --file, is a usage error.
    --text - reads the comment from standard input. Its line breaks become spaces.
    Prints the note's path (--file: the file's absolute path); --json: the result as JSON
    In a table cell, a | of the comment is written \\|. A --> in the comment is written -- >.
    A passage in a pending change is refused: accept or reject the change in Inkuno first.

example:
  inkuno comment Plan --passage \"two weeks\" --text \"Is this realistic?\"
  inkuno comment --file ~/notes/scratch.md --passage \"two weeks\" --text \"Is this realistic?\"
";

const RENDER_HELP: &str = "\
inkuno render [<file>] [--note N --block K] [--mermaid SRC] [--theme light|dark] [--width W] --out FILE.png
inkuno render --check --mermaid SRC|-
    a PNG of an SVG file of the vault, of the K-th mermaid block of a note (1 is the first),
    or of a Mermaid source (--mermaid -: read from standard input). The PNG is written to
    --out, and its size and kind are printed (--json: as JSON). --width is a number above 0.
    --check: prints ok and the kind when the source draws; when it does not, the engine's
    error goes to standard error and the status is 1 (--json: the result as JSON)

example:
  inkuno render --note Plan --block 1 --theme dark --out plan.png
";
