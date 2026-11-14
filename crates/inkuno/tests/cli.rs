//! The command line (`inkuno rev`, `inkuno keys`, `inkuno help`) run as a real binary, on a temporary
//! vault. Its configuration folder is a temporary one too: the real one is never touched.

use std::{
    fs,
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

/// A fresh temporary home (configuration folder included) and vault, named after the test.
fn sandbox(name: &str) -> (PathBuf, PathBuf) {
    let home = std::env::temp_dir().join(format!("inkuno-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&home);
    let vault = home.join("vault");
    fs::create_dir_all(vault.join(".inkuno")).unwrap();
    fs::create_dir_all(home.join("cfg")).unwrap();
    (home, vault)
}

/// Runs `inkuno` with `args` in `dir`, in the sandbox `home`, with no author in the environment.
fn inkuno(home: &Path, dir: &Path, args: &[&str]) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_inkuno"))
        .current_dir(dir)
        .args(args)
        .env("XDG_CONFIG_HOME", home.join("cfg"))
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
        .env("XDG_CONFIG_HOME", home.join("cfg"))
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
    let scratch = home.join("cfg/inkuno/scratch");
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
    let file = fs::read_to_string(home.join("cfg/inkuno/keybindings.json")).unwrap();
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
        !home.join("cfg/inkuno/keybindings.json").exists(),
        "nothing is written on a refusal"
    );

    let no_chord = inkuno(&home, &vault, &["keys", "set", "editor::Bold"]);
    assert_eq!(no_chord.code, 1);
    let unknown_reset = inkuno(&home, &vault, &["keys", "reset", "app::Nope"]);
    assert_eq!(unknown_reset.code, 1);
    let _ = fs::remove_dir_all(home);
}
