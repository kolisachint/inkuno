//! The window around the notes: badge, format bar, saved shortcuts and panes, first launch, empty page.

use super::*;
use crate::shell::render::badge_label;

#[test]
fn badge_shows_the_count_up_to_nine() {
    assert_eq!(
        ["1", "9", "9+", "9+"],
        [
            badge_label(1),
            badge_label(9),
            badge_label(10),
            badge_label(120)
        ]
    );
    assert_eq!(badge_label(0), "0");
}

#[test]
fn badge_text_reads_on_every_theme() {
    // The number is in the background colour, on the accent badge: it reads like a link's text.
    for (name, _) in THEMES {
        let prefs = Prefs {
            theme: name.to_string(),
            ..Prefs::parse("")
        };
        let t = Theme::of(&prefs, gpui::WindowAppearance::Light);
        assert!(
            contrast(t.accent, t.bg) >= 4.5,
            "{name}: {} on the badge",
            contrast(t.accent, t.bg)
        );
    }
}

#[test]
fn folded_bar_button_hides_what_it_covers() {
    // The folded bar's button sits over text: opaque, closed or open, and its sign is readable, on every theme.
    for (name, _) in THEMES {
        let prefs = Prefs {
            theme: name.to_string(),
            ..Prefs::parse("")
        };
        let t = Theme::of(&prefs, gpui::WindowAppearance::Light);
        for open in [false, true] {
            let fill = editor::bar_fill(&t, open);
            assert!(
                fill.a == 1.,
                "{name} {open}: translucent background {fill:?}"
            );
            let sign = if open { t.accent } else { t.soft };
            assert!(
                contrast(sign, fill) >= 3.,
                "{name} {open}: sign {} on the background",
                contrast(sign, fill)
            );
        }
    }
}

/// Changed shortcuts are read again at launch; a faulty line in the file is set aside
/// and reported, the others apply.
#[gpui::test]
fn keys_survive_a_restart(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-keys-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    vault::save_file("keys", "nav::ShowTree=secondary-shift-y\nno::Such=f8\n");
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, _| s.changes.clone()),
        keys::Changes::from([("nav::ShowTree".to_string(), "secondary-shift-y".to_string())])
    );
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Failed, "no::Such=f8")));
    // secondary-t still dispatches nothing (the tags view stays cut); secondary-e no longer
    // shows the tree (its keys moved with the action), secondary-shift-y does.
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(nav::Mode::Recent, false, window, cx)
    });
    cx.run_until_parked();
    for key in ["secondary-t", "secondary-e"] {
        cx.simulate_keystrokes(key);
        assert_eq!(shell.read_with(cx, |s, _| s.nav.mode), nav::Mode::Recent);
    }
    cx.simulate_keystrokes("secondary-shift-y");
    assert_eq!(shell.read_with(cx, |s, _| s.nav.mode), nav::Mode::Tree);
    // A file from before this version, or empty: the original shortcuts.
    let _ = fs::remove_dir_all(&root);
}

/// The second pane comes back at launch, with its width; a vanished file, or a config
/// from before panes, leaves a single pane.
#[gpui::test]
fn panes_come_back(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-panes-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    let (left, right) = (root.join("Gauche.md"), root.join("Droite.md"));
    fs::write(&left, "# Gauche\n").unwrap();
    fs::write(&right, "# Droite\n").unwrap();
    vault::save_file("panes", &format!("0.4\n{}\n", right.display()));
    cx.update(bind_keys);
    cx.update(init_fonts);
    // The right note is the last opened: the first pane takes the next one.
    let recent = vec![right.clone(), left.clone()];
    let (shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), recent, window, cx));
    cx.run_until_parked();
    let open = shell.read_with(cx, |s, _| {
        (
            s.active,
            s.layout.write(),
            s.panes
                .iter()
                .map(|pane| pane.path.clone())
                .collect::<Vec<_>>(),
        )
    });
    assert_eq!(
        open,
        (
            0,
            "row(0.4 0, 0.6 1)".to_string(),
            vec![Some(left.clone()), Some(right.clone())]
        )
    );
    cx.simulate_input("x");
    assert_eq!(
        shell.read_with(cx, |s, cx| s.panes[0].editor.read(cx).text().to_string()),
        "# Gauche\nx"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Without a vault, the welcome screen creates one with a click, in `Notes` of the home folder.
#[gpui::test]
fn starts_in_a_new_vault(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-start-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view(|window, cx| Shell::new(None, Vec::new(), window, cx));
    cx.run_until_parked();
    let at = cx.debug_bounds("welcome-create").unwrap().center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    let notes = root.join(".config").join("Notes");
    assert!(notes.is_dir());
    assert_eq!(
        shell.read_with(cx, |s, _| s.vault.clone()),
        Some(notes.clone())
    );
    // On Windows, the path is shown in full.
    if !cfg!(windows) {
        assert_eq!(vault::shown(&notes), "~/Notes");
    }
    let _ = fs::remove_dir_all(&root);
}

/// A note opens on its body: the first key does not break the front matter and does not
/// rename the note after its title, which would rewrite the other notes' links.
#[gpui::test]
fn opens_on_the_body(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-body-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    let plan = root.join("Plan.md");
    fs::write(&plan, "---\ntags: [a]\n---\n# Plan\n\nÉtapes\n").unwrap();
    fs::write(root.join("Accueil.md"), "# Accueil\n\n[[Plan]]\n").unwrap();
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (_shell, cx) = cx.add_window_view(|window, cx| {
        Shell::new(Some(root.clone()), vec![plan.clone()], window, cx)
    });
    cx.run_until_parked();
    cx.simulate_input("x");
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert_eq!(
        fs::read_to_string(&plan).unwrap(),
        "---\ntags: [a]\n---\n# Plan\n\nxÉtapes\n"
    );
    assert_eq!(
        fs::read_to_string(root.join("Accueil.md")).unwrap(),
        "# Accueil\n\n[[Plan]]\n"
    );

    // A new note written then emptied leaves nothing behind when leaving.
    cx.simulate_keystrokes("secondary-alt-n");
    cx.simulate_input("Brouillon");
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    assert!(root.join("Brouillon.md").is_file());
    cx.simulate_keystrokes("secondary-a backspace");
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    cx.simulate_keystrokes("secondary-alt-n");
    cx.run_until_parked();
    let mut notes: Vec<_> = fs::read_dir(&root)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .filter(|n| n.to_string_lossy().ends_with(".md"))
        .collect();
    notes.sort();
    assert_eq!(notes, ["Accueil.md", "Plan.md"]);
    let _ = fs::remove_dir_all(&root);
}

/// The empty page: today's date, the useful keys and the notes to revisit, under the line
/// where one writes. Everything fits in a narrow window, and disappears on the first keystroke.
#[gpui::test]
fn empty_page(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-empty-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    let week = date_name(markdown::day_of(markdown::day_number(today()) - 7));
    let (daily, old) = (root.join(format!("{week}.md")), root.join("Ancienne.md"));
    fs::write(&daily, format!("# {week}\n")).unwrap();
    fs::write(&old, "# Ancienne\n").unwrap();
    let long_ago = std::time::SystemTime::now() - Duration::from_secs(100 * 86_400);
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(long_ago)
        .unwrap();
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx.run_until_parked();
    let open = |cx: &mut gpui::VisualTestContext| shell.read_with(cx, |s, _| s.pane().path.clone());
    let text_of = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
    };
    assert_eq!(open(cx), None);

    // The logo is in the rail, whatever the system.
    assert!(cx.debug_bounds("rail-logo").is_some());
    // The notes to revisit: the daily note from a week ago, then an old note.
    assert_eq!(
        shell.read_with(cx, |s, _| s
            .recalled()
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<_>>()),
        [week.clone(), "Ancienne".to_string()]
    );
    // In the narrowest window, nothing overflows the pane.
    cx.simulate_resize(size(px(360.), px(640.)));
    cx.run_until_parked();
    let pane = cx.debug_bounds("pane-0").unwrap();
    assert!(pane.size.width < px(360.), "{pane:?}");
    for part in [
        "empty-today",
        "empty-key-0",
        "empty-key-1",
        "empty-key-2",
        "empty-recall-0",
        "empty-recall-1",
    ] {
        let bounds = cx
            .debug_bounds(part)
            .unwrap_or_else(|| panic!("{part} absent"));
        assert!(
            bounds.left() >= pane.left() && bounds.right() <= pane.right(),
            "{part} overflows: {bounds:?} outside {pane:?}"
        );
    }
    cx.simulate_resize(size(px(860.), px(720.)));
    cx.run_until_parked();

    // A click on a note to revisit opens it.
    let at = cx.debug_bounds("empty-recall-1").unwrap().center();
    cx.simulate_click(at, gpui::Modifiers::none());
    assert_eq!(open(cx), Some(old.clone()));
    // A note with text does not show the empty page: where the date was, the click
    // lands in the note. (gpui keeps the bounds of a vanished element: we click.)
    cx.run_until_parked();
    let at = cx.debug_bounds("empty-today").unwrap().center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(open(cx), Some(old.clone()));

    // A key of the empty page can be clicked: here the palette.
    cx.simulate_keystrokes("secondary-n");
    cx.run_until_parked();
    let at = cx.debug_bounds("empty-key-0").unwrap().center();
    cx.simulate_click(at, gpui::Modifiers::none());
    assert!(shell.read_with(cx, |s, _| s.palette.is_some()));
    cx.simulate_keystrokes("escape");

    // The date opens the daily note.
    let at = cx.debug_bounds("empty-today").unwrap().center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        open(cx),
        Some(root.join(format!("{}.md", date_name(today()))))
    );

    // The first keystroke clears everything: the page belongs to what is written.
    cx.simulate_keystrokes("secondary-n");
    cx.run_until_parked();
    cx.simulate_input("x");
    cx.run_until_parked();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(text_of(cx), "x");
    let _ = fs::remove_dir_all(&root);
}

/// Where the app draws the window buttons (Linux), they float over the top right corner: the panel header, on the whole window,
/// keeps its four buttons clear of them, and a narrow note starts under them, whatever the panel, the width or the language.
#[gpui::test]
fn window_buttons_leave_the_header_and_the_title_clear(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-pill-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    let note = root.join("Titre.md");
    fs::write(
        &note,
        "# Un titre très très très long qui passe à la ligne dans une fenêtre étroite\n\nTexte\n",
    )
    .unwrap();
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let mut shell = Shell::new(Some(root.clone()), vec![note.clone()], window, cx);
        shell.client = true;
        shell
    });
    cx.run_until_parked();
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    let top = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).top_margin())
    };
    // The panel on the whole window: its four buttons end where the window buttons begin, from a phone width to a very wide window.
    shell.update(cx, |s, cx| {
        s.nav.show_alone(nav::Mode::Tree);
        s.nav.panel = nav::Panel::Full;
        cx.notify();
    });
    for width in [320., 420., 860., 1600.] {
        cx.simulate_resize(size(px(width), px(600.)));
        settle(cx);
        let pill = cx
            .debug_bounds("window-pill")
            .expect("the window buttons are drawn");
        let mut last = px(0.);
        for button in ["nav-file", "nav-folder", "nav-fold", "nav-full"] {
            let bounds = cx
                .debug_bounds(button)
                .unwrap_or_else(|| panic!("{width}: {button} absent"));
            assert!(
                bounds.right() <= pill.left(),
                "{width}: {button} {bounds:?} under the window buttons {pill:?}"
            );
            assert!(
                bounds.left() >= last,
                "{width}: {button} overlaps the one before"
            );
            last = bounds.right();
        }
    }
    // The panel beside the note: a note narrowed by the panel (120, 260, 600 px) starts under the window buttons; in a window wide
    // enough for the page, the note starts under the handle strip.
    shell.update(cx, |s, cx| {
        s.nav.panel = nav::Panel::Split;
        cx.notify();
    });
    let pill_bottom =
        |cx: &mut gpui::VisualTestContext| cx.debug_bounds("window-pill").unwrap().bottom();
    for (window, panel) in [
        (860., 120.),
        (860., 260.),
        (860., 600.),
        (420., 120.),
        (1600., 260.),
        (1600., 600.),
    ] {
        cx.simulate_resize(size(px(window), px(600.)));
        shell.update(cx, |s, cx| {
            s.nav.list_w = px(panel);
            cx.notify();
        });
        settle(cx);
        let (pane, pill, top) = (cx.debug_bounds("pane-0").unwrap(), pill_bottom(cx), top(cx));
        // Narrow: the pane, not the page measure, sets the width of the text.
        let narrow = pane.size.width < editor::MAX_WIDTH + px(48.);
        if narrow {
            assert!(
                pane.top() + top >= pill,
                "{window}/{panel}: the text starts at {top:?}, the window buttons end at {pill:?}"
            );
        } else {
            assert_eq!(
                top,
                px(40.),
                "{window}/{panel}: a note wide enough starts under the window's handle strip"
            );
        }
    }
    // No panel, the page is as wide as the window lets it: the default window starts under the handle strip, a narrow one gives room.
    shell.update(cx, |s, cx| {
        s.nav.panel = nav::Panel::Rail;
        cx.notify();
    });
    for window in [860., 1600., 420., 320.] {
        cx.simulate_resize(size(px(window), px(600.)));
        settle(cx);
        let (pane, pill, top) = (cx.debug_bounds("pane-0").unwrap(), pill_bottom(cx), top(cx));
        // Narrow: the pane, not the page measure, sets the width of the text.
        let narrow = pane.size.width < editor::MAX_WIDTH + px(48.);
        if narrow {
            assert!(
                pane.top() + top >= pill,
                "{window}: the text starts at {top:?}, the window buttons end at {pill:?}"
            );
        } else {
            assert_eq!(
                top,
                px(40.),
                "{window}: a note wide enough starts under the window's handle strip"
            );
        }
    }
    // Zed-style header: the tree panel shows no vault name — its action icons sit left, ahead of
    // the (empty) title handle — and the folder moved to the window's title bar, drawn centered
    // in the top row where the app draws the bar itself.
    shell.update(cx, |s, cx| {
        s.nav.panel = nav::Panel::Split;
        s.nav.show_alone(nav::Mode::Tree);
        cx.notify();
    });
    settle(cx);
    let (head, title, file) = (
        cx.debug_bounds("tool-head-tree").unwrap(),
        cx.debug_bounds("tool-title-tree").unwrap(),
        cx.debug_bounds("nav-file").unwrap(),
    );
    assert!(
        file.right() <= title.left() + px(1.),
        "the tree icons sit left of the title handle: {file:?} {title:?}"
    );
    assert!(
        file.left() - head.left() < px(40.),
        "the tree icons sit at the header's left end: {file:?} {head:?}"
    );
    assert!(
        cx.debug_bounds("window-title").is_some(),
        "the vault name is drawn in the top row"
    );
    assert_eq!(
        shell.read_with(cx, |s, _| s.title.clone()),
        format!("Titre — {} — Inkuno", vault::stem(&root)),
        "the folder moved to the window's title"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Where the app draws the window buttons, no tab and no comment column goes under them: in a window of 420, 860
/// and 1 600 px, with eight tabs of long titles and a note that has comments, the tabs' row and the column's header
/// end before the buttons or start below them.
#[gpui::test]
fn window_buttons_have_the_top_row_to_themselves(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-pill-row-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let notes: Vec<_> = (0..8)
        .map(|i| root.join(format!("Une note au titre très très long numéro {i}.md")))
        .collect();
    for note in &notes {
        fs::write(note, "# Titre\n\nUn {==mot==}{>>remarque<<} et du texte.\n").unwrap();
    }
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let mut shell = Shell::new(Some(root.clone()), vec![notes[0].clone()], window, cx);
        shell.client = true;
        shell
    });
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| {
        for note in &notes[1..] {
            s.add_tab(window, cx);
            s.open_note(note, cx);
        }
    });
    // Shut-default (ui.md §12): the column starts closed; open it so the header geometry below is exercised.
    shell.update_in(cx, |s, _window, cx| {
        s.comments_shut = false;
        cx.notify();
    });
    cx.run_until_parked();
    let mut seen = 0;
    for width in [420., 860., 1600.] {
        cx.simulate_resize(size(px(width), px(600.)));
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
        let pill = cx
            .debug_bounds("window-pill")
            .expect("the window buttons are drawn");
        // The tabs scroll inside their row, which ends where the buttons begin.
        let row = cx
            .debug_bounds("tabs-row-0")
            .expect("the tabs' row is drawn");
        assert!(
            row.right() <= pill.left(),
            "{width}: the tabs' row {row:?} runs under the window buttons {pill:?}"
        );
        assert!(row.size.width > px(0.), "{width}: no room left for a tab");
        if let Some(shut) = cx.debug_bounds("comments-shut") {
            assert!(
                shut.top() >= pill.bottom() || shut.right() <= pill.left(),
                "{width}: the comments' header {shut:?} under the window buttons {pill:?}"
            );
            seen += 1;
        }
        // Zed-style: the vault name sits centered in the top row, clear of the window buttons.
        let name = cx
            .debug_bounds("window-title")
            .expect("the vault name is drawn in the tab bar");
        assert!(
            name.right() <= pill.left(),
            "{width}: the vault name {name:?} runs under the window buttons {pill:?}"
        );
    }
    assert!(seen > 0, "the comments column was never open");
    let _ = fs::remove_dir_all(&root);
}

/// Where the app draws its title bar, there is always a place to grab the window: in every layout, the top row of each pane
/// (tab bar past the last tab, or, without tab bar, a strip over the empty top of the note), the panel's header past its
/// title and the empty part of the rail. The text never starts under the strip.
#[gpui::test]
fn the_window_can_always_be_grabbed_at_the_top(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-grab-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (one, two) = (root.join("Une.md"), root.join("Deux.md"));
    fs::write(&one, "# Titre\n\nUn {==mot==}{>>remarque<<} et du texte.\n").unwrap();
    fs::write(&two, "# Autre\n\nTexte\n").unwrap();
    let (shell, cx) = cx.add_window_view(|window, cx| {
        let mut shell = Shell::new(Some(root.clone()), vec![one.clone()], window, cx);
        shell.client = true;
        shell
    });
    cx.run_until_parked();
    cx.simulate_resize(size(px(860.), px(720.)));
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    // The handles of the top row: bounds of the bare strip or of the bar's free part, with the widest one named.
    let handles = |cx: &mut gpui::VisualTestContext, group: usize| {
        let names = [
            ["drag-strip-0", "tabs-free-0"],
            ["drag-strip-1", "tabs-free-1"],
        ][group];
        names
            .map(|name| cx.debug_bounds(name))
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    };
    let wide = px(200.);
    let (mut seen, mut report) = (0, Vec::new());
    let mut check = |what: &str, groups: usize, cx: &mut gpui::VisualTestContext| {
        settle(cx);
        for group in 0..groups {
            let pane = cx.debug_bounds(["pane-0", "pane-1"][group]).unwrap();
            // Bounds of an element that is no longer drawn stay in the table: only what lies inside the pane counts.
            let found: Vec<_> = handles(cx, group)
                .into_iter()
                .filter(|b| b.left() >= pane.left() - px(1.) && b.right() <= pane.right() + px(1.))
                .collect();
            let best = found
                .iter()
                .map(|b| b.size.width)
                .fold(px(0.), |a, b| a.max(b));
            assert!(
                best >= wide,
                "{what}: pane {group} has no handle of {wide:?} at its top ({found:?})"
            );
            for handle in &found {
                assert!(
                    handle.top() <= pane.top() + px(1.) && handle.bottom() <= pane.top() + px(41.),
                    "{what}: {handle:?} not at the top of {pane:?}"
                );
            }
            report.push(format!("{what} pane {group}: {best:?}"));
            seen += 1;
        }
    };
    let panel = |cx: &mut gpui::VisualTestContext, panel| {
        shell.update(cx, |s, cx| {
            s.nav.panel = panel;
            cx.notify();
        });
    };
    // One pane, one tab: no tab bar, the strip; the text starts under it.
    panel(cx, nav::Panel::Rail);
    check("one tab, rail", 1, cx);
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).top_margin()),
        px(56.),
        "the text starts under the strip"
    );
    let rail = cx
        .debug_bounds("rail-free")
        .expect("the rail has an empty part");
    assert!(rail.size.height > px(20.), "{rail:?}");
    // Panel open beside the note (the header's free part past the title), with the comment column open.
    panel(cx, nav::Panel::Split);
    check("one tab, panel", 1, cx);
    let (head, title) = (
        cx.debug_bounds("tool-head-tree").unwrap(),
        cx.debug_bounds("tool-title-tree").unwrap(),
    );
    assert!(
        head.right() > title.right() + px(40.) || head.size.width < title.size.width + px(40.),
        "{head:?} {title:?}"
    );
    // Shut-default (ui.md §12): the column starts closed; open it before asserting it is drawn.
    shell.update_in(cx, |s, _window, cx| {
        s.comments_shut = false;
        cx.notify();
    });
    settle(cx);
    assert!(
        cx.debug_bounds("comments").is_some(),
        "the comments column is open"
    );
    // Two tabs: the bar, free past the last tab.
    shell.update_in(cx, |s, window, cx| {
        s.add_tab(window, cx);
        s.open_note(&two, cx);
    });
    check("two tabs, panel", 1, cx);
    panel(cx, nav::Panel::Rail);
    check("two tabs, rail", 1, cx);
    // Two panes of one tab each.
    shell.update_in(cx, |s, window, cx| {
        s.close_tab(1, window, cx);
        s.open_aside(&two, window, cx);
    });
    check("two panes", 2, cx);
    panel(cx, nav::Panel::Split);
    check("two panes, panel", 2, cx);
    assert_eq!(seen, 8, "{report:?}");
    let _ = fs::remove_dir_all(&root);
}

/// The panes take the window edge to edge (owner decision 2026-11-14, app-ui.md §10 item 4): the note and the
/// panel reach the window's top, right and bottom edges, and a hairline (`nav::DIVIDER`, 1 px) separates
/// neighbours, none between the rail and the note. In every width, with the panel folded and open, with one
/// pane and two, and at the smallest and largest zoom.
#[gpui::test]
fn sheets_tile_the_window_edge_to_edge_with_a_hairline(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "ground");
    let (one, two) = (root.join("Une.md"), root.join("Deux.md"));
    fs::write(&one, "# Titre\n\nTexte\n").unwrap();
    fs::write(&two, "# Autre\n\nTexte\n").unwrap();
    shell.update(cx, |s, cx| {
        s.rescan(&root, cx);
        s.open_note(&one, cx);
    });
    let line = nav::DIVIDER;
    assert_eq!(line, px(1.));
    let mut seen = 0;
    let mut check =
        |what: &str, width: f32, panels: bool, panes: usize, cx: &mut gpui::VisualTestContext| {
            cx.simulate_resize(size(px(width), px(720.)));
            for _ in 0..3 {
                cx.update(|window, _| window.refresh());
                cx.run_until_parked();
            }
            let sheets: Vec<_> = (0..panes)
                .map(|i| {
                    cx.debug_bounds(["pane-0", "pane-1"][i])
                        .unwrap_or_else(|| panic!("{what}: pane {i}"))
                })
                .collect();
            for (i, sheet) in sheets.iter().enumerate() {
                assert_eq!(
                    (sheet.top(), sheet.bottom()),
                    (px(0.), px(720.)),
                    "{what}: pane {i} {sheet:?}"
                );
            }
            assert_eq!(
                sheets.last().unwrap().right(),
                px(width),
                "{what}: right edge {sheets:?}"
            );
            for pair in sheets.windows(2) {
                assert_eq!(
                    pair[1].left() - pair[0].right(),
                    line,
                    "{what}: between the panes {pair:?}"
                );
            }
            let rail = cx.debug_bounds("rail-free").expect("the rail is drawn");
            if panels {
                let panel = cx.debug_bounds("nav-panel").expect("the panel is drawn");
                assert_eq!(
                    panel.left(),
                    rail.right(),
                    "{what}: the panel touches the rail's side {panel:?} {rail:?}"
                );
                assert_eq!(
                    (panel.top(), panel.bottom()),
                    (px(0.), px(720.)),
                    "{what}: panel {panel:?}"
                );
                assert_eq!(
                    sheets[0].left() - panel.right(),
                    line,
                    "{what}: between the panel and the note {panel:?} {sheets:?}"
                );
            } else {
                assert_eq!(
                    sheets[0].left() - rail.right(),
                    px(0.),
                    "{what}: the note touches the rail's side"
                );
            }
            seen += 1;
        };
    for width in [860., 1600., 420.] {
        shell.update(cx, |s, cx| {
            s.nav.panel = nav::Panel::Rail;
            cx.notify();
        });
        check(&format!("{width} folded"), width, false, 1, cx);
        shell.update(cx, |s, cx| {
            s.nav.panel = nav::Panel::Split;
            cx.notify();
        });
        check(&format!("{width} open"), width, true, 1, cx);
    }
    shell.update_in(cx, |s, window, cx| s.open_aside(&two, window, cx));
    shell.update(cx, |s, cx| {
        s.nav.panel = nav::Panel::Split;
        cx.notify();
    });
    for width in [860., 1600.] {
        check(&format!("{width} two panes"), width, true, 2, cx);
    }
    for keys in ["secondary-- ".repeat(5), "secondary-= ".repeat(21)] {
        cx.simulate_keystrokes(keys.trim_end());
        cx.run_until_parked();
        check("zoomed", 1600., true, 2, cx);
    }
    assert_eq!(seen, 10);
    let _ = fs::remove_dir_all(&root);
}

/// The interface follows the note's zoom, from 85 % to 150 % of its size at 16: the rail's buttons, the panel's
/// rows, the menus and the palette grow with it, their text with them.
#[gpui::test]
fn interface_follows_the_zoom(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "scale");
    fs::write(
        root.join("Alpha.md"),
        "# Alpha\n\nUn {==mot==}{>>remarque<<} et beaucoup de mots pour un long compteur.\n",
    )
    .unwrap();
    shell.update(cx, |s, cx| {
        s.rescan(&root, cx);
        s.open_note(&root.join("Alpha.md"), cx);
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.simulate_keystrokes("secondary-e");
    cx.run_until_parked();
    // Sizes of a button, a row of the tree, the row's menu and an entry of the command palette.
    let measure = |cx: &mut gpui::VisualTestContext| {
        let button = cx.debug_bounds("nav-file").unwrap().size.width;
        let row = cx.debug_bounds("nav-row-Alpha").unwrap().size.height;
        cx.simulate_keystrokes("shift-f10");
        cx.run_until_parked();
        let menu = cx.debug_bounds("menu").unwrap().size;
        cx.simulate_keystrokes("escape f10");
        cx.simulate_input("new note");
        cx.run_until_parked();
        let command = cx.debug_bounds("command-app::NewNote").unwrap().size.height;
        cx.simulate_keystrokes("escape secondary-e");
        cx.run_until_parked();
        [
            f32::from(button),
            f32::from(row),
            f32::from(menu.width),
            f32::from(menu.height),
            f32::from(command),
        ]
    };
    let base = measure(cx);
    for (keys, size, scale) in [
        ("secondary-= ".repeat(8), 24., 1.5),
        ("secondary-= ".repeat(6), 30., 1.5),
        ("secondary-0 ".into(), 16., 1.),
        ("secondary-- ".repeat(5), 11., 0.85),
    ] {
        cx.simulate_keystrokes(keys.trim_end());
        cx.run_until_parked();
        assert_eq!(shell.read_with(cx, |s, _| s.theme.size), size);
        let now = measure(cx);
        // The note's foot: counter and comments button side by side, none over another.
        let foot = ["counts", "comments-toggle"].map(|name| {
            cx.debug_bounds(name)
                .unwrap_or_else(|| panic!("{name} at {size}"))
        });
        assert!(foot[0].right() <= foot[1].left(), "at {size}: {foot:?}");
        for (i, (got, was)) in now.iter().zip(base).enumerate() {
            assert!(
                (got / was - scale).abs() < 0.04,
                "at {size}: measure {i} is {got}, {was} at 16 (×{scale} expected)"
            );
        }
    }
}

/// Every small target is at least 24 × 24 px: the window buttons,
/// "Reset all" and "default" in the help.
#[gpui::test]
fn small_targets_are_24_px(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "targets");
    fs::write(root.join("Alpha.md"), "# Alpha\n").unwrap();
    fs::write(root.join("keys"), "").unwrap();
    shell.update(cx, |s, cx| {
        s.client = true;
        s.rescan(&root, cx);
        s.open_note(&root.join("Alpha.md"), cx);
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.run_until_parked();
    let mut seen = Vec::new();
    for name in ["minimize", "maximize", "close"] {
        let b = cx
            .debug_bounds(name)
            .unwrap_or_else(|| panic!("{name} is drawn"));
        seen.push((name, b.size));
    }
    // The help: "Reset all", and the "default" of a changed shortcut.
    shell.update(cx, |s, cx| {
        s.key_set("app::OpenVault", "secondary-shift-k".into(), &[], cx)
    });
    cx.simulate_keystrokes("f1");
    cx.run_until_parked();
    for name in ["keys-reset", "key-reset-app::OpenVault"] {
        let b = cx
            .debug_bounds(name)
            .unwrap_or_else(|| panic!("{name} is drawn"));
        seen.push((name, b.size));
    }
    let small: Vec<_> = seen
        .iter()
        .filter(|(_, s)| s.width < px(24.) || s.height < px(24.))
        .collect();
    assert!(small.is_empty(), "under 24 px: {small:?}");
}

/// Small clarity fixes: the help's table of contents, the theme's default that
/// says it follows the system, the line of the active pane.
#[gpui::test]
fn clear_at_a_glance(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "clarity");
    fs::write(root.join("Alpha.md"), "# Alpha\n").unwrap();
    shell.update(cx, |s, cx| {
        s.rescan(&root, cx);
        s.open_note(&root.join("Alpha.md"), cx);
    });
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.run_until_parked();

    // The help: a link per section, which scrolls the list to it.
    cx.simulate_keystrokes("f1");
    cx.run_until_parked();
    let scroll =
        |cx: &mut gpui::VisualTestContext| shell.read_with(cx, |s, _| s.help.scroll.offset().y);
    assert!(cx.debug_bounds("help-contents").is_some());
    let last = shell.read_with(cx, |s, cx| s.help_rows(cx).len()) - 1;
    let link = cx
        .debug_bounds(leak(format!("help-to-{last}")))
        .expect("a link to the last section");
    assert!(link.size.height >= px(24.));
    cx.simulate_click(link.center(), gpui::Modifiers::none());
    cx.run_until_parked();
    assert!(scroll(cx) < px(0.), "the list scrolled to the last section");
    // Sections far from the window are empty boxes; the ones in view, after the jump and after wheel steps up, are drawn.
    let drawn = |cx: &mut gpui::VisualTestContext| {
        let (view, boxes, rows) = shell.read_with(cx, |s, cx| {
            // An item's bounds are where it lies unscrolled: the offset brings them to the window.
            let shift = point(px(0.), s.help.scroll.offset().y);
            let boxes: Vec<_> = (0..s.help_rows(cx).len())
                .map(|i| s.help.scroll.bounds_for_item(i).unwrap() + shift)
                .collect();
            (s.help.scroll.bounds(), boxes, s.help_rows(cx))
        });
        let mid = rows
            .iter()
            .zip(&boxes)
            .filter(|(_, b)| b.top() < view.bottom() && b.bottom() > view.top())
            // Diagrams and Typing have no shortcut line left (text-only rows):
            // nothing to point at, so they sit the check out.
            .filter(|(section, _)| section.1.iter().any(|row| row.action.is_some()))
            .collect::<Vec<_>>();
        // Frames parked wholly inside a text-only section (Diagrams, Typing:
        // no shortcut line) show no actionable section,
        // so there is nothing to check on that frame.
        if mid.is_empty() {
            return view.center();
        }
        // gpui keeps the debug bounds of earlier frames: the key must be inside its section, in this frame.
        for (section, at) in mid {
            let action = section
                .1
                .iter()
                .find_map(|row| row.action)
                .expect("a section with a shortcut");
            let key = cx.debug_bounds(leak(format!("key-{action}")));
            assert!(
                key.is_some_and(|key| at.contains(&key.center())),
                "{} is drawn in view: {key:?} in {at:?}",
                section.0
            );
        }
        view.center()
    };
    let centre = drawn(cx);
    for _ in 0..60 {
        cx.simulate_event(gpui::ScrollWheelEvent {
            position: centre,
            delta: gpui::ScrollDelta::Pixels(point(px(0.), px(300.))),
            ..Default::default()
        });
        cx.run_until_parked();
        drawn(cx);
        if scroll(cx) == px(0.) {
            break;
        }
    }
    assert_eq!(scroll(cx), px(0.), "back at the top");
    cx.simulate_keystrokes("escape escape");

    // The theme's default says what it is.
    cx.dispatch_action(ChooseTheme);
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, cx| s
            .palette
            .as_ref()
            .and_then(|p| p.read(cx).choice_at(0))),
        Some("Default: Inkuno, light or dark like the system".to_string())
    );
    cx.simulate_keystrokes("escape");

    // Two panes: a ring around the active one's sheet.
    cx.simulate_keystrokes("secondary-\\");
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    let ring = cx
        .debug_bounds("pane-active")
        .expect("the active pane is marked");
    let active = shell.read_with(cx, |s, _| s.active);
    let pane = cx.debug_bounds(leak(format!("pane-{active}"))).unwrap();
    assert_eq!(ring, pane, "the ring runs along the sheet's edge");
}

/// The rail reads as three groups (navigation, views, actions) then the bottom ones: in a 900, 720 and
/// 480 px high window the buttons come in that order, none overlaps the next, and the groups are a
/// button apart (at 480 px the rail scrolls, so the layout is the same).
#[gpui::test]
fn rail_groups_keep_their_order_and_room(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-rail-groups-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (_shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx.run_until_parked();
    let order = ["nav-commands", "nav-tree", "nav-search", "nav-new"];
    for height in [900., 720., 480.] {
        cx.simulate_resize(size(px(860.), px(height)));
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
        let b: Vec<_> = order
            .iter()
            .map(|id| {
                cx.debug_bounds(id)
                    .unwrap_or_else(|| panic!("{id} is drawn at {height}"))
            })
            .collect();
        for (pair, ids) in b.windows(2).zip(order.windows(2)) {
            assert!(
                pair[0].bottom() <= pair[1].top(),
                "{} overlaps {} at {height}",
                ids[0],
                ids[1]
            );
        }
        let gap = |from: &str, to: &str| {
            let at = |id: &str| b[order.iter().position(|o| *o == id).unwrap()];
            at(to).top() - at(from).bottom()
        };
        for (from, to) in [("nav-commands", "nav-tree"), ("nav-tree", "nav-search")] {
            assert!(
                gap(from, to) >= px(24.),
                "{from} and {to} are not set apart at {height}"
            );
        }
        assert!(
            gap("nav-search", "nav-new") < px(8.),
            "find and new stay one group at {height}"
        );
    }
}

/// Ctrl+Shift+S hides the bar the app draws, with its window buttons. The rail keeps a button
/// that brings it back, and the choice survives a restart (the layout file).
#[gpui::test]
fn the_title_bar_hides_and_comes_back_from_the_rail(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-title-bar-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx.run_until_parked();
    // The test platform draws no bar of its own: this one is the app's, as on Linux.
    shell.update(cx, |s, _| s.client = true);
    // The bar the app draws leaves a strip at the top of the note when it stands alone (the
    // strip is what the frame laid out, so it says what is on screen). Its window buttons are
    // not read through `debug_bounds`: gpui keeps those of a frame no longer drawn.
    let drawn = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).strip) > px(0.)
    };
    assert!(drawn(cx), "the bar is drawn at first");

    cx.simulate_keystrokes("secondary-shift-s");
    assert!(!shell.read_with(cx, |s, _| s.nav.title_bar));
    assert!(!drawn(cx), "the bar is hidden: no strip under it");
    assert!(vault::load_layout().contains("titlebar=hidden"));

    // The rail's button brings the bar back.
    let at = cx
        .debug_bounds("rail-titlebar")
        .expect("the rail keeps the way back")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    assert!(shell.read_with(cx, |s, _| s.nav.title_bar));
    assert!(drawn(cx), "the bar is back");

    // Hidden again: a restart opens without it.
    cx.simulate_keystrokes("secondary-shift-s");
    let (shell, cx) =
        cx.add_window_view(|window, cx| Shell::new(Some(root.clone()), Vec::new(), window, cx));
    cx.run_until_parked();
    assert!(
        !shell.read_with(cx, |s, _| s.nav.title_bar),
        "the choice survives a restart"
    );
    let _ = fs::remove_dir_all(&root);
}

/// F9 gives the note the window alone, edge to edge: no rail, no docks, no tab bar, no bar drawn
/// by the app, no frame inset. F9 again, or Esc in the note, puts back exactly the chrome there was.
#[gpui::test]
fn focus_mode_gives_the_note_the_window_and_restores_the_chrome(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "focus-mode");
    fs::write(root.join("A.md"), "# A\n\nun\n").unwrap();
    fs::write(root.join("B.md"), "# B\n\ndeux\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    for name in ["A", "B"] {
        shell.update_in(cx, |s, window, cx| {
            s.open_from_nav(&root.join(format!("{name}.md")), window, cx)
        });
    }
    shell.update(cx, |s, _| {
        s.client = true;
        s.nav.panel = nav::Panel::Split;
        s.nav.right_shown = vec![nav::Mode::Outline];
    });
    cx.run_until_parked();
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    let chrome = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, _| {
            (
                s.nav.panel,
                s.nav.left_shown.clone(),
                s.nav.right_shown.clone(),
                s.nav.title_bar,
                s.groups[0].tabs.len(),
            )
        })
    };
    let in_note = |cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| {
            shell
                .read(cx)
                .pane()
                .editor
                .focus_handle(cx)
                .is_focused(window)
        })
    };
    settle(cx);
    assert_eq!(chrome(cx).4, 2, "two tabs: a tab bar");
    let before = chrome(cx);
    let normal = cx.debug_bounds("pane-0").expect("the note is drawn");
    assert!(
        normal.origin.x > px(0.) && normal.origin.y > px(0.),
        "with the chrome, the note starts inside the window: {normal:?}"
    );

    cx.simulate_keystrokes("f9");
    settle(cx);
    let note = cx.debug_bounds("pane-0").expect("the note is drawn");
    let window_size = cx.update(|window, _| window.viewport_size());
    assert_eq!(note.origin, point(px(0.), px(0.)), "edge to edge: {note:?}");
    assert_eq!(note.size, window_size, "the note fills the window");
    assert_eq!(
        cx.update(|window, _| window.client_inset()),
        Some(px(0.)),
        "no shadow margin around the window in focus mode"
    );
    assert_eq!(chrome(cx), before, "the chrome is not changed to hide it");

    cx.simulate_keystrokes("f9");
    settle(cx);
    assert_eq!(chrome(cx), before);
    assert_eq!(
        cx.debug_bounds("pane-0").expect("the note is drawn"),
        normal,
        "F9 again: the note is where it was"
    );

    // From the tree, F9 gives the keys back to the note; Esc in the note leaves the mode.
    cx.update(|window, cx| {
        let tree = shell.read(cx).nav.focus.clone();
        window.focus(&tree);
    });
    cx.simulate_keystrokes("f9");
    assert!(
        in_note(cx),
        "the keys leave the dock that is no longer drawn"
    );
    assert!(shell.read_with(cx, |s, _| s.focus_mode));
    cx.simulate_keystrokes("escape");
    assert!(
        !shell.read_with(cx, |s, _| s.focus_mode),
        "Esc in the note exits"
    );
    settle(cx);
    assert_eq!(chrome(cx), before);
}

/// Normal mode is edge to edge: the first note starts at the content's origin and two panes tile with the
/// hairline between them and no gap. The bar the app draws (`client`) puts the note's strip at the top of
/// the content; hidden (`app::ToggleTitleBar`), the note starts at the content's top with no strip. The
/// content's origin is the window's corner, less the frame the app draws on Linux: its margin (`client_inset`)
/// and the 1 px border of the framed content.
#[gpui::test]
fn normal_mode_starts_at_the_content_origin_and_tiles_with_a_hairline(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "origin");
    fs::write(root.join("Une.md"), "# Une\n\ntexte\n").unwrap();
    fs::write(root.join("Deux.md"), "# Deux\n\ntexte\n").unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| {
        s.open_from_nav(&root.join("Une.md"), window, cx)
    });
    shell.update_in(cx, |s, window, cx| {
        s.open_aside(&root.join("Deux.md"), window, cx)
    });
    shell.update(cx, |s, cx| {
        s.nav.panel = nav::Panel::Split;
        cx.notify();
    });
    let settle = |cx: &mut gpui::VisualTestContext| {
        for _ in 0..3 {
            cx.update(|window, _| window.refresh());
            cx.run_until_parked();
        }
    };
    settle(cx);

    // The test platform draws no frame: the content's origin is the window's corner.
    let first = cx.debug_bounds("pane-0").expect("the first note is drawn");
    let second = cx.debug_bounds("pane-1").expect("the second note is drawn");
    assert_eq!(first.origin.y, px(0.), "{first:?}");
    assert_eq!(
        second.left() - first.right(),
        nav::DIVIDER,
        "{first:?} {second:?}"
    );
    assert_eq!(
        (first.top(), first.bottom()),
        (second.top(), second.bottom()),
        "the panes share their top and bottom edges"
    );

    // With the bar the app draws: the note's strip sits at the content's top.
    shell.update(cx, |s, _| s.client = true);
    settle(cx);
    let inset = cx
        .update(|window, _| window.client_inset())
        .unwrap_or(px(0.))
        + px(1.);
    let note = cx.debug_bounds("pane-0").expect("the first note is drawn");
    assert_eq!(
        note.top(),
        inset,
        "the first pane starts at the content's top"
    );
    let strip = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).strip)
    };
    assert!(strip(cx) > px(0.), "the bar is drawn: its strip is kept");

    // Hidden: no strip, and the note starts at the content's top.
    cx.simulate_keystrokes("secondary-shift-s");
    settle(cx);
    assert_eq!(strip(cx), px(0.), "hidden: no strip");
    let note = cx.debug_bounds("pane-0").expect("the first note is drawn");
    assert_eq!(
        note.top() - inset,
        px(0.),
        "hidden: the note starts at the content's top"
    );
    let _ = fs::remove_dir_all(&root);
}
