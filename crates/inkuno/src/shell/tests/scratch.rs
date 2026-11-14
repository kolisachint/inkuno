//! Scratch notes (Ctrl+N): a raw thought in a tab, written to the app's own folder, never to the
//! vault until it is kept there. Each path is driven with keystrokes, on disk.

use super::*;

/// The notes (`.md` files) directly in the vault folder: hidden folders (the app's own, the
/// scratch one among them) are not the vault.
fn vault_notes(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.') && name.ends_with(".md"))
        .collect();
    names.sort();
    names
}

/// The text of the shown note.
fn text_of(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
}

/// Lets the debounced write happen.
fn settle(cx: &mut gpui::VisualTestContext) {
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
}

#[gpui::test]
fn ctrl_n_writes_a_scratch_note_and_no_vault_file(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-outside");
    let before = vault_notes(&root);
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("# Idée\n\nun brouillon");
    settle(cx);
    assert_eq!(vault_notes(&root), before);
    let files = platform::scratch::list();
    assert_eq!(files.len(), 1);
    assert!(platform::scratch::holds(&files[0]));
    assert_eq!(
        fs::read_to_string(&files[0]).unwrap(),
        "# Idée\n\nun brouillon"
    );
    // Not a vault note: no index entry, no recent entry, no link target.
    assert!(shell.read_with(cx, |s, _| {
        s.pane().scratch
            && s.notes.iter().all(|n| n.path != files[0])
            && s.recent.iter().all(|p| *p != files[0])
    }));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn ctrl_n_works_without_a_vault(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-scratch-novault-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view(|window, cx| Shell::new(None, Vec::new(), window, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("sans coffre");
    settle(cx);
    assert_eq!(text_of(&shell, cx), "sans coffre");
    let files = platform::scratch::list();
    assert_eq!(files.len(), 1);
    assert_eq!(fs::read_to_string(&files[0]).unwrap(), "sans coffre");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn ctrl_alt_n_still_creates_a_vault_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-vault-note");
    cx.simulate_keystrokes("secondary-alt-n");
    cx.simulate_input("# Dans le coffre");
    settle(cx);
    assert!(root.join("Dans le coffre.md").is_file());
    assert!(platform::scratch::list().is_empty());
    assert!(shell.read_with(cx, |s, _| !s.pane().scratch));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_blank_scratch_note_leaves_no_file_behind(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-blank");
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("brouillon");
    settle(cx);
    assert_eq!(platform::scratch::list().len(), 1);
    // A second tab takes the input; the first is emptied, then closed: silently, and its file goes.
    cx.simulate_keystrokes("secondary-n ctrl-tab");
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| {
        let first = s.groups[0].tabs[0];
        s.focus_pane(first, window, cx);
    });
    cx.simulate_keystrokes("secondary-a backspace");
    settle(cx);
    cx.simulate_keystrokes("secondary-w");
    cx.run_until_parked();
    assert!(platform::scratch::list().is_empty());
    assert!(shell.read_with(cx, |s, _| s.panes.len() == 1));
    let _ = fs::remove_dir_all(root);
}

/// Types a scratch note, then asks to close it: the list of what becomes of it is shown.
fn write_and_close(cx: &mut gpui::VisualTestContext, text: &str) {
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input(text);
    settle(cx);
    cx.simulate_keystrokes("secondary-w");
    cx.run_until_parked();
}

#[gpui::test]
fn closing_a_scratch_note_asks_and_keep_as_scratch_is_the_default(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-keep");
    let before = vault_notes(&root);
    write_and_close(cx, "# Idée\n\nà garder");
    assert!(shell.read_with(cx, |s, _| s.palette.is_some() && s.pane().scratch));
    // Enter takes the default: the tab closes, the file stays, the vault is untouched.
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    let files = platform::scratch::list();
    assert_eq!(files.len(), 1);
    assert_eq!(fs::read_to_string(&files[0]).unwrap(), "# Idée\n\nà garder");
    assert_eq!(vault_notes(&root), before);
    assert!(shell.read_with(cx, |s, _| s.palette.is_none() && !s.pane().scratch));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn discard_deletes_the_scratch_file_and_escape_cancels_the_close(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-discard");
    write_and_close(cx, "# Jeter\n\nnon");
    // Escape cancels: the tab stays with its text, the file stays.
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, cx| s.palette.is_none()
        && s.pane().scratch
        && s.pane().editor.read(cx).text() == "# Jeter\n\nnon"));
    assert_eq!(platform::scratch::list().len(), 1);
    // Discard is the last of the three: down twice, then Enter.
    cx.simulate_keystrokes("secondary-w");
    cx.run_until_parked();
    cx.simulate_keystrokes("down down enter");
    cx.run_until_parked();
    assert!(platform::scratch::list().is_empty());
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn keep_in_vault_moves_the_note_under_its_title(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-vault");
    write_and_close(cx, "# Projet Lune\n\nle corps");
    // Down once: Keep in vault.
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    let kept = root.join("Projet Lune.md");
    assert_eq!(
        fs::read_to_string(&kept).unwrap(),
        "# Projet Lune\n\nle corps"
    );
    assert!(platform::scratch::list().is_empty());
    // The tab stays, now a note of the vault: it is indexed and no longer scratch.
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()
        && !s.pane().scratch
        && s.pane().path.as_deref() == Some(kept.as_path())
        && s.notes.iter().any(|n| n.path == kept)));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn quitting_or_closing_the_window_never_asks(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-quit");
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("à l'instant");
    settle(cx);
    assert!(cx.simulate_close());
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    let files = platform::scratch::list();
    assert_eq!(files.len(), 1);
    assert_eq!(fs::read_to_string(&files[0]).unwrap(), "à l'instant");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn scratch_tabs_come_back_after_a_restart(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scratch-restart");
    let anchor = root.join("Ancre.md");
    fs::write(&anchor, "# Ancre\n").unwrap();
    shell.update_in(cx, |s, window, cx| s.open_from_nav(&anchor, window, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("# Reprise\n\nencore");
    settle(cx);
    // The layout is saved as the window draws.
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let file = platform::scratch::list().remove(0);
    drop(shell);
    // A restart: a new window on the same vault and the same config folder.
    let (restarted, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx.run_until_parked();
    assert_eq!(text_of(&restarted, cx), "# Reprise\n\nencore");
    assert!(restarted.read_with(cx, |s, _| {
        s.pane().scratch && s.pane().path.as_deref() == Some(file.as_path()) && s.panes.len() == 2
    }));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn scratch_tabs_come_back_without_a_vault(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!(
        "inkuno-scratch-restart-novault-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view(|window, cx| Shell::new(None, Vec::new(), window, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("sans coffre");
    settle(cx);
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    drop(shell);
    let (restarted, cx) = cx.add_window_view(|window, cx| Shell::new(None, Vec::new(), window, cx));
    cx.run_until_parked();
    assert_eq!(text_of(&restarted, cx), "sans coffre");
    assert!(restarted.read_with(cx, |s, _| s.pane().scratch));
    let _ = fs::remove_dir_all(root);
}
