//! Typing, lists, autosave, the palette, recent notes, wikilinks.

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let (shell, root) = (&j.shell, &j.root);
    let text = |cx: &mut gpui::VisualTestContext| j.text(cx);
    cx.simulate_input("# Test");
    // Footer: the counter follows typing, then counts only the selection.
    let counts = |cx: &mut gpui::VisualTestContext| {
        shell.update(cx, |s, cx| s.pane().editor.update(cx, |e, _| e.counts()))
    };
    assert_eq!(counts(cx), "1 word · 6 characters");
    cx.simulate_keystrokes("shift-left shift-left");
    assert_eq!(counts(cx), "Selection: 1 word · 2 characters");
    cx.simulate_keystrokes("right");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("1. un");
    cx.simulate_keystrokes("enter");
    cx.simulate_input("deux");
    cx.simulate_keystrokes("enter enter");
    cx.simulate_input("[] tâche");
    cx.simulate_keystrokes("secondary-enter enter");
    cx.simulate_input("suite");
    cx.simulate_keystrokes("tab");
    assert_eq!(
        text(cx),
        "# Test\n1. un\n2. deux\n- [x] tâche\n    - [ ] suite"
    );
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(cx), "# Test\n1. un\n2. deux\n- [x] tâche\n- [ ] suite");

    // Autosave, named after the title.
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert_eq!(fs::read_to_string(root.join("Test.md")).unwrap(), text(cx));

    // Palette: fuzzy search then open.
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("crs");
    cx.simulate_keystrokes("enter");
    assert_eq!(text(cx), "# Courses\n\n- lait #maison\n");

    // Recent: the last opened note first, in the app as in the config.
    let recent = shell.read_with(cx, |s, _| s.recent.clone());
    assert_eq!(recent, [root.join("Courses.md"), root.join("Test.md")]);
    assert_eq!(vault::load_config(), (Some(root.clone()), recent));
    let names = shell.read_with(cx, |s, _| {
        s.by_recency()
            .iter()
            .map(|n| n.name.clone())
            .collect::<Vec<_>>()
    });
    assert_eq!(names, ["Courses", "Test", "Plan"]);

    // Palette: the text of notes is searched too (« deux » is in no name),
    // and all the words of the query must be found in it.
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("deux");
    cx.simulate_keystrokes("enter");
    assert!(text(cx).starts_with("# Test\n1. un\n2. deux"));
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("lait maison");
    cx.simulate_keystrokes("enter");
    assert_eq!(text(cx), "# Courses\n\n- lait #maison\n");

    // Wikilink completed then new note created from the palette.
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_input("[[te");
    cx.simulate_keystrokes("enter");
    assert!(text(cx).ends_with("[[Test]]"));
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("Idées");
    cx.simulate_keystrokes("enter");
    assert_eq!(text(cx), "# Idées\n\n");
    assert!(root.join("Idées.md").is_file());
    assert!(
        fs::read_to_string(root.join("Courses.md"))
            .unwrap()
            .ends_with("[[Test]]")
    );
}
