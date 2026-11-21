//! The window chrome: the top band (one height, one top edge, the lights room only at the top left) and
//! the sidebar's buttons (every former rail action is still a button, or a key).

use super::*;
use crate::ui::theme::{BAND, BAND_HIDDEN, LIGHTS_ROOM};

/// The top band (`app-ui.md` §20.1) has one height in every state: 38 px with the title bar shown, 12 px of
/// padding without it, in the editor as in the sidebar. The left band, the note and the right dock's header
/// share one top edge, with the left sidebar open or hidden.
#[gpui::test]
fn the_top_band_has_one_height_and_one_top_edge(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "band");
    fs::write(root.join("A.md"), "# A\n\nun\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("A.md"), window, cx)
    });
    shell.update(cx, |s, _| {
        s.client = true;
        s.nav.right_shown = vec![nav::Mode::Outline];
    });
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    for (panel, title_bar) in [
        (nav::Panel::Split, true),
        (nav::Panel::Hidden, true),
        (nav::Panel::Split, false),
        (nav::Panel::Hidden, false),
    ] {
        shell.update(cx, |s, _| {
            s.nav.panel = panel;
            s.nav.title_bar = title_bar;
        });
        settle(cx);
        let state = format!("{panel:?}, title bar {title_bar}");
        let (band, pad) = match title_bar {
            true => (px(BAND), px(0.)),
            false => (px(BAND_HIDDEN), px(BAND_HIDDEN)),
        };
        // The note starts under the band (its strip, or the padding alone).
        let strip = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).strip);
        assert_eq!(strip, band, "{state}: the note's top");
        // The right dock: its first header is the band (the switcher), padded when the bar is hidden.
        let right = cx
            .debug_bounds("dock-right")
            .expect("the right dock is drawn");
        let head = cx
            .debug_bounds("tool-head-outline")
            .expect("the right dock's first header");
        assert_eq!(
            head.top() - right.top(),
            pad,
            "{state}: the right dock's top"
        );
        assert_eq!(
            head.size.height,
            px(if title_bar { BAND } else { 32. }),
            "{state}: the right header's height"
        );
        let note = cx.debug_bounds("pane-0").expect("the note is drawn");
        if panel == nav::Panel::Hidden {
            assert!(
                !shell.read_with(cx, |s, _| s.left_open()),
                "{state}: no left panel"
            );
            assert_eq!(
                note.top(),
                right.top(),
                "{state}: the note and the right dock share a top"
            );
        } else {
            let left = cx
                .debug_bounds("sidebar-band")
                .expect("the left band is drawn");
            assert_eq!(left.size.height, band, "{state}: the left band");
            assert_eq!(
                left.top(),
                right.top(),
                "{state}: the left band and the right dock"
            );
            assert_eq!(
                left.top(),
                note.top(),
                "{state}: the left band and the note"
            );
            assert!(
                cx.debug_bounds("sidebar-foot").is_some(),
                "{state}: the foot row"
            );
        }
    }
    let _ = fs::remove_dir_all(&root);
}

/// The lights' room (macOS) is in the band at the window's top left, and nowhere else: with the sidebar open,
/// the band holds it and the note starts past the sidebar; hidden, the note's own strip holds it, then the
/// sidebar toggle, then the note's title. With the title bar hidden, no lights and no room.
#[gpui::test]
fn the_lights_room_is_only_in_the_band_at_the_top_left(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "lights");
    fs::write(root.join("A.md"), "# A\n\nun\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("A.md"), window, cx)
    });
    // No client decorations here: the system draws the bar, so the lights float over the top left.
    shell.update(cx, |s, _| {
        s.client = false;
        s.nav.panel = nav::Panel::Split;
    });
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    settle(cx);
    let lights = px(LIGHTS_ROOM);
    let edge = |cx: &mut gpui::VisualTestContext| shell.read_with(cx, |s, _| s.strip_edge(0));
    // Sidebar open: the band starts its buttons past the lights; the note has no room to make for them.
    assert_eq!(edge(cx), px(0.), "the note's strip starts at its own edge");
    let toggle = cx.debug_bounds("nav-sidebar").expect("the sidebar toggle");
    assert!(
        toggle.left() >= lights,
        "the toggle is past the lights: {toggle:?}"
    );
    assert!(
        toggle.top() < cx.debug_bounds("sidebar-band").unwrap().bottom(),
        "the toggle is in the band"
    );
    assert_eq!(
        edge(cx),
        px(0.),
        "no note title in the band: the note's strip is its own"
    );
    // Sidebar hidden: the note's strip holds the lights, the toggle, then the title.
    shell.update(cx, |s, _| s.nav.panel = nav::Panel::Hidden);
    settle(cx);
    let edge_now = edge(cx);
    assert_eq!(
        edge_now,
        lights + px(32.),
        "the strip clears the lights and the toggle"
    );
    let toggle = cx
        .debug_bounds("nav-sidebar")
        .expect("the toggle, in the strip");
    assert!(
        toggle.left() >= lights,
        "the toggle is past the lights: {toggle:?}"
    );
    let title = cx
        .debug_bounds("strip-title")
        .expect("the note's title, in the strip");
    assert!(
        title.left() >= toggle.right(),
        "the title starts after the toggle: {title:?}"
    );
    // Title bar hidden: no lights, so no room; the band is 12 px of padding.
    shell.update(cx, |s, _| {
        s.nav.panel = nav::Panel::Split;
        s.nav.title_bar = false;
    });
    settle(cx);
    assert_eq!(edge(cx), px(0.), "no lights without the title bar");
    assert!(
        cx.debug_bounds("nav-sidebar").is_some(),
        "the foot keeps the sidebar toggle with the bar hidden"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Every action the rail held still has a button, or a key that does the same: the commands logo, the tree,
/// search, new note, the sidebar toggle and the title bar toggle. The band keeps three of them; with the bar
/// hidden the band is gone and they move to the foot.
#[gpui::test]
fn every_former_rail_action_is_still_a_button(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "rail-actions");
    // Inkuno's own chrome (the band, the foot, the tree header): Quiet, the default, has its own sidebar.
    pin_inkuno(&shell, cx);
    fs::write(root.join("A.md"), "# A\n\nun\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("A.md"), window, cx)
    });
    shell.update(cx, |s, _| {
        s.client = true;
        s.nav.panel = nav::Panel::Split;
    });
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    settle(cx);
    let click = |cx: &mut gpui::VisualTestContext, id: &str| {
        let at = cx
            .debug_bounds(leak(id.to_string()))
            .unwrap_or_else(|| panic!("{id} is drawn"))
            .center();
        cx.simulate_click(at, gpui::Modifiers::none());
        cx.run_until_parked();
    };
    // The band: sidebar toggle, search, new note, right-aligned in that order.
    let band = ["nav-sidebar", "nav-search", "nav-new"].map(|id| {
        cx.debug_bounds(id)
            .unwrap_or_else(|| panic!("{id} in the band"))
    });
    assert!(band[0].right() <= band[1].left() && band[1].right() <= band[2].left());
    // The foot: the commands logo, the tree, the title bar toggle, in that order.
    let foot = ["nav-commands", "nav-tree", "nav-titlebar"].map(|id| {
        cx.debug_bounds(id)
            .unwrap_or_else(|| panic!("{id} in the foot"))
    });
    assert!(foot[0].right() <= foot[1].left() && foot[1].right() <= foot[2].left());

    // The commands logo opens the palette on commands.
    click(cx, "nav-commands");
    assert!(
        shell.read_with(cx, |s, _| s.palette.is_some()),
        "the logo opens the palette"
    );
    cx.simulate_keystrokes("escape");
    // Search opens the palette too.
    click(cx, "nav-search");
    assert!(
        shell.read_with(cx, |s, _| s.palette.is_some()),
        "search opens the palette"
    );
    cx.simulate_keystrokes("escape");
    // The tree: from another tool, it shows the tree again.
    shell.update(cx, |s, _| s.nav.activate(nav::Mode::Recent));
    settle(cx);
    click(cx, "nav-tree");
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.mode),
        nav::Mode::Tree,
        "the tree button"
    );
    // New note: a note appears in the vault.
    click(cx, "nav-new");
    cx.run_until_parked();
    assert!(
        shell.read_with(cx, |s, _| s.pane().path.is_none() && s.pane().fresh),
        "a new note is open, not yet filed"
    );
    // The sidebar toggle hides the panel; the strip's toggle brings it back.
    click(cx, "nav-sidebar");
    assert_eq!(shell.read_with(cx, |s, _| s.nav.panel), nav::Panel::Hidden);
    settle(cx);
    click(cx, "nav-sidebar");
    assert_eq!(shell.read_with(cx, |s, _| s.nav.panel), nav::Panel::Split);
    // The title bar toggle hides the bar: the band collapses and its buttons join the foot.
    click(cx, "nav-titlebar");
    assert!(!shell.read_with(cx, |s, _| s.nav.title_bar));
    settle(cx);
    let band_height = cx.debug_bounds("sidebar-band").unwrap().size.height;
    assert_eq!(band_height, px(BAND_HIDDEN), "the band is padding only");
    for id in [
        "nav-sidebar",
        "nav-search",
        "nav-new",
        "nav-commands",
        "nav-tree",
        "nav-titlebar",
    ] {
        let bounds = cx
            .debug_bounds(id)
            .unwrap_or_else(|| panic!("{id} in the foot"));
        assert!(
            bounds.top() > cx.debug_bounds("sidebar-band").unwrap().bottom() - px(1.),
            "{id} is in the foot, under the band"
        );
    }
    // And the same button brings the bar back.
    click(cx, "nav-titlebar");
    assert!(shell.read_with(cx, |s, _| s.nav.title_bar));
    let _ = fs::remove_dir_all(&root);
}
