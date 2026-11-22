//! Pending changes in the note: an agent's proposed lines wait between marks until the person
//! accepts or rejects each one. Typing in them is refused with a message; accept, reject and
//! comment act on them, one undo step each; an outside change still lands.

use super::*;
use crate::logic::pending;

/// One kept line, one change (a removed and an added line), one kept line after it.
const NOTE: &str = "# Plan\n\nkeep\n{--\nold\n--}\n{++\nnew\n++}\nend\n";
/// Two changes, one after the other, to act on all of them.
const TWO: &str = "a\n{--\none\n--}\n{++\nun\n++}\nb\n{++\ndeux\n++}\nc\n";
pub(super) const REFUSED: &str = "This change is pending: accept or reject it first.";
/// A table inside an added change, its columns not yet aligned.
const TABLE_CHANGE: &str = "keep\n{++\n| a | bbbb |\n|---|---|\n| 1 | 2 |\n++}\nend\n";

/// Lets the window draw what changed, then the tasks run.
pub(super) fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// A note with `content` open, its editor focused.
fn open_note<'a>(
    name: &str,
    content: &str,
    cx: &'a mut TestAppContext,
) -> (
    PathBuf,
    Entity<Shell>,
    &'a mut gpui::VisualTestContext,
    std::sync::MutexGuard<'static, ()>,
) {
    let (root, shell, cx, config) = start(cx, name);
    let path = root.join("Plan.md");
    fs::write(&path, content).unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| s.open_from_nav(&path, window, cx));
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.pane().editor.focus_handle(cx))
    });
    settle(cx);
    (root, shell, cx, config)
}

pub(super) fn text(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
}

/// Puts the caret at byte `at`.
pub(super) fn put(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, at: usize) {
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| e.set_view(at, 0., cx))
    });
}

/// Whether the shell shows the message `what` now.
pub(super) fn says(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, what: &str) -> bool {
    shell.read_with(cx, |s, _| s.toasts.iter().any(|t| t.text == what))
}

#[gpui::test]
fn typing_inside_a_change_changes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-typing", NOTE, cx);
    // In an added line, in a removed line, and on each of the four marker lines.
    for needle in ["new", "old", "{--", "--}", "{++", "++}"] {
        put(&shell, cx, NOTE.find(needle).unwrap() + 1);
        cx.simulate_input("x");
        settle(cx);
        assert_eq!(
            text(&shell, cx),
            NOTE,
            "typing in {needle:?} changes nothing"
        );
    }
    assert!(says(&shell, cx, REFUSED), "the refusal is said");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn typing_at_the_edges_of_a_change(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-edges", NOTE, cx);
    // Before the change: at the end of the kept line, its break stays where it is.
    put(&shell, cx, NOTE.find("{--").unwrap() - 1);
    cx.simulate_input("x");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE.replace("keep\n", "keepx\n"));
    // After the change: at the start of the kept line that follows it.
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE);
    put(&shell, cx, NOTE.find("end").unwrap());
    cx.simulate_input("x");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE.replace("end", "xend"));
    // At the start of the first marker line, a typed letter would be glued to the mark: refused.
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    put(&shell, cx, NOTE.find("{--").unwrap());
    cx.simulate_input("x");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE, "no letter glued to a mark");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn enter_at_the_edges_of_a_change_works(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-enter", NOTE, cx);
    // At the end of the kept line before the change: a line break is added there.
    put(&shell, cx, NOTE.find("{--").unwrap() - 1);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE.replace("keep\n", "keep\n\n"));
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    // At the start of the kept line after the change: the same.
    put(&shell, cx, NOTE.find("end").unwrap());
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE.replace("end\n", "\nend\n"));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn backspace_after_a_change_is_refused(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-backspace", NOTE, cx);
    // At the start of the kept line after the change: the break of its last mark would go.
    put(&shell, cx, NOTE.find("end").unwrap());
    cx.simulate_keystrokes("backspace");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE);
    assert!(says(&shell, cx, REFUSED));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn paste_and_cut_over_a_change_are_refused(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-cut", NOTE, cx);
    // A selection from the kept line into the added line: it crosses the change.
    let from = NOTE.find("keep").unwrap();
    let len = NOTE.find("new").unwrap() + 1 - from;
    put(&shell, cx, from);
    cx.simulate_keystrokes(&"shift-right ".repeat(len));
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("Z".into())));
    cx.simulate_keystrokes("secondary-v");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE, "paste refused");
    cx.simulate_keystrokes("secondary-x");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE, "cut refused");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn replace_all_is_refused_when_a_match_is_in_a_change(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) =
        open_note("pending-replace", "a one\n{++\na two\n++}\na three\n", cx);
    cx.simulate_keystrokes("secondary-r");
    cx.simulate_input("a");
    cx.simulate_keystrokes("secondary-r");
    cx.simulate_input("b");
    cx.simulate_keystrokes("secondary-enter");
    settle(cx);
    assert_eq!(text(&shell, cx), "a one\n{++\na two\n++}\na three\n");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn replace_all_works_outside_the_changes(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) =
        open_note("pending-replace-ok", "a one\n{++\nzzz\n++}\na three\n", cx);
    cx.simulate_keystrokes("secondary-r");
    cx.simulate_input("a");
    cx.simulate_keystrokes("secondary-r");
    cx.simulate_input("b");
    cx.simulate_keystrokes("secondary-enter");
    settle(cx);
    assert_eq!(text(&shell, cx), "b one\n{++\nzzz\n++}\nb three\n");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn accept_keeps_the_added_lines_and_one_undo_brings_the_change_back(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-accept", NOTE, cx);
    put(&shell, cx, NOTE.find("new").unwrap() + 1);
    cx.dispatch_action(editor::AcceptChange);
    settle(cx);
    assert_eq!(text(&shell, cx), "# Plan\n\nkeep\nnew\nend\n");
    let at = "# Plan\n\nkeep\n".len();
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).span()),
        at..at,
        "the caret is where the change was"
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn reject_keeps_the_removed_lines(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-reject", NOTE, cx);
    put(&shell, cx, NOTE.find("old").unwrap());
    cx.dispatch_action(editor::RejectChange);
    settle(cx);
    assert_eq!(text(&shell, cx), "# Plan\n\nkeep\nold\nend\n");
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn accept_and_reject_all_are_the_logic_of_the_whole_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-all", TWO, cx);
    cx.dispatch_action(editor::AcceptAllChanges);
    settle(cx);
    assert_eq!(text(&shell, cx), pending::accept_all(TWO));
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), TWO);
    cx.dispatch_action(editor::RejectAllChanges);
    settle(cx);
    assert_eq!(text(&shell, cx), pending::reject_all(TWO));
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), TWO, "one undo step each");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn with_no_change_the_actions_change_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-none", "# Plan\n\nkeep\n", cx);
    cx.dispatch_action(editor::AcceptChange);
    settle(cx);
    assert_eq!(text(&shell, cx), "# Plan\n\nkeep\n");
    assert!(says(&shell, cx, "No pending change."));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_comment_on_a_change_is_written_and_the_change_stays_pending(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-comment", NOTE, cx);
    put(&shell, cx, NOTE.find("new").unwrap() + 1);
    cx.dispatch_action(editor::CommentChange);
    settle(cx);
    cx.simulate_input("à relire");
    cx.simulate_keystrokes("enter");
    settle(cx);
    let marked = text(&shell, cx);
    assert!(marked.contains("new<!-- comment: à relire -->"), "{marked}");
    assert!(pending::has_pending(&marked), "still pending");
    assert_eq!(pending::areas(&marked).len(), 1);
    assert_eq!(
        pending::reject_all(&marked),
        "# Plan\n\nkeep\nold\nend\n",
        "rejected, the person's lines are back"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_outside_change_with_a_pending_change_still_applies(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-outside", NOTE, cx);
    let at = NOTE.len();
    let edit = (at..at, "{++\nlu dehors\n++}\n".to_string());
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.absorb(std::slice::from_ref(&edit), cx))
    });
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        format!("{NOTE}{{++\nlu dehors\n++}}\n"),
        "the outside text lands"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_hint_counts_the_changes_and_moves_the_caret(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-hint", TWO, cx);
    assert!(
        cx.debug_bounds("pending-hint").is_some(),
        "the hint is drawn"
    );
    let at = cx.debug_bounds("pending-hint").expect("drawn").center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    let caret = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position());
    assert_eq!(
        caret,
        TWO.find("{--").unwrap(),
        "the caret goes to the first change"
    );
    let _ = fs::remove_dir_all(root);
}

const MOVED: &str = "Caret moved to the next change. Repeat to accept or reject it.";

#[gpui::test]
fn a_first_press_outside_a_change_only_moves_the_caret(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-first-press", TWO, cx);
    put(&shell, cx, 0);
    cx.dispatch_action(editor::AcceptChange);
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        TWO,
        "nothing is accepted by the first press"
    );
    assert!(says(&shell, cx, MOVED), "the move is said");
    let caret = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position());
    assert_eq!(
        caret,
        TWO.find("{--").unwrap(),
        "the caret is in the first change"
    );
    cx.dispatch_action(editor::AcceptChange);
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        pending::accept(TWO, 0),
        "the second press accepts it"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_first_press_after_the_last_change_wraps_to_the_first(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-wrap", TWO, cx);
    put(&shell, cx, TWO.len());
    cx.dispatch_action(editor::RejectChange);
    settle(cx);
    assert_eq!(text(&shell, cx), TWO);
    let caret = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position());
    assert_eq!(
        caret,
        TWO.find("{--").unwrap(),
        "the caret wraps to the first change"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_comment_outside_a_change_only_moves_the_caret(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-comment-first", TWO, cx);
    put(&shell, cx, 0);
    cx.dispatch_action(editor::CommentChange);
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        TWO,
        "no comment box opens for the first press"
    );
    assert!(says(&shell, cx, MOVED));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_comment_finds_its_change_again_after_an_outside_edit_moved_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-comment-moved", TWO, cx);
    put(&shell, cx, TWO.find("deux").unwrap() + 1);
    cx.dispatch_action(editor::CommentChange);
    settle(cx);
    // While the prompt is open, an outside edit removes the first change: the second is now first.
    let first = pending::areas(TWO)[0].range.clone();
    let edit = (first, String::new());
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.absorb(std::slice::from_ref(&edit), cx))
    });
    settle(cx);
    cx.simulate_input("à relire");
    cx.simulate_keystrokes("enter");
    settle(cx);
    let marked = text(&shell, cx);
    assert!(
        marked.contains("deux<!-- comment: à relire -->"),
        "{marked}"
    );
    assert!(
        !marked.contains("one"),
        "the first change went away: {marked}"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_comment_whose_change_is_gone_changes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-comment-gone", TWO, cx);
    put(&shell, cx, TWO.find("deux").unwrap() + 1);
    cx.dispatch_action(editor::CommentChange);
    settle(cx);
    // The outside edit takes the change away while the prompt is open.
    let second = pending::areas(TWO)[1].range.clone();
    let edit = (second, String::new());
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.absorb(std::slice::from_ref(&edit), cx))
    });
    settle(cx);
    let before = text(&shell, cx);
    cx.simulate_input("trop tard");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(text(&shell, cx), before, "nothing is commented");
    assert!(says(
        &shell,
        cx,
        "That change is gone or changed: nothing was commented."
    ));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_style_over_lines_with_a_change_is_all_or_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-style", NOTE, cx);
    // From the kept line into the added line: the kept word would be bold, the added one is refused.
    let from = NOTE.find("keep").unwrap();
    let len = NOTE.find("new").unwrap() + 1 - from;
    put(&shell, cx, from);
    cx.simulate_keystrokes(&"shift-right ".repeat(len));
    cx.dispatch_action(editor::Bold);
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        NOTE,
        "no sign is put on the kept line alone"
    );
    assert!(says(&shell, cx, REFUSED));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_refused_comment_takes_no_undo_step(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-undo", NOTE, cx);
    put(&shell, cx, NOTE.find("keep").unwrap() + 4);
    cx.simulate_input("y");
    settle(cx);
    let typed = NOTE.replace("keep\n", "keepy\n");
    assert_eq!(text(&shell, cx), typed);
    // The comment of an added line is refused: it must not be a step of the undo history.
    put(&shell, cx, NOTE.find("new").unwrap() + 1);
    cx.dispatch_action(editor::Comment);
    settle(cx);
    assert_eq!(text(&shell, cx), typed);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        NOTE,
        "one undo takes back the letter only"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn ctrl_enter_accepts_the_change_the_caret_is_in(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-chord-accept", NOTE, cx);
    put(&shell, cx, NOTE.find("new").unwrap() + 1);
    settle(cx);
    // The chord wins over the Editor binding of Ctrl+Enter (ToggleTask) inside a change.
    cx.simulate_keystrokes("secondary-enter");
    settle(cx);
    assert_eq!(text(&shell, cx), pending::accept(NOTE, 0));
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE, "one undo step");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn ctrl_backspace_rejects_the_change_the_caret_is_in(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-chord-reject", NOTE, cx);
    put(&shell, cx, NOTE.find("old").unwrap() + 1);
    settle(cx);
    cx.simulate_keystrokes("secondary-backspace");
    settle(cx);
    assert_eq!(text(&shell, cx), pending::reject(NOTE, 0));
    assert_eq!(text(&shell, cx), "# Plan\n\nkeep\nold\nend\n");
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE, "one undo step");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn ctrl_enter_outside_a_change_toggles_the_task_and_accepts_nothing(cx: &mut TestAppContext) {
    let note = "# Plan\n\n- [ ] milk\n{--\nold\n--}\n{++\nnew\n++}\nend\n";
    let (root, shell, cx, _config) = open_note("pending-chord-task", note, cx);
    put(&shell, cx, note.find("milk").unwrap());
    settle(cx);
    cx.simulate_keystrokes("secondary-enter");
    settle(cx);
    assert_eq!(text(&shell, cx), note.replace("- [ ] milk", "- [x] milk"));
    assert!(
        text(&shell, cx).contains("{++\nnew\n++}"),
        "nothing is accepted"
    );
    let _ = fs::remove_dir_all(root);
}

/// The change of `NOTE`, markers included.
const CHANGE: &str = "{--\nold\n--}\n{++\nnew\n++}";

/// Outside a change, `secondary-backspace` (Cmd+Backspace on macOS, Ctrl+Backspace elsewhere) does
/// not touch the pending change: on macOS it has no binding there, elsewhere it is the word key.
#[gpui::test]
fn secondary_backspace_outside_a_change_leaves_the_change_pending(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-chord-secondary", NOTE, cx);
    put(&shell, cx, NOTE.find("keep").unwrap() + 4);
    settle(cx);
    cx.simulate_keystrokes("secondary-backspace");
    settle(cx);
    assert!(
        text(&shell, cx).contains(CHANGE),
        "the change is neither rejected nor altered"
    );
    assert!(!says(&shell, cx, REFUSED), "nothing is refused either");
    let _ = fs::remove_dir_all(root);
}

/// The word key deletes the word before the caret, outside a change, and the change stays pending.
/// It is `alt-backspace` on macOS and `ctrl-backspace` elsewhere (`secondary-backspace` there).
#[gpui::test]
fn word_backspace_outside_a_change_deletes_the_word_as_before(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-chord-word", NOTE, cx);
    put(&shell, cx, NOTE.find("keep").unwrap() + 4);
    settle(cx);
    let word_key = if cfg!(target_os = "macos") {
        "alt-backspace"
    } else {
        "ctrl-backspace"
    };
    cx.simulate_keystrokes(word_key);
    settle(cx);
    assert_eq!(text(&shell, cx), NOTE.replacen("keep", "", 1));
    assert!(
        text(&shell, cx).contains("{++\nnew\n++}"),
        "the change stays pending"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_renumbering_above_a_change_leaves_the_change_and_the_list_below_alone(
    cx: &mut TestAppContext,
) {
    let note = "1. one\n2. two\n{--\nold\n--}\n5. three\n6. four\n";
    let (root, shell, cx, _config) = open_note("pending-list-above", note, cx);
    // Enter at the end of the first item inserts an item: the list above is renumbered, and only it.
    put(&shell, cx, note.find("one").unwrap() + 3);
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        "1. one\n2. \n3. two\n{--\nold\n--}\n5. three\n6. four\n",
        "the list above is renumbered, the change and the list below are not"
    );
    let notices = shell.read_with(cx, |s, _| {
        s.toasts
            .iter()
            .filter(|t| t.text.contains("keeps its numbers") || t.text.contains("not realigned"))
            .count()
    });
    assert_eq!(notices, 0, "no notice is shown");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn typing_in_a_table_inside_a_change_is_refused(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-table-typing", TABLE_CHANGE, cx);
    put(&shell, cx, TABLE_CHANGE.find("| 1").unwrap() + 2);
    cx.simulate_input("x");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        TABLE_CHANGE,
        "the table text is unchanged"
    );
    assert!(says(&shell, cx, REFUSED), "the refusal is said");
    let _ = fs::remove_dir_all(root);
}

/// A save takes the marks off the file: its own text is written, and the change waits beside it in
/// `.inkuno/pending`, where a reopen finds it again (the editor shows it marked as before).
#[gpui::test]
fn a_save_keeps_the_file_free_of_marks_and_the_change_beside_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note("pending-clean-save", NOTE, cx);
    put(&shell, cx, NOTE.find("keep").unwrap() + 4);
    cx.simulate_input("!");
    settle(cx);
    shell.update(cx, |s, cx| s.flush(cx));
    settle(cx);
    let file = fs::read_to_string(root.join("Plan.md")).unwrap();
    assert_eq!(
        file, "# Plan\n\nkeep!\nold\nend\n",
        "the file keeps its own text"
    );
    let changes = crate::platform::sidecar::read(
        &crate::platform::revisions::Home::Vault(root.clone()),
        &root.join("Plan.md"),
    );
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(
        (changes[0].old.as_str(), changes[0].new.as_str()),
        ("old\n", "new\n")
    );
    // The editor still shows the change, marked; accepting it writes its text into the file.
    assert!(
        text(&shell, cx).contains("{++\nnew\n++}"),
        "{}",
        text(&shell, cx)
    );
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.pane().editor.focus_handle(cx))
    });
    put(&shell, cx, NOTE.find("{++").unwrap() + 1);
    cx.dispatch_action(editor::AcceptChange);
    settle(cx);
    shell.update(cx, |s, cx| s.flush(cx));
    settle(cx);
    assert_eq!(
        fs::read_to_string(root.join("Plan.md")).unwrap(),
        "# Plan\n\nkeep!\nnew\nend\n"
    );
    assert!(
        crate::platform::sidecar::read(
            &crate::platform::revisions::Home::Vault(root.clone()),
            &root.join("Plan.md"),
        )
        .is_empty()
    );
    let _ = fs::remove_dir_all(root);
}

/// Comments written in the earlier form are written in the new one on the next save, and the
/// editor takes the new form too.
#[gpui::test]
fn a_save_writes_the_old_comments_in_the_new_form(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open_note(
        "pending-migrate",
        "# Plan\n\nkeep {==this==}{>>old note<<} here\n{>>standalone<<}\n",
        cx,
    );
    assert!(
        text(&shell, cx).contains("keep this<!-- comment: old note --> here"),
        "{}",
        text(&shell, cx)
    );
    let end = text(&shell, cx).len();
    put(&shell, cx, end);
    cx.simulate_input("x");
    settle(cx);
    shell.update(cx, |s, cx| s.flush(cx));
    settle(cx);
    assert_eq!(
        fs::read_to_string(root.join("Plan.md")).unwrap(),
        "# Plan\n\nkeep this<!-- comment: old note --> here\n<!-- comment: standalone -->\nx"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_orphan_is_counted_in_the_pill_and_discard_removes_it(cx: &mut TestAppContext) {
    use crate::{logic::pending::Entry, platform::revisions::Home};
    let (root, shell, cx, _config) = open_note("pending-orphan", "a\nz\nb\n", cx);
    assert!(
        cx.debug_bounds("pending-hint").is_none(),
        "no change, no pill"
    );
    // A change whose context the note no longer has: it is shown nowhere, and the pill counts it.
    let note = root.join("Plan.md");
    let home = Home::Vault(root.clone());
    let orphan = Entry {
        id: 1,
        author: "assistant".into(),
        at: 2,
        old: "gone\n".into(),
        new: "x\n".into(),
        before: "q\n".into(),
        after: "r\n".into(),
        orphan_since: None,
    };
    crate::platform::sidecar::store(&home, &note, std::slice::from_ref(&orphan)).unwrap();
    settle(cx);
    assert!(
        cx.debug_bounds("pending-hint").is_some(),
        "the pill says so"
    );
    let at = cx
        .debug_bounds("pending-discard")
        .expect("Discard is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    assert!(
        crate::platform::sidecar::read(&home, &note).is_empty(),
        "Discard removes the orphan from the list"
    );
    // Absence is read from the state the render gates the pill on: a stale debug bound may outlive
    // the element (GPUI keeps the last bounds of an element drawn once).
    assert_eq!(
        shell.read_with(cx, |s, _| s.unplaced_changes()),
        0,
        "no orphan is left to count"
    );
    assert_eq!(
        text(&shell, cx),
        "a\nz\nb\n",
        "the note's text is untouched"
    );
    let _ = fs::remove_dir_all(root);
}
