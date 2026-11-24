//! The file line numbers of the editor's gutter (`app-ui.md` §23.7): the note's own lines, a wrapped line
//! numbered once, code blocks without their own numbers while these show, and a setting that hides them.

use super::*;
use crate::shell::settings_tab::Section;

#[gpui::test]
fn the_gutter_numbers_the_file_lines_and_a_setting_hides_them(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "line-numbers");
    // Line 6 is long enough to wrap onto several visual rows: it is numbered once.
    let long = "A paragraph long enough to wrap onto several visual rows at this width, so that its one \
                source line is numbered once and not once for each row it takes on the screen.";
    let text =
        format!("---\ntitle: Front\n---\n# Title\n\n{long}\n\n```rust\nfn main() {{}}\n```\n");
    fs::write(root.join("Note.md"), &text).unwrap();
    shell.update(cx, |s, cx| {
        s.rescan(&root, cx);
        s.open_note(&root.join("Note.md"), cx);
    });
    cx.run_until_parked();
    let numbers = |cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, cx| s.panes[0].editor.read(cx).file_line_numbers())
    };
    // On by default: the lines that are drawn carry the file's numbers, the front matter included (it is shown).
    // The blank lines (5, 7 and the empty last line 11) are collapsed to a 9 px gap and carry none.
    assert_eq!(numbers(cx), [1, 2, 3, 4, 6, 8, 9, 10]);
    // While the numbers show, the code block draws none of its own.
    assert!(!shell.read_with(cx, |s, _| s.theme.numbered));

    // Switched off in the Editor page: no numbers, the code block's come back, and the file keeps it.
    cx.simulate_keystrokes("secondary-,");
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| {
        s.set_section(Section::Editor, window, cx)
    });
    cx.run_until_parked();
    let at = cx
        .debug_bounds("line_numbers")
        .expect("the Line numbers switch")
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    cx.run_until_parked();
    assert_eq!(
        shell.read_with(cx, |s, _| s.prefs.line_numbers.clone()),
        "off"
    );
    assert!(vault::load_settings().contains("line_numbers=off\n"));
    assert!(numbers(cx).is_empty());
    assert!(shell.read_with(cx, |s, _| s.theme.numbered));
}
