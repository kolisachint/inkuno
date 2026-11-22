//! Multiple cursors, comments and their panel, the badge, narrow windows.

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let shell = &j.shell;
    let text = |cx: &mut gpui::VisualTestContext| j.text(cx);
    // Multiple cursors: Ctrl+D takes the word, then its next occurrences; typing,
    // deleting and moving apply to all, a single Ctrl+Z undoes it all,
    // Escape returns to one cursor. Alt+click adds one more.
    let cursors = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).cursors())
    };
    j.load(cx, "un chat\nun chien\nun rat", 0);
    cx.simulate_keystrokes("secondary-d secondary-d secondary-d");
    assert_eq!(cursors(cx), 3);
    cx.simulate_keystrokes("secondary-d");
    assert_eq!(cursors(cx), 3);
    cx.simulate_input("le");
    assert_eq!(text(cx), "le chat\nle chien\nle rat");
    cx.simulate_keystrokes("end");
    cx.simulate_input("s !");
    cx.simulate_keystrokes("backspace backspace home delete");
    assert_eq!(
        (text(cx), cursors(cx)),
        ("e chats\ne chiens\ne rats".to_string(), 3)
    );
    cx.simulate_keystrokes("shift-end secondary-alt-b");
    assert_eq!(text(cx), "**e chats**\n**e chiens**\n**e rats**");
    // One undo step reverts the gesture for the three cursors at once, and only it.
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(
        (text(cx), cursors(cx)),
        ("e chats\ne chiens\ne rats".to_string(), 1)
    );
    j.load(cx, "un chat\nun chien\nun rat", 0);
    cx.simulate_keystrokes("secondary-d secondary-d escape");
    assert_eq!(cursors(cx), 1);
    cx.simulate_input("X");
    // The last cursor added stays: the one of the second occurrence.
    assert_eq!(text(cx), "un chat\nX chien\nun rat");
    j.load(cx, "ab\ncd", 0);
    let middle = cx.update(|window, _| window.viewport_size());
    cx.simulate_mouse_down(
        point(middle.width / 2., middle.height / 2.),
        MouseButton::Left,
        gpui::Modifiers {
            alt: true,
            ..Default::default()
        },
    );
    cx.simulate_mouse_up(
        point(middle.width / 2., middle.height / 2.),
        MouseButton::Left,
        gpui::Modifiers::none(),
    );
    cx.simulate_input("X");
    assert_eq!((text(cx).matches('X').count(), cursors(cx)), (2, 2));
    cx.simulate_keystrokes("escape");
    // Comments: Ctrl+Shift+M comments the selection (without one, the line, after its bullet),
    // the palette lists them and leads there, the same shortcut on a comment resolves it.
    j.load(cx, "- un mot ici\nfin", 5);
    cx.simulate_keystrokes("shift-right shift-right shift-right secondary-shift-m");
    // The text goes to the draft card's field; Enter writes it into the note (RP-19/20).
    cx.simulate_input("à revoir");
    cx.simulate_keystrokes("enter");
    assert_eq!(text(cx), "- un mot<!-- comment: à revoir --> ici\nfin");
    cx.simulate_keystrokes("down secondary-shift-m");
    cx.simulate_input("ok");
    cx.simulate_keystrokes("enter");
    assert_eq!(
        text(cx),
        "- un mot<!-- comment: à revoir --> ici\nfin<!-- comment: ok -->"
    );
    // The Comments view of the right panel: Ctrl+Alt+2 shows it, one card per comment. A click on a card
    // leads into its comment and flashes its passage (selected for a moment, then the caret goes to the
    // comment); its check mark resolves it. The palette still lists the comments and leads there.
    cx.simulate_keystrokes("secondary-alt-2");
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()
        && s.nav.right_shown == [Mode::Comments]
        && s.comment_cards.len() == 2));
    // Click the first card itself, wherever the view lays it out. A pointer hovers first, as in
    // production, and the click lands on the card's lower body.
    let card = cx
        .debug_bounds("comment-0")
        .expect("the comments view shows its first card");
    cx.simulate_mouse_move(
        point(card.left() + px(24.), card.bottom() - px(18.)),
        None,
        gpui::Modifiers::none(),
    );
    cx.run_until_parked();
    let card = cx
        .debug_bounds("comment-0")
        .expect("the comments view shows its first card");
    cx.simulate_click(card.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    // The passage "un mot" (2..8, after the bullet) is selected: the flash. Then the caret goes to the comment's words.
    let flashed = shell.read_with(cx, |s, cx| {
        let e = s.pane().editor.read(cx);
        (e.position(), e.selected().to_string())
    });
    assert_eq!(flashed, (8, "un mot".to_string()));
    cx.executor()
        .advance_clock(std::time::Duration::from_secs(1));
    cx.run_until_parked();
    let pos = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position());
    assert_eq!(pos, "- un mot<!-- comment: ".len());
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.resolve_comment(40, cx))
    });
    assert_eq!(text(cx), "- un mot<!-- comment: à revoir --> ici\nfin");
    cx.simulate_keystrokes("secondary-z");
    // The view marks the comment under the caret, and none elsewhere.
    let seen = |cx: &mut gpui::VisualTestContext, at: usize| {
        shell.update(cx, |s, cx| {
            s.pane().editor.update(cx, |e, cx| e.place_cursor(at, cx))
        });
        cx.run_until_parked();
        shell.read_with(cx, |s, _| s.comments_seen)
    };
    assert_eq!(
        (seen(cx, 40), seen(cx, 36), seen(cx, 8)),
        (Some(1), None, Some(0))
    );
    // Resolved by the keys (the editor keeps Tab, so the shortcut is the way), and undone.
    cx.simulate_keystrokes("secondary-shift-m");
    cx.run_until_parked();
    assert_eq!(text(cx), "- un mot ici\nfin<!-- comment: ok -->");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(
        text(cx),
        "- un mot<!-- comment: à revoir --> ici\nfin<!-- comment: ok -->"
    );
    // The note's corner badge opens the view on the caret's card; the band's toggle hides the panel,
    // and shows it again on the same view.
    cx.run_until_parked();
    let corner = cx.update(|window, _| window.viewport_size());
    let badge = cx.debug_bounds("comments-badge").unwrap();
    assert!(
        badge.right() <= corner.width && badge.size.width >= px(15.),
        "{badge:?}"
    );
    cx.simulate_keystrokes("secondary-alt-1");
    for _ in 0..3 {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    }
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown == [Mode::Outline]));
    let at = cx.debug_bounds("comments-badge").unwrap().center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown == [Mode::Comments]));
    shell.update_in(cx, |s, window, cx| s.toggle_right_panel(window, cx));
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown.is_empty()
        && s.nav.right_view == Mode::Comments));
    shell.update_in(cx, |s, window, cx| s.toggle_right_panel(window, cx));
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.nav.right_shown == [Mode::Comments]));
    // The palette lists the comments and leads to the one named.
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("comments");
    // The matching command is the first row (P-09 puts Create last), so no `down`.
    cx.simulate_keystrokes("enter");
    cx.simulate_input("revoir");
    cx.simulate_keystrokes("enter");
    // The caret goes to the passage's start, after the bullet.
    assert!(shell.read_with(cx, |s, cx| s.palette.is_none()
        && s.pane().editor.read(cx).position() == "- ".len()));
    cx.simulate_keystrokes("secondary-shift-m");
    assert_eq!(text(cx), "- un mot ici\nfin<!-- comment: ok -->");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(
        text(cx),
        "- un mot<!-- comment: à revoir --> ici\nfin<!-- comment: ok -->"
    );
    // The badge at its limits: one comment or a hundred and twenty. It stays in the window, away from the
    // band's toggle, and its size does not change.
    let kept = text(cx);
    let many = |n: usize| {
        format!(
            "# T\n\n{}\n",
            (0..n)
                .map(|i| format!("mot<!-- comment: c{i} -->"))
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let mut seen = Vec::new();
    for n in [1, 9, 10, 12, 120] {
        j.load(cx, &many(n), 0);
        let (badge, toggle) = (
            cx.debug_bounds("comments-badge").unwrap(),
            cx.debug_bounds("comments-toggle").unwrap(),
        );
        let window = cx.update(|window, _| window.viewport_size());
        assert!(
            badge.right() <= window.width - px(8.),
            "{n}: the badge leaves the window's edge {badge:?}"
        );
        assert!(
            badge.left() >= toggle.left() - px(12.)
                && badge.top() >= toggle.top() - px(4.)
                && badge.size.height == px(15.),
            "{n}: {badge:?} {toggle:?}"
        );
        seen.push(badge);
    }
    assert!(
        seen.iter()
            .all(|b| b.size.width == seen[0].size.width && b.right() == seen[0].right()),
        "{seen:?}"
    );
    j.load(cx, &kept, 0);
    // The panel is 280 px beside the note, the note keeps its 200 px at least; the Comments view, the
    // assistant and the outline are one panel at the same width.
    let whole = cx.update(|window, _| window.viewport_size());
    let lay = |cx: &mut gpui::VisualTestContext, width: f32, view: Mode, panel: Panel| {
        cx.simulate_resize(size(px(width), px(600.)));
        shell.update(cx, |s, cx| {
            s.nav.show_alone(Mode::Tree);
            s.nav.panel = panel;
            s.nav.list_w = px(260.);
            s.nav.right_view = view;
            s.nav.right_shown = if panel == Panel::Hidden {
                Vec::new()
            } else {
                vec![view]
            };
            cx.notify();
        });
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    let within = |cx: &mut gpui::VisualTestContext, part: &'static str, zone: &'static str| {
        let (part_at, zone_at) = (
            cx.debug_bounds(part)
                .unwrap_or_else(|| panic!("{part} absent")),
            cx.debug_bounds(zone).unwrap(),
        );
        assert!(
            part_at.left() >= zone_at.left() && part_at.right() <= zone_at.right(),
            "{part} overflows {zone}: {part_at:?} outside {zone_at:?}"
        );
    };
    lay(cx, 860., Mode::Comments, Panel::Split);
    let (dock, pane) = (
        cx.debug_bounds("dock-right").unwrap(),
        cx.debug_bounds("pane-0").unwrap(),
    );
    assert!(
        dock.size.width == px(280.) && dock.right() == px(860.) && pane.right() <= dock.left(),
        "{dock:?} {pane:?}"
    );
    assert!(pane.size.width >= px(199.), "{pane:?}");
    // The view's cards and its name sit inside the panel, at every width.
    for width in [420., 860., 1600.] {
        lay(cx, width, Mode::Comments, Panel::Split);
        let dock = cx.debug_bounds("dock-right").unwrap();
        for id in ["comment-0", "comment-1", "view-head-comments"] {
            let card = cx
                .debug_bounds(id)
                .unwrap_or_else(|| panic!("{width}: {id} absent"));
            assert!(
                card.left() >= dock.left() && card.right() <= dock.right(),
                "{width}: {id} {card:?} outside {dock:?}"
            );
        }
    }
    lay(cx, 420., Mode::Comments, Panel::Split);
    cx.simulate_keystrokes("secondary-f");
    cx.run_until_parked();
    within(cx, "find-bar", "pane-0");
    cx.simulate_keystrokes("escape");
    cx.simulate_resize(whole);
    shell.update(cx, |s, cx| {
        s.nav.show_alone(Mode::Tree);
        s.nav.right_shown.clear();
        s.nav.panel = Panel::Split;
        cx.notify();
    });
    cx.run_until_parked();
    j.load(cx, "rien", 0);
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("comments");
    cx.simulate_keystrokes("enter");
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()
        && s.told(Tone::Info, "has no comments")));
    shell.update(cx, |s, _| s.toasts.clear());
}
