//! Versions of a note (`Ctrl+Alt+S`): a revision per save, the author and the line on what changed,
//! refused when nothing changed, for a vault note and a scratch note. Each path is driven with
//! keystrokes, on disk.

use super::*;

fn open(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, path: &Path) {
    shell.update_in(cx, |s, window, cx| s.open_from_nav(path, window, cx));
    cx.run_until_parked();
}

/// Lets the debounced write happen.
fn settle(cx: &mut gpui::VisualTestContext) {
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
}

/// `Ctrl+Alt+S`, then the line on what changed (`memo`, empty to skip), then Enter.
fn save_version(cx: &mut gpui::VisualTestContext, memo: &str) {
    cx.simulate_keystrokes("secondary-alt-s");
    cx.run_until_parked();
    if !memo.is_empty() {
        cx.simulate_input(memo);
    }
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
}

#[gpui::test]
fn the_chord_saves_revision_1_then_2_with_their_notes(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-chord");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n\none idea\n").unwrap();
    open(&shell, cx, &plan);

    save_version(cx, "dropped the plugin idea");
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Done, "Revision 1 saved")));
    let first = root.join(".inkuno/revisions/Plan.md/0001.md");
    let file = fs::read_to_string(&first).unwrap();
    assert!(file.contains("note: dropped the plugin idea\n"), "{file}");
    assert!(file.ends_with("---\n# Plan\n\none idea\n"), "{file}");
    assert!(file.starts_with("---\nrevision: 1\n"), "{file}");
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));

    // Back in the note: a new line, then a save with no note (Enter alone skips it).
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_input("two\n");
    settle(cx);
    save_version(cx, "");
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Done, "Revision 2 saved")));
    let second = fs::read_to_string(root.join(".inkuno/revisions/Plan.md/0002.md")).unwrap();
    assert!(
        second.contains("parent: 1\n") && !second.contains("note:"),
        "{second}"
    );
    assert!(
        second.ends_with("---\n# Plan\n\none idea\ntwo\n"),
        "{second}"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn unchanged_text_says_so_and_writes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-same");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n").unwrap();
    open(&shell, cx, &plan);
    save_version(cx, "first");
    assert!(root.join(".inkuno/revisions/Plan.md/0001.md").exists());

    cx.simulate_keystrokes("secondary-alt-s");
    cx.run_until_parked();
    // No question is asked: nothing changed since revision 1.
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    assert!(shell.read_with(cx, |s, _| {
        s.told(Tone::Info, "Nothing changed since revision 1")
    }));
    assert!(!root.join(".inkuno/revisions/Plan.md/0002.md").exists());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn escape_cancels_the_version(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-escape");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n").unwrap();
    open(&shell, cx, &plan);
    cx.simulate_keystrokes("secondary-alt-s");
    cx.run_until_parked();
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    assert!(!root.join(".inkuno").exists());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_scratch_note_gets_its_versions_in_the_scratch_folder(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-scratch");
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("# Idée\n\nun brouillon");
    settle(cx);
    save_version(cx, "kept");
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Done, "Revision 1 saved")));
    let file = platform::scratch::list().remove(0);
    let name = file.file_name().unwrap().to_string_lossy().into_owned();
    let saved = platform::scratch::dir()
        .join(".revisions")
        .join(&name)
        .join("0001.md");
    let text = fs::read_to_string(&saved).unwrap();
    assert!(text.ends_with("---\n# Idée\n\nun brouillon"), "{text}");
    // Not a vault file: the vault gets nothing.
    assert!(!root.join(".inkuno").exists());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_history_is_hidden_from_the_tree_and_search(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-hidden");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n").unwrap();
    open(&shell, cx, &plan);
    save_version(cx, "one");
    // The index the tree, the search and the palette read, after a rescan of the vault.
    let root_for_scan = root.clone();
    shell.update_in(cx, |s, _, cx| s.rescan(&root_for_scan, cx));
    cx.run_until_parked();
    assert!(root.join(".inkuno/revisions/Plan.md/0001.md").exists());
    assert!(shell.read_with(cx, |s, _| {
        s.notes
            .iter()
            .all(|n| !n.path.components().any(|c| c.as_os_str() == ".inkuno"))
            && s.dirs
                .iter()
                .all(|d| !d.components().any(|c| c.as_os_str() == ".inkuno"))
            && s.notes.iter().any(|n| n.path == plan)
    }));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_author_setting_is_the_author_of_the_versions(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-author");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n").unwrap();
    open(&shell, cx, &plan);
    // The setting is a typed line: spaces around it are dropped, Enter keeps it.
    shell.update_in(cx, |s, window, cx| {
        s.choose_setting(Setting::Author, window, cx)
    });
    cx.run_until_parked();
    cx.simulate_input("  Ana Lee ");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.prefs.author == "Ana Lee"
        && s.palette.is_none()));
    let saved = vault::load_settings();
    assert!(saved.contains("author=Ana Lee\n"), "{saved}");
    save_version(cx, "");
    let file = fs::read_to_string(root.join(".inkuno/revisions/Plan.md/0001.md")).unwrap();
    assert!(file.contains("author: Ana Lee\n"), "{file}");
    let _ = fs::remove_dir_all(root);
}
