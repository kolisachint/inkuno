//! Status messages and links.

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let shell = &j.shell;
    let text = |cx: &mut gpui::VisualTestContext| j.text(cx);
    let caret = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position())
    };
    // An empty note stays empty: a click in the note lands the cursor, nothing else.
    j.load(cx, "untitled", 0);
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    assert_eq!(caret(cx), 0);
    // No headings and no comments: the comments palette says so instead of opening, which
    // leaves the status message the clicks below dismiss.
    cx.simulate_keystrokes("secondary-p");
    cx.simulate_input("comments");
    cx.simulate_keystrokes("down enter");
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()
        && s.told(Tone::Info, "no comments")));
    // Status messages: a click removes the one it targets, the others go by themselves after four
    // seconds; the one of an operation in progress stays, until the message that gives its outcome.
    shell.update(cx, |s, cx| s.say(Tone::Busy, "Working", cx));
    let view = cx.update(|window, _| window.viewport_size());
    cx.simulate_click(
        point(view.width - px(60.), view.height - px(70.)),
        gpui::Modifiers::none(),
    );
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Info, "no comments")
        && !s.told(Tone::Busy, "")));
    shell.update(cx, |s, cx| s.say(Tone::Busy, "Working", cx));
    cx.executor().advance_clock(TOAST);
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.toasts.len() == 1
        && s.told(Tone::Busy, "Working")));
    shell.update(cx, |s, cx| s.say(Tone::Done, "Done", cx));
    assert!(shell.read_with(cx, |s, _| s.toasts.len() == 1 && s.told(Tone::Done, "Done")));
    cx.executor().advance_clock(TOAST);
    cx.run_until_parked();
    assert!(shell.read_with(cx, |s, _| s.toasts.is_empty()));

    // Links: an address pasted on a selection makes it a link; Ctrl+K on a link
    // selects its address, to retype; Ctrl+K on a selection waits for the address.
    j.load(cx, "see the doc here", 5);
    shell.update_in(cx, |s, window, cx| {
        window.focus(&s.pane().editor.focus_handle(cx))
    });
    cx.simulate_keystrokes(
        "shift-right shift-right shift-right shift-right shift-right shift-right",
    );
    cx.write_to_clipboard(ClipboardItem::new_string("https://a.b/c\n".into()));
    cx.simulate_keystrokes("secondary-v");
    assert_eq!(text(cx), "see [the doc](https://a.b/c) here");
    cx.simulate_keystrokes("secondary-k");
    cx.simulate_input("https://x.y");
    assert_eq!(text(cx), "see [the doc](https://x.y) here");
    cx.simulate_keystrokes("secondary-z secondary-z");
    assert_eq!(text(cx), "see the doc here");
    j.load(cx, "word", 0);
    cx.write_to_clipboard(ClipboardItem::new_string("none".into()));
    cx.simulate_keystrokes("shift-right shift-right shift-right secondary-k");
    cx.simulate_input("https://z.fr");
    assert_eq!(text(cx), "[word](https://z.fr)");
}
