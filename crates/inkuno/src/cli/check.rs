//! One temporary vault per test. The operations are called the way the commands call them.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use serde_json::{Value, json};

use super::{draw, guide, ops as notes, outcome::Outcome, store::Vault};

fn scratch() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "inkuno-mcp-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    // Reached through a link, as macOS's temp folder (`/var` → `/private/var`) and Windows'
    // canonical paths (`\\?\C:`) are: a path given by an agent must still find the index's note.
    #[cfg(unix)]
    {
        let link = root.with_extension("via");
        let _ = fs::remove_file(&link);
        std::os::unix::fs::symlink(&root, &link).unwrap();
        return link;
    }
    #[allow(unreachable_code)]
    root
}

fn touch(path: &std::path::Path, secs: u64) {
    let file = fs::File::options().write(true).open(path).unwrap();
    file.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        .unwrap();
}

fn open(root: &std::path::Path) -> Vault {
    Vault::open(root.to_path_buf()).unwrap()
}

fn ok(outcome: Outcome) -> Value {
    assert!(!outcome.is_error, "{}", outcome.structured);
    outcome.structured
}

fn err(outcome: Outcome) -> String {
    assert!(outcome.is_error, "{}", outcome.structured);
    outcome.structured["error"].as_str().unwrap().to_string()
}

fn write(root: &std::path::Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn a_missing_vault_is_a_sentence() {
    let message = match Vault::open(scratch().join("missing")) {
        Ok(_) => panic!("a missing folder opened"),
        Err(message) => message,
    };
    assert_eq!(message, "not found");
    assert!(
        ok(draw::check_mermaid(
            &json!({"source": "classDiagram\nClass A"})
        ))["readable"]
            .as_bool()
            .unwrap()
    );
    // A picture of a note needs a vault: without one, the sentence the command gives is returned.
    let blind = draw::render(None, "no vault", &json!({"note": "A", "block": 1}));
    assert_eq!(blind.sentence(), "no vault");
}

#[test]
fn notes_round_trip_through_the_tools() {
    let root = scratch();
    let mut vault = open(&root);
    let made = ok(notes::create_note(
        &mut vault,
        &json!({"title": "Réunion 🌿", "text": "See [[Plan]] and #work\n\nHello vault."}),
    ));
    assert_eq!(made["path"], "Réunion 🌿.md");
    write(&root, "Plan.md", "# Plan\n\nThe plan.\n");
    write(&root, "Templates/Journal.md", "# {{title}}\n\n{{date}}\n");
    write(&root, "Templates/Réunion 🌿.md", "# Template\n");
    vault.rescan();
    let info = ok(notes::vault_info(&vault));
    assert_eq!(info["notes"], 2);
    assert_eq!(info["templates"], 2);
    assert!(info["tags"][0]["tag"] == "work");
    let read = ok(notes::read_note(&vault, &json!({"note": "Réunion 🌿"})));
    assert!(read["text"].as_str().unwrap().contains("Hello vault"));
    assert_eq!(read["links"][0]["target"], "Plan");
    assert_eq!(read["links"][0]["path"], "Plan.md");
    let plan = ok(notes::read_note(&vault, &json!({"note": "Plan"})));
    let token = plan["token"].as_str().unwrap().to_string();
    assert_eq!(
        ok(notes::search_notes(&vault, &json!({"query": "plan"})))["hits"][0]["title"],
        "Plan"
    );
    assert_eq!(
        ok(notes::search_notes(
            &vault,
            &json!({"query": "hello vault"})
        ))["hits"][0]["where"],
        "text"
    );
    let hits = ok(notes::search_notes(
        &vault,
        &json!({"query": "hello vault", "tag": "work"}),
    ));
    assert_eq!(hits["hits"][0]["title"], "Réunion 🌿");
    assert!(
        ok(notes::search_notes(&vault, &json!({"query": "Template"})))["hits"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let listed = ok(notes::list_notes(
        &vault,
        &json!({"kind": "note", "sort": "name"}),
    ));
    assert!(listed["notes"].as_array().unwrap().len() >= 2);
    assert_eq!(
        ok(notes::note_links(&vault, &json!({"note": "Plan"})))["backlinks"][0]["title"],
        "Réunion 🌿"
    );
    assert_eq!(
        ok(notes::read_note(
            &vault,
            &json!({"note": "Templates/Réunion 🌿.md"})
        ))["path"],
        "Templates/Réunion 🌿.md"
    );
    write(&root, "Templates/Skeleton.md", "# Skeleton\n");
    vault.rescan();
    assert!(err(notes::read_note(&vault, &json!({"note": "Skeleton"}))).contains("no note"));
    let edited = ok(notes::edit_note(
        &mut vault,
        &json!({"note": "Plan", "token": token, "edits": [{"old": "The plan.", "new": "The revised plan."}]}),
    ));
    assert!(!edited["renamed"].as_bool().unwrap());
    ok(notes::append_to_note(
        &mut vault,
        &json!({"note": "Plan", "text": "More.", "under": "Plan"}),
    ));
    assert!(
        fs::read_to_string(root.join("Plan.md"))
            .unwrap()
            .contains("More.")
    );
    assert!(
        err(notes::edit_note(
            &mut vault,
            &json!({"note": "Plan", "token": token, "edits": [{"old": "The plan.", "new": "x"}]})
        ))
        .contains("changed since")
    );
    assert!(
        err(notes::edit_note(
            &mut vault,
            &json!({"note": "Plan", "edits": [{"old": "missing", "new": "x"}]})
        ))
        .contains("was not found")
    );
    assert!(
        err(notes::edit_note(
            &mut vault,
            &json!({"note": "Plan", "edits": [{"old": "", "new": "x"}]})
        ))
        .contains("empty")
    );
    write(&root, "Twice.md", "# Twice\n\nword word\n");
    vault.rescan();
    assert!(
        err(notes::edit_note(
            &mut vault,
            &json!({"note": "Twice", "edits": [{"old": "word", "new": "W"}]})
        ))
        .contains("2 times")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn names_aliases_and_a_change_on_disk() {
    let root = scratch();
    write(&root, "Alpha.md", "---\naliases: [nom]\n---\n# Alpha\n");
    write(&root, "Other/Alpha.md", "# Alpha\n\nsecond\n");
    write(&root, "Named.md", "# Named\n");
    touch(&root.join("Alpha.md"), 1_000);
    touch(&root.join("Other/Alpha.md"), 2_000);
    touch(&root.join("Named.md"), 3_000);
    let mut vault = open(&root);
    let first = ok(notes::read_note(&vault, &json!({"note": "Alpha"})));
    assert_eq!(first["path"], "Other/Alpha.md");
    // The alias is not the note that is also named that way: a real name wins, and here `nom` is only an alias.
    assert_eq!(
        ok(notes::read_note(&vault, &json!({"note": "nom"})))["path"],
        "Alpha.md"
    );
    fs::write(root.join("Fresh.md"), "# Fresh\n\njust written\n").unwrap();
    vault.rescan();
    assert_eq!(
        ok(notes::read_note(&vault, &json!({"note": "Fresh"})))["title"],
        "Fresh"
    );
    let long = "あ".repeat(200);
    let made = ok(notes::create_note(&mut vault, &json!({"title": long})));
    assert_eq!(
        made["path"]
            .as_str()
            .unwrap()
            .trim_end_matches(".md")
            .chars()
            .count(),
        80
    );
    assert_eq!(
        ok(notes::create_note(&mut vault, &json!({"title": "A/B:C"})))["path"],
        "ABC.md"
    );
    assert!(err(notes::create_note(&mut vault, &json!({"title": "abc"}))).contains("already"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn editing_renames_and_comments_follow_the_app() {
    let root = scratch();
    write(&root, "Old.md", "# Old\n\nbody\n");
    write(&root, "Other/Old.md", "# Old\n\nstill this name\n");
    write(
        &root,
        "Link.md",
        "# Link\n\nSee [[Old#part|the old]] and ![](Shot.png)\n",
    );
    write(&root, "Shot.png", "not a png");
    let mut vault = open(&root);
    let renamed = ok(notes::edit_note(
        &mut vault,
        &json!({"note": "Old.md", "edits": [{"old": "# Old", "new": "# New"}]}),
    ));
    assert_eq!(renamed["renamed"], true);
    assert_eq!(renamed["path"], "New.md");
    assert_eq!(renamed["rewritten"], 0);
    let link = fs::read_to_string(root.join("Link.md")).unwrap();
    assert!(link.contains("[[Old#part|the old]]"), "{link}");
    let moved = ok(notes::rename_note(
        &mut vault,
        &json!({"note": "Shot.png", "name": "Pic"}),
    ));
    assert_eq!(moved["path"], "Pic.png");
    let link = fs::read_to_string(root.join("Link.md")).unwrap();
    assert!(
        link.contains("Pic.png") && !link.contains("Shot.png"),
        "{link}"
    );
    write(&root, "Solo.md", "# Solo\n\nbody\n");
    write(&root, "Points.md", "# Points\n\n[[Solo]]\n");
    vault.rescan();
    let gone = ok(notes::rename_note(
        &mut vault,
        &json!({"note": "Solo", "name": "Only", "folder": "Box"}),
    ));
    assert_eq!(gone["path"], "Box/Only.md");
    assert!(gone["rewritten"].as_u64().unwrap() >= 1);
    assert!(!gone["links_left"].as_bool().unwrap());
    assert!(
        fs::read_to_string(root.join("Points.md"))
            .unwrap()
            .contains("[[Only]]")
    );
    write(
        &root,
        "Table.md",
        "# Table\n\n| a | b |\n| --- | --- |\n| hello | there |\n",
    );
    vault.rescan();
    ok(notes::add_comment(
        &mut vault,
        &json!({"note": "Table", "passage": "hello", "comment": "yes|no"}),
    ));
    let table = fs::read_to_string(root.join("Table.md")).unwrap();
    assert!(table.contains("{==hello==}{>>yes\\|no<<}"), "{table}");
    write(&root, "Dup.md", "# Dup\n\none one\n");
    vault.rescan();
    assert!(
        err(notes::add_comment(
            &mut vault,
            &json!({"note": "Dup", "passage": "one", "comment": "c"})
        ))
        .contains("more than once")
    );
    let trashed = ok(notes::trash_note(&mut vault, &json!({"note": "Dup"})));
    assert!(trashed["path"].as_str().unwrap().starts_with(".trash/"));
    assert!(
        ok(notes::list_notes(&vault, &json!({})))["notes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| !n["path"].as_str().unwrap().contains(".trash"))
    );
    let back = ok(notes::restore_note(
        &mut vault,
        &json!({"path": trashed["path"]}),
    ));
    assert_eq!(back["path"], "Dup.md");
    assert!(
        err(notes::restore_note(&mut vault, &json!({"path": "Dup.md"}))).contains(".trash")
            || err(notes::restore_note(
                &mut vault,
                &json!({"path": "../Dup.md"})
            ))
            .contains("leave")
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_history_follows_a_note_through_rename_trash_and_restore() {
    use crate::platform::revisions::{self, Home};
    let root = scratch();
    write(&root, "Plan.md", "# Plan\n\none\n");
    let home = Home::Vault(root.clone());
    let plan = root.join("Plan.md");
    revisions::save(&home, &plan, "ana", None, "# Plan\n\none\n", 0).unwrap();
    let mut vault = open(&root);
    let moved = ok(notes::rename_note(
        &mut vault,
        &json!({"note": "Plan.md", "name": "Idea"}),
    ));
    assert_eq!(moved["path"], "Idea.md");
    let idea = root.join("Idea.md");
    assert_eq!(revisions::all(&home, &idea).unwrap().len(), 1);
    assert!(!root.join(".inkuno/revisions/Plan.md").exists());
    // Trashed, the history waits in the bin; restored, it comes back with the note.
    let trashed = ok(notes::trash_note(&mut vault, &json!({"note": "Idea"})));
    assert_eq!(trashed["path"], ".trash/Idea.md");
    assert!(revisions::all(&home, &idea).unwrap().is_empty());
    ok(notes::restore_note(
        &mut vault,
        &json!({"path": ".trash/Idea.md"}),
    ));
    assert!(idea.is_file());
    assert_eq!(revisions::all(&home, &idea).unwrap().len(), 1);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn bytes_on_disk_stay_bytes() {
    let root = scratch();
    fs::write(
        root.join("Crlf.md"),
        "\u{feff}---\r\na: 1\r\n---\r\n# Crlf\r\n\r\nword\r\n",
    )
    .unwrap();
    let mut vault = open(&root);
    let read = ok(notes::read_note(&vault, &json!({"note": "Crlf"})));
    assert!(read["text"].as_str().unwrap().starts_with('\u{feff}'));
    assert!(read["text"].as_str().unwrap().contains("a: 1"));
    ok(notes::edit_note(
        &mut vault,
        &json!({"note": "Crlf", "edits": [{"old": "word", "new": "autre"}]}),
    ));
    let text = fs::read_to_string(root.join("Crlf.md")).unwrap();
    assert!(
        text.starts_with('\u{feff}')
            && text.contains("\r\n")
            && text.contains("a: 1")
            && text.contains("autre")
    );
    let daily = ok(notes::daily_note(
        &mut vault,
        &json!({"date": "2026-01-02", "append": "a line"}),
    ));
    assert!(daily["path"].as_str().unwrap().contains("2026-01-02"));
    assert!(
        fs::read_to_string(root.join("2026-01-02.md"))
            .unwrap()
            .contains("a line")
    );
    assert!(
        err(notes::daily_note(&mut vault, &json!({"date": "yesterday"}))).contains("YYYY-MM-DD")
    );
    // A day the calendar does not have, and the last day of a leap February.
    assert!(
        err(notes::daily_note(
            &mut vault,
            &json!({"date": "2026-02-30"})
        ))
        .contains("YYYY-MM-DD")
    );
    assert!(
        err(notes::daily_note(
            &mut vault,
            &json!({"date": "2026-13-01"})
        ))
        .contains("YYYY-MM-DD")
    );
    ok(notes::daily_note(
        &mut vault,
        &json!({"date": "2028-02-29"}),
    ));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn paths_cannot_leave_the_vault() {
    let root = scratch();
    write(&root, "Here.md", "# Here\n");
    write(&root, ".secret/Hidden.md", "# Hidden\n");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/tmp", root.join("escape")).unwrap();
    let mut vault = open(&root);
    assert!(err(notes::read_note(&vault, &json!({"note": "../Here.md"}))).contains("leave"));
    assert!(
        err(notes::create_note(
            &mut vault,
            &json!({"title": "Out", "folder": ".secret"})
        ))
        .contains("hidden")
    );
    #[cfg(unix)]
    assert!(err(notes::read_note(&vault, &json!({"note": "escape/file.md"}))).contains("leave"));
    let listed = ok(notes::list_notes(&vault, &json!({"kind": "all"})));
    assert!(
        listed["notes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|n| !n["path"].as_str().unwrap().contains("Hidden"))
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_note_path_may_leave_out_its_extension() {
    let root = scratch();
    write(&root, "ideas/Plan.md", "# Plan\n");
    write(&root, "pics/a.svg", "<svg></svg>");
    let vault = open(&root);
    // `ideas/Plan` is `ideas/Plan.md`, for a note as for a file; a note that is not there is not found.
    let note = vault.find_note("ideas/Plan").ok().unwrap();
    assert_eq!(note.path, vault.root.join("ideas/Plan.md"));
    assert_eq!(
        vault.find_file("ideas/Plan").unwrap(),
        vault.root.join("ideas/Plan.md")
    );
    assert!(vault.find_note("ideas/Missing").is_err());
    // A file that is not a note keeps its name, extension and all.
    assert_eq!(
        vault.find_file("pics/a.svg").unwrap(),
        vault.root.join("pics/a.svg")
    );
    // A path without the extension that leaves the vault is refused all the same.
    assert!(vault.find_note("../Plan").err().unwrap().contains("leave"));
    assert!(vault.find_file("../Plan").unwrap_err().contains("leave"));
    let read = ok(notes::read_note(&vault, &json!({"note": "ideas/Plan"})));
    assert_eq!(read["title"], "Plan");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn an_unreadable_folder_does_not_hide_the_rest() {
    let root = scratch();
    write(&root, "Ok.md", "# Ok\n");
    fs::create_dir(root.join("locked")).unwrap();
    write(&root, "locked/No.md", "# No\n");
    let mut perms = fs::metadata(root.join("locked")).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o0);
        fs::set_permissions(root.join("locked"), perms).unwrap();
    }
    let vault = open(&root);
    let info = ok(notes::vault_info(&vault));
    assert!(info["notes"].as_u64().unwrap() >= 1);
    #[cfg(unix)]
    assert!(!info["unreadable"].as_array().unwrap().is_empty());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(root.join("locked")).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(root.join("locked"), perms).unwrap();
    }
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn the_guide_pages_are_text_and_a_picture_needs_a_vault() {
    let root = scratch();
    write(
        &root,
        "Note.md",
        "# Note\n\n```mermaid\nflowchart LR\n  A --> B\n```\n",
    );
    assert!(guide::names().contains(&"writing"));
    assert!(guide::page("writing").unwrap().contains("Writing notes"));
    assert!(guide::page("nope").is_none());
    let vault = open(&root);
    let rendered = draw::render(Some(&vault), "", &json!({"note": "Note", "block": 1}));
    assert!(!rendered.is_error, "{}", rendered.structured);
    assert!(
        rendered
            .png
            .as_deref()
            .is_some_and(|png| png.starts_with(b"\x89PNG"))
    );
    let started = Instant::now();
    let _ = notes::search_notes(&vault, &json!({"query": "Note"}));
    assert!(started.elapsed().as_secs() < 5);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn five_thousand_notes_stay_searchable() {
    let root = scratch();
    for i in 0..5000 {
        fs::write(
            root.join(format!("Note {i}.md")),
            format!("# Note {i}\n\nbody {i} shared-token\n"),
        )
        .unwrap();
    }
    let vault = open(&root);
    let started = Instant::now();
    let hits = ok(notes::search_notes(
        &vault,
        &json!({"query": "shared-token", "limit": 20}),
    ));
    let listed = ok(notes::list_notes(
        &vault,
        &json!({"limit": 100, "sort": "name"}),
    ));
    assert!(
        started.elapsed().as_secs() < 5,
        "search and list took {:?}",
        started.elapsed()
    );
    assert_eq!(hits["hits"].as_array().unwrap().len(), 20);
    assert_eq!(listed["notes"].as_array().unwrap().len(), 100);
    assert!(listed["next"].is_string());
    let _ = fs::remove_dir_all(&root);
}
