//! The outline, status messages and links.

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let shell = &j.shell;
    let text = |cx: &mut gpui::VisualTestContext| j.text(cx);
    // Note outline: Ctrl+Shift+O opens the right dock on the headings (the cursor's section
    // first), the fenced heading excluded; a click jumps the cursor there and focuses the note,
    // Escape in the list focuses it back; folding hides a section; an empty note has an empty state.
    let plan = "# Un\n\ntexte\n\n## Deux\n\n```\n# code\n```\n\n## Deux\n\nfin\n";
    j.load(cx, plan, plan.find("texte").unwrap());
    let bounds = |cx: &mut gpui::VisualTestContext, name: &str| {
        cx.update(|window, _| window.refresh());
        cx.run_until_parked();
        cx.debug_bounds(leak(name.to_string()))
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
            (rows.len(), active)
        })
    };
    let caret = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position())
    };
    let editing = |cx: &mut gpui::VisualTestContext| {
        cx.update(|window, cx| {
            shell
                .read(cx)
                .pane()
                .editor
                .focus_handle(cx)
                .is_focused(window)
        })
    };
    cx.simulate_keystrokes("secondary-shift-o");
    assert!(bounds(cx, "dock-right").is_some());
    assert_eq!(outline(cx), (3, Some(0)));
    assert!(bounds(cx, "outline-empty").is_none());
    let row = bounds(cx, "outline-row-1").unwrap().center();
    cx.simulate_click(row, gpui::Modifiers::none());
    assert_eq!(caret(cx), plan.find("## Deux").unwrap());
    assert!(editing(cx));
    cx.simulate_keystrokes("secondary-shift-o");
    cx.simulate_keystrokes("escape");
    assert!(editing(cx));
    // Folding the first heading hides its section; unfolding brings it back.
    let fold = bounds(cx, "outline-fold-0").unwrap().center();
    cx.simulate_click(fold, gpui::Modifiers::none());
    assert_eq!(outline(cx).0, 1);
    let fold = bounds(cx, "outline-fold-0").unwrap().center();
    cx.simulate_click(fold, gpui::Modifiers::none());
    assert_eq!(outline(cx).0, 3);
    j.load(cx, "sans titre", 0);
    assert!(bounds(cx, "outline-empty").is_some());
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    assert_eq!(caret(cx), 0);
    // Status messages: a click removes the one it targets, the others go by themselves after four
    // seconds; the one of an operation in progress stays, until the message that gives its outcome.
    shell.update(cx, |s, cx| s.say(Tone::Busy, "En cours", cx));
    let view = cx.update(|window, _| window.viewport_size());
    cx.simulate_click(
        point(view.width - px(60.), view.height - px(70.)),
        gpui::Modifiers::none(),
    );
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Info, "titres")
        && !s.told(Tone::Busy, "")));
    shell.update(cx, |s, cx| s.say(Tone::Busy, "En cours", cx));
    cx.executor().advance_clock(TOAST);
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.toasts.len() == 1
        && s.told(Tone::Busy, "En cours")));
    shell.update(cx, |s, cx| s.say(Tone::Done, "Fini", cx));
    assert!(shell.read_with(cx, |s, _| s.toasts.len() == 1 && s.told(Tone::Done, "Fini")));
    cx.executor().advance_clock(TOAST);
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.toasts.is_empty()));

    // Links: an address pasted on a selection makes it a link; Ctrl+K on a link
    // selects its address, to retype; Ctrl+K on a selection waits for the address.
    j.load(cx, "voir la doc ici", 5);
    cx.simulate_keystrokes(
        "shift-right shift-right shift-right shift-right shift-right shift-right",
    );
    cx.write_to_clipboard(ClipboardItem::new_string("https://a.b/c\n".into()));
    cx.simulate_keystrokes("secondary-v");
    assert_eq!(text(cx), "voir [la doc](https://a.b/c) ici");
    cx.simulate_keystrokes("secondary-k");
    cx.simulate_input("https://x.y");
    assert_eq!(text(cx), "voir [la doc](https://x.y) ici");
    cx.simulate_keystrokes("secondary-z secondary-z");
    assert_eq!(text(cx), "voir la doc ici");
    j.load(cx, "mot", 0);
    cx.write_to_clipboard(ClipboardItem::new_string("rien".into()));
    cx.simulate_keystrokes("shift-right shift-right shift-right secondary-k");
    cx.simulate_input("https://z.fr");
    assert_eq!(text(cx), "[mot](https://z.fr)");
}
