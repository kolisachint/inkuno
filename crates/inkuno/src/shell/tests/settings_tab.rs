//! The Settings tab: one per window, reached by Ctrl+, and F1; its pages write the preferences
//! the palette's choices write, and it never takes a note's text.

use super::*;
use crate::shell::settings_tab::Section;

#[gpui::test]
fn ctrl_comma_opens_one_settings_tab_and_again_focuses_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "settings-tab");
    fs::write(root.join("Note.md"), "# Note\n\ntexte\n").unwrap();
    shell.update(cx, |s, cx| {
        s.rescan(&root, cx);
        s.open_note(&root.join("Note.md"), cx);
    });
    cx.run_until_parked();
    let tabs = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, _| s.panes.iter().filter(|p| p.settings.is_some()).count())
    };
    cx.simulate_keystrokes("secondary-,");
    cx.run_until_parked();
    assert_eq!(tabs(cx), 1);
    assert_eq!(
        shell.read_with(cx, |s, _| s.settings_section()),
        Some(Section::Appearance)
    );
    // The tab is named by what it is, has no file, and is not the one the note shows.
    let (name, path) = shell.read_with(cx, |s, cx| {
        let tab = s.panes.iter().position(|p| p.settings.is_some()).unwrap();
        (s.tab_name(tab, cx), s.panes[tab].path.clone())
    });
    assert_eq!((name.as_str(), path), ("Settings", None));
    // Back to the note, then Ctrl+, again: the same tab comes to the front.
    shell.update_in(cx, |s, window, cx| s.focus_pane(0, window, cx));
    cx.simulate_keystrokes("secondary-,");
    cx.run_until_parked();
    assert_eq!(tabs(cx), 1);
    assert!(shell.read_with(cx, |s, _| s.pane().settings.is_some()));
    // Typing on the page never reaches a note.
    cx.simulate_input("abc");
    assert_eq!(
        shell.read_with(cx, |s, cx| s.panes[0].editor.read(cx).text().to_string()),
        "# Note\n\ntexte\n"
    );
}

#[gpui::test]
fn text_size_and_theme_on_the_settings_page_write_the_preferences(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "settings-tab-prefs");
    fs::write(root.join("Note.md"), "# Note\n\ntexte\n").unwrap();
    shell.update(cx, |s, cx| {
        s.rescan(&root, cx);
        s.open_note(&root.join("Note.md"), cx);
    });
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-,");
    cx.run_until_parked();
    // Text size 16: the preference, the note's size, and the file.
    let at = cx
        .debug_bounds("size-2")
        .expect("the 16 choice of Text size")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(shell.read_with(cx, |s, _| (s.prefs.size, s.theme.size)), (16., 16.));
    assert!(vault::load_settings().contains("size=16\n"));
    // Theme: Dark is Quiet Dark, System goes back to the default.
    let at = cx.debug_bounds("theme-2").expect("the Dark choice").center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, _| (s.prefs.theme.clone(), s.theme.bg)),
        ("Quiet Dark".to_string(), Hsla::from(gpui::rgb(0x1c1c1e)))
    );
    let at = cx.debug_bounds("theme-0").expect("the System choice").center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(shell.read_with(cx, |s, _| s.prefs.theme.clone()), "");
    // Page width: Full is the key the palette's list writes.
    let at = cx.debug_bounds("width-3").expect("the Full width choice").center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(shell.read_with(cx, |s, _| s.prefs.page.clone()), "full");
    // F1 opens the same tab on its shortcuts page, with the search empty.
    cx.simulate_keystrokes("f1");
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, _| s.settings_section()),
        Some(Section::Shortcuts)
    );
    cx.simulate_input("comment");
    let found = shell.read_with(cx, |s, cx| s.help_rows(cx).len());
    assert!(found > 0 && found < 200, "{found}");
}
