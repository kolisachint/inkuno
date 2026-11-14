//! The assistant: its round button at the note's foot, its panel in the right dock, the prompt box,
//! and the seam the process side fills in. No process is started: the tests call the seam by hand
//! and read what the panel shows.

use super::*;
use crate::shell::assistant::{Ask, Status, Who};

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

#[gpui::test]
fn the_prompt_box_sends_what_is_typed(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-prompt", cx);
    click_button(cx);
    // Enter with nothing typed does nothing.
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert!(entries(&shell, cx).is_empty());
    assert!(shell.read_with(cx, |s, _| s.assistant.sent.is_empty()));

    // Typed, then Enter: a line of the user, the box cleared, the prompt sent.
    cx.simulate_input("Summarise this note");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        entries(&shell, cx),
        [(Who::You, "Summarise this note".to_string())]
    );
    assert_eq!(
        shell.read_with(cx, |s, _| s.assistant.sent.clone()),
        ["Summarise this note"]
    );
    assert!(shell.read_with(cx, |s, cx| s.assistant.field.read(cx).line.text.is_empty()));
    assert!(cx.debug_bounds("assistant-entry-0").is_some());

    // "Review this note" sends the review, while a note is open and the agent is free.
    let review = cx
        .debug_bounds("assistant-review")
        .expect("the review button");
    cx.simulate_click(review.center(), gpui::Modifiers::none());
    settle(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.assistant.sent.clone()),
        ["Summarise this note", "Review this note"]
    );
    assert_eq!(entries(&shell, cx).len(), 2);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_seam_streams_asks_and_answers(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("assistant-seam", cx);
    click_button(cx);
    // Text streamed in pieces makes one line of the agent.
    shell.update(cx, |s, cx| {
        s.assistant_more("Hel", cx);
        s.assistant_more("lo", cx);
    });
    settle(cx);
    assert_eq!(entries(&shell, cx), [(Who::Agent, "Hello".to_string())]);
    assert!(cx.debug_bounds("assistant-entry-0").is_some());

    // A request shows Allow and Deny, and the status is NeedsYou with its dot.
    shell.update(cx, |s, cx| {
        s.assistant_asks(
            Some(Ask {
                id: "1".into(),
                text: "Edit the note?".into(),
            }),
            cx,
        )
    });
    settle(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.assistant.status),
        Status::NeedsYou
    );
    assert!(cx.debug_bounds("assistant-allow").is_some());
    assert!(cx.debug_bounds("assistant-deny").is_some());
    assert!(
        cx.debug_bounds("assistant-badge").is_some(),
        "the dot on the button"
    );

    // Allow clears the request.
    let allow = cx.debug_bounds("assistant-allow").unwrap();
    cx.simulate_click(allow.center(), gpui::Modifiers::none());
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.assistant.ask.is_none()));

    // While the agent works, Stop is offered, and the review is not.
    shell.update(cx, |s, cx| s.assistant_status(Status::Working, cx));
    settle(cx);
    assert!(cx.debug_bounds("assistant-stop").is_some());
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
