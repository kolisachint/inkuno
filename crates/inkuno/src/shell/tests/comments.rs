//! A save writes the comments of the earlier form in the new one, and the caret stays with the text.

use super::*;

fn open(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, path: &Path) {
    shell.update_in(cx, |s, window, cx| s.open_from_nav(path, window, cx));
    cx.run_until_parked();
}

fn place(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, at: usize) {
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| e.place_cursor(at, cx))
    });
}

fn position(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> usize {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position())
}

#[gpui::test]
fn a_save_that_migrates_nothing_keeps_the_caret(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "migrate-nothing");
    let note = root.join("Note.md");
    fs::write(&note, "un mot ici\nfin\n").unwrap();
    open(&shell, cx, &note);
    place(&shell, cx, 7);
    cx.simulate_input("x");
    assert_eq!(position(&shell, cx), 8);
    shell.update(cx, |s, cx| s.flush(cx));
    assert_eq!(fs::read_to_string(&note).unwrap(), "un mot xici\nfin\n");
    assert_eq!(position(&shell, cx), 8);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_migrated_comment_before_the_caret_moves_the_caret_with_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "migrate-before");
    let note = root.join("Note.md");
    fs::write(&note, "un mot ici\nfin\n").unwrap();
    open(&shell, cx, &note);
    // Typed at the start: the earlier form is written in the new one on save, the caret after the
    // space that was typed, and the text after it moves on as the comment's new form is longer.
    place(&shell, cx, 0);
    cx.simulate_input("{==mot==}{>>note<<} ");
    assert_eq!(position(&shell, cx), 20);
    shell.update(cx, |s, cx| s.flush(cx));
    let saved = "mot<!-- comment: note --> un mot ici\nfin\n";
    assert_eq!(fs::read_to_string(&note).unwrap(), saved);
    assert_eq!(position(&shell, cx), saved.find("un").unwrap());
    let _ = fs::remove_dir_all(root);
}
