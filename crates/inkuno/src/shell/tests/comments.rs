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

fn text(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
}

/// Selects `mot` in "un mot ici" and asks for a comment on it (Ctrl+Shift+M).
fn comment_on_mot(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.select_passage(3..6, cx))
    });
    cx.simulate_keystrokes("secondary-shift-m");
    settle(cx);
}

fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

#[gpui::test]
fn a_comment_opens_a_draft_card_and_enter_writes_what_is_said(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "draft-enter");
    let note = root.join("Note.md");
    fs::write(&note, "un mot ici\nfin\n").unwrap();
    open(&shell, cx, &note);
    shell.update(cx, |s, _| s.prefs.author = "Ana".into());
    comment_on_mot(&shell, cx);
    // The Comments view is open on the draft: its card is drawn, the empty card of the same comment is not.
    assert!(cx.debug_bounds("comment-draft").is_some(), "the draft card");
    assert!(cx.debug_bounds("comment-0").is_none(), "one card for it");
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_some()));
    cx.simulate_input("à relire");
    cx.simulate_keystrokes("enter");
    settle(cx);
    // The passage stays, the comment is written with its author and its minute, the draft is gone.
    assert_eq!(
        without_clock(&text(&shell, cx)),
        "un mot<!-- comment @Ana <T>: à relire --> ici\nfin\n"
    );
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_none()));
    // gpui keeps a removed element's bounds in its frame map, so only a drawn card is checked here.
    assert!(
        cx.debug_bounds("review-card-0").is_some(),
        "the card of the comment"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn escape_in_the_draft_takes_the_comment_out_and_keeps_the_passage(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "draft-escape");
    let note = root.join("Note.md");
    fs::write(&note, "un mot ici\nfin\n").unwrap();
    open(&shell, cx, &note);
    comment_on_mot(&shell, cx);
    cx.simulate_input("pas écrit");
    cx.simulate_keystrokes("escape");
    settle(cx);
    assert_eq!(text(&shell, cx), "un mot ici\nfin\n");
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_none()));
    let _ = fs::remove_dir_all(root);
}

/// The draft of the shell, its field focused: what the keys reach while it is open.
fn focus_draft_field(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    shell.update_in(cx, |s, window, cx| {
        let field = s.comment_draft.as_ref().expect("a draft").field.clone();
        window.focus(&field.focus_handle(cx), cx);
    });
}

#[gpui::test]
fn typing_above_the_draft_then_escape_leaves_the_other_comments_alone(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "draft-above");
    let note = root.join("Note.md");
    fs::write(&note, "un<!-- comment: gardé --> mot ici\nfin\n").unwrap();
    open(&shell, cx, &note);
    let at = text(&shell, cx).find("mot").unwrap();
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.select_passage(at..at + 3, cx))
    });
    cx.simulate_keystrokes("secondary-shift-m");
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_some()));
    // Typed in the note above the comment, while its draft is open: the comment moves, the draft follows it.
    shell.update_in(cx, |s, window, cx| {
        let editor = s.pane().editor.clone();
        editor.update(cx, |e, cx| e.place_cursor(0, cx));
        window.focus(&editor.focus_handle(cx), cx);
    });
    cx.simulate_input("Une longue phrase ajoutée ici ");
    focus_draft_field(&shell, cx);
    cx.simulate_keystrokes("escape");
    settle(cx);
    // Only the draft's own empty marker goes; the comment before it keeps its text.
    assert_eq!(
        text(&shell, cx),
        "Une longue phrase ajoutée ici un<!-- comment: gardé --> mot ici\nfin\n"
    );
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_none()));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn closing_the_tab_while_drafting_drops_the_draft_and_writes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "draft-closed");
    let note = root.join("Note.md");
    fs::write(&note, "un mot ici\nfin\n").unwrap();
    open(&shell, cx, &note);
    comment_on_mot(&shell, cx);
    cx.simulate_input("perdu");
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_some()));
    // A second pane, so that the drafting tab may close (the window keeps one tab).
    let tab = shell.read_with(cx, |s, _| s.active);
    shell.update_in(cx, |s, window, cx| s.add_pane(Side::Right, window, cx));
    shell.update_in(cx, |s, window, cx| s.close_tab(tab, window, cx));
    settle(cx);
    // The draft ends with its tab: its card is gone, and what was typed in its field is never written.
    assert!(shell.read_with(cx, |s, _| s.comment_draft.is_none()));
    let saved = fs::read_to_string(&note).unwrap();
    assert!(!saved.contains("perdu"), "{saved:?}");
    let _ = fs::remove_dir_all(root);
}
