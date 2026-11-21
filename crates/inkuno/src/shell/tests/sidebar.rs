//! The left sidebar (P2, files only): it never enters Versions, Assistant or Outline, which live in
//! the right dock; its Search row opens the palette as Ctrl+P does; its Scratch section lists the
//! scratch notes. No process is started.

use super::*;

/// Lets the window draw what changed (bounds are read from the last frame), then the tasks run.
fn settle(cx: &mut gpui::VisualTestContext) {
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
}

/// The tools of the left dock, in order.
fn left(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<Mode> {
    shell.read_with(cx, |s, _| s.nav.left_shown.clone())
}

/// The tools shown in the right dock, in order.
fn right(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<Mode> {
    shell.read_with(cx, |s, _| s.nav.right_shown.clone())
}

#[gpui::test]
fn the_left_sidebar_never_enters_versions_assistant_or_outline(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "sidebar-left-banned");
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Tree, false, window, cx)
    });
    settle(cx);
    assert_eq!(left(&shell, cx), [Mode::Tree]);
    // Asked for by the keys or the palette, each one shows in the right dock, never on the left.
    for mode in [Mode::Versions, Mode::Assistant, Mode::Outline] {
        shell.update_in(cx, |s, window, cx| s.show_nav(mode, false, window, cx));
        settle(cx);
        assert!(!left(&shell, cx).contains(&mode), "{mode:?} on the left");
        assert!(right(&shell, cx).contains(&mode), "{mode:?} not in the right dock");
    }
    cx.dispatch_action(ShowVersions);
    settle(cx);
    assert_eq!(left(&shell, cx), [Mode::Tree]);
    assert!(right(&shell, cx).contains(&Mode::Versions));
    // Stacked with the shift-click path too: the left dock keeps the tree alone.
    shell.update_in(cx, |s, window, cx| {
        s.stack_tool(Mode::Versions, window, cx)
    });
    settle(cx);
    assert_eq!(left(&shell, cx), [Mode::Tree]);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_search_row_opens_the_palette_as_ctrl_p_does(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "sidebar-search-row");
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Tree, false, window, cx)
    });
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    let at = cx
        .debug_bounds("nav-search")
        .expect("the search row is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.palette.is_some()));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_scratch_section_lists_a_new_scratch_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "sidebar-scratch-section");
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Tree, false, window, cx)
    });
    cx.simulate_resize(size(px(1000.), px(720.)));
    settle(cx);
    // Ctrl+N makes a scratch note; its text is written as it is typed.
    cx.simulate_keystrokes("secondary-n");
    cx.simulate_input("# Brouillon barre");
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    // The section is a part of the left sidebar, and it lists the note.
    assert!(shell.read_with(cx, |s, _| s.docked(Mode::Scratch)));
    let names: Vec<String> = shell.read_with(cx, |s, _| {
        s.scratch_rows().into_iter().map(|r| r.name).collect()
    });
    assert_eq!(names.len(), 1, "{names:?}");
    settle(cx);
    assert!(
        cx.debug_bounds(leak(format!("nav-row-{}", names[0]))).is_some(),
        "the scratch note is drawn in the sidebar"
    );
    // Folded, the section keeps its header and is no longer open. (GPUI's debug bounds keep the last
    // drawn rows of a frame, so an element that stops being drawn cannot be read as absent: the fold
    // state is what is checked.)
    shell.update_in(cx, |s, window, cx| s.hide_tool(Mode::Scratch, window, cx));
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.nav.folded.contains(&Mode::Scratch)));
    assert!(cx.debug_bounds("tool-fold-scratch").is_some(), "the header stays");
    assert!(shell.read_with(cx, |s, _| s.docked(Mode::Scratch)));
    let _ = fs::remove_dir_all(root);
}

/// The sidebar in Quiet, the tree open, a note in the vault and the window of a usual size.
fn quiet_sidebar<'a>(
    cx: &'a mut TestAppContext,
    name: &str,
) -> (
    PathBuf,
    Entity<Shell>,
    &'a mut gpui::VisualTestContext,
    std::sync::MutexGuard<'static, ()>,
) {
    let (root, shell, cx, config) = start(cx, name);
    fs::write(root.join("Note.md"), "# Note\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.prefs.theme = "Quiet Light".into();
        s.restyle(window, cx);
        s.show_nav(Mode::Tree, false, window, cx);
    });
    cx.simulate_resize(size(px(1000.), px(720.)));
    settle(cx);
    (root, shell, cx, config)
}

#[gpui::test]
fn quiet_lists_search_today_notes_tree_scratch_and_settings_from_the_top(cx: &mut TestAppContext) {
    let (root, _shell, cx, _config) = quiet_sidebar(cx, "sidebar-quiet-order");
    let top = |cx: &mut gpui::VisualTestContext, id: &'static str| {
        cx.debug_bounds(id)
            .unwrap_or_else(|| panic!("{id} is drawn"))
    };
    let ids = [
        "nav-search",
        "nav-today",
        "nav-notes",
        "nav-row-Note",
        "tool-fold-scratch",
        "nav-settings",
    ];
    let rows: Vec<_> = ids.iter().map(|id| top(cx, id)).collect();
    assert!(
        rows.windows(2).all(|w| w[0].origin.y < w[1].origin.y),
        "the sidebar's order from the top: {rows:?}"
    );
    // The rows of the sidebar are 30 px high; the search row is one of them.
    assert_eq!(rows[0].size.height, px(30.));
    assert_eq!(rows[5].size.height, px(30.));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn quiet_scratch_header_folds_its_section(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = quiet_sidebar(cx, "sidebar-quiet-fold");
    let at = cx
        .debug_bounds("tool-fold-scratch")
        .expect("the scratch header is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.nav.folded.contains(&Mode::Scratch)));
    // Folded, the section is shorter: its header has moved down, so it is found again.
    let at = cx
        .debug_bounds("tool-fold-scratch")
        .expect("the scratch header is still drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    assert!(shell.read_with(cx, |s, _| !s.nav.folded.contains(&Mode::Scratch)));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn quiet_today_row_opens_the_daily_note(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = quiet_sidebar(cx, "sidebar-quiet-today");
    let at = cx
        .debug_bounds("nav-today")
        .expect("the today row is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    // The daily note is named after the date, as the empty page's Today card names it.
    let path = shell.read_with(cx, |s, _| s.pane().path.clone()).expect("a note is open");
    let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
    assert!(path.starts_with(&root), "{path:?}");
    assert_eq!(stem.len(), 10, "{stem}");
    assert_eq!(stem.matches('-').count(), 2, "{stem}");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn quiet_settings_row_opens_the_settings_tab(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = quiet_sidebar(cx, "sidebar-quiet-settings");
    let at = cx
        .debug_bounds("nav-settings")
        .expect("the settings row is drawn")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
    // The Settings tab opens (or is found again), as the settings chord does.
    assert!(shell.read_with(cx, |s, _| s.panes.iter().any(|pane| pane.settings.is_some())));
    let _ = fs::remove_dir_all(root);
}
