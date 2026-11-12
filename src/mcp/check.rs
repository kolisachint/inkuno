//! One temporary vault per test. The tools are called the way a client calls them.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use serde_json::{Value, json};

use super::{Outcome, Server, notes, tools, vault::Vault};

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
    let mut server = Server::open(Some(scratch().join("missing")), false);
    let message = err(tools::call(&mut server, "vault_info", &json!({})));
    assert_eq!(message, "not found");
    let mut blind = Server {
        vault: None,
        blind: "no vault: open one in Inkuno, or pass `--vault`".into(),
        read_only: false,
    };
    assert!(err(tools::call(&mut blind, "read_note", &json!({"note": "A"}))).contains("no vault"));
    assert!(
        ok(tools::call(
            &mut blind,
            "check_mermaid",
            &json!({"source": "classDiagram\nClass A"})
        ))["readable"]
            .as_bool()
            .unwrap()
    );
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
    let revision = plan["revision"].as_str().unwrap().to_string();
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
        &json!({"note": "Plan", "revision": revision, "edits": [{"old": "The plan.", "new": "The revised plan."}]}),
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
    assert!(err(notes::edit_note(&mut vault, &json!({"note": "Plan", "revision": revision, "edits": [{"old": "The plan.", "new": "x"}]}))).contains("changed since"));
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
    vault.refresh();
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
fn read_only_hides_writing_and_the_guide_is_a_resource() {
    let root = scratch();
    write(
        &root,
        "Note.md",
        "# Note\n\n```mermaid\nflowchart LR\n  A --> B\n```\n",
    );
    let mut server = Server::open(Some(root.clone()), true);
    let names: Vec<String> = tools::list(true)
        .into_iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"read_note".into()) && !names.contains(&"create_note".into()));
    assert!(
        err(tools::call(
            &mut server,
            "create_note",
            &json!({"title": "X"})
        ))
        .contains("read-only")
    );
    let init = server.answer(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{}}}"#).unwrap();
    assert!(
        init["result"]["instructions"]
            .as_str()
            .unwrap()
            .contains("read-only")
    );
    let page = server.answer(r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"inkuno://guide/writing"}}"#).unwrap();
    assert!(
        page["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Writing notes")
    );
    let note = server.answer(r#"{"jsonrpc":"2.0","id":4,"method":"resources/read","params":{"uri":"inkuno://note/Note.md"}}"#).unwrap();
    assert!(
        note["result"]["contents"][0]["text"]
            .as_str()
            .unwrap()
            .contains("flowchart")
    );
    let prompt = server.answer(r#"{"jsonrpc":"2.0","id":5,"method":"prompts/get","params":{"name":"review_note","arguments":{"note":"Note"}}}"#).unwrap();
    assert!(
        prompt["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .contains("add_comment")
    );
    assert_eq!(server.answer(r#"{"jsonrpc":"2.0","id":6,"method":"prompts/get","params":{"name":"review_note","arguments":{}}}"#).unwrap()["error"]["code"], -32602);
    let prompts = server
        .answer(r#"{"jsonrpc":"2.0","id":7,"method":"prompts/list"}"#)
        .unwrap();
    assert_eq!(prompts["result"]["prompts"].as_array().unwrap().len(), 6);
    let text = |server: &mut Server, request: &str| {
        server.answer(request).unwrap()["result"]["messages"][0]["content"]["text"]
            .as_str()
            .unwrap()
            .to_string()
    };
    let week = text(
        &mut server,
        r#"{"jsonrpc":"2.0","id":8,"method":"prompts/get","params":{"name":"weekly_review"}}"#,
    );
    assert!(
        week.contains("holds today") && week.contains("Do not call daily_note"),
        "{week}"
    );
    let week = text(
        &mut server,
        r#"{"jsonrpc":"2.0","id":9,"method":"prompts/get","params":{"name":"weekly_review","arguments":{"date":"2026-10-28"}}}"#,
    );
    assert!(week.contains("holds 2026-10-28"), "{week}");
    let orphans = text(
        &mut server,
        r#"{"jsonrpc":"2.0","id":10,"method":"prompts/get","params":{"name":"link_orphans","arguments":{}}}"#,
    );
    assert!(
        orphans.contains("in the vault") && orphans.contains("note_links"),
        "{orphans}"
    );
    let orphans = text(
        &mut server,
        r#"{"jsonrpc":"2.0","id":11,"method":"prompts/get","params":{"name":"link_orphans","arguments":{"folder":"Projets/Été"}}}"#,
    );
    assert!(orphans.contains("the folder \"Projets/Été\""), "{orphans}");
    let tags = text(
        &mut server,
        r#"{"jsonrpc":"2.0","id":12,"method":"prompts/get","params":{"name":"tidy_tags"}}"#,
    );
    assert!(
        tags.contains("list_tags") && tags.contains("wait for the user"),
        "{tags}"
    );
    assert_eq!(
        server
            .answer(r#"{"jsonrpc":"2.0","id":13,"method":"prompts/get","params":{"name":"nope"}}"#)
            .unwrap()["error"]["code"],
        -32602
    );
    let rendered = tools::call(&mut server, "render", &json!({"note": "Note", "block": 1}));
    assert!(!rendered.is_error, "{}", rendered.structured);
    assert_eq!(rendered.content[1]["mimeType"], "image/png");
    let started = Instant::now();
    let _ = tools::call(&mut server, "search_notes", &json!({"query": "Note"}));
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
