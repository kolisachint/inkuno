//! The assistant: its round button at the note's foot, its panel in the right dock, the prompt box,
//! and the seam the process side fills in. No process is started: the tests call the seam by hand
//! and read what the panel shows.

use super::pending::unstamped;
use super::*;
use crate::actions::AssistantImprove;
use crate::platform::assistant::Event;
use crate::platform::revisions::{self as history, Home};
use crate::shell::assistant::{Status, Who};
use std::{ops::Range, time::Duration};

/// Lets the window draw what changed (bounds are read from the last frame), then the tasks run.
pub(super) fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// A note open in a window of its own size, with the panel closed.
fn note_open<'a>(
    name: &str,
    cx: &'a mut TestAppContext,
) -> (
    PathBuf,
    Entity<Shell>,
    &'a mut gpui::VisualTestContext,
    std::sync::MutexGuard<'static, ()>,
) {
    note_with(name, "# Plan\n\nun peu de texte\n", cx)
}

/// A note `Plan.md` with `content`, open in a window of its own size, with the panel closed.
fn note_with<'a>(
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
    fs::write(root.join("Plan.md"), content).unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("Plan.md"), window, cx)
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    settle(cx);
    (root, shell, cx, config)
}

/// Toggles the assistant's view of the right panel, as its palette command does (the round button is
/// gone: decision 11; its switcher button is `switch-assistant`).
pub(super) fn click_button(cx: &mut gpui::VisualTestContext) {
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
}

pub(super) fn entries(
    shell: &Entity<Shell>,
    cx: &mut gpui::VisualTestContext,
) -> Vec<(Who, String)> {
    shell.read_with(cx, |s, _| {
        s.assistant
            .entries
            .iter()
            .map(|e| (e.who, e.text.clone()))
            .collect()
    })
}

#[gpui::test]
fn the_assistant_view_opens_and_shuts_with_the_panel(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-button", cx);
    // The window's own buttons (and the band's toggle beside them) are drawn with the client's bar.
    shell.update(cx, |s, _| s.client = true);
    settle(cx);
    // No counter any more, and no round button at the note's foot: the assistant is a view of the panel.
    assert!(cx.debug_bounds("counts").is_none(), "the counter is gone");
    assert!(
        cx.debug_bounds("assistant-toggle").is_none(),
        "the round button is gone"
    );
    assert!(
        cx.debug_bounds("assistant").is_none(),
        "the panel waits hidden"
    );
    // The palette's action opens the view in the right panel; its switcher button shows the same view.
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
    let at = cx
        .debug_bounds("switch-assistant")
        .expect("the switcher's assistant button is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    assert!(cx.debug_bounds("assistant").is_some(), "the view is shown");
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.right_shown.clone()),
        [Mode::Assistant]
    );
    // The band's toggle shuts the panel and opens it again on the same view.
    let toggle = cx
        .debug_bounds("right-panel-toggle")
        .expect("the band's toggle is drawn")
        .center();
    cx.simulate_click(toggle, gpui::Modifiers::none());
    settle(cx);
    assert!(
        !shell.read_with(cx, |s, _| s.right_open()),
        "the panel is hidden"
    );
    // Shut, the toggle moves to the band's right end: its place is read again.
    let toggle = cx
        .debug_bounds("right-panel-toggle")
        .expect("the band's toggle is drawn")
        .center();
    cx.simulate_click(toggle, gpui::Modifiers::none());
    settle(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.right_shown.clone()),
        [Mode::Assistant]
    );
    // The palette's action shuts it, and opens it again.
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
    assert!(!shell.read_with(cx, |s, _| s.right_open()), "and shuts it");
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
    assert!(
        shell.read_with(cx, |s, _| s.right_open()),
        "the palette action opens it"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn focus_mode_hides_the_assistant_panel(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-focus", cx);
    click_button(cx);
    assert!(cx.debug_bounds("switch-assistant").is_some());
    cx.simulate_keystrokes("f9");
    settle(cx);
    // The panel is not drawn in focus mode (the harness keeps old bounds, so the state is read).
    assert!(
        shell.read_with(cx, |s, _| s.focus_mode),
        "F9 gives the note the window"
    );
    cx.simulate_keystrokes("f9");
    settle(cx);
    assert!(shell.read_with(cx, |s, _| !s.focus_mode));
    assert!(cx.debug_bounds("switch-assistant").is_some());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn opening_the_assistant_takes_the_place_of_the_outline(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-outline", cx);
    shell.update(cx, |s, _| s.nav.right_shown = vec![Mode::Outline]);
    settle(cx);
    click_button(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.right_shown.clone()),
        [Mode::Assistant],
        "one view of the panel at a time"
    );
    assert!(
        !shell.read_with(cx, |s, _| s.nav.right_shown.contains(&Mode::Outline)),
        "the outline is no longer shown"
    );
    assert!(cx.debug_bounds("assistant").is_some());
    let _ = fs::remove_dir_all(root);
}

/// Feeds the agent's events to the panel, as its reader thread's signal does.
pub(super) fn feed(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, events: Vec<Event>) {
    shell.update_in(cx, |s, window, cx| s.assistant_events(events, window, cx));
    settle(cx);
}

pub(super) fn status(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Status {
    shell.read_with(cx, |s, _| s.assistant.status)
}

/// The prompts the agent was sent, in order.
pub(super) fn sent(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
    shell.read_with(cx, |s, _| s.assistant.sent.clone())
}

/// The text of the note in the active tab.
pub(super) fn editor_text(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
}

/// Gives the keys to the note, as a click in it would.
pub(super) fn focus_note(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.pane().editor.focus_handle(cx), cx)
    });
    settle(cx);
}

/// Clicks "Review this note", where it is drawn now.
pub(super) fn click_review(cx: &mut gpui::VisualTestContext) {
    let at = cx
        .debug_bounds("assistant-review")
        .expect("the review button is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

/// The answer of a review that finds one comment, and one passage that is not in the note.
pub(super) const ONE_FINDING_AND_ONE_MISS: &str = "PASSAGE: un peu de texte\nCOMMENT: Trop court.\n\
    PASSAGE: absent du texte\nCOMMENT: Rien.\n";

#[gpui::test]
fn the_prompt_box_sends_what_is_typed(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-prompt", cx);
    click_button(cx);
    // Enter with nothing typed does nothing.
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert!(entries(&shell, cx).is_empty());
    assert!(sent(&shell, cx).is_empty());

    // Typed, then Enter: the transcript shows the question only, the box is cleared, and the
    // words sent name the note's file and carry the question.
    cx.simulate_input("Summarise this note");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        entries(&shell, cx),
        [(Who::You, "Summarise this note".to_string())]
    );
    let words = sent(&shell, cx);
    assert_eq!(words.len(), 1);
    assert!(words[0].contains("the file `Plan.md`"), "{}", words[0]);
    assert!(words[0].ends_with("Summarise this note"), "{}", words[0]);
    assert!(shell.read_with(cx, |s, cx| s.assistant.field.read(cx).line.text.is_empty()));
    assert!(cx.debug_bounds("assistant-entry-0").is_some());
    assert_eq!(status(&shell, cx), Status::Working);

    // While the agent works, the box refuses a second prompt.
    cx.simulate_input("again");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(sent(&shell, cx).len(), 1, "one prompt at a time");

    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("It is short.".into()),
            Event::Ended,
        ],
    );
    assert_eq!(status(&shell, cx), Status::Idle, "shown: Idle at the end");
    assert_eq!(entries(&shell, cx)[1], (Who::Agent, "It is short.".into()));

    // "Review this note" sends the review: a You line and a quiet "Reviewing…" line.
    click_review(cx);
    let words = sent(&shell, cx);
    assert_eq!(words.len(), 2);
    assert_eq!(
        words[1],
        crate::logic::assistant::review_prompt("Plan.md", &[])
    );
    let lines = entries(&shell, cx);
    assert_eq!(lines[2], (Who::You, "Review this note".to_string()));
    assert_eq!(lines[3].0, Who::Note);
    assert_eq!(status(&shell, cx), Status::Working);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_events_stream_text_into_one_agent_line(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-stream", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    feed(&shell, cx, vec![Event::Started]);
    assert!(
        cx.debug_bounds("assistant-stop").is_some(),
        "Stop while working"
    );
    // Text in pieces makes one line of the agent; a tool's name is not shown. The text of a
    // message after a tool call starts a new paragraph of the same line.
    feed(
        &shell,
        cx,
        vec![
            Event::Text("Hel".into()),
            Event::Break,
            Event::Tool("Read".into()),
            Event::Text("lo".into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        entries(&shell, cx)[1..],
        [(Who::Agent, "Hel\n\nlo".to_string())]
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_run_with_the_panel_hidden_waits_as_needs_you(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-hidden", cx);
    shell.update(cx, |s, _| s.client = true);
    settle(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    assert!(
        !shell.read_with(cx, |s, _| s.right_open()),
        "the panel is hidden"
    );
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text("Answer".into()), Event::Ended],
    );
    assert_eq!(status(&shell, cx), Status::NeedsYou);
    assert!(
        cx.debug_bounds("switch-dot").is_some(),
        "the dot, on the band's toggle"
    );
    // Showing the panel is seeing the answer.
    shell.update_in(cx, |s, window, cx| s.toggle_assistant(window, cx));
    settle(cx);
    assert!(
        shell.read_with(cx, |s, _| s.right_open()),
        "the panel is shown"
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_writes_its_findings_as_comments_in_one_undo_step(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-write", cx);
    let original = unstamped(&editor_text(&shell, cx));
    click_button(cx);
    click_review(cx);
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text(ONE_FINDING_AND_ONE_MISS.into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\nun peu de texte<!-- comment: Trop court. -->\n"
    );
    // The answer is not shown; the summary is the last line.
    let lines = entries(&shell, cx);
    assert_eq!(
        lines.last(),
        Some(&(
            Who::Note,
            "Wrote 1 comment. 1 finding could not be placed.".to_string()
        ))
    );
    assert!(
        !lines.iter().any(|(_, text)| text.contains("PASSAGE")),
        "{lines:?}"
    );

    // The note is saved by the usual way, with the comment in it.
    shell.update(cx, |s, cx| s.flush(cx));
    settle(cx);
    assert!(
        unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap())
            .contains("un peu de texte<!-- comment: Trop court. -->")
    );

    // One undo takes the whole review back.
    focus_note(&shell, cx);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(unstamped(&editor_text(&shell, cx)), original);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_answers_a_listed_comment_with_one_reply(cx: &mut TestAppContext) {
    let note = "# Plan\n\nun peu de texte<!-- comment @Ana 2026-11-24T10:05Z: trop court ? -->\n";
    let (root, shell, cx, _config) = note_with("assistant-reply", note, cx);
    click_button(cx);
    click_review(cx);
    // The prompt numbers the open comment, so that the answer can name it.
    let prompt = sent(&shell, cx).pop().unwrap();
    assert!(
        prompt.contains("C1 on \"un peu de texte\" — @Ana: trop court ?"),
        "{prompt}"
    );
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("REPLY: C1\nCOMMENT: Oui, on peut allonger.\n".into()),
            Event::Ended,
        ],
    );
    let text = editor_text(&shell, cx);
    assert!(
        text.contains("<!-- comment @Ana 2026-11-24T10:05Z: trop court ? ↳ @assistant "),
        "{text}"
    );
    assert!(text.contains(": Oui, on peut allonger. -->"), "{text}");
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "Wrote 1 reply.".to_string()))
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_with_no_findings_changes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-none", cx);
    let original = unstamped(&editor_text(&shell, cx));
    click_button(cx);
    click_review(cx);
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("NO FINDINGS".into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "No findings.".to_string()))
    );
    assert_eq!(unstamped(&editor_text(&shell, cx)), original);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_of_a_note_whose_tab_closed_writes_the_file(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-closed", cx);
    click_button(cx);
    click_review(cx);
    // The tab is taken out (the window keeps a new empty one in its place).
    shell.update_in(cx, |s, window, cx| {
        assert!(s.close_now(window, cx));
    });
    settle(cx);
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text(ONE_FINDING_AND_ONE_MISS.into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()),
        "# Plan\n\nun peu de texte<!-- comment: Trop court. -->\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_of_a_closed_note_keeps_a_pending_change_placed(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-closed-pending", cx);
    click_button(cx);
    click_review(cx);
    shell.update_in(cx, |s, window, cx| {
        assert!(s.close_now(window, cx));
    });
    settle(cx);
    // A change proposed under the heading waits beside the note, and is placed in its text.
    let home = crate::platform::revisions::Home::Vault(root.clone());
    let plan = root.join("Plan.md");
    crate::platform::sidecar::save(
        Some(&home),
        &plan,
        "# Plan\n{++\nnew\n++}\n\nun peu de texte\n",
        "assistant",
    )
    .unwrap();
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text(ONE_FINDING_AND_ONE_MISS.into()),
            Event::Ended,
        ],
    );
    let disk = fs::read_to_string(&plan).unwrap();
    assert_eq!(
        unstamped(&disk),
        "# Plan\n\nun peu de texte<!-- comment: Trop court. -->\n"
    );
    assert!(crate::platform::sidecar::unplaced(&home, &plan, &disk).is_empty());
    assert_eq!(crate::platform::sidecar::read(&home, &plan).len(), 1);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_refused_prompt_shows_the_error_and_the_login_hint(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-refused", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    feed(
        &shell,
        cx,
        vec![Event::Refused(
            "No API key found for anthropic. Set it first.".into(),
        )],
    );
    let lines = entries(&shell, cx);
    assert_eq!(
        lines[1],
        (
            Who::Note,
            "No API key found for anthropic. Set it first.".into()
        )
    );
    assert_eq!(lines[2].0, Who::Note);
    assert!(
        lines[2].1.starts_with("Log in with hoocode first"),
        "{lines:?}"
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_agent_that_exits_is_reported_and_a_prompt_starts_it_again(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-exit", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("one".into(), window, cx)
    });
    settle(cx);
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text("Let me ".into())],
    );
    feed(&shell, cx, vec![Event::Exited("boom".into())]);
    let lines = entries(&shell, cx);
    assert!(
        lines.contains(&(Who::Note, "The assistant stopped.".to_string())),
        "{lines:?}"
    );
    assert!(
        lines.contains(&(Who::Note, "boom".to_string())),
        "{lines:?}"
    );
    assert_ne!(status(&shell, cx), Status::Working);

    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("two".into(), window, cx)
    });
    settle(cx);
    let words = sent(&shell, cx);
    assert_eq!(words.len(), 2);
    assert!(words[1].ends_with("two"), "{}", words[1]);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn stop_during_a_review_writes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-stop-review", cx);
    let original = unstamped(&editor_text(&shell, cx));
    click_button(cx);
    click_review(cx);
    // The whole answer has arrived, but the person stops before the agent ends.
    feed(
        &shell,
        cx,
        vec![Event::Text(ONE_FINDING_AND_ONE_MISS.into())],
    );
    let stop = cx
        .debug_bounds("assistant-stop")
        .expect("Stop is drawn while the agent works")
        .center();
    cx.simulate_click(stop, gpui::Modifiers::none());
    settle(cx);
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "Stopped.".to_string()))
    );
    feed(&shell, cx, vec![Event::Ended]);
    assert_eq!(unstamped(&editor_text(&shell, cx)), original);
    assert!(!unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()).contains("{=="));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn unsaved_edits_are_on_disk_before_the_prompt_is_sent(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-save", cx);
    focus_note(&shell, cx);
    cx.simulate_input("ajout ");
    settle(cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    assert!(
        unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()).contains("ajout"),
        "the note was saved before the agent read it"
    );
    assert_eq!(sent(&shell, cx).len(), 1);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_missing_command_offers_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-missing", cx);
    shell.update(cx, |s, cx| {
        s.assistant.found = false;
        s.assistant_check(cx);
    });
    settle(cx);
    assert_eq!(status(&shell, cx), Status::Missing);
    click_button(cx);
    assert!(
        cx.debug_bounds("assistant-empty").is_some(),
        "the empty state says so"
    );
    // Enter and Review do nothing.
    cx.simulate_input("Hi");
    cx.simulate_keystrokes("enter");
    settle(cx);
    shell.update_in(cx, |s, _, cx| s.assistant_review(cx));
    settle(cx);
    assert!(sent(&shell, cx).is_empty());
    assert!(entries(&shell, cx).is_empty());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_new_vault_clears_the_transcript(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-reset", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    assert_eq!(entries(&shell, cx).len(), 1);
    shell.update(cx, |s, cx| s.assistant_reset(cx));
    settle(cx);
    assert!(entries(&shell, cx).is_empty());
    assert_eq!(status(&shell, cx), Status::Idle);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_assistant_setting_is_the_command_that_starts_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-setting", cx);
    shell.update_in(cx, |s, window, cx| {
        s.choose_setting(Setting::Assistant, window, cx)
    });
    settle(cx);
    cx.simulate_input("  mybot ");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.prefs.assistant_command().to_string()),
        "mybot"
    );
    let saved = vault::load_settings();
    assert!(saved.contains("assistant=mybot\n"), "{saved}");
    // An empty entry goes back to the default.
    shell.update_in(cx, |s, window, cx| {
        s.choose_setting(Setting::Assistant, window, cx)
    });
    settle(cx);
    // The prompt is prefilled with the command: clear it, then Enter keeps the empty entry.
    cx.simulate_keystrokes("secondary-a backspace");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.prefs.assistant_command().to_string()),
        "hoocode"
    );
    assert!(!vault::load_settings().contains("assistant="));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_panel_is_shown_again_after_a_restart(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-restart", cx);
    click_button(cx);
    assert!(
        vault::load_layout().contains("show=assistant"),
        "{}",
        vault::load_layout()
    );
    // Hidden, the file says so, and keeps the view it showed.
    click_button(cx);
    assert!(
        !vault::load_layout().contains("show="),
        "{}",
        vault::load_layout()
    );
    assert!(vault::load_layout().contains("view=assistant"));
    click_button(cx);
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown.contains(&Mode::Assistant)));
    // A restart: the panel comes back shown.
    let (shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    settle(cx);
    assert!(
        shell.read_with(cx, |s, _| s.nav.right_shown.contains(&Mode::Assistant)),
        "the panel is shown again"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_first_review_of_a_session_writes_its_comment(cx: &mut TestAppContext) {
    // No agent runs yet: the review starts one, and the review waits for its answer through it.
    let (root, shell, cx, _config) = note_open("assistant-first-review", cx);
    click_button(cx);
    click_review(cx);
    assert_eq!(sent(&shell, cx).len(), 1, "the review was sent");
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("PASSAGE: un peu de texte\nCOMMENT: Trop court.\n".into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\nun peu de texte<!-- comment: Trop court. -->\n"
    );
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "Wrote 1 comment.".to_string()))
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_reads_the_text_of_separate_messages(cx: &mut TestAppContext) {
    // The agent says a line, reads the note, then answers: the line is not glued to the finding.
    let (root, shell, cx, _config) = note_open("assistant-review-messages", cx);
    click_button(cx);
    click_review(cx);
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("Let me read the note.".into()),
            Event::Tool("Read".into()),
            Event::Break,
            Event::Text("PASSAGE: un peu de texte\nCOMMENT: Trop court.".into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\nun peu de texte<!-- comment: Trop court. -->\n"
    );
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "Wrote 1 comment.".to_string()))
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_chat_separates_the_messages_of_the_agent_by_a_paragraph(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-paragraphs", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("Let me read the note.".into()),
            Event::Tool("Read".into()),
            Event::Break,
            Event::Text("It is short.".into()),
            // A message that ends with no text after it adds no blank line.
            Event::Break,
            Event::Ended,
        ],
    );
    assert_eq!(
        entries(&shell, cx)[1..],
        [(
            Who::Agent,
            "Let me read the note.\n\nIt is short.".to_string()
        )]
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn stop_frees_the_panel_when_the_agent_does_not_end(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-stop-timer", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    feed(&shell, cx, vec![Event::Started]);
    let stop = cx
        .debug_bounds("assistant-stop")
        .expect("Stop is drawn while the agent works")
        .center();
    cx.simulate_click(stop, gpui::Modifiers::none());
    settle(cx);
    // The abort was sent, and the agent does not answer it: no Ended comes.
    let aborts = shell.read_with(cx, |s, _| {
        s.assistant
            .agent
            .as_ref()
            .map(|agent| agent.written())
            .unwrap_or_default()
    });
    assert!(
        aborts
            .iter()
            .any(|line| line.contains("\"type\":\"abort\"")),
        "{aborts:?}"
    );
    cx.executor().advance_clock(Duration::from_secs(2));
    settle(cx);
    assert_eq!(status(&shell, cx), Status::Working, "2 s: not yet");
    cx.executor().advance_clock(Duration::from_secs(1));
    settle(cx);
    assert_eq!(status(&shell, cx), Status::Idle, "3 s: the panel is free");
    assert!(shell.read_with(cx, |s, _| s.assistant.agent.is_none()));
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "Stopped.".to_string()))
    );

    // Enter works again: the next prompt starts an agent and is sent.
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.assistant.field.focus_handle(cx), cx)
    });
    cx.simulate_input("again");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(sent(&shell, cx).len(), 2);
    assert_eq!(status(&shell, cx), Status::Working);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn stop_without_an_agent_to_abort_frees_the_panel_at_once(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-stop-now", cx);
    click_button(cx);
    shell.update_in(cx, |s, window, cx| {
        s.assistant_prompt("Hi".into(), window, cx)
    });
    settle(cx);
    // The abort cannot be sent: the agent is gone while the panel still says Working.
    shell.update(cx, |s, _| s.assistant.agent = None);
    shell.update(cx, |s, cx| s.assistant_stop(cx));
    settle(cx);
    assert_eq!(status(&shell, cx), Status::Idle, "no wait: killed at once");
    let _ = fs::remove_dir_all(root);
}

/// Clicks "Improve selection" or "Improve this note", where it is drawn now.
pub(super) fn click_improve(cx: &mut gpui::VisualTestContext) {
    let at = cx
        .debug_bounds("assistant-improve")
        .expect("the improve button is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

/// Selects the bytes `range` of the note: the caret goes to its start, then Shift+Right to its end.
/// The text is ASCII in the tests that use it.
pub(super) fn select(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, range: Range<usize>) {
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.set_view(range.start, 0., cx))
    });
    focus_note(shell, cx);
    cx.simulate_keystrokes(&"shift-right ".repeat(range.len()));
    settle(cx);
}

/// Puts the caret at byte `at` of the note, and gives the note the keys.
pub(super) fn caret(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, at: usize) {
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| e.set_view(at, 0., cx))
    });
    focus_note(shell, cx);
}

/// The answer of a rewrite of "un peu de texte" into "Un peu plus clair.".
pub(super) const CLEARER: &str = "BEGIN TEXT\nUn peu plus clair.\nEND TEXT\n";

#[gpui::test]
fn improve_a_selection_sends_the_passage_and_shows_no_answer(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-selection", cx);
    let original = unstamped(&editor_text(&shell, cx));
    select(&shell, cx, 8..23);
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).selected().to_string()),
        "un peu de texte"
    );
    click_button(cx);
    click_improve(cx);

    let prompts = sent(&shell, cx);
    assert_eq!(prompts.len(), 1, "one prompt");
    assert!(
        prompts[0].contains("BEGIN PASSAGE\nun peu de texte\nEND PASSAGE"),
        "the passage is sent verbatim: {}",
        prompts[0]
    );
    assert!(
        prompts[0].contains("Improve the writing: clearer and tighter"),
        "the default instruction is sent"
    );
    assert_eq!(status(&shell, cx), Status::Working);
    let lines = entries(&shell, cx);
    assert_eq!(lines[0], (Who::You, "Improve selection".to_string()));
    assert_eq!(lines[1], (Who::Note, "Working on a proposal…".to_string()));

    // The answer is collected, not shown; the note gets it as one pending change.
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    let lines = entries(&shell, cx);
    assert!(
        !lines.iter().any(|(_, text)| text.contains("BEGIN TEXT")),
        "the answer is not shown: {lines:?}"
    );
    assert_eq!(
        lines.last(),
        Some(&(
            Who::Note,
            "1 change is pending in the note: accept or reject it there.".to_string()
        ))
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\n{--\nun peu de texte\n--}\n{++\nUn peu plus clair.\n++}\n"
    );
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).pending_changes()),
        1
    );

    // Accepting it gives the agent's text; one undo takes the whole proposal back.
    focus_note(&shell, cx);
    cx.simulate_keystrokes("secondary-enter");
    settle(cx);
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\nUn peu plus clair.\n"
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\n{--\nun peu de texte\n--}\n{++\nUn peu plus clair.\n++}\n"
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        original,
        "one undo takes the proposal back"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn improve_the_whole_note_takes_the_instruction_typed_in_the_box(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-whole", cx);
    click_button(cx);
    cx.simulate_input("Make it formal");
    settle(cx);
    click_improve(cx);

    let prompts = sent(&shell, cx);
    assert!(prompts[0].contains("the whole note"), "{}", prompts[0]);
    assert!(prompts[0].contains("Make it formal"), "{}", prompts[0]);
    assert!(
        !prompts[0].contains("BEGIN PASSAGE"),
        "no passage: the whole note"
    );
    assert_eq!(
        entries(&shell, cx)[0],
        (Who::You, "Improve this note: Make it formal".to_string())
    );
    assert_eq!(
        shell.read_with(cx, |s, cx| s.assistant.field.read(cx).line.text.clone()),
        "",
        "the box is cleared"
    );

    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text("BEGIN TEXT\n# Plan\n\nUn peu plus clair.\nEND TEXT".into()),
            Event::Ended,
        ],
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\n{--\nun peu de texte\n--}\n{++\nUn peu plus clair.\n++}\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_palette_action_improves_the_selection(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-palette", cx);
    select(&shell, cx, 8..23);
    cx.dispatch_action(AssistantImprove);
    settle(cx);
    let prompts = sent(&shell, cx);
    assert_eq!(prompts.len(), 1, "one prompt");
    // The selected passage, and only it, is in the prompt; the note is named for the context.
    assert!(
        prompts[0].contains("BEGIN PASSAGE\nun peu de texte\nEND PASSAGE"),
        "the selected passage is sent verbatim: {}",
        prompts[0]
    );
    assert!(!prompts[0].contains("the whole note"), "{}", prompts[0]);
    assert!(prompts[0].contains("the file `Plan.md`"), "{}", prompts[0]);
    assert_eq!(
        entries(&shell, cx)[0],
        (Who::You, "Improve selection".to_string())
    );
    assert_eq!(status(&shell, cx), Status::Working);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn improve_is_refused_while_the_note_has_pending_changes(cx: &mut TestAppContext) {
    let pending = "# Plan\n\n{--\nold\n--}\n{++\nnew\n++}\n";
    let (root, shell, cx, _config) = note_with("assistant-improve-pending", pending, cx);
    click_button(cx);
    click_improve(cx);
    assert!(sent(&shell, cx).is_empty(), "nothing is sent");
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(
            Who::Note,
            "Accept or reject the pending changes first.".to_string()
        ))
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    assert_eq!(unstamped(&editor_text(&shell, cx)), pending);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn stop_during_an_improvement_writes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-stop", cx);
    let original = unstamped(&editor_text(&shell, cx));
    select(&shell, cx, 8..23);
    click_button(cx);
    click_improve(cx);
    feed(&shell, cx, vec![Event::Text(CLEARER.into())]);
    let stop = cx
        .debug_bounds("assistant-stop")
        .expect("Stop is drawn while the agent works")
        .center();
    cx.simulate_click(stop, gpui::Modifiers::none());
    settle(cx);
    feed(&shell, cx, vec![Event::Ended]);
    assert_eq!(unstamped(&editor_text(&shell, cx)), original);
    assert!(
        !unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()).contains("{==")
            && !unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()).contains("{++"),
        "nothing written"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_answer_without_a_change_writes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-none", cx);
    let original = unstamped(&editor_text(&shell, cx));
    select(&shell, cx, 8..23);
    click_button(cx);
    for answer in ["NO CHANGE", "Sure, here is a better version: ..."] {
        click_improve(cx);
        feed(
            &shell,
            cx,
            vec![Event::Started, Event::Text(answer.into()), Event::Ended],
        );
        assert_eq!(
            entries(&shell, cx).last(),
            Some(&(Who::Note, "The assistant proposed no change.".to_string())),
            "for {answer:?}"
        );
        assert_eq!(
            unstamped(&editor_text(&shell, cx)),
            original,
            "for {answer:?}"
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_change_of_another_line_meanwhile_is_merged(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_with(
        "assistant-improve-merge",
        "# Plan\n\nun peu de texte\nentre les deux\nla fin\n",
        cx,
    );
    click_button(cx);
    select(&shell, cx, 8..23);
    click_improve(cx);
    // The person types at the end of the last line while the agent works.
    caret(
        &shell,
        cx,
        "# Plan\n\nun peu de texte\nentre les deux\nla fin".len(),
    );
    cx.simulate_input("!");
    settle(cx);
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan\n\n{--\nun peu de texte\n--}\n{++\nUn peu plus clair.\n++}\nentre les deux\nla fin!\n"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_change_of_the_same_line_meanwhile_is_refused(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-clash", cx);
    select(&shell, cx, 8..23);
    click_button(cx);
    click_improve(cx);
    // The person types inside the line the agent rewrites.
    caret(&shell, cx, 8);
    cx.simulate_input("x");
    settle(cx);
    let typed = unstamped(&editor_text(&shell, cx));
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(
            Who::Note,
            "You changed the same lines meanwhile, so nothing was proposed. Ask again.".to_string()
        ))
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        typed,
        "only the person's change is there"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_improvement_of_a_note_that_is_not_open_writes_its_file(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-closed", cx);
    select(&shell, cx, 8..23);
    click_button(cx);
    click_improve(cx);
    // The tab is taken out while the agent works.
    shell.update_in(cx, |s, window, cx| {
        assert!(s.close_now(window, cx));
    });
    settle(cx);
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    // The file keeps its own text; the proposal waits beside it, in `.inkuno`, until a person decides.
    assert_eq!(
        unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()),
        "# Plan\n\nun peu de texte\n"
    );
    let changes = crate::platform::sidecar::read(
        &crate::platform::revisions::Home::Vault(root.clone()),
        &root.join("Plan.md"),
    );
    assert_eq!(changes.len(), 1, "{changes:?}");
    assert_eq!(changes[0].old, "un peu de texte\n");
    assert_eq!(changes[0].new, "Un peu plus clair.\n");
    assert_eq!(changes[0].author, "assistant");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_improvement_in_a_vault_keeps_two_automatic_revisions(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-revisions", cx);
    select(&shell, cx, 8..23);
    click_button(cx);
    click_improve(cx);
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    let found = history::all(&Home::Vault(root.clone()), &root.join("Plan.md")).unwrap();
    let kinds: Vec<&str> = found.iter().map(|rev| rev.kind.word()).collect();
    assert_eq!(
        kinds,
        ["auto", "auto"],
        "the text before, then the proposal"
    );
    assert_eq!(
        found[0].text, "# Plan\n\nun peu de texte\n",
        "the text before"
    );
    // The memo is history data: the fixed English words of `inkuno write`, whatever the language.
    assert_eq!(
        found[0].note.as_deref(),
        Some("before a change by assistant")
    );
    assert_eq!(found[1].author, "assistant");
    assert_eq!(found[1].note.as_deref(), Some("proposed, pending"));
    assert_eq!(
        found[1].text,
        crate::logic::pending::accept_all(&unstamped(&editor_text(&shell, cx))),
        "the proposed text, as it would be once every change is accepted"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_improvement_after_a_title_change_lands_in_the_renamed_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-rename", cx);
    // The `# Title` line changes: the note is renamed on its save, which has not happened yet.
    focus_note(&shell, cx);
    caret(&shell, cx, 6);
    cx.simulate_input(" v2");
    settle(cx);
    assert!(root.join("Plan.md").exists(), "not saved yet");
    select(&shell, cx, 11..26);
    click_button(cx);
    click_improve(cx);

    // The save renamed the note first: the prompt names the new file, and the passage is sent.
    let prompts = sent(&shell, cx);
    assert!(
        prompts[0].contains("the file `Plan v2.md`"),
        "{}",
        prompts[0]
    );
    assert!(
        prompts[0].contains("BEGIN PASSAGE\nun peu de texte\nEND PASSAGE"),
        "{}",
        prompts[0]
    );
    assert!(root.join("Plan v2.md").exists(), "renamed on the save");
    assert!(!root.join("Plan.md").exists());

    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    assert_eq!(
        unstamped(&editor_text(&shell, cx)),
        "# Plan v2\n\n{--\nun peu de texte\n--}\n{++\nUn peu plus clair.\n++}\n",
        "the proposal is in the renamed note"
    );
    assert!(shell.read_with(cx, |s, _| {
        s.pane()
            .path
            .as_deref()
            .is_some_and(|p| p.ends_with("Plan v2.md"))
    }));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_after_a_title_change_names_the_renamed_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-rename", cx);
    focus_note(&shell, cx);
    caret(&shell, cx, 6);
    cx.simulate_input(" v2");
    settle(cx);
    assert!(root.join("Plan.md").exists(), "not saved yet");
    click_button(cx);
    click_review(cx);
    let words = sent(&shell, cx);
    assert_eq!(
        words[0],
        crate::logic::assistant::review_prompt("Plan v2.md", &[])
    );
    assert!(root.join("Plan v2.md").exists(), "renamed on the save");
    feed(
        &shell,
        cx,
        vec![Event::Text(ONE_FINDING_AND_ONE_MISS.into()), Event::Ended],
    );
    assert!(
        unstamped(&editor_text(&shell, cx)).contains("Trop court."),
        "the finding is in the renamed note: {}",
        unstamped(&editor_text(&shell, cx))
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_failed_refused_or_exited_improvement_drops_its_job_and_writes_nothing(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = note_open("assistant-improve-ends-badly", cx);
    let original = unstamped(&editor_text(&shell, cx));
    select(&shell, cx, 8..23);
    click_button(cx);
    let ends = [
        (
            "failed",
            vec![
                Event::Started,
                Event::Text(CLEARER.into()),
                Event::Failed("The model failed.".into()),
            ],
        ),
        (
            "refused",
            vec![Event::Refused("The prompt was refused.".into())],
        ),
        (
            "exited",
            vec![
                Event::Started,
                Event::Text(CLEARER.into()),
                Event::Exited("boom".into()),
            ],
        ),
    ];
    for (name, events) in ends {
        click_improve(cx);
        assert_eq!(status(&shell, cx), Status::Working, "{name}: the job runs");
        feed(&shell, cx, events);
        assert!(
            shell.read_with(cx, |s, _| s.assistant.job.is_none()),
            "{name}: the job is dropped"
        );
        // The end of the run that comes after finds no job: nothing is written.
        feed(&shell, cx, vec![Event::Ended]);
        assert_eq!(unstamped(&editor_text(&shell, cx)), original, "{name}");
        assert!(
            !unstamped(&fs::read_to_string(root.join("Plan.md")).unwrap()).contains("{++"),
            "{name}: nothing written"
        );
        assert_ne!(
            status(&shell, cx),
            Status::Working,
            "{name}: the panel is free"
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_proposal_lands_in_its_note_when_another_tab_is_shown_meanwhile(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-improve-tabs", cx);
    fs::write(root.join("Other.md"), "# Other\n\nautre\n").unwrap();
    select(&shell, cx, 8..23);
    click_button(cx);
    click_improve(cx);
    // The person opens another note while the agent works: Plan.md keeps its tab.
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("Other.md"), window, cx)
    });
    settle(cx);
    assert!(
        shell.read_with(cx, |s, _| s
            .pane()
            .path
            .as_deref()
            .is_some_and(|p| p.ends_with("Other.md"))),
        "Other.md is the shown tab"
    );

    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    let plan = pane_text(&shell, cx, "Plan.md");
    assert_eq!(
        plan, "# Plan\n\n{--\nun peu de texte\n--}\n{++\nUn peu plus clair.\n++}\n",
        "the proposal is in Plan.md"
    );
    assert_eq!(
        pane_text(&shell, cx, "Other.md"),
        "# Other\n\nautre\n",
        "the other note is untouched"
    );
    let _ = fs::remove_dir_all(root);
}

/// The text of the open tab of the note whose path ends with `name`.
fn pane_text(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, name: &str) -> String {
    shell.read_with(cx, |s, cx| {
        let pane = s
            .panes
            .iter()
            .find(|pane| pane.path.as_deref().is_some_and(|p| p.ends_with(name)))
            .expect("the note has a tab");
        pane.editor.read(cx).text().to_string()
    })
}

#[gpui::test]
fn a_prompt_that_cannot_start_the_agent_stays_in_the_box(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-start-fails", cx);
    click_button(cx);
    shell.update(cx, |s, _| {
        s.assistant.refuse_start = Some("The assistant could not start.".into())
    });
    cx.simulate_input("Summarise this note");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert!(sent(&shell, cx).is_empty(), "nothing was sent");
    assert_eq!(
        shell.read_with(cx, |s, cx| s.assistant.field.read(cx).line.text.clone()),
        "Summarise this note",
        "the typed prompt is kept"
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    assert_eq!(
        entries(&shell, cx).last(),
        Some(&(Who::Note, "The assistant could not start.".to_string()))
    );

    // Once the agent can start, the same text is sent and the box is emptied.
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(sent(&shell, cx).len(), 1);
    assert!(sent(&shell, cx)[0].ends_with("Summarise this note"));
    assert!(shell.read_with(cx, |s, cx| s.assistant.field.read(cx).line.text.is_empty()));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_improvement_that_cannot_start_leaves_no_job_and_keeps_the_instruction(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = note_open("assistant-improve-start-fails", cx);
    let original = unstamped(&editor_text(&shell, cx));
    select(&shell, cx, 8..23);
    click_button(cx);
    cx.simulate_input("Make it formal");
    settle(cx);
    shell.update(cx, |s, _| {
        s.assistant.refuse_start = Some("The assistant could not start.".into())
    });
    click_improve(cx);
    assert!(sent(&shell, cx).is_empty(), "nothing was sent");
    assert!(
        shell.read_with(cx, |s, _| s.assistant.job.is_none()),
        "no job is left behind"
    );
    assert_eq!(
        shell.read_with(cx, |s, cx| s.assistant.field.read(cx).line.text.clone()),
        "Make it formal",
        "the instruction is kept"
    );
    assert_eq!(status(&shell, cx), Status::Idle);
    assert_eq!(unstamped(&editor_text(&shell, cx)), original);
    let _ = fs::remove_dir_all(root);
}

/// Whether the note of the active tab has the keys, as the test sees the window now.
fn note_focused(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> bool {
    shell.update_in(cx, |s, window, cx| {
        s.pane().editor.focus_handle(cx).is_focused(window)
    })
}

#[gpui::test]
fn a_review_that_writes_a_comment_opens_its_card_and_keeps_the_note_focused(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = note_open("assistant-reveal-review", cx);
    click_button(cx);
    click_review(cx);
    focus_note(&shell, cx);
    assert!(note_focused(&shell, cx));
    feed(
        &shell,
        cx,
        vec![
            Event::Started,
            Event::Text(ONE_FINDING_AND_ONE_MISS.into()),
            Event::Ended,
        ],
    );
    // The new comment's card is open in the Review view, and the note still has the keys.
    let open = shell.read_with(cx, |s, _| s.review_open);
    assert!(
        matches!(open, Some(crate::shell::review::Key::Comment(_))),
        "{open:?}"
    );
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown.contains(&Mode::Review)));
    assert!(note_focused(&shell, cx), "the note keeps the keys");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_improvement_opens_its_run_in_the_review_panel_and_keeps_the_note_focused(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = note_open("assistant-reveal-improve", cx);
    select(&shell, cx, 8..23);
    cx.dispatch_action(AssistantImprove);
    settle(cx);
    focus_note(&shell, cx);
    feed(
        &shell,
        cx,
        vec![Event::Started, Event::Text(CLEARER.into()), Event::Ended],
    );
    // The run's first change is open as its card, in the Review view; no focus moved.
    let open = shell.read_with(cx, |s, _| s.review_open);
    assert!(
        matches!(open, Some(crate::shell::review::Key::Change(_))),
        "{open:?}"
    );
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown.contains(&Mode::Review)));
    assert!(note_focused(&shell, cx), "the note keeps the keys");
    let _ = fs::remove_dir_all(root);
}
