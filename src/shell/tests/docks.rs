//! Docks: the side panel's tools on the left or the right of the panes.

use super::*;
use crate::actions::{MoveTool, Outline};

#[gpui::test]
fn tools_move_between_docks(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "docks");
    let note = |name: &str| root.join(format!("{name}.md"));
    fs::write(note("Alpha"), "# Alpha\n\n[[Beta]]\n").unwrap();
    fs::write(note("Beta"), "# Beta\n\n## Section\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    cx.simulate_resize(size(px(1600.), px(900.)));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&note("Alpha"), window, cx);
        s.open_from_nav(&note("Beta"), window, cx);
        s.show_nav(Mode::Recent, false, window, cx);
    });
    cx.run_until_parked();
    let places = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, _| {
            (
                s.nav.right.clone(),
                if s.left_open() {
                    s.dock_tools(false)
                } else {
                    vec![]
                },
                if s.right_open() {
                    s.dock_tools(true)
                } else {
                    vec![]
                },
            )
        })
    };
    let bounds = |cx: &mut gpui::VisualTestContext, name: &str| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.debug_bounds(leak(name.to_string()))
            .unwrap_or_else(|| panic!("{name}"))
    };

    // The recent notes go to the right dock; the left dock shows the tree in their place.
    cx.dispatch_action(MoveTool);
    cx.run_until_parked();
    assert_eq!(
        places(cx),
        (
            vec![Mode::Outline, Mode::Recent],
            vec![Mode::Tree],
            vec![Mode::Recent]
        )
    );
    let (left, pane, right) = (
        bounds(cx, "nav-panel"),
        bounds(cx, "pane-0"),
        bounds(cx, "dock-right"),
    );
    assert!(
        left.right() <= pane.left() + px(0.5)
            && pane.right() <= right.left() + px(0.5)
            && right.right() <= px(1600.5),
        "{left:?} {pane:?} {right:?}"
    );
    // No line between the zones: each resize handle is the 8 px of ground between its dock and the note, found
    // by the pointer though nothing is drawn there at rest.
    let (grip, dock_grip) = (bounds(cx, "nav-divider"), bounds(cx, "dock-divider"));
    assert_eq!(
        (grip.size.width, grip.left(), grip.right()),
        (px(8.), left.right(), pane.left()),
        "{grip:?} {left:?}"
    );
    assert_eq!(
        (dock_grip.size.width, dock_grip.left(), dock_grip.right()),
        (px(8.), pane.right(), right.left()),
        "{dock_grip:?} {right:?}"
    );
    // Both lists are drawn at once, each from its own rows: a click in the right one opens the
    // note, and the right dock is the one the keys reach afterwards.
    let alpha = bounds(cx, "nav-row-Alpha");
    assert!(
        alpha.left() >= right.left(),
        "the recent note is in the right dock: {alpha:?}"
    );
    cx.simulate_click(alpha.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, _| (s.pane().path.clone(), s.nav.mode)),
        (Some(note("Alpha")), Mode::Recent)
    );

    // A narrow window: the right dock hides before the panes get under their minimum, and comes
    // back with the room.
    cx.simulate_resize(size(px(700.), px(720.)));
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.right_width()) >= px(160.));
    let (pane, right) = (bounds(cx, "pane-0"), bounds(cx, "dock-right"));
    assert!(
        pane.size.width >= pane::MIN_PANE - px(1.) && pane.right() <= right.left() + px(0.5),
        "{pane:?} {right:?}"
    );
    cx.simulate_resize(size(px(480.), px(720.)));
    cx.run_until_parked();
    assert_eq!(shell.read_with(cx, |s, _| s.right_width()), px(0.));
    cx.simulate_resize(size(px(1600.), px(900.)));
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.right_width()) >= px(160.));

    // Back to the left: the right dock, empty, closes.
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Recent, false, window, cx)
    });
    cx.dispatch_action(MoveTool);
    cx.run_until_parked();
    assert_eq!(
        places(cx),
        (vec![Mode::Outline], vec![Mode::Tree, Mode::Recent], vec![])
    );
}

#[gpui::test]
fn tools_stacked_folded_and_dragged(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "docks-stack");
    let note = |name: &str| root.join(format!("{name}.md"));
    fs::write(note("Alpha"), "# Alpha\n\n[[Beta]] #projet\n").unwrap();
    fs::write(note("Beta"), "# Beta\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    cx.simulate_resize(size(px(1600.), px(720.)));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&note("Beta"), window, cx);
        s.show_nav(Mode::Tree, false, window, cx);
    });
    let bounds = |cx: &mut gpui::VisualTestContext, name: &str| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.debug_bounds(leak(name.to_string()))
            .unwrap_or_else(|| panic!("{name}"))
    };
    let tools = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, _| {
            (
                s.dock_tools(false),
                if s.right_open() {
                    s.dock_tools(true)
                } else {
                    vec![]
                },
            )
        })
    };
    let none = gpui::Modifiers::none();
    // Stacking through the dock gate (the rail shows the tree only): the tools of the left dock.
    for mode in [Mode::Recent, Mode::Search] {
        shell.update_in(cx, |s, window, cx| s.stack_tool(mode, window, cx));
        cx.run_until_parked();
    }
    assert_eq!(tools(cx).0, [Mode::Tree, Mode::Recent, Mode::Search]);
    // Three tools in 720 px: each keeps its header, one under the other, none over the next.
    let mut above = px(0.);
    for mode in tools(cx).0 {
        let b = bounds(cx, &format!("tool-{}", mode.name()));
        assert!(
            b.top() >= above - px(0.5) && b.size.height >= px(44.) - px(0.5),
            "{mode:?} {b:?}"
        );
        above = b.bottom();
    }
    assert!(above <= px(720.5), "the stack fits the window: {above:?}");
    // Folded, a tool keeps only its header and leaves its room to the others.
    let recent = bounds(cx, "tool-recent").size.height;
    let at = bounds(cx, "tool-fold-recent").center();
    cx.simulate_click(at, none);
    let folded = bounds(cx, "tool-recent").size.height;
    assert!(
        (folded - px(32.)).abs() < px(2.) && folded < recent,
        "{folded:?} {recent:?}"
    );
    // No cross in the header anymore: Shift+click on the rail (the stack gate) hides a tool
    // from the stack again, the palette and the keys do the same.
    shell.update_in(cx, |s, window, cx| s.stack_tool(Mode::Recent, window, cx));
    cx.run_until_parked();
    assert_eq!(tools(cx).0, [Mode::Tree, Mode::Search]);

    // Its title dragged onto the right edge, with the right dock closed: a strip takes it there.
    let hold = |cx: &mut gpui::VisualTestContext, title: &str, to: Point<Pixels>| {
        let from = bounds(cx, title).center();
        cx.simulate_mouse_down(from, MouseButton::Left, none);
        cx.simulate_mouse_move(from + point(px(12.), px(2.)), MouseButton::Left, none);
        cx.simulate_mouse_move(to, MouseButton::Left, none);
        cx.run_until_parked();
    };
    hold(cx, "tool-title-search", point(px(1000.), px(300.)));
    let strip = bounds(cx, "dock-right-drop");
    cx.simulate_mouse_move(strip.center(), MouseButton::Left, none);
    assert_eq!(shell.read_with(cx, |s, _| s.tool_aim), Some((true, 0)));
    cx.simulate_mouse_up(strip.center(), MouseButton::Left, none);
    assert_eq!(tools(cx), (vec![Mode::Tree], vec![Mode::Search]));
    // Over the upper half of the right dock's only tool: above it, a line says so.
    let search = bounds(cx, "tool-search");
    let top = point(search.center().x, search.top() + px(60.));
    hold(cx, "tool-title-recent", top);
    assert_eq!(shell.read_with(cx, |s, _| s.tool_aim), Some((true, 0)));
    assert!(bounds(cx, "tool-aim").top() <= search.top() + px(1.));
    cx.simulate_mouse_up(top, MouseButton::Left, none);
    assert_eq!(
        tools(cx),
        (vec![Mode::Tree], vec![Mode::Recent, Mode::Search])
    );
    // Within its own dock: the recent notes go under the search.
    let search = bounds(cx, "tool-search");
    let below = point(search.center().x, search.bottom() - px(10.));
    hold(cx, "tool-title-recent", below);
    cx.simulate_mouse_up(below, MouseButton::Left, none);
    assert_eq!(tools(cx).1, [Mode::Search, Mode::Recent]);
    // Escape halfway: nothing moves.
    hold(cx, "tool-title-recent", below);
    cx.simulate_keystrokes("escape");
    cx.simulate_mouse_up(below, MouseButton::Left, none);
    assert_eq!(tools(cx).1, [Mode::Search, Mode::Recent]);

    // Restart: both stacks, in their order, the folded tool folded.
    let (again, cx2) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx2.run_until_parked();
    let back = again.read_with(cx2, |s, _| {
        (
            s.dock_tools(false),
            s.dock_tools(true),
            s.nav.folded.contains(&Mode::Recent),
        )
    });
    assert_eq!(
        back,
        (vec![Mode::Tree], vec![Mode::Search, Mode::Recent], true)
    );

    // Showing one tool: that tool alone in its dock, as the panel always did.
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Outline, true, window, cx)
    });
    cx.run_until_parked();
    assert_eq!(tools(cx), (vec![Mode::Tree], vec![Mode::Outline]));
}

#[gpui::test]
fn the_layout_file_old_new_and_odd(cx: &mut TestAppContext) {
    let (root, _shell, cx, _config) = start(cx, "docks-file");
    let read = |cx: &mut gpui::VisualTestContext, text: &str| {
        vault::save_layout(text);
        let (shell, cx2) =
            cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
        cx2.run_until_parked();
        shell.read_with(cx2, |s, _| {
            (
                s.nav.left_shown.clone(),
                s.nav.right.clone(),
                s.nav.right_shown.clone(),
            )
        })
    };
    use Mode::*;
    // Before docks: everything on the left; the outline waits in the right dock, hidden.
    assert_eq!(
        read(cx, "recent split 300 0\n"),
        (vec![Recent], vec![Outline], vec![])
    );
    // A tool on both sides keeps the left; an unknown one is dropped; one shown on the right that
    // does not live there is not shown.
    assert_eq!(
        read(
            cx,
            "search split 300 0 right=recent,bogus,search,recent show=tree width=250\n"
        ),
        (vec![Search], vec![Recent], vec![])
    );
    assert_eq!(
        read(cx, "tree rail 260 0 width=x right=outline show=outline\n"),
        (vec![Tree], vec![Outline], vec![Outline])
    );
    assert_eq!(
        read(
            cx,
            "tree split 260 0 left=recent,tree,recent right=recent,search show=search,outline fold=recent,zzz\n"
        ),
        (vec![Recent, Tree], vec![Search], vec![Search])
    );
    // Empty or absent: the defaults.
    assert_eq!(read(cx, ""), (vec![Tree], vec![Outline], vec![]));
    // The outline, shown on purpose: placed, opened, folded.
    assert_eq!(
        read(
            cx,
            "tree split 300 0 right=outline show=outline width=280\n"
        ),
        (vec![Tree], vec![Outline], vec![Outline])
    );
    assert_eq!(
        read(
            cx,
            "tree split 300 0 right=outline,search show=search fold=outline\n"
        ),
        (vec![Tree], vec![Outline, Search], vec![Search])
    );
}

#[gpui::test]
fn outline_dock_opens_jumps_and_persists(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "docks-outline");
    let plan = "# Un\n\ntexte\n\n## Deux\n\n```\n# code\n```\n\n## Deux\n\nfin\n";
    let note = root.join("Note.md");
    fs::write(&note, plan).unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| s.open_from_nav(&note, window, cx));
    cx.simulate_resize(size(px(1600.), px(900.)));
    let bounds = |cx: &mut gpui::VisualTestContext, name: &str| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.debug_bounds(leak(name.to_string()))
            .unwrap_or_else(|| panic!("{name}"))
    };
    let note = root.join("Note.md");
    fs::write(&note, plan).unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| s.open_from_nav(&note, window, cx));
    cx.simulate_resize(size(px(1600.), px(900.)));
    let shown =
        |cx: &mut gpui::VisualTestContext| shell.read_with(cx, |s, _| s.nav.right_shown.clone());
    let editor_focused = |cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| {
            shell
                .read(cx)
                .pane()
                .editor
                .focus_handle(cx)
                .is_focused(window)
        })
    };
    let dock_focused = |cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| shell.read(cx).nav.focus.is_focused(window))
    };
    let outline = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| {
            let editor = s.pane().editor.read(cx);
            let text = editor.text().to_string();
            let (rows, active) = nav::outline_visible(
                &markdown::headings(&text),
                editor.position(),
                &s.nav.outline_collapsed,
            );
            (
                rows.len(),
                active,
                rows.iter().map(|r| r.label.clone()).collect::<Vec<_>>(),
            )
        })
    };
    // Available but hidden: the stock right dock writes no `right=` line.
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.right.clone()),
        vec![Mode::Outline]
    );
    assert_eq!(shown(cx), vec![]);
    assert!(!shell.read_with(cx, |s, _| s.right_open()));
    shell.update_in(cx, |s, window, cx| s.settle_nav(window, cx));
    assert!(!vault::load_layout().contains("right="));
    // The chord opens the dock on the cursor's section: three rows, the fenced one is none.
    cx.dispatch_action(Outline);
    cx.run_until_parked();
    assert_eq!(shown(cx), vec![Mode::Outline]);
    assert!(dock_focused(cx));
    assert_eq!(
        outline(cx),
        (
            3,
            Some(0),
            vec!["Un".to_string(), "Deux".to_string(), "Deux".to_string()]
        )
    );
    assert!(cx.debug_bounds(leak("dock-right".to_string())).is_some());
    // A click jumps the cursor to the heading and gives the keys back to the note.
    let row = bounds(cx, "outline-row-1").center();
    cx.simulate_click(row, gpui::Modifiers::none());
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position()),
        plan.find("## Deux").unwrap()
    );
    assert!(editor_focused(cx));
    // Open, the chord takes the keys; Escape in the list gives them back to the note.
    cx.dispatch_action(Outline);
    cx.run_until_parked();
    assert!(dock_focused(cx));
    cx.simulate_keystrokes("escape");
    assert!(editor_focused(cx));
    assert_eq!(shown(cx), vec![Mode::Outline]);
    // From its list, a second press hides it again, like the rail.
    cx.dispatch_action(Outline);
    cx.run_until_parked();
    assert!(dock_focused(cx));
    cx.dispatch_action(Outline);
    cx.run_until_parked();
    assert_eq!(shown(cx), vec![]);
    // Folding a parent hides its section; unfolding brings it back.
    cx.dispatch_action(Outline);
    cx.run_until_parked();
    let fold = bounds(cx, "outline-fold-0").center();
    cx.simulate_click(fold, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(outline(cx).0, 1);
    let fold = bounds(cx, "outline-fold-0").center();
    cx.simulate_click(fold, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(outline(cx).0, 3);
    // Show and tool-fold persist; the heading fold, in-memory, does not.
    shell.update(cx, |s, cx| s.fold_tool(Mode::Outline, cx));
    shell.update_in(cx, |s, window, cx| s.settle_nav(window, cx));
    let layout = vault::load_layout();
    assert!(
        layout.contains("right=outline")
            && layout.contains("show=outline")
            && layout.contains("fold=outline")
    );
    let (again, cx2) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx2.run_until_parked();
    assert_eq!(
        again.read_with(cx2, |s, _| (
            s.nav.right.clone(),
            s.nav.right_shown.clone(),
            s.nav.folded.contains(&Mode::Outline)
        )),
        (vec![Mode::Outline], vec![Mode::Outline], true)
    );
}
