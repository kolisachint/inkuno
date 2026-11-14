//! The command line (`inkuno rev`, `inkuno keys`, `inkuno help`) run as a real binary, on a temporary
//! vault. Its configuration folder is a temporary one too: the real one is never touched.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde_json::Value;

/// One run of the binary: its exit status and what it printed.
struct Run {
    code: i32,
    out: String,
    err: String,
}

/// The configuration folder the binary reads under the sandbox `home`, as `vault::config_dir` picks
/// it: macOS keeps it in `Library/Application Support` under `HOME` and ignores `XDG_CONFIG_HOME`.
fn config_folder(home: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support")
    } else {
        home.join("cfg")
    }
}

/// A fresh temporary home (configuration folder included) and vault, named after the test.
fn sandbox(name: &str) -> (PathBuf, PathBuf) {
    let home = std::env::temp_dir().join(format!("inkuno-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let vault = home.join("vault");
    fs::create_dir_all(vault.join(".inkuno")).unwrap();
    fs::create_dir_all(config_folder(&home)).unwrap();
    (home, vault)
}

/// Runs `inkuno` with `args` in `dir`, in the sandbox `home`, with no author in the environment.
fn inkuno(home: &Path, dir: &Path, args: &[&str]) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_inkuno"))
        .current_dir(dir)
        .args(args)
        .env("XDG_CONFIG_HOME", config_folder(home))
        .env("HOME", home)
        .env_remove("INKUNO_AUTHOR")
        .env_remove("USER")
        .env_remove("USERNAME")
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs");
    Run {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Runs `inkuno` with `input` on its standard input, as a pipe would give it.
fn inkuno_in(home: &Path, dir: &Path, args: &[&str], input: &str) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_inkuno"))
        .current_dir(dir)
        .args(args)
        .env("XDG_CONFIG_HOME", config_folder(home))
        .env("HOME", home)
        .env_remove("INKUNO_AUTHOR")
        .env_remove("USER")
        .env_remove("USERNAME")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().expect("the binary ends");
    Run {
        code: output.status.code().unwrap_or(-1),
        out: String::from_utf8_lossy(&output.stdout).into_owned(),
        err: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The token `read --json` gives for a note.
fn token_of(home: &Path, vault: &Path, note: &str) -> String {
    let run = inkuno(home, vault, &["read", note, "--json"]);
    assert_eq!(run.code, 0, "{}", run.err);
    let read: Value = serde_json::from_str(&run.out).unwrap();
    read["token"].as_str().unwrap().to_string()
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn save_list_show_and_diff_round_trip() {
    let (home, vault) = sandbox("round");
    let note = vault.join("ideas/plan.md");
    write(&note, "# Plan\n\nOne idea.\n");
    let first = inkuno(
        &home,
        &vault,
        &[
            "rev",
            "save",
            "ideas/plan.md",
            "--author",
            "ana",
            "-m",
            "first draft",
        ],
    );
    assert_eq!((first.code, first.out.trim()), (0, "1"), "{}", first.err);
    write(&note, "# Plan\n\nTwo ideas.\n");
    let second = inkuno(
        &home,
        &vault,
        &["rev", "save", "ideas/plan.md", "--author", "bo"],
    );
    assert_eq!((second.code, second.out.trim()), (0, "2"), "{}", second.err);

    // The history is a folder under the vault's hidden .inkuno, one file per revision.
    let folder = vault.join(".inkuno/revisions/ideas/plan.md");
    assert!(folder.join("0001.md").is_file() && folder.join("0002.md").is_file());
    let file = fs::read_to_string(folder.join("0002.md")).unwrap();
    assert!(
        file.starts_with("---\nrevision: 2\nparent: 1\nauthor: bo\n"),
        "{file}"
    );
    assert!(file.ends_with("---\n# Plan\n\nTwo ideas.\n"), "{file}");

    let list = inkuno(&home, &vault, &["rev", "list", "ideas/plan.md"]);
    assert_eq!(list.code, 0);
    let lines: Vec<&str> = list.out.lines().collect();
    assert_eq!(lines.len(), 2, "{}", list.out);
    assert!(lines[0].contains("bo") && lines[0].trim_start().starts_with('2'));
    assert!(lines[1].contains("ana") && lines[1].ends_with("first draft"));

    let show = inkuno(&home, &vault, &["rev", "show", "ideas/plan.md", "1"]);
    assert_eq!((show.code, show.out.as_str()), (0, "# Plan\n\nOne idea.\n"));

    // Nothing changed since revision 2: the diff against the file is empty.
    let same = inkuno(&home, &vault, &["rev", "diff", "ideas/plan.md"]);
    assert_eq!((same.code, same.out.as_str()), (0, ""));

    write(&note, "# Plan\n\nOne idea, then two.\n");
    let latest = inkuno(&home, &vault, &["rev", "diff", "ideas/plan.md"]);
    assert_eq!(latest.code, 0);
    assert_eq!(
        latest.out,
        "--- a/plan.md@2\n+++ b/plan.md@current\n@@ -1,3 +1,3 @@\n # Plan\n \n-Two ideas.\n+One idea, then two.\n"
    );
    let two = inkuno(&home, &vault, &["rev", "diff", "ideas/plan.md", "1", "2"]);
    assert!(
        two.out.starts_with("--- a/plan.md@1\n+++ b/plan.md@2\n"),
        "{}",
        two.out
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn two_authors_and_a_message_are_kept() {
    let (home, vault) = sandbox("authors");
    write(&vault.join("a.md"), "one\n");
    assert_eq!(
        inkuno(&home, &vault, &["rev", "save", "a.md", "--author", "ana"]).code,
        0
    );
    write(&vault.join("a.md"), "two\n");
    let memo = "kept: the plugin idea is out: \"local-first\"";
    assert_eq!(
        inkuno(
            &home,
            &vault,
            &["rev", "save", "a.md", "--author", "bo", "-m", memo]
        )
        .code,
        0
    );
    let json = inkuno(&home, &vault, &["rev", "list", "a.md", "--json"]);
    let items: Value = serde_json::from_str(&json.out).expect("the list is JSON");
    assert_eq!(items[0]["author"], "bo");
    assert_eq!(items[0]["note"], memo);
    assert_eq!(items[1]["author"], "ana");
    assert!(items[1]["note"].is_null());
    assert_eq!(items[0]["parent"], 1);
    assert!(items[1]["parent"].is_null());
    let _ = fs::remove_dir_all(home);
}

#[test]
fn the_author_defaults_to_the_environment_then_unknown() {
    let (home, vault) = sandbox("author");
    write(&vault.join("a.md"), "one\n");
    let run = Command::new(env!("CARGO_BIN_EXE_inkuno"))
        .current_dir(&vault)
        .args(["rev", "save", "a.md"])
        .env("XDG_CONFIG_HOME", config_folder(&home))
        .env("HOME", &home)
        .env("INKUNO_AUTHOR", "  kim  ")
        .output()
        .unwrap();
    assert!(run.status.success());
    let json = inkuno(&home, &vault, &["rev", "show", "a.md", "1", "--json"]);
    let rev: Value = serde_json::from_str(&json.out).unwrap();
    assert_eq!(rev["author"], "kim");
    write(&vault.join("a.md"), "two\n");
    assert_eq!(inkuno(&home, &vault, &["rev", "save", "a.md"]).code, 0);
    let rev: Value =
        serde_json::from_str(&inkuno(&home, &vault, &["rev", "show", "a.md", "2", "--json"]).out)
            .unwrap();
    assert_eq!(rev["author"], "unknown");
    let _ = fs::remove_dir_all(home);
}

#[test]
fn identical_text_is_refused_with_no_new_file() {
    let (home, vault) = sandbox("same");
    write(&vault.join("a.md"), "text\n");
    assert_eq!(
        inkuno(&home, &vault, &["rev", "save", "a.md", "--author", "ana"]).code,
        0
    );
    let again = inkuno(&home, &vault, &["rev", "save", "a.md", "--author", "ana"]);
    assert_eq!(again.code, 1);
    assert!(
        again.err.contains("nothing changed since revision 1"),
        "{}",
        again.err
    );
    assert!(again.out.is_empty());
    assert!(!vault.join(".inkuno/revisions/a.md/0002.md").exists());
    let _ = fs::remove_dir_all(home);
}

#[test]
fn critic_marks_the_changes_inline() {
    let (home, vault) = sandbox("critic");
    write(&vault.join("a.md"), "keep\nold\nend\n");
    assert_eq!(
        inkuno(&home, &vault, &["rev", "save", "a.md", "--author", "ana"]).code,
        0
    );
    write(&vault.join("a.md"), "keep\nnew\nend\n");
    let out = inkuno(
        &home,
        &vault,
        &["rev", "diff", "a.md", "1", "--format", "critic"],
    );
    assert_eq!(
        (out.code, out.out.as_str()),
        (0, "keep\n{--old--}\n{++new++}\nend\n")
    );
    let bad = inkuno(&home, &vault, &["rev", "diff", "a.md", "--format", "html"]);
    assert_eq!(bad.code, 1);
    assert!(bad.err.contains("unified or critic"), "{}", bad.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_unified_diff_applies_with_patch() {
    if Command::new("patch").arg("--version").output().is_err() {
        eprintln!("patch is not installed: the test is skipped");
        return;
    }
    let (home, vault) = sandbox("patch");
    let old = "# Title\n\nline 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nno end";
    write(&vault.join("a.md"), old);
    assert_eq!(
        inkuno(&home, &vault, &["rev", "save", "a.md", "--author", "ana"]).code,
        0
    );
    let new = "# Title\n\nline 1\nLINE 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8 and more\nno end\n";
    write(&vault.join("a.md"), new);
    let diff = inkuno(&home, &vault, &["rev", "diff", "a.md", "1"]);
    assert_eq!(diff.code, 0, "{}", diff.err);
    let patch_file = home.join("change.patch");
    write(&patch_file, &diff.out);
    let target = home.join("target.md");
    write(&target, old);
    let applied = Command::new("patch")
        .args(["--no-backup-if-mismatch", "-o"])
        .arg(home.join("result.md"))
        .arg(&target)
        .stdin(fs::File::open(&patch_file).unwrap())
        .output()
        .unwrap();
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert_eq!(fs::read_to_string(home.join("result.md")).unwrap(), new);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_vault_above_the_note_holds_its_history() {
    let (home, vault) = sandbox("above");
    write(&vault.join("deep/er/note.md"), "x\n");
    assert_eq!(
        inkuno(
            &home,
            &vault.join("deep/er"),
            &["rev", "save", "note.md", "--author", "ana"]
        )
        .code,
        0
    );
    assert!(
        vault
            .join(".inkuno/revisions/deep/er/note.md/0001.md")
            .is_file()
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_scratch_note_keeps_its_history_in_the_scratch_folder() {
    let (home, vault) = sandbox("scratch");
    let scratch = config_folder(&home).join("inkuno/scratch");
    write(&scratch.join("2026-11-14-103200.md"), "idea\n");
    let note = scratch
        .join("2026-11-14-103200.md")
        .to_string_lossy()
        .into_owned();
    let run = inkuno(&home, &vault, &["rev", "save", &note, "--author", "ana"]);
    assert_eq!((run.code, run.out.trim()), (0, "1"), "{}", run.err);
    assert!(
        scratch
            .join(".revisions/2026-11-14-103200.md/0001.md")
            .is_file()
    );
    assert!(
        !vault
            .join(".inkuno/revisions")
            .join("2026-11-14-103200.md")
            .exists()
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn dot_dot_paths_never_write_outside_the_vault_history() {
    let (home, vault) = sandbox("dots");
    write(&vault.join("notes/x.md"), "# X\n");
    write(&home.join("outside/y.md"), "# Y\n");
    // `notes/../../../<home>/vault/notes/x.md` names `vault/notes/x.md` on disk, but folding its
    // `..` by name would climb out of `.inkuno/revisions` and write into the vault's own tree.
    let name = home.file_name().unwrap().to_string_lossy().into_owned();
    let climbing = vault.join(format!("notes/../../../{name}/vault/notes/x.md"));
    let run = inkuno(
        &home,
        &vault,
        &[
            "rev",
            "save",
            &climbing.to_string_lossy(),
            "--author",
            "ana",
        ],
    );
    assert_eq!((run.code, run.out.trim()), (0, "1"), "{}", run.err);
    assert!(vault.join(".inkuno/revisions/notes/x.md/0001.md").is_file());
    assert_eq!(top_level(&vault), [".inkuno", "notes"]);

    // A note outside the vault gets no revision in the vault at all: it is a note of its own
    // folder, which keeps its history in that folder's `.inkuno`, as for any note outside a vault.
    let outside = vault.join("notes/../../outside/y.md");
    let run = inkuno(
        &home,
        &vault,
        &["rev", "save", &outside.to_string_lossy(), "--author", "ana"],
    );
    assert_eq!((run.code, run.out.trim()), (0, "1"), "{}", run.err);
    assert!(
        home.join("outside/.inkuno/revisions/y.md/0001.md")
            .is_file()
    );
    assert!(!vault.join(".inkuno/outside").exists());
    assert_eq!(top_level(&vault), [".inkuno", "notes"]);
    let _ = fs::remove_dir_all(home);
}

/// The names directly in `dir`, sorted.
fn top_level(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

// Symbolic links are made on unix only: creating one on Windows needs a privilege the tests do not have.
#[cfg(unix)]
#[test]
fn a_symlinked_note_in_the_vault_keeps_its_history_with_the_note() {
    let (home, vault) = sandbox("link");
    write(&vault.join("ideas/plan.md"), "# Plan\n\nOne.\n");
    std::os::unix::fs::symlink(vault.join("ideas/plan.md"), vault.join("link.md")).unwrap();
    let first = inkuno(
        &home,
        &vault,
        &["rev", "save", "ideas/../ideas/plan.md", "--author", "ana"],
    );
    assert_eq!((first.code, first.out.trim()), (0, "1"), "{}", first.err);
    write(&vault.join("ideas/plan.md"), "# Plan\n\nTwo.\n");
    let second = inkuno(&home, &vault, &["rev", "save", "link.md", "--author", "bo"]);
    assert_eq!((second.code, second.out.trim()), (0, "2"), "{}", second.err);
    // Both saves land in the folder of the real note, not in one named after the link.
    let folder = vault.join(".inkuno/revisions/ideas/plan.md");
    assert!(folder.join("0001.md").is_file() && folder.join("0002.md").is_file());
    assert!(!vault.join(".inkuno/revisions/link.md").exists());
    let list = inkuno(&home, &vault, &["rev", "list", "ideas/plan.md"]);
    assert_eq!(list.out.lines().count(), 2, "{}", list.out);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn missing_notes_and_revisions_fail_on_stderr() {
    let (home, vault) = sandbox("missing");
    let no_note = inkuno(&home, &vault, &["rev", "list", "nope.md"]);
    assert_eq!(no_note.code, 1);
    assert!(no_note.err.contains("no such note"), "{}", no_note.err);
    assert!(no_note.out.is_empty());
    write(&vault.join("a.md"), "x\n");
    let none_yet = inkuno(&home, &vault, &["rev", "show", "a.md", "1"]);
    assert_eq!(none_yet.code, 1);
    assert!(none_yet.err.contains("none saved yet"), "{}", none_yet.err);
    assert_eq!(
        inkuno(&home, &vault, &["rev", "save", "a.md", "--author", "ana"]).code,
        0
    );
    let missing = inkuno(&home, &vault, &["rev", "show", "a.md", "9"]);
    assert_eq!(missing.code, 1);
    assert!(missing.err.contains("no revision 9"), "{}", missing.err);
    let bad_number = inkuno(&home, &vault, &["rev", "show", "a.md", "two"]);
    assert_eq!(bad_number.code, 1);
    let usage = inkuno(&home, &vault, &["rev", "frobnicate"]);
    assert_eq!(usage.code, 2);
    assert!(usage.err.contains("unknown rev command"), "{}", usage.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn help_names_every_command_with_an_example() {
    let (home, vault) = sandbox("help");
    let top = inkuno(&home, &vault, &["--help"]);
    assert!(
        top.out.contains("inkuno rev")
            && top.out.contains("inkuno keys")
            && top.out.contains("inkuno help")
    );
    let rev = inkuno(&home, &vault, &["help", "rev"]);
    assert_eq!(rev.code, 0);
    assert!(rev.out.contains("examples:") && rev.out.contains("inkuno rev save"));
    let keys = inkuno(&home, &vault, &["keys", "--help"]);
    assert!(keys.out.contains("examples:") && keys.out.contains("keys set"));
    let _ = fs::remove_dir_all(home);
}

#[test]
fn keys_lists_sets_and_resets_shortcuts() {
    let (home, vault) = sandbox("keys");
    let listed = inkuno(&home, &vault, &["keys", "list"]);
    assert_eq!(listed.code, 0);
    assert!(
        listed
            .out
            .lines()
            .any(|l| l.starts_with("app::OpenPalette") && l.ends_with("default")),
        "{}",
        listed.out
    );

    let set = inkuno(
        &home,
        &vault,
        &["keys", "set", "app::OpenPalette", "secondary-shift-o"],
    );
    assert_eq!(
        (set.code, set.out.as_str()),
        (0, "app::OpenPalette: secondary-shift-o\n"),
        "{}",
        set.err
    );
    let file = fs::read_to_string(config_folder(&home).join("inkuno/keybindings.json")).unwrap();
    let json: Value = serde_json::from_str(&file).expect("the file is JSON");
    assert_eq!(json["version"], 1);
    assert_eq!(json["bindings"]["app::OpenPalette"][0], "secondary-shift-o");
    let after = inkuno(&home, &vault, &["keys", "list", "--json"]);
    let all: Value = serde_json::from_str(&after.out).unwrap();
    let palette = all
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["action"] == "app::OpenPalette")
        .unwrap();
    assert_eq!(palette["changed"], true);
    assert_eq!(palette["keys"][0], "secondary-shift-o");

    let reset = inkuno(&home, &vault, &["keys", "reset", "app::OpenPalette"]);
    assert_eq!(reset.code, 0, "{}", reset.err);
    let back: Value =
        serde_json::from_str(&inkuno(&home, &vault, &["keys", "list", "--json"]).out).unwrap();
    let palette = back
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["action"] == "app::OpenPalette")
        .unwrap();
    assert_eq!(palette["changed"], false);
    assert_eq!(palette["keys"], palette["default"]);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn keys_refuses_bad_actions_chords_and_duplicates() {
    let (home, vault) = sandbox("keys-refuse");
    let typo = inkuno(
        &home,
        &vault,
        &["keys", "set", "app::OpenPalete", "secondary-p"],
    );
    assert_eq!(typo.code, 1);
    assert!(
        typo.err.contains("did you mean") && typo.err.contains("app::OpenPalette"),
        "{}",
        typo.err
    );

    let unreadable = inkuno(
        &home,
        &vault,
        &["keys", "set", "editor::Bold", "not-a-chord-at-all"],
    );
    assert_eq!(unreadable.code, 1);
    assert!(
        unreadable.err.contains("unreadable chord"),
        "{}",
        unreadable.err
    );

    let typing = inkuno(&home, &vault, &["keys", "set", "editor::Bold", "f"]);
    assert_eq!(typing.code, 1);
    assert!(typing.err.contains("types text"), "{}", typing.err);

    // Ctrl+P is the default of the palette: another action cannot take it in the same place.
    let taken = inkuno(
        &home,
        &vault,
        &["keys", "set", "editor::Bold", "secondary-p"],
    );
    assert_eq!(taken.code, 1);
    assert!(
        taken
            .err
            .contains("already the shortcut of app::OpenPalette"),
        "{}",
        taken.err
    );
    assert!(
        !config_folder(&home)
            .join("inkuno/keybindings.json")
            .exists(),
        "nothing is written on a refusal"
    );

    let no_chord = inkuno(&home, &vault, &["keys", "set", "editor::Bold"]);
    assert_eq!(no_chord.code, 1);
    let unknown_reset = inkuno(&home, &vault, &["keys", "reset", "app::Nope"]);
    assert_eq!(unknown_reset.code, 1);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn vault_reports_the_folder_and_its_notes() {
    let (home, vault) = sandbox("vault");
    write(&vault.join("ideas/plan.md"), "# Plan\n");
    let run = inkuno(&home, &vault, &["vault", "--json"]);
    assert_eq!(run.code, 0, "{}", run.err);
    let info: Value = serde_json::from_str(&run.out).unwrap();
    assert_eq!(info["notes"], 1);
    assert_eq!(info["folders"][0], "ideas");
    // Without --json: key: value lines, a list joined by commas.
    let text = inkuno(&home, &vault, &["vault"]);
    assert_eq!(text.code, 0, "{}", text.err);
    assert!(
        text.out.lines().any(|line| line == "notes: 1"),
        "{}",
        text.out
    );
    assert!(
        text.out.lines().any(|line| line == "folders: ideas"),
        "{}",
        text.out
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn new_takes_its_body_from_standard_input_and_read_gives_the_token() {
    let (home, vault) = sandbox("new");
    let made = inkuno_in(
        &home,
        &vault,
        &["new", "Meeting", "notes", "--folder", "work", "--text", "-"],
        "First line.\n",
    );
    assert_eq!(
        (made.code, made.out.as_str()),
        (0, "work/Meeting notes.md\n"),
        "{}",
        made.err
    );
    let text = inkuno(&home, &vault, &["read", "work/Meeting notes.md"]);
    assert_eq!(text.out, "# Meeting notes\n\nFirst line.\n");
    let read = inkuno(&home, &vault, &["read", "Meeting notes", "--json"]);
    let read: Value = serde_json::from_str(&read.out).unwrap();
    assert_eq!(read["text"], "# Meeting notes\n\nFirst line.\n");
    assert_eq!(read["token"].as_str().unwrap().len(), 64);
    // Without --text, standard input is not read: the note has no body, whatever is piped in.
    let bare = inkuno_in(&home, &vault, &["new", "Bare"], "not read\n");
    assert_eq!(bare.code, 0, "{}", bare.err);
    assert_eq!(
        fs::read_to_string(vault.join("Bare.md")).unwrap(),
        "# Bare\n\n"
    );
    // An empty standard input under --text - is no body at all.
    let empty = inkuno_in(&home, &vault, &["new", "Empty", "--text", "-"], "");
    assert_eq!(empty.code, 0, "{}", empty.err);
    assert_eq!(
        fs::read_to_string(vault.join("Empty.md")).unwrap(),
        "# Empty\n\n"
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn edit_takes_the_token_and_refuses_a_stale_one() {
    let (home, vault) = sandbox("edit");
    write(&vault.join("plan.md"), "# Plan\n\nDraft.\n");
    let token = token_of(&home, &vault, "plan.md");
    let good = inkuno(
        &home,
        &vault,
        &[
            "edit",
            "plan.md",
            "--old",
            "Draft.",
            "--new",
            "Final.",
            "--token",
            token.as_str(),
        ],
    );
    assert_eq!(
        (good.code, good.out.as_str()),
        (0, "plan.md\n"),
        "{}",
        good.err
    );
    let file = vault.join("plan.md");
    assert_eq!(fs::read_to_string(&file).unwrap(), "# Plan\n\nFinal.\n");
    // The token of the text before the edit no longer matches: refused, and the file is kept.
    let stale = inkuno(
        &home,
        &vault,
        &[
            "edit",
            "plan.md",
            "--old",
            "Final.",
            "--new",
            "Last.",
            "--token",
            token.as_str(),
        ],
    );
    assert_eq!(stale.code, 1);
    assert!(stale.out.is_empty(), "{}", stale.out);
    assert!(stale.err.contains("changed since"), "{}", stale.err);
    assert_eq!(fs::read_to_string(&file).unwrap(), "# Plan\n\nFinal.\n");
    // A list of edits is all or nothing: one that does not match changes nothing.
    let partly = inkuno(
        &home,
        &vault,
        &[
            "edit",
            "plan.md",
            "--edits",
            r#"[{"old":"Final.","new":"Done."},{"old":"missing","new":"x"}]"#,
        ],
    );
    assert_eq!(partly.code, 1);
    assert_eq!(fs::read_to_string(&file).unwrap(), "# Plan\n\nFinal.\n");
    let all = inkuno(
        &home,
        &vault,
        &[
            "edit",
            "plan.md",
            "--edits",
            r#"[{"old":"Final.","new":"Done."}]"#,
        ],
    );
    assert_eq!(all.code, 0, "{}", all.err);
    assert_eq!(fs::read_to_string(&file).unwrap(), "# Plan\n\nDone.\n");
    let _ = fs::remove_dir_all(home);
}

#[test]
fn append_goes_under_a_heading() {
    let (home, vault) = sandbox("append");
    write(
        &vault.join("plan.md"),
        "# Plan\n\n## Next\n\n- one\n\n## Later\n\nsoon\n",
    );
    let run = inkuno_in(
        &home,
        &vault,
        &["append", "plan.md", "--under", "Next", "--text", "-"],
        "- two\n",
    );
    assert_eq!(
        (run.code, run.out.as_str()),
        (0, "plan.md\n"),
        "{}",
        run.err
    );
    let text = fs::read_to_string(vault.join("plan.md")).unwrap();
    assert!(text.contains("- one\n\n- two\n\n## Later"), "{text}");
    // An empty text is refused, and nothing is written.
    let empty = inkuno_in(&home, &vault, &["append", "plan.md", "--text", "-"], "");
    assert_eq!(empty.code, 1);
    assert_eq!(fs::read_to_string(vault.join("plan.md")).unwrap(), text);
    // Without --text there is no text to add: a usage error, and standard input is not read.
    let none = inkuno_in(&home, &vault, &["append", "plan.md"], "not read\n");
    assert_eq!(none.code, 2, "{}", none.err);
    assert_eq!(fs::read_to_string(vault.join("plan.md")).unwrap(), text);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn move_renames_a_note_and_rewrites_the_links_to_it() {
    let (home, vault) = sandbox("move");
    write(&vault.join("Old.md"), "# Old\n\nbody\n");
    write(&vault.join("other.md"), "# Other\n\nSee [[Old]] now.\n");
    let run = inkuno(
        &home,
        &vault,
        &[
            "move", "Old", "--name", "New", "--folder", "archive", "--json",
        ],
    );
    assert_eq!(run.code, 0, "{}", run.err);
    let moved: Value = serde_json::from_str(&run.out).unwrap();
    assert_eq!(moved["path"], "archive/New.md");
    assert_eq!(moved["rewritten"], 1);
    assert!(vault.join("archive/New.md").is_file());
    assert!(!vault.join("Old.md").exists());
    assert!(
        fs::read_to_string(vault.join("archive/New.md"))
            .unwrap()
            .starts_with("# New")
    );
    let other = fs::read_to_string(vault.join("other.md")).unwrap();
    assert!(other.contains("See [[New]] now."), "{other}");
    let _ = fs::remove_dir_all(home);
}

#[test]
fn move_prints_the_path_and_says_what_became_of_the_links() {
    let (home, vault) = sandbox("move-text");
    write(&vault.join("Old.md"), "# Old\n\nbody\n");
    write(&vault.join("other.md"), "# Other\n\nSee [[Old]] now.\n");
    let run = inkuno(
        &home,
        &vault,
        &["move", "Old", "--name", "New", "--folder", "archive"],
    );
    assert_eq!(
        (run.code, run.out.as_str()),
        (0, "archive/New.md\nlinks rewritten: 1\n"),
        "{}",
        run.err
    );
    // A new name alone keeps the note in its folder, and a path without .md finds it.
    let kept = inkuno(&home, &vault, &["move", "archive/New", "--name", "Newer"]);
    assert_eq!(
        (kept.code, kept.out.as_str()),
        (0, "archive/Newer.md\nlinks rewritten: 1\n"),
        "{}",
        kept.err
    );
    // Another note still has the old name: nothing is rewritten, and the text says so.
    write(&vault.join("a/Dup.md"), "# Dup\n");
    write(&vault.join("b/Dup.md"), "# Dup\n");
    write(&vault.join("c.md"), "See [[Dup]].\n");
    let left = inkuno(&home, &vault, &["move", "a/Dup", "--name", "Dup2"]);
    assert_eq!(
        (left.code, left.out.as_str()),
        (
            0,
            "a/Dup2.md\nanother note still has the old name, so the links were left alone\n"
        ),
        "{}",
        left.err
    );
    assert_eq!(
        fs::read_to_string(vault.join("c.md")).unwrap(),
        "See [[Dup]].\n"
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_note_or_file_may_be_named_without_its_extension() {
    let (home, vault) = sandbox("bare-path");
    write(&vault.join("ideas/Plan.md"), "# Plan\n\nDraft.\n");
    write(
        &vault.join("pics/a.svg"),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"10\"><rect width=\"5\" height=\"5\"/></svg>",
    );
    let read = inkuno(&home, &vault, &["read", "ideas/Plan"]);
    assert_eq!(
        (read.code, read.out.as_str()),
        (0, "# Plan\n\nDraft.\n"),
        "{}",
        read.err
    );
    // A file that is not a note keeps its extension; a path out of the vault is still refused.
    let out = home.join("a.png");
    let picture = inkuno(
        &home,
        &vault,
        &["render", "pics/a.svg", "--out", out.to_str().unwrap()],
    );
    assert_eq!(picture.code, 0, "{}", picture.err);
    assert!(picture.out.starts_with("wrote "), "{}", picture.out);
    let escape = inkuno(&home, &vault, &["read", "../ideas/Plan"]);
    assert_eq!(escape.code, 1);
    assert!(
        escape.err.contains("cannot leave the vault"),
        "{}",
        escape.err
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn trash_moves_a_note_away_and_restore_brings_it_back() {
    let (home, vault) = sandbox("trash");
    write(&vault.join("Gone.md"), "# Gone\n");
    let trashed = inkuno(&home, &vault, &["trash", "Gone", "--json"]);
    assert_eq!(trashed.code, 0, "{}", trashed.err);
    let path: Value = serde_json::from_str(&trashed.out).unwrap();
    assert_eq!(path["path"], ".trash/Gone.md");
    assert!(!vault.join("Gone.md").exists());
    assert!(vault.join(".trash/Gone.md").is_file());
    let back = inkuno(&home, &vault, &["restore", ".trash/Gone.md"]);
    assert_eq!(
        (back.code, back.out.as_str()),
        (0, "Gone.md\n"),
        "{}",
        back.err
    );
    assert!(vault.join("Gone.md").is_file());
    // Without --json: the path in .trash, then nothing else.
    let text = inkuno(&home, &vault, &["trash", "Gone"]);
    assert_eq!(
        (text.code, text.out.as_str()),
        (0, ".trash/Gone.md\n"),
        "{}",
        text.err
    );
    // A file outside .trash is not restored, and a path that climbs out is refused.
    let climb = inkuno(&home, &vault, &["restore", "../Gone.md"]);
    assert_eq!(climb.code, 1);
    assert!(climb.err.contains("cannot leave"), "{}", climb.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn list_search_tags_and_links_read_the_vault() {
    let (home, vault) = sandbox("reading");
    write(&vault.join("A.md"), "# A\n\n#work, see [[B]].\n");
    write(&vault.join("B.md"), "# B\n\nhello world #work\n");
    let listed = inkuno(
        &home,
        &vault,
        &["list", "--kind", "note", "--sort", "name", "--json"],
    );
    let listed: Value = serde_json::from_str(&listed.out).unwrap();
    assert_eq!(listed["notes"].as_array().unwrap().len(), 2);
    assert!(listed["next"].is_null());
    let names = inkuno(&home, &vault, &["list", "--kind", "note", "--sort", "name"]);
    assert_eq!(
        (names.code, names.out.as_str()),
        (0, "A.md\nB.md\n"),
        "{}",
        names.err
    );
    // A page that is not the last names the next one on standard error.
    let page = inkuno(&home, &vault, &["list", "--kind", "note", "--limit", "1"]);
    assert_eq!(
        (page.out.as_str(), page.err.as_str()),
        ("A.md\n", "more: --cursor 1\n")
    );
    let hits = inkuno(&home, &vault, &["search", "hello", "world", "--json"]);
    let hits: Value = serde_json::from_str(&hits.out).unwrap();
    assert_eq!(hits["hits"][0]["title"], "B");
    let hit_text = inkuno(&home, &vault, &["search", "hello", "world"]);
    assert_eq!(
        hit_text.out, "B.md: hello world #work\n",
        "{}",
        hit_text.err
    );
    let tags = inkuno(&home, &vault, &["tags", "--json"]);
    let tags: Value = serde_json::from_str(&tags.out).unwrap();
    assert_eq!(
        tags["tags"][0],
        serde_json::json!({"tag": "work", "count": 2})
    );
    let tag_text = inkuno(&home, &vault, &["tags"]);
    assert_eq!(tag_text.out, "#work\t2\n", "{}", tag_text.err);
    let links = inkuno(&home, &vault, &["links", "A.md", "--json"]);
    assert_eq!(links.code, 0, "{}", links.err);
    let links: Value = serde_json::from_str(&links.out).unwrap();
    assert_eq!(links["outgoing"][0]["path"], "B.md");
    let link_text = inkuno(&home, &vault, &["links", "A.md"]);
    assert_eq!(
        link_text.out, "links:\n  B.md:3\ntags:\n  #work\n",
        "{}",
        link_text.err
    );
    let back = inkuno(&home, &vault, &["links", "B.md", "--json"]);
    let back: Value = serde_json::from_str(&back.out).unwrap();
    assert_eq!(back["backlinks"][0]["title"], "A");
    let back_text = inkuno(&home, &vault, &["links", "B.md"]);
    assert_eq!(
        back_text.out, "backlinks:\n  A.md: #work, see [[B]].\ntags:\n  #work\n",
        "{}",
        back_text.err
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn comment_wraps_one_passage_and_daily_writes_its_day() {
    let (home, vault) = sandbox("comment");
    write(&vault.join("c.md"), "# C\n\nThe passage here.\n");
    let run = inkuno(
        &home,
        &vault,
        &["comment", "c.md", "--passage", "passage", "--text", "more?"],
    );
    assert_eq!((run.code, run.out.as_str()), (0, "c.md\n"), "{}", run.err);
    assert!(
        fs::read_to_string(vault.join("c.md"))
            .unwrap()
            .contains("{==passage==}{>>more?<<}")
    );
    let missing = inkuno(&home, &vault, &["comment", "c.md", "--passage", "here"]);
    assert_eq!(missing.code, 2);
    let daily = inkuno(
        &home,
        &vault,
        &["daily", "--date", "2026-03-14", "--append", "Shipped."],
    );
    assert_eq!(
        (daily.code, daily.out.as_str()),
        (0, "2026-03-14.md\n"),
        "{}",
        daily.err
    );
    let note = fs::read_to_string(vault.join("2026-03-14.md")).unwrap();
    assert!(note.contains("Shipped."), "{note}");
    let bad = inkuno(&home, &vault, &["daily", "--date", "2026-02-30"]);
    assert_eq!(bad.code, 2);
    assert!(bad.err.contains("YYYY-MM-DD"), "{}", bad.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn guide_lists_its_pages_and_prints_one() {
    let (home, vault) = sandbox("guide");
    let pages = inkuno(&home, &vault, &["guide"]);
    assert_eq!(pages.code, 0, "{}", pages.err);
    assert!(
        pages.out.lines().any(|line| line == "writing"),
        "{}",
        pages.out
    );
    let page = inkuno(&home, &vault, &["guide", "writing"]);
    assert_eq!(page.code, 0, "{}", page.err);
    assert!(page.out.contains("Writing notes"));
    let typo = inkuno(&home, &vault, &["guide", "writng"]);
    assert_eq!(typo.code, 1);
    assert!(typo.err.contains("did you mean `writing`"), "{}", typo.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn render_checks_a_mermaid_source_and_draws_a_png() {
    let (home, vault) = sandbox("render");
    let good = inkuno(
        &home,
        &vault,
        &["render", "--check", "--mermaid", "flowchart LR\n  A --> B"],
    );
    assert_eq!(
        (good.code, good.out.as_str()),
        (0, "ok flowchart\n"),
        "{}",
        good.err
    );
    let good_json = inkuno(
        &home,
        &vault,
        &[
            "render",
            "--check",
            "--json",
            "--mermaid",
            "flowchart LR\n  A --> B",
        ],
    );
    assert_eq!(good_json.code, 0, "{}", good_json.err);
    let good_json: Value = serde_json::from_str(&good_json.out).unwrap();
    assert_eq!(good_json["readable"], true);

    // A source that does not draw exits 1, with the reason on standard error and no text on standard output.
    let source = "flowchart LR\n  A -->\n  ]]]\n";
    let bad = inkuno_in(
        &home,
        &vault,
        &["render", "--check", "--mermaid", "-"],
        source,
    );
    assert_eq!(bad.code, 1, "{}", bad.out);
    assert!(bad.out.is_empty(), "{}", bad.out);
    assert!(bad.err.starts_with("inkuno: "), "{}", bad.err);
    // With --json the result is still printed on standard output, and the status is 1.
    let bad = inkuno_in(
        &home,
        &vault,
        &["render", "--check", "--json", "--mermaid", "-"],
        source,
    );
    assert_eq!(bad.code, 1, "{}", bad.err);
    let bad: Value = serde_json::from_str(&bad.out).unwrap();
    assert_eq!(bad["readable"], false);
    assert!(bad["error"].is_string());

    // The source comes from standard input with --mermaid -.
    let out = home.join("x.png");
    let picture = inkuno_in(
        &home,
        &vault,
        &["render", "--mermaid", "-", "--out", out.to_str().unwrap()],
        "flowchart LR\n  A --> B\n",
    );
    assert_eq!(picture.code, 0, "{}", picture.err);
    let png = fs::read(&out).unwrap();
    assert!(png.starts_with(&[0x89, b'P', b'N', b'G']), "not a PNG");
    assert!(
        picture
            .out
            .starts_with(&format!("wrote {} (", out.display()))
            && picture.out.ends_with("mermaid)\n"),
        "{}",
        picture.out
    );
    let meta = inkuno_in(
        &home,
        &vault,
        &[
            "render",
            "--mermaid",
            "-",
            "--out",
            out.to_str().unwrap(),
            "--json",
        ],
        "flowchart LR\n  A --> B\n",
    );
    let meta: Value = serde_json::from_str(&meta.out).unwrap();
    assert_eq!(meta["kind"], "mermaid");
    // A check without a source is a usage error: standard input is read only with --mermaid -.
    let unsourced = inkuno_in(
        &home,
        &vault,
        &["render", "--check"],
        "flowchart LR\n  A --> B\n",
    );
    assert_eq!(unsourced.code, 2, "{}", unsourced.err);
    // A picture without --out has nowhere to go: a usage error.
    let nowhere = inkuno(
        &home,
        &vault,
        &["render", "--mermaid", "flowchart LR\n  A --> B"],
    );
    assert_eq!(nowhere.code, 2);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_vault_is_found_from_a_subfolder_and_a_path_cannot_leave_it() {
    let (home, vault) = sandbox("found");
    write(&vault.join("ideas/plan.md"), "# Plan\n");
    write(&home.join("secret.md"), "# Secret\n");
    let inside = inkuno(&home, &vault.join("ideas"), &["list", "--json"]);
    assert_eq!(inside.code, 0, "{}", inside.err);
    let listed: Value = serde_json::from_str(&inside.out).unwrap();
    assert_eq!(listed["notes"][0]["path"], "ideas/plan.md");
    let read = inkuno(&home, &vault, &["read", "../../secret.md"]);
    assert_eq!(read.code, 1);
    assert!(read.err.contains("cannot leave the vault"), "{}", read.err);
    let made = inkuno(&home, &vault, &["new", "Out", "--folder", "../.."]);
    assert_eq!(made.code, 1);
    assert!(!home.join("Out.md").exists());
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_wrong_command_line_exits_two_and_a_mistyped_command_is_suggested() {
    let (home, vault) = sandbox("usage");
    let typo = inkuno(&home, &vault, &["lsit"]);
    assert_eq!(typo.code, 2);
    assert!(typo.err.contains("did you mean"), "{}", typo.err);
    assert!(typo.err.contains("`list`"), "{}", typo.err);
    let bogus = inkuno(&home, &vault, &["list", "--bogus"]);
    assert_eq!(bogus.code, 2);
    let none = inkuno(&home, &vault, &["read"]);
    assert_eq!(none.code, 2);
    assert!(none.err.contains("needs a note"), "{}", none.err);
    // The server is gone: its word is no command either.
    assert_eq!(inkuno(&home, &vault, &["mcp"]).code, 2);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn every_command_has_help_with_an_example() {
    let (home, vault) = sandbox("every");
    let commands = [
        "vault", "list", "search", "read", "links", "tags", "show", "new", "edit", "append",
        "move", "trash", "restore", "daily", "comment", "render", "guide",
    ];
    let top = inkuno(&home, &vault, &["help"]);
    for command in commands {
        let help = inkuno(&home, &vault, &[command, "--help"]);
        assert_eq!(help.code, 0, "{command}: {}", help.err);
        assert!(help.out.contains("example"), "{command}: {}", help.out);
        let by_name = inkuno(&home, &vault, &["help", command]);
        assert_eq!(by_name.out, help.out, "{command}");
        assert!(
            top.out.contains(&format!("inkuno {command}")),
            "{command} is not in the top help"
        );
    }
    let _ = fs::remove_dir_all(home);
}

#[test]
fn no_vault_and_a_missing_vault_say_so() {
    let (home, _) = sandbox("novault");
    let none = inkuno(&home, &home, &["vault"]);
    assert_eq!(none.code, 1);
    assert!(
        none.err
            .contains("no vault: open one in Inkuno, or pass `--vault`"),
        "{}",
        none.err
    );
    let gone = inkuno(
        &home,
        &home,
        &["vault", "--vault", home.join("nope").to_str().unwrap()],
    );
    assert_eq!(gone.code, 1);
    assert!(gone.err.contains("no vault at"), "{}", gone.err);
    // With a vault, `show` finds the note, and says the app is not open: nothing opens a window.
    let (shown_home, vault) = sandbox("shown");
    write(&vault.join("x.md"), "# X\n");
    let shown = inkuno(&shown_home, &vault, &["show", "x.md"]);
    assert_eq!(shown.code, 1);
    assert!(shown.err.contains("not open"), "{}", shown.err);
    let _ = fs::remove_dir_all(home);
    let _ = fs::remove_dir_all(shown_home);
}

#[test]
fn options_may_come_before_the_command_word_and_never_open_the_window() {
    let (home, vault) = sandbox("lead");
    write(&vault.join("Plan.md"), "# Plan\n");
    let dir = vault.to_str().unwrap();
    let listed = inkuno(&home, &home, &["--vault", dir, "list"]);
    assert_eq!(
        (listed.code, listed.out.as_str()),
        (0, "Plan.md\n"),
        "{}",
        listed.err
    );
    let json = inkuno(&home, &home, &["--json", &format!("--vault={dir}"), "list"]);
    assert_eq!(json.code, 0, "{}", json.err);
    let value: Value = serde_json::from_str(&json.out).unwrap();
    assert_eq!(value["notes"][0]["path"], "Plan.md");
    // An option that is no option of ours is a wrong command line, not the app.
    let unknown = inkuno(&home, &home, &["--bogus", "list"]);
    assert_eq!(unknown.code, 2, "{}", unknown.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_bare_dash_dash_makes_the_rest_words() {
    let (home, vault) = sandbox("dashes");
    let made = inkuno(&home, &vault, &["new", "--", "-x"]);
    assert_eq!(
        (made.code, made.out.as_str()),
        (0, "-x.md\n"),
        "{}",
        made.err
    );
    assert!(vault.join("-x.md").is_file());
    let found = inkuno(&home, &vault, &["search", "--", "-x"]);
    assert_eq!(found.code, 0, "{}", found.err);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn an_empty_or_dot_folder_is_the_vault_root_and_a_listing_folder_is_checked() {
    let (home, vault) = sandbox("root");
    write(&vault.join("sub/Note.md"), "# Note\n");
    write(&vault.join("ideas/Idea.md"), "# Idea\n");
    let moved = inkuno(&home, &vault, &["move", "sub/Note", "--folder", ""]);
    assert_eq!(moved.code, 0, "{}", moved.err);
    assert!(vault.join("Note.md").is_file());
    assert!(!vault.join("sub/Note.md").exists());
    let everything = inkuno(&home, &vault, &["list", "--folder", ""]);
    assert_eq!(everything.out.lines().count(), 2, "{}", everything.out);
    let dot = inkuno(&home, &vault, &["list", "--folder", "."]);
    assert_eq!(dot.out.lines().count(), 2, "{}", dot.out);
    for folder in ["./ideas", "ideas/"] {
        let listed = inkuno(&home, &vault, &["list", "--folder", folder]);
        assert_eq!(
            (listed.code, listed.out.as_str()),
            (0, "ideas/Idea.md\n"),
            "{folder}"
        );
        let found = inkuno(&home, &vault, &["search", "idea", "--folder", folder]);
        assert_eq!(found.code, 0, "{}", found.err);
        assert!(found.out.contains("ideas/Idea.md"), "{}", found.out);
    }
    let missing = inkuno(&home, &vault, &["list", "--folder", "nowhere"]);
    assert_eq!(missing.code, 1);
    assert!(
        missing.err.contains("no folder `nowhere`"),
        "{}",
        missing.err
    );
    let outside = inkuno(&home, &vault, &["search", "idea", "--folder", "../.."]);
    assert_eq!(outside.code, 1);
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_refused_move_or_note_leaves_no_folder_behind() {
    let (home, vault) = sandbox("refused");
    write(&vault.join("Zz.md"), "# Zz\n");
    let bad_name = inkuno(
        &home,
        &vault,
        &["move", "Zz", "--folder", "brandnew", "--name", "."],
    );
    assert_eq!(bad_name.code, 1, "{}", bad_name.out);
    let template = inkuno(
        &home,
        &vault,
        &["new", "Idea", "--folder", "made", "--template", "nowhere"],
    );
    assert_eq!(template.code, 1, "{}", template.out);
    for dir in ["brandnew", "made"] {
        assert!(!vault.join(dir).exists(), "{dir} was made");
    }
    assert!(vault.join("Zz.md").is_file());
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_case_only_rename_is_not_reported_as_a_clash() {
    let (home, vault) = sandbox("case");
    write(&vault.join("Zz.md"), "# Zz\n");
    let renamed = inkuno(&home, &vault, &["move", "Zz", "--name", "zz", "--json"]);
    assert_eq!(renamed.code, 0, "{}", renamed.err);
    let value: Value = serde_json::from_str(&renamed.out).unwrap();
    assert_eq!(value["path"], "zz.md");
    assert_eq!(value["links_left"], false);
    let _ = fs::remove_dir_all(home);
}

#[cfg(unix)]
#[test]
fn a_symlink_cannot_reach_a_hidden_folder() {
    let (home, vault) = sandbox("hidden");
    write(&vault.join(".inkuno/x.md"), "secret\n");
    std::os::unix::fs::symlink(vault.join(".inkuno"), vault.join("linkh")).unwrap();
    let read = inkuno(&home, &vault, &["read", "linkh/x.md"]);
    assert_eq!(read.code, 1);
    assert!(read.err.contains("hidden folders"), "{}", read.err);
    let moved = inkuno(&home, &vault, &["move", "linkh/x.md", "--name", "y"]);
    assert_eq!(moved.code, 1);
    assert!(vault.join(".inkuno/x.md").is_file());
    assert!(!vault.join(".inkuno/y.md").exists());
    let _ = fs::remove_dir_all(home);
}

#[test]
fn a_value_outside_its_range_is_a_usage_error_before_any_work() {
    let (home, vault) = sandbox("values");
    write(&vault.join("Plan.md"), "# Plan\n");
    let mermaid = "flowchart LR\n  A --> B";
    let cases: &[&[&str]] = &[
        &["list", "--kind", "bogus"],
        &["list", "--sort", "bogus"],
        &["list", "--limit", "0"],
        &["list", "--limit", "201"],
        &["search", "plan", "--limit", "51"],
        &["daily", "--date", "2026-02-30"],
        &[
            "render",
            "--mermaid",
            mermaid,
            "--out",
            "x.png",
            "--block",
            "0",
        ],
        &[
            "render",
            "--mermaid",
            mermaid,
            "--out",
            "x.png",
            "--theme",
            "bogus",
        ],
        &[
            "render",
            "--mermaid",
            mermaid,
            "--out",
            "x.png",
            "--width",
            "nan",
        ],
        &[
            "render",
            "--mermaid",
            mermaid,
            "--out",
            "x.png",
            "--width",
            "inf",
        ],
        &[
            "render",
            "--mermaid",
            mermaid,
            "--out",
            "x.png",
            "--width",
            "-5",
        ],
        &["edit", "Plan", "--edits", "[{\"old\": \"Plan\"}]"],
        &[
            "edit",
            "Plan",
            "--edits",
            "[{\"old\": \"Plan\", \"new\": 3}]",
        ],
        &["edit", "Plan", "--old", "Plan", "--new", "X", "--token="],
        &["append", "Plan", "--token=", "--text", "x"],
    ];
    for args in cases {
        let run = inkuno(&home, &vault, args);
        assert_eq!(run.code, 2, "{args:?}: {}", run.err);
    }
    assert!(!vault.join("x.png").exists());
    assert_eq!(
        fs::read_to_string(vault.join("Plan.md")).unwrap(),
        "# Plan\n"
    );
    let _ = fs::remove_dir_all(home);
}

#[test]
fn links_of_an_unreadable_note_is_an_error_not_an_empty_answer() {
    let (home, vault) = sandbox("unreadable");
    fs::write(vault.join("Bad.md"), b"# Bad\n\xff\n").unwrap();
    let links = inkuno(&home, &vault, &["links", "Bad"]);
    assert_eq!(links.code, 1, "{}", links.out);
    assert!(links.err.contains("UTF-8"), "{}", links.err);
    let _ = fs::remove_dir_all(home);
}
