//! Editing journeys: `/` commands, block handles, inline formatting and its bar.

use super::*;

/// What can be done is found without knowing it: the command palette (F10, or ">"), which
/// searches them by what they do, the view's first; help opened on the view; a table's bar.
#[gpui::test]
fn commands_are_found(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-commands-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view({
        let root = root.clone();
        |window, cx| Shell::new(Some(root), Vec::new(), window, cx)
    });
    cx.run_until_parked();
    let text = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
    };
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.pane().editor.focus_handle(cx))
    });
    cx.simulate_input("mot");
    let runs = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| {
            s.palette
                .as_ref()
                .map(|p| p.read(cx).runs())
                .unwrap_or_default()
        })
    };
    let mode = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.palette.as_ref().map(|p| p.read(cx).mode()))
    };

    // F10 opens the command palette: only the commands the note can do (not those
    // of a grid), under the name of what is being viewed.
    cx.simulate_keystrokes("f10");
    assert_eq!(mode(cx), Some((true, "Note")));
    let all = runs(cx);
    assert!(
        all.contains(&"app::NewNote") && all.contains(&"editor::Bold"),
        "{all:?}"
    );
    assert!(!all.contains(&"sheet::Copy") && !all.contains(&"canvas::Erase"));
    // Escape closes it and gives input back to the note.
    cx.simulate_keystrokes("escape");
    assert_eq!(mode(cx), None);
    assert!(shell.update_in(cx, |s, window, cx| {
        s.pane().editor.focus_handle(cx).is_focused(window)
    }));
    // Typing filters by what the command does; Enter runs it, on the note.
    cx.simulate_keystrokes("secondary-a f10");
    cx.simulate_input("bold");
    assert_eq!(runs(cx), ["editor::Bold"]);
    cx.simulate_keystrokes("enter");
    assert_eq!(mode(cx), None);
    assert_eq!(text(cx), "**mot**");
    // The rail's first button opens it too; a click on a command runs it.
    let at = cx
        .debug_bounds("nav-commands")
        .expect("the rail button")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    assert_eq!(mode(cx).map(|m| m.0), Some(true));
    cx.simulate_input("shortcuts");
    let at = cx
        .debug_bounds("command-app::ToggleHelp")
        .expect("the command is shown")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    assert!(shell.read_with(cx, |s, _| s.palette.is_none() && s.help.open));
    shell.update_in(cx, |s, window, cx| s.set_help(false, window, cx));

    // In the notes palette, a leading ">" switches to commands, and is erased; without it, a command
    // is found too, but Enter still creates the note of that name.
    cx.simulate_keystrokes("secondary-z secondary-a secondary-p");
    cx.simulate_input("italic");
    assert_eq!(runs(cx), ["editor::Italic"]);
    cx.simulate_keystrokes("home");
    cx.simulate_input(">");
    assert_eq!(mode(cx), Some((true, "Note")));
    assert_eq!(
        shell.read_with(cx, |s, cx| s.palette.as_ref().unwrap().read(cx).listed()),
        1
    );
    cx.simulate_keystrokes("enter");
    assert_eq!(text(cx), "*mot*");
    fs::remove_dir_all(&root).unwrap();
}

/// Block handles: hovering a line, dragging the handle moves its block,
/// a click opens its menu, the "+" puts a block below with the `/` list; Alt+arrow moves it.
#[gpui::test]
fn block_handles(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-blocks-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view({
        let root = root.clone();
        |window, cx| Shell::new(Some(root), Vec::new(), window, cx)
    });
    cx.run_until_parked();
    let note = "# Titre\n\nUn\n\nDeux\n\n- a\n    - sous\n- b\n";
    let text = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
    };
    let load = |cx: &mut gpui::VisualTestContext, text: &str| {
        shell.update_in(cx, |s, window, cx| {
            s.pane()
                .editor
                .update(cx, |e, cx| e.load(text.to_string(), 0, cx));
            window.focus(&s.pane().editor.focus_handle(cx));
        });
        cx.run_until_parked();
    };
    load(cx, note);
    let none = gpui::Modifiers::none();
    let spot = |cx: &mut gpui::VisualTestContext, needle: &str| {
        let at = text(cx).find(needle).unwrap();
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).point_of(at).unwrap())
            + gpui::point(px(3.), px(0.))
    };
    let hover = |cx: &mut gpui::VisualTestContext, needle: &str| {
        let at = spot(cx, needle);
        cx.simulate_mouse_move(at, None, none);
        shell.read_with(cx, |s, cx| {
            s.pane().editor.read(cx).hovered_block().map(str::to_string)
        })
    };
    let handle = |cx: &mut gpui::VisualTestContext| {
        shell
            .read_with(cx, |s, cx| s.pane().editor.read(cx).block_handle())
            .expect("the handle is there")
    };

    // Hovering a line shows its block's handle: an item carries its sub-items. The
    // title, which names the note, has none.
    assert_eq!(hover(cx, "Titre"), None);
    assert_eq!(hover(cx, "Un").as_deref(), Some("Un"));
    assert_eq!(hover(cx, "- a").as_deref(), Some("- a\n    - sous"));
    assert_eq!(hover(cx, "sous").as_deref(), Some("    - sous"));
    // A sub-item's handle is at its indent.
    let inner = handle(cx).0;
    hover(cx, "- b");
    assert!(inner.left() > handle(cx).0.left() + px(10.), "{inner:?}");
    // Dragged to the top of another line, the handle drops the block there, with its blank lines.
    hover(cx, "Un");
    let (grip, plus) = handle(cx);
    assert!(plus.is_some_and(|plus| plus.right() <= grip.left()));
    let to = spot(cx, "- a") - gpui::point(px(0.), px(8.));
    cx.simulate_mouse_down(grip.center(), MouseButton::Left, none);
    cx.simulate_mouse_move(
        grip.center() + gpui::point(px(0.), px(10.)),
        MouseButton::Left,
        none,
    );
    cx.simulate_mouse_move(to, MouseButton::Left, none);
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    assert_eq!(text(cx), "# Titre\n\nDeux\n\nUn\n\n- a\n    - sous\n- b\n");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(cx), note);
    // An item drops among the others, without a blank line.
    hover(cx, "- b");
    let (grip, _) = handle(cx);
    let to = spot(cx, "- a") - gpui::point(px(0.), px(8.));
    cx.simulate_mouse_down(grip.center(), MouseButton::Left, none);
    cx.simulate_mouse_move(to, MouseButton::Left, none);
    cx.simulate_mouse_up(to, MouseButton::Left, none);
    assert_eq!(text(cx), "# Titre\n\nUn\n\nDeux\n\n- b\n- a\n    - sous\n");
    cx.simulate_keystrokes("secondary-z");

    // A click on the handle takes the block and opens its menu; "Titre 2" turns it into a heading.
    hover(cx, "Deux");
    let (grip, _) = handle(cx);
    cx.simulate_click(grip.center(), none);
    assert!(shell.read_with(cx, |s, _| s.menu.as_ref().is_some_and(|m| m.block)));
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).selected().to_string()),
        "Deux"
    );
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(
            nav::Do::Block(editor::BlockOp::Turn(block::Turn::Heading(2))),
            None,
            window,
            cx,
        )
    });
    assert_eq!(
        text(cx),
        "# Titre\n\nUn\n\n## Deux\n\n- a\n    - sous\n- b\n"
    );
    // The same block, still targeted: duplicated, then deleted.
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::Block(editor::BlockOp::Duplicate), None, window, cx)
    });
    assert_eq!(
        text(cx),
        "# Titre\n\nUn\n\n## Deux\n\n## Deux\n\n- a\n    - sous\n- b\n"
    );
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::Block(editor::BlockOp::Delete), None, window, cx)
    });
    cx.simulate_keystrokes("secondary-z secondary-z secondary-z");
    assert_eq!(text(cx), note);

    // Alt+Down moves the cursor's block down one block, Alt+Up moves it back up; an item, among items.
    load(cx, note);
    let place = |cx: &mut gpui::VisualTestContext, needle: &str| {
        let at = note.find(needle).unwrap();
        shell.update(cx, |s, cx| {
            s.pane().editor.update(cx, |e, cx| e.place_cursor(at, cx))
        });
    };
    place(cx, "Un");
    cx.simulate_keystrokes("alt-down");
    assert_eq!(text(cx), "# Titre\n\nDeux\n\nUn\n\n- a\n    - sous\n- b\n");
    cx.simulate_keystrokes("alt-up");
    assert_eq!(text(cx), note);
    cx.simulate_keystrokes("alt-up");
    assert_eq!(text(cx), note, "nothing goes above the title");
    place(cx, "- a");
    cx.simulate_keystrokes("alt-down");
    assert_eq!(text(cx), "# Titre\n\nUn\n\nDeux\n\n- b\n- a\n    - sous\n");

    // The "+" puts a line below the block, spaced like its neighbours, and opens the `/` list.
    load(cx, note);
    hover(cx, "Un");
    let (_, plus) = handle(cx);
    cx.simulate_click(plus.unwrap().center(), none);
    assert_eq!(
        text(cx),
        "# Titre\n\nUn\n\n/\n\nDeux\n\n- a\n    - sous\n- b\n"
    );
    cx.simulate_input("h1");
    cx.simulate_keystrokes("enter");
    assert!(text(cx).starts_with("# Titre\n\nUn\n\n# "), "{}", text(cx));
    fs::remove_dir_all(&root).unwrap();
}

/// Aliases for chorded actions the catalogue does not name: the multicursor alias is offered
/// in a note, the column-align aliases only where a table is under the cursor.
#[gpui::test]
fn command_aliases_follow_the_view(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-alias-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(
        root.join("Note.md"),
        "text\n\n| a | b |\n| --- | --- |\n| 1 | 2 |\n",
    )
    .unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view({
        let root = root.clone();
        |window, cx| Shell::new(Some(root), Vec::new(), window, cx)
    });
    cx.run_until_parked();
    shell.update(cx, |s, cx| s.open_note(&root.join("Note.md"), cx));
    cx.run_until_parked();
    let runs = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| {
            s.palette
                .as_ref()
                .map(|p| p.read(cx).runs())
                .unwrap_or_default()
        })
    };
    // In plain text: the multicursor alias, but no column-align aliases.
    cx.simulate_keystrokes("secondary-home f10");
    assert!(runs(cx).contains(&"editor::SelectNext"), "{:?}", runs(cx));
    assert!(!runs(cx).contains(&"editor::AlignLeft"), "{:?}", runs(cx));
    // Typed by what it does: `multicurs` starts both the catalogue line and its alias.
    cx.simulate_input("multicurs");
    assert_eq!(runs(cx), ["editor::SelectNext"]);
    cx.simulate_keystrokes("escape escape");
    // Block transforms are ›-reachable too (overrule of the /+grip-only stance): offered
    // in a note, found by what they do.
    cx.simulate_keystrokes("f10");
    let blocks = [
        "editor::BlockHeading1",
        "editor::BlockHeading2",
        "editor::BlockHeading3",
        "editor::BlockBullet",
        "editor::BlockOrdered",
        "editor::BlockTask",
        "editor::BlockQuote",
        "editor::BlockCode",
        "editor::BlockDuplicate",
    ];
    for action in blocks {
        assert!(
            runs(cx).contains(&action),
            "{action} missing from {:?}",
            runs(cx)
        );
    }
    cx.simulate_keystrokes("escape");
    crate::lang::in_english(|| {
        for (typed, found) in [
            (
                "heading",
                vec![
                    "editor::BlockHeading1",
                    "editor::BlockHeading2",
                    "editor::BlockHeading3",
                ],
            ),
            ("bulleted", vec!["editor::BlockBullet"]),
            ("numbered", vec!["editor::BlockOrdered"]),
            ("task list", vec!["editor::BlockTask"]),
            ("quote", vec!["editor::BlockQuote"]),
            ("code block", vec!["editor::BlockCode"]),
            ("duplicate", vec!["editor::BlockDuplicate"]),
        ] {
            cx.simulate_keystrokes("f10");
            cx.simulate_input(typed);
            assert_eq!(runs(cx), found, "typed {typed:?}");
            cx.simulate_keystrokes("escape");
        }
    });
    // In the table itself (header row): the align family joins the table gestures.
    cx.simulate_keystrokes("secondary-home down down f10");
    let table = runs(cx);
    for action in [
        "editor::AlignLeft",
        "editor::AlignCenter",
        "editor::AlignRight",
        "editor::TableRowAbove",
    ] {
        assert!(table.contains(&action), "{action} missing from {table:?}");
    }
    // Block group rules still hold there: the block transforms stay in notes.
    for action in [
        "editor::BlockHeading1",
        "editor::BlockBullet",
        "editor::BlockDuplicate",
    ] {
        assert!(
            !table.contains(&action),
            "{action} should not be in {table:?}"
        );
    }
    cx.simulate_keystrokes("escape");
    let _ = fs::remove_dir_all(&root);
}

/// Text formatting: bold, italic, underline, strikethrough, highlight, code, by keyboard and by
/// the bottom bar, with their limits (cursor alone, blanks, several lines, code, Backspace).
#[gpui::test]
fn formatting_journey(cx: &mut TestAppContext) {
    let root = std::env::temp_dir().join(format!("inkuno-fmt-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Note.md"), "x").unwrap();
    let _config = isolated_config(&root);
    cx.update(bind_keys);
    cx.update(init_fonts);
    let (shell, cx) = cx.add_window_view({
        let root = root.clone();
        |window, cx| Shell::new(Some(root), Vec::new(), window, cx)
    });
    cx.simulate_resize(size(px(900.), px(600.)));
    cx.run_until_parked();
    let text = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
    };
    // The text loaded, then `len` characters selected from `from` (in characters).
    let pick = |cx: &mut gpui::VisualTestContext, note: &str, from: usize, len: usize| {
        let at = note.char_indices().nth(from).map_or(note.len(), |(i, _)| i);
        shell.update(cx, |s, cx| {
            s.pane()
                .editor
                .update(cx, |e, cx| e.load(note.to_string(), at, cx))
        });
        cx.run_until_parked();
        if len > 0 {
            cx.simulate_keystrokes(&"shift-right ".repeat(len));
        }
    };
    let selected = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).selected_text())
    };
    let styles = |cx: &mut gpui::VisualTestContext| {
        shell.update(cx, |s, cx| s.pane().editor.update(cx, |e, _| e.styles()))
    };
    // Each style: its marks around the word, the selection stays on the word, the same gesture removes them.
    for (key, shown, flag) in [
        ("secondary-b", "un **mot** ici", markdown::BOLD),
        ("secondary-i", "un *mot* ici", markdown::ITALIC),
        ("secondary-u", "un <u>mot</u> ici", markdown::UNDER),
        ("secondary-shift-x", "un ~~mot~~ ici", markdown::STRIKE),
        ("secondary-shift-h", "un ==mot== ici", markdown::HIGHLIGHT),
        ("secondary-shift-k", "un `mot` ici", markdown::CODE),
    ] {
        pick(cx, "un mot ici", 3, 3);
        assert_eq!(styles(cx), 0, "{key}");
        cx.simulate_keystrokes(key);
        assert_eq!(
            (text(cx), selected(cx).as_str()),
            (shown.to_string(), "mot"),
            "{key}"
        );
        assert_eq!(styles(cx), flag, "{key}: the button lights up");
        cx.simulate_keystrokes(key);
        assert_eq!(
            (text(cx), selected(cx).as_str()),
            ("un mot ici".to_string(), "mot"),
            "{key} removed"
        );
        // A single Backspace undoes the insertion; redo puts it back.
        cx.simulate_keystrokes(key);
        cx.simulate_keystrokes("secondary-z");
        assert_eq!(text(cx), "un mot ici", "{key} undone");
        cx.simulate_keystrokes("secondary-shift-z");
        assert_eq!(text(cx), shown, "{key} redone");
    }
    // Several styles on a word, then one in the middle is removed: the others stay.
    pick(cx, "un mot ici", 3, 3);
    cx.simulate_keystrokes("secondary-b secondary-u secondary-shift-h");
    assert_eq!(text(cx), "un **<u>==mot==</u>** ici");
    assert_eq!(
        styles(cx),
        markdown::BOLD | markdown::UNDER | markdown::HIGHLIGHT
    );
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "un <u>==mot==</u> ici");
    cx.simulate_keystrokes("secondary-u");
    assert_eq!(
        (text(cx), selected(cx).as_str()),
        ("un ==mot== ici".to_string(), "mot")
    );
    // Bold around italic: the cursor in the italic removes the bold without touching the italic.
    pick(cx, "**a *b* c**", 5, 0);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "a *b* c");
    // Cursor alone: two marks and the cursor between them; typing fills them; the next gesture removes them.
    pick(cx, "ab", 1, 0);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "a****b");
    cx.simulate_input("x");
    assert_eq!(text(cx), "a**x**b");
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "axb");
    // Empty pair: the gesture removes it instead of stacking another.
    pick(cx, "ab", 1, 0);
    cx.simulate_keystrokes("secondary-b secondary-b");
    assert_eq!(text(cx), "ab");
    // Edge blanks stay outside, the bullet and the heading too.
    pick(cx, "un mot ici", 3, 4);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "un **mot** ici");
    pick(cx, "- liste", 0, 7);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "- **liste**");
    pick(cx, "# Titre", 2, 5);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "# **Titre**");
    // Only blanks: nothing moves, and nothing to undo.
    pick(cx, "a   b", 1, 3);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "a   b");
    // Text in other scripts, accents, emojis, wide characters: the marks land on the right bytes.
    for note in ["日本語 é🙂", "é", "🙂", "a|b\\c \"d\""] {
        pick(cx, note, 0, note.chars().count());
        cx.simulate_keystrokes("secondary-b secondary-shift-h");
        assert_eq!(text(cx), format!("**=={note}==**"), "{note}");
        cx.simulate_keystrokes("secondary-shift-h secondary-b");
        assert_eq!(text(cx), note, "{note}");
    }
    // Several lines: each has its marks, except code blocks and blank lines; one gesture, one undo.
    let note = "- un\n- deux\n\n```\ncode\n```\nfin";
    pick(cx, note, 0, note.chars().count());
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "- **un**\n- **deux**\n\n```\ncode\n```\n**fin**");
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), note);
    cx.simulate_keystrokes("secondary-b secondary-z");
    assert_eq!(text(cx), note);
    // An unclosed code block is not one: its text gets the style.
    pick(cx, "```\nsuite", 4, 5);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "```\n**suite**");
    // Inside a code block, nothing changes.
    pick(cx, "```\ncode\n```", 5, 4);
    cx.simulate_keystrokes("secondary-b secondary-u");
    assert_eq!(text(cx), "```\ncode\n```");
    pick(cx, "```\ncode\n```", 6, 0);
    cx.simulate_keystrokes("secondary-b");
    assert_eq!(text(cx), "```\ncode\n```");
    // A table cell, and a comment: the selection stays valid there.
    pick(cx, "| a | b |\n| --- | --- |\n| c | d |", 26, 1);
    cx.simulate_keystrokes("secondary-b");
    assert!(text(cx).contains("| **c** |"), "{}", text(cx));
    // Several cursors: each gets its marks, a single undo removes them all.
    pick(cx, "un chat\nun chien", 0, 0);
    cx.simulate_keystrokes("secondary-d secondary-d secondary-u");
    assert_eq!(text(cx), "<u>un</u> chat\n<u>un</u> chien");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(cx), "un chat\nun chien");
    cx.simulate_keystrokes("escape");

    // No persistent bottom bar: neither the formatting bar nor its toggle is drawn — the module
    // stays (placement, tools and highlight colours for the palette), but at any width or zoom
    // the note shows no bar. Formatting goes through the keys and the palette.
    assert!(cx.debug_bounds("format-bar").is_none());
    assert!(cx.debug_bounds("format-toggle").is_none());
    assert!(shell.read_with(cx, |s, cx| !s.pane().editor.read(cx).bar_shown()));
    // The styles from the keys: applied to the selection, which stays; a second press removes them.
    let mark = |cx: &mut gpui::VisualTestContext, keys: &str| {
        cx.simulate_keystrokes(keys);
        cx.run_until_parked();
    };
    pick(cx, "un mot ici", 3, 3);
    mark(cx, "secondary-i");
    assert_eq!(text(cx), "un *mot* ici");
    mark(cx, "secondary-i");
    assert_eq!(text(cx), "un mot ici");
    pick(cx, "un mot ici", 3, 3);
    mark(cx, "secondary-b");
    assert_eq!(
        (text(cx), selected(cx).as_str()),
        ("un **mot** ici".to_string(), "mot")
    );
    assert_eq!(styles(cx), markdown::BOLD);
    mark(cx, "secondary-u");
    mark(cx, "secondary-shift-h");
    assert_eq!(
        (text(cx), selected(cx).as_str()),
        ("un **<u>==mot==</u>** ici".to_string(), "mot")
    );
    mark(cx, "secondary-b");
    mark(cx, "secondary-u");
    mark(cx, "secondary-shift-h");
    assert_eq!(text(cx), "un mot ici");
    mark(cx, "secondary-shift-x");
    mark(cx, "secondary-shift-k");
    assert_eq!(text(cx), "un ~~`mot`~~ ici");
    // The highlight from the keys: the default colour, on one line or several; one undo takes it back.
    pick(cx, "un mot ici", 3, 3);
    mark(cx, "secondary-shift-h");
    assert_eq!(text(cx), "un ==mot== ici");
    mark(cx, "secondary-shift-h");
    assert_eq!(text(cx), "un mot ici");
    pick(cx, "- a\n- b", 0, 7);
    mark(cx, "secondary-shift-h");
    assert_eq!(text(cx), "- ==a==\n- ==b==");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(cx), "- a\n- b");
    // The link from the keys: the selection becomes the text, the cursor waits for the address.
    pick(cx, "un mot ici", 3, 3);
    mark(cx, "secondary-k");
    assert_eq!(text(cx), "un [mot]() ici");
    // Lists from the keys: the cycle goes bullets, numbered, tasks, then back to text;
    // the palette offers the same restyles by name.
    pick(cx, "un mot ici\ndeux", 0, 14);
    mark(cx, "secondary-shift-b");
    assert_eq!(text(cx), "- un mot ici\n- deux");
    mark(cx, "secondary-shift-b");
    assert_eq!(text(cx), "1. un mot ici\n2. deux");
    mark(cx, "secondary-shift-b");
    assert_eq!(text(cx), "- [ ] un mot ici\n- [ ] deux");
    mark(cx, "secondary-shift-b");
    assert_eq!(text(cx), "un mot ici\ndeux");
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("bulleted");
    cx.simulate_keystrokes("down enter");
    cx.run_until_parked();
    assert_eq!(text(cx), "- un mot ici\n- deux");
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(cx), "un mot ici\ndeux");
    // The table from `/table`: Enter takes the grid's 3×3; its rows and columns from the palette.
    pick(cx, "avant\n", 6, 0);
    cx.simulate_input("/table");
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
    assert!(
        text(cx).contains("|  |  |  |\n| --- | --- | --- |") || text(cx).contains("| --- |"),
        "{}",
        text(cx)
    );
    assert!(shell.read_with(cx, |s, cx| s.pane().editor.read(cx).in_table()));
    cx.run_until_parked();
    let rows =
        |cx: &mut gpui::VisualTestContext| text(cx).lines().filter(|l| l.starts_with('|')).count();
    let columns = |cx: &mut gpui::VisualTestContext| {
        text(cx)
            .lines()
            .find(|l| l.starts_with('|'))
            .unwrap()
            .matches('|')
            .count()
            - 1
    };
    let (r, c) = (rows(cx), columns(cx));
    let grid = |cx: &mut gpui::VisualTestContext, typed: &str| {
        cx.simulate_keystrokes("secondary-p");
        cx.simulate_input(typed);
        cx.simulate_keystrokes("down enter");
        cx.run_until_parked();
    };
    grid(cx, "row below");
    assert_eq!((rows(cx), columns(cx)), (r + 1, c), "{:?}", text(cx));
    grid(cx, "column to the right");
    assert_eq!((rows(cx), columns(cx)), (r + 1, c + 1));
    grid(cx, "delete the row");
    grid(cx, "delete the column");
    assert_eq!((rows(cx), columns(cx)), (r, c));
    // A single undo gesture for each.
    cx.simulate_keystrokes("secondary-z");
    assert_eq!((rows(cx), columns(cx)), (r, c + 1));
    // No bar at any width: resizing never draws one, the text keeps its full height.
    for width in [1400., 900., 600., 320.] {
        cx.simulate_resize(size(px(width), px(600.)));
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        assert!(
            shell.read_with(cx, |s, cx| !s.pane().editor.read(cx).bar_shown()),
            "{width}"
        );
        assert!(cx.debug_bounds("format-bar").is_none(), "{width}");
        assert!(cx.debug_bounds("format-toggle").is_none(), "{width}");
    }
    cx.simulate_resize(size(px(900.), px(600.)));
    cx.run_until_parked();
    // Setting « Barre de mise en forme… »: the choice still survives a restart, but no bar is
    // drawn either way — the module stays for the palette, the note shows nothing.
    // An empty, missing or unknown value leaves the preference on.
    let choose = |cx: &mut gpui::VisualTestContext, keys: &str| {
        shell.update_in(cx, |s, window, cx| {
            s.choose_setting(Setting::Bar, window, cx)
        });
        cx.run_until_parked();
        cx.simulate_keystrokes(keys);
        cx.run_until_parked();
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
    };
    choose(cx, "down enter");
    assert!(shell.read_with(cx, |s, _| s.prefs.bar == "off" && !s.theme.bar));
    assert!(
        !shell.read_with(cx, |s, cx| s.pane().editor.read(cx).bar_shown())
            && cx.debug_bounds("counts").is_some()
    );
    assert!(vault::load_settings().contains("bar=off\n"));
    // Escape during the list: the previewed choice is taken back.
    choose(cx, "up escape");
    assert!(shell.read_with(cx, |s, _| s.prefs.bar == "off"));
    choose(cx, "up enter");
    assert!(shell.read_with(cx, |s, cx| s.theme.bar
        && !s.pane().editor.read(cx).bar_shown()));
    for (text, shown) in [
        ("", true),
        ("bar=\n", true),
        ("bar=on\n", true),
        ("bar=zzz\n", true),
        ("bar=off\n", false),
    ] {
        assert_eq!(Prefs::parse(text).bar != "off", shown, "{text:?}");
    }
    assert_eq!(Prefs::parse(&Prefs::parse("bar=off").to_text()).bar, "off");
    assert!(!Prefs::parse("").to_text().contains("bar="));
    let _ = fs::remove_dir_all(&root);
}
