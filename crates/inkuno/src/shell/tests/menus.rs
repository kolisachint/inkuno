//! Menus, the palette, the links the pickers write, and the selection toolbar (P5).

use super::*;
use crate::views::nav::Do;
use gpui::{Bounds, Hsla, Modifiers, Pixels, rgb};

fn bounds_of(cx: &mut gpui::VisualTestContext, name: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(name)
        .unwrap_or_else(|| panic!("{name} absent"))
}

/// A note in a folder: `[[` writes a link to a note by its path from that folder, and `@` a day
/// as `[title](path)` too, its file at the vault's root as the daily note is.
#[gpui::test]
fn the_pickers_write_relative_links(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "menus-links");
    fs::create_dir_all(root.join("Idées")).unwrap();
    let note = root.join("Idées").join("Note.md");
    fs::write(&note, "# Note\n").unwrap();
    fs::write(root.join("Plan.md"), "# Plan\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| s.open(&note, window, cx));
    cx.run_until_parked();
    let text = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
    };
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.load("# Note\n\n".into(), 9, cx))
    });
    shell.update_in(cx, |s, window, cx| {
        let focus = s.pane().editor.focus_handle(cx);
        window.focus(&focus);
    });
    cx.run_until_parked();
    cx.simulate_input("[[Pla");
    cx.simulate_keystrokes("enter");
    assert_eq!(text(cx), "# Note\n\n[Plan](../Plan.md)");
    cx.simulate_input(" @2026-11-02");
    cx.simulate_keystrokes("enter");
    assert_eq!(
        text(cx),
        "# Note\n\n[Plan](../Plan.md) [2026-11-02](../2026-11-02.md)"
    );
}

/// A selection shows the toolbar; Bold wraps the selection, and the toolbar stays for the next step.
#[gpui::test]
fn a_selection_shows_the_toolbar_and_bold_wraps_it(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "menus-toolbar");
    let note = root.join("Voir.md");
    fs::write(&note, "# Voir\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| s.open(&note, window, cx));
    cx.run_until_parked();
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.load("voir deux ici".into(), 0, cx))
    });
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| {
        let focus = s.pane().editor.focus_handle(cx);
        window.focus(&focus);
    });
    cx.simulate_keystrokes(
        "right right right right right shift-right shift-right shift-right shift-right",
    );
    cx.run_until_parked();
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let bold = bounds_of(cx, "toolbar-tb-bold");
    cx.simulate_click(bold.center(), Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string()),
        "voir **deux** ici"
    );
}

/// A full-text match is two lines in Quiet: the name, then the line where the word is.
#[gpui::test]
fn a_palette_match_shows_its_line(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "menus-palette");
    fs::write(
        root.join("Zèbre.md"),
        "# Zèbre\n\nLa girafe mange des feuilles\n",
    )
    .unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("girafe");
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
    let row = bounds_of(cx, "palette-item-0");
    assert!(
        row.size.height > px(40.),
        "a snippet line under the name: {row:?}"
    );
    assert!(cx.debug_bounds("palette-panel").is_some());
}

/// Deleting is the caution red in Quiet (the accent for the trash in the other themes); other entries
/// keep the menu's text colour.
#[gpui::test]
fn destructive_entries_are_red_in_quiet(cx: &mut TestAppContext) {
    let (_root, shell, cx, _config) = start(cx, "menus-red");
    let t = shell.read_with(cx, |s, _| s.theme);
    assert!(t.quiet, "the default theme is Quiet");
    let caution = if t.bg.l > 0.5 { 0xc93a3a } else { 0xff7b72 };
    let red: Hsla = rgb(caution).into();
    for what in [Do::Trash, Do::DeleteScratch] {
        assert_eq!(Shell::ink_of(&what, t), Some(red));
    }
    assert_eq!(Shell::ink_of(&Do::Rename, t), None);
    assert_ne!(red, t.accent, "not the blue of the accent");
}

/// The palette's "Create «…»" row is always the last one, under the notes and commands that
/// match: typing "trash" finds the restore command first, and Enter on the row is still there.
#[gpui::test]
fn the_create_row_is_always_last(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "menus-create-last");
    let note = root.join("Idées.md");
    fs::write(&note, "# Idées\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| s.open(&note, window, cx));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("trash");
    cx.run_until_parked();
    let (last, listed) = shell.read_with(cx, |s, cx| {
        let palette = s.palette.as_ref().expect("the palette is open").read(cx);
        (palette.create_is_last(), palette.listed())
    });
    assert_eq!(last, Some(true), "the create row is offered and is last");
    assert!(listed > 1, "the commands matching \"trash\" come before it");
}
