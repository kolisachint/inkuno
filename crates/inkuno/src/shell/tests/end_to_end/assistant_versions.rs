//! The assistant, the pending changes and the Versions panel, as a person uses them together. No agent
//! runs: the answers are fed by hand, as the agent's reader would feed them. A review writes comments,
//! an improvement comes back as pending changes that the chords decide, and the history of the note
//! lists its versions and compares one of them.

use super::super::{assistant as asst, pending as pend, versions as vers};
use super::*;
use crate::platform::assistant::Event;
use crate::shell::assistant::Who;

/// The note of the review: two passages to comment on.
const REVIEW: &str = "# Revue\n\nun peu de texte\nla fin du texte\n";
/// The agent's answer: two findings, one comment each.
const TWO_FINDINGS: &str = "PASSAGE: un peu de texte\nCOMMENT: Trop court.\n\
    PASSAGE: la fin du texte\nCOMMENT: Conclure.\n";
const REVIEWED: &str = "# Revue\n\nun peu de texte<!-- comment: Trop court. -->\nla fin du texte<!-- comment: Conclure. -->\n";

/// The note of the chords: a passage to improve, a kept line, then a task line outside the passage.
const CHORD: &str = "# Plan\n\nun peu de texte\nentre les deux\nla fin du texte\n- [ ] lait\n";
/// The agent's answer: the first and the third line change, the middle one is kept.
const TWO_PLACES: &str =
    "BEGIN TEXT\nUn peu plus clair.\nentre les deux\nLa fin, plus nette.\nEND TEXT\n";

/// The note of the whole-note improvement, without a line break at its end.
const WHOLE: &str = "# Plan\n\nun peu de texte\nentre les deux\nla fin, sans saut de ligne";
/// The agent's answer for the whole note: two places change, the line between them is kept.
const WHOLE_ANSWER: &str =
    "BEGIN TEXT\n# Plan\n\nUn peu plus clair.\nentre les deux\nLa fin, nette.\nEND TEXT";

/// The note of the versions, and the saves and the improvement it goes through.
const JOURNAL: &str = "# Journal\n\npremier jet\n";
/// The text of the second save.
const SECOND_SAVE: &str = "# Journal\n\npremier jet\ndeuxième ligne\n";
/// The note before the improvement: a line typed after the second save.
const UNSAVED: &str = "# Journal\n\npremier jet\ndeuxième ligne\ntroisième ligne\n";
/// The first save compared with the note now: the two lines added since.
const COMPARED: &str = "# Journal\n\npremier jet\n{++\ndeuxième ligne\ntroisième ligne\n++}\n";

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    // The assistant's buttons and the Versions panel need the window of the focused tests.
    cx.simulate_resize(size(px(1400.), px(900.)));
    j.settle(cx);
    a_review_writes_its_findings_and_one_undo_takes_them_back(j, cx);
    the_chords_decide_the_change_under_the_caret(j, cx);
    improving_the_whole_note_then_accept_all_and_reject_all(j, cx);
    the_versions_panel_lists_the_saves_and_compares_one(j, cx);
    restore_a_version_on_the_note_as_pending_changes(j, cx);
}

/// Opens `name` with `content` in the only tab, its editor given the keys.
fn open_note(j: &Journey, cx: &mut gpui::VisualTestContext, name: &str, content: &str) -> PathBuf {
    let path = j.root.join(name);
    fs::write(&path, content).unwrap();
    j.shell.update(cx, |s, cx| s.rescan(&j.root, cx));
    j.shell.update_in(cx, |s, window, cx| {
        s.close_all_files(window, cx);
        s.open_from_nav(&path, window, cx)
    });
    asst::settle(cx);
    asst::focus_note(&j.shell, cx);
    path
}

/// The assistant's panel, shown. Its round button toggles it, so it is clicked only when it is shut.
fn show_assistant(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    if !shell.read_with(cx, |s, _| s.right_open()) {
        asst::click_button(cx);
    }
}

/// The palette, typed and chosen with Enter. The keys are the note's when the palette opens, as when a
/// person types in it. The command is chosen with `down` as many times as `downs` says, then Enter.
fn palette(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, query: &str, downs: usize) {
    asst::focus_note(shell, cx);
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input(query);
    for _ in 0..downs {
        cx.simulate_keystrokes("down");
    }
    cx.simulate_keystrokes("enter");
    asst::settle(cx);
}

/// Clicks Stop, where it is drawn while the agent works.
fn click_stop(cx: &mut gpui::VisualTestContext) {
    let at = cx
        .debug_bounds("assistant-stop")
        .expect("Stop is drawn while the agent works")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    asst::settle(cx);
}

/// A right click at `at`: the mouse goes down and up, as a person's does.
fn right_click(cx: &mut gpui::VisualTestContext, at: gpui::Point<Pixels>) {
    cx.simulate_event(gpui::MouseDownEvent {
        button: MouseButton::Right,
        position: at,
        modifiers: gpui::Modifiers::none(),
        click_count: 1,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        button: MouseButton::Right,
        position: at,
        modifiers: gpui::Modifiers::none(),
        click_count: 1,
    });
    vers::frame(cx);
}

/// How many changes the note of the shell holds pending now.
fn pending_count(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> usize {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).pending_changes())
}

/// Walk 1: "Review this note" writes two findings as comments, one undo takes both away, and a
/// review stopped before its end writes nothing.
fn a_review_writes_its_findings_and_one_undo_takes_them_back(
    j: &Journey,
    cx: &mut gpui::VisualTestContext,
) {
    let shell = &j.shell;
    let path = open_note(j, cx, "Revue.md", REVIEW);
    let original = j.text(cx);
    // The panel opens from its round button at the note's foot, and "Review this note" is asked in it.
    show_assistant(shell, cx);
    asst::click_review(cx);
    assert!(asst::entries(shell, cx).contains(&(Who::You, "Review this note".to_string())));
    asst::feed(
        shell,
        cx,
        vec![
            Event::Started,
            Event::Text(TWO_FINDINGS.into()),
            Event::Ended,
        ],
    );
    assert_eq!(j.text(cx), REVIEWED);
    assert_eq!(
        asst::entries(shell, cx).last(),
        Some(&(Who::Note, "Wrote 2 comments.".to_string()))
    );
    shell.update(cx, |s, cx| s.flush(cx));
    asst::settle(cx);
    assert_eq!(fs::read_to_string(&path).unwrap(), REVIEWED);

    // Ctrl+Z takes the whole review back, both comments at once.
    asst::focus_note(shell, cx);
    cx.simulate_keystrokes("secondary-z");
    asst::settle(cx);
    assert_eq!(j.text(cx), original);
    shell.update(cx, |s, cx| s.flush(cx));
    asst::settle(cx);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    // A second review, stopped before the agent ends: the answer that came is not written.
    asst::click_review(cx);
    asst::feed(shell, cx, vec![Event::Text(TWO_FINDINGS.into())]);
    click_stop(cx);
    assert_eq!(
        asst::entries(shell, cx).last(),
        Some(&(Who::Note, "Stopped.".to_string()))
    );
    asst::feed(shell, cx, vec![Event::Ended]);
    assert_eq!(j.text(cx), original);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

/// Walk 2: an improvement of a selection comes back as two pending changes; the chords decide the
/// change under the caret, one undo per decision, and outside a change they keep their own meaning.
fn the_chords_decide_the_change_under_the_caret(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let shell = &j.shell;
    open_note(j, cx, "Accords.md", CHORD);
    let start = CHORD.find("un peu").unwrap();
    let end = CHORD.find("\n- [ ]").unwrap();
    asst::select(shell, cx, start..end);
    show_assistant(shell, cx);

    // An answer that says NO CHANGE proposes nothing, and the panel says so.
    palette(shell, cx, "improve", 1);
    asst::feed(
        shell,
        cx,
        vec![
            Event::Started,
            Event::Text("NO CHANGE".into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        asst::entries(shell, cx).last(),
        Some(&(Who::Note, "The assistant proposed no change.".to_string()))
    );
    assert_eq!(j.text(cx), CHORD);
    assert_eq!(pending_count(shell, cx), 0);

    // The real answer: two separate places change, the line between them is kept.
    palette(shell, cx, "improve", 1);
    asst::feed(
        shell,
        cx,
        vec![Event::Started, Event::Text(TWO_PLACES.into()), Event::Ended],
    );
    let proposed = j.text(cx);
    assert_eq!(pending_count(shell, cx), 2);
    assert!(
        cx.debug_bounds("pending-hint").is_some(),
        "the hint counts 2"
    );
    assert!(proposed.contains("\nentre les deux\n"), "{proposed}");

    // Typing inside a change is refused, with its message, and the note keeps its text.
    let first = proposed.find("un peu de texte").unwrap();
    asst::caret(shell, cx, first + 3);
    cx.simulate_input("x");
    asst::settle(cx);
    assert_eq!(j.text(cx), proposed);
    assert!(pend::says(shell, cx, pend::REFUSED));

    // The caret in the first change, Ctrl+Enter accepts it: the hint counts 1.
    asst::caret(shell, cx, first + 3);
    cx.simulate_keystrokes("secondary-enter");
    asst::settle(cx);
    let accepted = j.text(cx);
    assert_eq!(
        accepted,
        "# Plan\n\nUn peu plus clair.\nentre les deux\n{--\nla fin du texte\n--}\n{++\nLa fin, plus nette.\n++}\n- [ ] lait\n"
    );
    assert_eq!(pending_count(shell, cx), 1);

    // The caret in the second change, Ctrl+Backspace rejects it: the old text is back, the hint is gone.
    let second = accepted.find("la fin du texte").unwrap();
    asst::caret(shell, cx, second + 3);
    cx.simulate_keystrokes("secondary-backspace");
    asst::settle(cx);
    assert_eq!(
        j.text(cx),
        "# Plan\n\nUn peu plus clair.\nentre les deux\nla fin du texte\n- [ ] lait\n"
    );
    // The hint is drawn only while changes are pending. Its bounds of the last frame stay in the harness,
    // so the count is what says it is gone.
    assert_eq!(pending_count(shell, cx), 0);

    // One undo per decision: the rejection first, then the acceptance.
    asst::focus_note(shell, cx);
    cx.simulate_keystrokes("secondary-z");
    asst::settle(cx);
    assert_eq!((j.text(cx), pending_count(shell, cx)), (accepted, 1));
    cx.simulate_keystrokes("secondary-z");
    asst::settle(cx);
    assert_eq!((j.text(cx), pending_count(shell, cx)), (proposed, 2));

    // Outside a change, the same keys do what they do in the editor: Ctrl+Enter checks the task,
    // and changes nothing pending; Ctrl+Backspace deletes the word before the caret, on the
    // systems where it has a binding (on macOS, Cmd+Backspace has none here).
    let task = j.text(cx).find("- [ ] lait").unwrap();
    asst::caret(shell, cx, task + 3);
    cx.simulate_keystrokes("secondary-enter");
    asst::settle(cx);
    assert!(j.text(cx).contains("- [x] lait"), "{}", j.text(cx));
    assert_eq!(pending_count(shell, cx), 2, "nothing is accepted");
    cx.simulate_keystrokes("end secondary-backspace");
    asst::settle(cx);
    if cfg!(target_os = "macos") {
        assert!(j.text(cx).ends_with("- [x] lait\n"), "{}", j.text(cx));
    } else {
        assert!(j.text(cx).ends_with("- [x] \n"), "{}", j.text(cx));
    }
    assert_eq!(pending_count(shell, cx), 2, "the changes stay pending");
    // Rejected from the palette, so that the next walk starts from a note with no change.
    palette(shell, cx, "reject all", 1);
    assert_eq!(pending_count(shell, cx), 0);
}

/// Walk 3: an improvement of the whole note, accepted all, then undone and rejected all: the note
/// is the one it was, byte for byte, with no line break at its end.
fn improving_the_whole_note_then_accept_all_and_reject_all(
    j: &Journey,
    cx: &mut gpui::VisualTestContext,
) {
    let shell = &j.shell;
    let path = open_note(j, cx, "Entier.md", WHOLE);
    let original = j.text(cx);
    show_assistant(shell, cx);
    // No selection: the palette's improvement is the whole note.
    palette(shell, cx, "improve", 1);
    assert!(
        asst::sent(shell, cx)
            .last()
            .is_some_and(|prompt| prompt.contains("the whole note"))
    );
    asst::feed(
        shell,
        cx,
        vec![
            Event::Started,
            Event::Text(WHOLE_ANSWER.into()),
            Event::Ended,
        ],
    );
    let marked = j.text(cx);
    assert_eq!(pending_count(shell, cx), 2);
    assert!(marked.contains("Un peu plus clair."), "{marked}");

    // Accept all from the palette: the note is the proposed text, nothing pending.
    palette(shell, cx, "accept all", 1);
    assert_eq!(
        j.text(cx),
        "# Plan\n\nUn peu plus clair.\nentre les deux\nLa fin, nette."
    );
    assert_eq!(pending_count(shell, cx), 0);

    // One undo: the changes are pending again, as they were.
    asst::focus_note(shell, cx);
    cx.simulate_keystrokes("secondary-z");
    asst::settle(cx);
    assert_eq!((j.text(cx), pending_count(shell, cx)), (marked, 2));

    // Reject all: the note is as it was, byte for byte, on disk too.
    palette(shell, cx, "reject all", 1);
    assert_eq!(j.text(cx), original);
    assert_eq!(pending_count(shell, cx), 0);
    shell.update(cx, |s, cx| s.flush(cx));
    asst::settle(cx);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}

/// Walk 4: two saves and an improvement; the panel lists the versions newest first, the automatic
/// ones marked; Enter on the oldest compares it with the note now, read-only, with its difference
/// pending; the row menu's copy puts another version's text on the clipboard.
fn the_versions_panel_lists_the_saves_and_compares_one(
    j: &Journey,
    cx: &mut gpui::VisualTestContext,
) {
    let shell = &j.shell;
    open_note(j, cx, "Journal.md", JOURNAL);
    vers::save_version(cx, "premier jet");
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_input("deuxième ligne\n");
    vers::settle(cx);
    vers::save_version(cx, "ajout");
    assert_eq!(j.text(cx), SECOND_SAVE);

    // A line typed after the last save: the text before the improvement is not a version yet, so the
    // improvement keeps two automatic versions, its text before and its proposal.
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_input("troisième ligne\n");
    vers::settle(cx);
    palette(shell, cx, "improve", 1);
    asst::feed(
        shell,
        cx,
        vec![
            Event::Started,
            Event::Text(
                "BEGIN TEXT\n# Journal\n\nPremier jet, revu.\ndeuxième ligne\ntroisième ligne\nEND TEXT".into(),
            ),
            Event::Ended,
        ],
    );
    assert_eq!(pending_count(shell, cx), 1);
    palette(shell, cx, "reject all", 1);
    assert_eq!(j.text(cx), UNSAVED);

    // The panel, newest first; the two automatic versions say so. The Versions list takes the keys
    // with the assistant's panel shut, as the panel's own tests have it.
    if shell.read_with(cx, |s, _| s.right_open()) {
        cx.dispatch_action(ToggleAssistant);
        asst::settle(cx);
    }
    palette(shell, cx, "versions", 1);
    vers::frame(cx);
    let me = shell.read_with(cx, |s, _| s.revision_author());
    let names = vers::names(shell, cx);
    assert_eq!(
        names,
        [
            "#4 · assistant · auto · proposed, pending".to_string(),
            format!("#3 · {me} · auto · before a change by assistant"),
            format!("#2 · {me} · ajout"),
            format!("#1 · {me} · premier jet"),
        ]
    );

    // The oldest, clicked: a comparison tab, read-only, the new line pending. (The keys reach the
    // panel in its own tests; after the palette's command the row is clicked.)
    let oldest = cx
        .debug_bounds(leak(format!("nav-row-{}", names[3])))
        .expect("the oldest save is listed")
        .center();
    cx.simulate_click(oldest, gpui::Modifiers::none());
    vers::settle(cx);
    let compare = vers::compare_tab(shell, cx);
    assert_eq!(
        shell.read_with(cx, |s, cx| s.tab_name(compare, cx)),
        "Journal · #1 ↔ now"
    );
    assert_eq!(j.text(cx), COMPARED);
    assert!(shell.read_with(cx, |s, cx| {
        let tab = &s.panes[compare];
        let editor = tab.editor.read(cx);
        editor.is_readonly() && editor.pending_changes() == 1 && tab.path.is_none() && !tab.dirty
    }));

    // Typing in the comparison is refused, with its message.
    let at = j.text(cx).find("deuxième").unwrap() + 2;
    vers::put(shell, cx, at);
    cx.simulate_input("x");
    vers::settle(cx);
    assert_eq!(j.text(cx), COMPARED);
    assert!(vers::says(shell, cx, vers::COMPARISON));

    // The file gestures are refused, whatever the panel: nothing is done to the history. As in the
    // panel's own tests, the tree is shown with the version as its target (a stale selection).
    let version = vers::row_path(shell, cx, 3);
    let before = fs::read(&version).unwrap();
    let files = vers::files_under(&j.root);
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Tree, false, window, cx);
        s.menu_do(nav::Do::Trash, Some(version.clone()), window, cx);
        s.menu_do(nav::Do::Rename, Some(version.clone()), window, cx);
    });
    vers::settle(cx);
    assert!(vers::says(
        shell,
        cx,
        "The history of the notes is kept by the app: nothing is done to it."
    ));
    assert_eq!(fs::read(&version).unwrap(), before);
    assert_eq!(vers::files_under(&j.root), files);

    // The row menu of the second save: its copy puts that version's text on the clipboard.
    vers::show_versions(shell, cx);
    let row = cx
        .debug_bounds(leak(format!("nav-row-{}", names[2])))
        .expect("the row of the second save is listed")
        .center();
    right_click(cx, row);
    let copy = cx
        .debug_bounds(leak(format!(
            "menu-entry-{}",
            tr(
                "Copy this version's text",
                "Copier le texte de cette version"
            )
        )))
        .expect("the row menu offers the copy")
        .center();
    cx.simulate_click(copy, gpui::Modifiers::none());
    vers::settle(cx);
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
        Some(SECOND_SAVE)
    );
}

/// Walk 5: a version restored from the panel's menu, on the note's own tab, comes back as pending
/// changes; accepted, the note is that version's text, and the list gains the two automatic versions
/// on top; restoring the text the note already has says so; one undo takes the restore back.
fn restore_a_version_on_the_note_as_pending_changes(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let shell = &j.shell;
    let journal = j.root.join("Journal.md");
    let note = shell.read_with(cx, |s, _| {
        s.panes
            .iter()
            .position(|p| p.compare.is_none() && p.path.as_deref() == Some(journal.as_path()))
            .expect("the note's tab is open")
    });
    let compare = vers::compare_tab(shell, cx);
    shell.update_in(cx, |s, window, cx| s.focus_pane(note, window, cx));
    vers::show_versions(shell, cx);
    let before = vers::names(shell, cx);
    assert_eq!(before.len(), 4);
    // Version 2 is the third row, the note has its own text now.
    let second = vers::row_path(shell, cx, 2);

    // A comparison refuses the restore, whatever the list shows.
    shell.update_in(cx, |s, window, cx| s.focus_pane(compare, window, cx));
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::RestoreVersion, Some(second.clone()), window, cx)
    });
    vers::settle(cx);
    assert!(vers::says(shell, cx, vers::COMPARISON));
    assert_eq!(
        shell.read_with(cx, |s, _| s
            .panes
            .iter()
            .filter(|p| p.compare.is_some())
            .count()),
        1
    );

    // The note's own tab: the row's menu, Restore this version.
    shell.update_in(cx, |s, window, cx| s.focus_pane(note, window, cx));
    vers::show_versions(shell, cx);
    restore_from_row_menu(cx, &before[2]);
    assert_eq!(j.text(cx), RESTORED_PENDING);
    assert_eq!(pending_count(shell, cx), 1);

    // Accepted, the note is exactly that version's text.
    palette(shell, cx, "accept all", 1);
    assert_eq!(j.text(cx), SECOND_SAVE);
    assert_eq!(pending_count(shell, cx), 0);

    // The list: the two automatic versions on top, the four before them unchanged.
    vers::frame(cx);
    let me = shell.read_with(cx, |s, _| s.revision_author());
    let rows = vers::names(shell, cx);
    assert_eq!(rows.len(), 6);
    assert_eq!(
        rows[0],
        format!("#6 · {me} · auto · version 2 restored, pending")
    );
    assert_eq!(
        rows[1],
        format!("#5 · {me} · auto · before restoring version 2")
    );
    assert_eq!(rows[2..], before[..]);

    // Restoring the text the note already has changes nothing.
    let same = rows.iter().position(|r| r.starts_with("#2 · ")).unwrap();
    restore_from_row_menu(cx, &rows[same]);
    assert!(vers::says(shell, cx, "The note is this version already."));
    assert_eq!(j.text(cx), SECOND_SAVE);
    assert_eq!(vers::names(shell, cx).len(), 6);

    // One undo takes the accepting back, a second one the restore.
    asst::focus_note(shell, cx);
    cx.simulate_keystrokes("secondary-z");
    asst::settle(cx);
    assert_eq!(j.text(cx), RESTORED_PENDING);
    cx.simulate_keystrokes("secondary-z");
    asst::settle(cx);
    assert_eq!(j.text(cx), UNSAVED);
    assert_eq!(pending_count(shell, cx), 0);
}

/// The text of the note when version 2 is restored: its one removed line pending.
const RESTORED_PENDING: &str =
    "# Journal\n\npremier jet\ndeuxième ligne\n{--\ntroisième ligne\n--}\n";

/// Right-click on the panel row named `name`, then its menu's "Restore this version".
fn restore_from_row_menu(cx: &mut gpui::VisualTestContext, name: &str) {
    vers::frame(cx);
    let row = cx
        .debug_bounds(leak(format!("nav-row-{name}")))
        .expect("the version is listed")
        .center();
    right_click(cx, row);
    let entry = cx
        .debug_bounds(leak(format!(
            "menu-entry-{}",
            tr("Restore this version", "Restaurer cette version")
        )))
        .expect("the row menu offers the restore")
        .center();
    cx.simulate_click(entry, gpui::Modifiers::none());
    vers::settle(cx);
}
