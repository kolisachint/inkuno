//! Footnotes: Ctrl+click a reference jumps to its definition, the definition's label
//! cycles through its references, an undefined reference is a plain click.

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let shell = &j.shell;
    let note = "See [^a] here and [^a] again.\n\nBut [^x] stays.\n\n[^a]: the note.\n";
    j.load(cx, note, 0);
    let at = |s: &str| note.find(s).unwrap();
    let (first, missing, def) = (at("[^a]"), at("[^x]"), at("[^a]:"));
    let second = note[first + 1..].find("[^a]").unwrap() + first + 1;
    let held = gpui::Modifiers::secondary_key();
    let caret = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).position())
    };
    let click = |cx: &mut gpui::VisualTestContext, byte: usize, modifiers: gpui::Modifiers| {
        let spot = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).point_of(byte).unwrap());
        cx.simulate_click(spot, modifiers);
    };
    // Ctrl+clicking a reference jumps to its definition.
    click(cx, first + 2, held);
    assert_eq!(caret(cx), def);
    // Ctrl+clicking the definition's label cycles through its references.
    for want in [first, second, first] {
        click(cx, def + 2, held);
        assert_eq!(caret(cx), want);
    }
    // Ctrl+clicking an undefined reference is a plain click: no jump.
    click(cx, missing + 2, held);
    assert_eq!(caret(cx), missing + 2);
    // The pointer turns into a hand over defined footnotes only, with Ctrl held.
    let hand = |cx: &mut gpui::VisualTestContext, byte: usize, held: gpui::Modifiers| {
        let spot = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).point_of(byte).unwrap());
        cx.simulate_mouse_move(spot, None, held);
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).hand())
    };
    assert_eq!(hand(cx, first + 2, held), Some(CursorStyle::PointingHand));
    assert_eq!(hand(cx, def + 2, held), Some(CursorStyle::PointingHand));
    assert_eq!(hand(cx, missing + 2, held), None);
    assert_eq!(hand(cx, first + 2, gpui::Modifiers::none()), None);
}
