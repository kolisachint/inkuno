//! The outline of the note: its chord and its palette command show it in the right dock, beside
//! the note, and hide it again. The headings come from the note itself; the dock is the one the
//! assistant shares. No process is started.

use super::*;

/// Lets the window draw what changed (bounds are read from the last frame), then the tasks run.
fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// A note with two headings, open in a window of its own size, with the right dock closed.
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
    fs::write(
        root.join("Plan.md"),
        "# Plan\n\n## Premier\n\ntexte\n\n## Second\n",
    )
    .unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("Plan.md"), window, cx)
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    settle(cx);
    (root, shell, cx, config)
}

/// The tools shown in the right dock, in order.
fn shown(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<Mode> {
    shell.read_with(cx, |s, _| s.nav.right_shown.clone())
}

#[gpui::test]
fn the_outline_action_shows_the_outline_in_the_right_dock(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("outline-show", cx);
    assert!(shown(&shell, cx).is_empty(), "the outline waits hidden");
    assert!(!shell.read_with(cx, |s, _| s.right_open()));

    cx.dispatch_action(ToggleOutline);
    settle(cx);
    assert_eq!(shown(&shell, cx), [Mode::Outline]);
    assert!(
        cx.debug_bounds("tool-title-outline").is_some(),
        "the outline is drawn in the right dock"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_second_press_of_the_outline_chord_hides_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("outline-hide", cx);
    cx.simulate_keystrokes("secondary-shift-o");
    settle(cx);
    assert_eq!(shown(&shell, cx), [Mode::Outline]);

    cx.simulate_keystrokes("secondary-shift-o");
    settle(cx);
    assert!(shown(&shell, cx).is_empty(), "the second press hides it");
    assert!(!shell.read_with(cx, |s, _| s.right_open()));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_outline_chord_switches_the_right_dock_from_the_assistant(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("outline-switch", cx);
    cx.dispatch_action(ToggleAssistant);
    settle(cx);
    assert_eq!(shown(&shell, cx), [Mode::Assistant]);

    cx.simulate_keystrokes("secondary-shift-o");
    settle(cx);
    assert_eq!(
        shown(&shell, cx),
        [Mode::Outline],
        "the outline takes the dock in place of the assistant"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_outline_chord_leaves_the_full_window_panel(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("outline-full", cx);
    // The panel fills the window, so the note is not drawn: the keys are the panel's.
    shell.update_in(cx, |s, window, cx| {
        s.nav.panel = nav::Panel::Full;
        window.focus(&s.nav.focus, cx);
    });
    settle(cx);
    cx.simulate_keystrokes("secondary-shift-o");
    settle(cx);
    assert_eq!(shown(&shell, cx), [Mode::Outline]);
    assert!(
        shell.read_with(cx, |s, _| s.right_open()),
        "the right dock is drawn again"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_palette_lists_outline_of_the_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("outline-palette", cx);
    let listed = shell.update_in(cx, |s, window, cx| s.palette_commands(window, cx).0);
    let outline = listed
        .iter()
        .find(|command| command.action == "app::ToggleOutline")
        .expect("the palette lists the outline of the note");
    assert!(!outline.keys.is_empty(), "with its chord");
    let _ = fs::remove_dir_all(root);
}
