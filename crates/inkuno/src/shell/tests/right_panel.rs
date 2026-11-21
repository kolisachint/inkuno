//! The right panel: one panel with four views (Outline, Comments, Versions, Assistant). Its switcher
//! shows one view at a time, Ctrl+Alt+1 to 4 pick them, the band's toggle shows and hides the panel,
//! and the Versions view saves a version. No process is started.

use super::*;
use super::versions as vers;

/// Lets the window draw what changed (bounds are read from the last frame), then the tasks run.
fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// A note with a comment and two headings, open in a window with the window's own buttons drawn.
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
        "# Plan\n\n## Premier\n\nun mot<!-- comment: à revoir --> ici\n\n## Second\n",
    )
    .unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("Plan.md"), window, cx)
    });
    shell.update(cx, |s, _| s.client = true);
    cx.simulate_resize(size(px(1400.), px(900.)));
    settle(cx);
    (root, shell, cx, config)
}

fn shown(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<Mode> {
    shell.read_with(cx, |s, _| s.nav.right_shown.clone())
}

fn click(cx: &mut gpui::VisualTestContext, id: &'static str) {
    let at = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("{id} is not drawn"))
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

#[gpui::test]
fn the_switcher_shows_one_view_at_a_time(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("panel-switch", cx);
    // Closed to start; the switcher is in the panel, so the first view is opened by its key.
    cx.simulate_keystrokes("secondary-alt-1");
    settle(cx);
    assert_eq!(shown(&shell, cx), [Mode::Outline]);
    for (switch, view, head) in [
        ("switch-comments", Mode::Comments, "view-head-comments"),
        ("switch-versions", Mode::Versions, "view-head-versions"),
        ("switch-assistant", Mode::Assistant, "view-head-assistant"),
        ("switch-outline", Mode::Outline, "view-head-outline"),
    ] {
        click(cx, switch);
        assert_eq!(shown(&shell, cx), [view], "{switch}");
        assert!(cx.debug_bounds(head).is_some(), "{head} is drawn");
    }
    // The band's toggle shuts the panel; again it opens on the view it showed.
    click(cx, "right-panel-toggle");
    assert!(shown(&shell, cx).is_empty(), "the toggle hides the panel");
    click(cx, "right-panel-toggle");
    assert_eq!(shown(&shell, cx), [Mode::Outline]);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn ctrl_alt_digits_pick_the_four_views(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("panel-keys", cx);
    for (keys, view) in [
        ("secondary-alt-1", Mode::Outline),
        ("secondary-alt-2", Mode::Comments),
        ("secondary-alt-3", Mode::Versions),
        ("secondary-alt-4", Mode::Assistant),
    ] {
        cx.simulate_keystrokes(keys);
        settle(cx);
        assert_eq!(shown(&shell, cx), [view], "{keys}");
    }
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_comments_view_lists_the_cards(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("panel-cards", cx);
    cx.simulate_keystrokes("secondary-alt-2");
    settle(cx);
    assert!(cx.debug_bounds("comment-0").is_some(), "the card is drawn");
    assert!(cx.debug_bounds("comment-1").is_none(), "one comment, one card");
    assert!(shell.read_with(cx, |s, _| s.comment_cards.len() == 1));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_versions_view_saves_a_version(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("panel-save", cx);
    cx.simulate_keystrokes("secondary-alt-3");
    settle(cx);
    click(cx, "version-save");
    // The same prompt as Ctrl+Alt+S: a name, then Enter.
    cx.simulate_input("depuis le panneau");
    cx.simulate_keystrokes("enter");
    settle(cx);
    let names = vers::names(&shell, cx);
    assert!(
        names.iter().any(|n| n.contains("depuis le panneau")),
        "{names:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_palette_has_a_command_for_each_view(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("panel-palette", cx);
    let listed = shell.update_in(cx, |s, window, cx| s.palette_commands(window, cx).0);
    for action in [
        "app::ToggleOutline",
        "app::ShowCommentsView",
        "app::ShowVersions",
        "app::ToggleAssistant",
        "app::ToggleRightPanel",
    ] {
        assert!(
            listed.iter().any(|c| c.action == action),
            "{action} is in the palette"
        );
    }
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_switcher_is_clear_of_the_window_buttons_at_any_panel_width(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = note_open("panel-minimum", cx);
    for width in [160., 280., 420.] {
        shell.update(cx, |s, _| {
            s.nav.right_w = px(width);
            s.nav.right_shown = vec![Mode::Comments];
            s.nav.right_view = Mode::Comments;
        });
        cx.simulate_resize(size(px(900.), px(700.)));
        settle(cx);
        let dock = cx.debug_bounds("dock-right").expect("the panel is drawn");
        let pill = cx.debug_bounds("window-pill").expect("the window buttons are drawn");
        for id in ["switch-outline", "switch-comments", "switch-versions", "switch-assistant"] {
            let b = cx.debug_bounds(id).unwrap_or_else(|| panic!("{width}: {id} absent"));
            assert!(
                b.left() >= dock.left() && b.right() <= dock.right(),
                "{width}: {id} {b:?} outside {dock:?}"
            );
            assert!(b.top() >= pill.bottom(), "{width}: {id} {b:?} under the window buttons {pill:?}");
        }
    }
    let _ = fs::remove_dir_all(root);
}
