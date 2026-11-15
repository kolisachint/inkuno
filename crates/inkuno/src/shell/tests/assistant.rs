//! The assistant: its round button at the note's foot, its panel in the right dock, the prompt box,
//! and the seam the process side fills in. No process is started: the tests call the seam by hand
//! and read what the panel shows.

use super::*;
use crate::platform::assistant::Event;
use crate::shell::assistant::{Status, Who};
use std::time::Duration;

/// Lets the window draw what changed (bounds are read from the last frame), then the tasks run.
fn settle(cx: &mut gpui::VisualTestContext) {
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
    let (root, shell, cx, config) = start(cx, name);
    fs::write(root.join("Plan.md"), "# Plan\n\nun peu de texte\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("Plan.md"), window, cx)
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    settle(cx);
    (root, shell, cx, config)
}

/// Clicks the round button, where it is drawn now.
fn click_button(cx: &mut gpui::VisualTestContext) {
    let at = cx
        .debug_bounds("assistant-toggle")
        .expect("the round button is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

fn entries(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<(Who, String)> {
    shell.read_with(cx, |s, _| {
        s.assistant
            .entries
            .iter()
            .map(|e| (e.who, e.text.clone()))
            .collect()
    })
}

#[gpui::test]
fn the_round_button_opens_and_shuts_the_panel(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-button", cx);
    // No counter any more; the round button sits at the note's foot, inside the note.
    assert!(cx.debug_bounds("counts").is_none(), "the counter is gone");
    let pane = cx.debug_bounds("pane-0").expect("the note is drawn");
    let button = cx
        .debug_bounds("assistant-toggle")
        .expect("the round button is drawn");
    assert_eq!(button.size, size(px(30.), px(30.)));
    assert!(
        button.right() <= pane.right() && button.bottom() <= pane.bottom(),
        "{button:?} outside {pane:?}"
    );
    assert!(
        cx.debug_bounds("assistant").is_none(),
        "the panel waits hidden"
    );

    // A click opens the panel in the right dock; another click shuts it.
    click_button(cx);
    assert!(cx.debug_bounds("assistant").is_some(), "the panel is shown");
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.right_shown.clone()),
        [Mode::Assistant]
    );
    click_button(cx);
    // Bounds of an element no longer drawn keep their last place in the test harness: the state says it.
    assert!(
        !shell.read_with(cx, |s, _| s.right_open()),
        "the panel is hidden"
    );

    // The palette's action does the same, from the keys.
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
    assert!(
        shell.read_with(cx, |s, _| s.right_open()),
        "the palette action opens it"
    );
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
    assert!(!shell.read_with(cx, |s, _| s.right_open()), "and shuts it");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn focus_mode_hides_the_round_button(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-focus", cx);
    assert!(cx.debug_bounds("assistant-toggle").is_some());
    cx.simulate_keystrokes("f9");
    settle(cx);
    // The button is not drawn in focus mode (the harness keeps old bounds, so the state is read).
    assert!(
        shell.read_with(cx, |s, _| s.focus_mode),
        "F9 gives the note the window"
    );
    cx.simulate_keystrokes("f9");
    settle(cx);
    assert!(shell.read_with(cx, |s, _| !s.focus_mode));
    assert!(cx.debug_bounds("assistant-toggle").is_some());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn opening_the_assistant_keeps_the_outline_shown(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-outline", cx);
    shell.update(cx, |s, _| s.nav.right_shown = vec![Mode::Outline]);
    settle(cx);
    click_button(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.right_shown.clone()),
        [Mode::Outline, Mode::Assistant],
        "the assistant stacks under the outline"
    );
    assert!(
        cx.debug_bounds("tool-title-outline").is_some(),
        "the outline is still drawn"
    );
    assert!(cx.debug_bounds("assistant").is_some());
    let _ = fs::remove_dir_all(root);
}

/// Feeds the agent's events to the panel, as its reader thread's signal does.
fn feed(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, events: Vec<Event>) {
    shell.update_in(cx, |s, window, cx| s.assistant_events(events, window, cx));
    settle(cx);
}

fn status(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Status {
    shell.read_with(cx, |s, _| s.assistant.status)
}

/// The prompts the agent was sent, in order.
fn sent(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
    shell.read_with(cx, |s, _| s.assistant.sent.clone())
}

/// The text of the note in the active tab.
fn editor_text(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
}

/// Gives the keys to the note, as a click in it would.
fn focus_note(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.pane().editor.focus_handle(cx))
    });
    settle(cx);
}

/// Clicks "Review this note", where it is drawn now.
fn click_review(cx: &mut gpui::VisualTestContext) {
    let at = cx
        .debug_bounds("assistant-review")
        .expect("the review button is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

/// The answer of a review that finds one comment, and one passage that is not in the note.
const ONE_FINDING_AND_ONE_MISS: &str = "PASSAGE: un peu de texte\nCOMMENT: Trop court.\n\
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
    assert_eq!(words[1], crate::logic::assistant::review_prompt("Plan.md"));
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
    assert!(cx.debug_bounds("assistant-badge").is_some(), "the dot");
    // Showing the panel is seeing the answer.
    click_button(cx);
    assert_eq!(status(&shell, cx), Status::Idle);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_writes_its_findings_as_comments_in_one_undo_step(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-write", cx);
    let original = editor_text(&shell, cx);
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
        editor_text(&shell, cx),
        "# Plan\n\n{==un peu de texte==}{>>Trop court.<<}\n"
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
        fs::read_to_string(root.join("Plan.md"))
            .unwrap()
            .contains("{==un peu de texte==}{>>Trop court.<<}")
    );

    // One undo takes the whole review back.
    focus_note(&shell, cx);
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(editor_text(&shell, cx), original);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_review_with_no_findings_changes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-review-none", cx);
    let original = editor_text(&shell, cx);
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
    assert_eq!(editor_text(&shell, cx), original);
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
        fs::read_to_string(root.join("Plan.md")).unwrap(),
        "# Plan\n\n{==un peu de texte==}{>>Trop court.<<}\n"
    );
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
    let original = editor_text(&shell, cx);
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
    assert_eq!(editor_text(&shell, cx), original);
    assert!(
        !fs::read_to_string(root.join("Plan.md"))
            .unwrap()
            .contains("{==")
    );
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
        fs::read_to_string(root.join("Plan.md"))
            .unwrap()
            .contains("ajout"),
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
        vault::load_layout().contains("assistant"),
        "{}",
        vault::load_layout()
    );
    // Back to the stock layout: the panel is hidden again, and the file says so.
    click_button(cx);
    assert!(
        !vault::load_layout().contains("assistant"),
        "{}",
        vault::load_layout()
    );
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
        editor_text(&shell, cx),
        "# Plan\n\n{==un peu de texte==}{>>Trop court.<<}\n"
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
        editor_text(&shell, cx),
        "# Plan\n\n{==un peu de texte==}{>>Trop court.<<}\n"
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
        window.focus(&s.assistant.field.focus_handle(cx))
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
