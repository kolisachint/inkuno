//! The side panel: tree, new note in a folder, divider, recent notes.

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let (shell, root) = (&j.shell, &j.root);
    let text = |cx: &mut gpui::VisualTestContext| j.text(cx);
    // Tree: Ctrl+E opens it and gives it focus; the arrows show a preview (or the tab of a note
    // already open), which does not count as an opening.
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.simulate_keystrokes("secondary-e up");
    assert_eq!(
        shell.read_with(cx, |s, _| (s.nav.mode, s.nav.panel)),
        (Mode::Tree, Panel::Split)
    );
    assert_eq!(text(cx), "# Courses\n\n- lait #maison\n[[Test]]");
    // Courses is open in a tab already: that tab is shown, without counting as an opening.
    assert!(shell.read_with(cx, |s, _| !s.pane().preview
        && s.recent[0] == root.join("Idées.md")));
    // The folder expands with Right; Enter opens the note and hands control back to the editor.
    cx.simulate_keystrokes("up right down");
    // The collapse button collapses the whole tree, then expands it entirely.
    let open = |s: &mut Shell| {
        s.fold_all();
        s.nav.open.len()
    };
    assert_eq!(shell.update(cx, |s, _| (open(s), open(s))), (0, 1));
    assert_eq!(text(cx), "# Plan\n\n[[Courses]]\n");
    cx.simulate_keystrokes("enter");
    assert!(shell.read_with(cx, |s, _| !s.pane().preview
        && s.recent[0] == root.join("Projets/Plan.md")));
    assert!(cx.update(|window, cx| {
        shell
            .read(cx)
            .pane()
            .editor
            .focus_handle(cx)
            .is_focused(window)
    }));

    // New note from the tree: filed in the selection's folder.
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("# Sous");
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert!(root.join("Projets/Sous.md").is_file());

    // Separator: it sets the width, and collapses the panel at the end of its travel.
    let drag = |cx: &mut gpui::VisualTestContext, from: f32, to: f32| {
        let at = |x: f32| point(px(x), px(300.));
        cx.simulate_mouse_down(at(from), MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_move(at(to), MouseButton::Left, gpui::Modifiers::none());
        cx.simulate_mouse_up(at(to), MouseButton::Left, gpui::Modifiers::none());
    };
    drag(cx, 302., 450.);
    assert_eq!(vault::load_layout(), "tree split 410 0\n");
    drag(cx, 452., 60.);
    assert_eq!(shell.read_with(cx, |s, _| s.nav.panel), Panel::Rail);

    // Recent notes (no chord: the palette), full screen (Ctrl+M), then Escape gives back the room and focus to the note.
    cx.dispatch_action(nav::ShowRecent);
    cx.simulate_keystrokes("secondary-m");
    assert_eq!(
        shell.read_with(cx, |s, _| (s.nav.mode, s.nav.panel)),
        (Mode::Recent, Panel::Full)
    );
    cx.simulate_keystrokes("down escape");
    assert_eq!(shell.read_with(cx, |s, _| s.nav.panel), Panel::Split);
    assert_eq!(text(cx), "# Sous");
    // Asking again for the shown mode: first focus, then collapsing to the rail.
    cx.dispatch_action(nav::ShowRecent);
    cx.dispatch_action(nav::ShowRecent);
    assert_eq!(vault::load_layout(), "recent rail 410 0\n");

    // Full screen: the selection no longer moves the note; Enter gives it its place back.
    cx.simulate_keystrokes("secondary-e secondary-m tab");
    let open = shell.read_with(cx, |s, _| s.pane().path.clone());
    assert!(shell.read_with(cx, |s, _| s.nav.panel == Panel::Full
        && s.nav.sel != s.pane().path));
    cx.simulate_keystrokes("enter");
    assert!(shell.read_with(cx, |s, _| s.nav.panel == Panel::Split
        && s.pane().path != open));
}
