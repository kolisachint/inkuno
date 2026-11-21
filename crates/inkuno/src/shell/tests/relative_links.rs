//! Relative links `[text](note.md#heading)` open their note at the heading, and follow a note renamed.

use super::*;
use crate::logic::markdown::{self, Link};

fn fixtures(root: &Path) {
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(
        root.join("Home.md"),
        "# Home\n\n[x](sub/Target.md#Second%20part)\n[y](Missing.md)\n[z](#fin)\n\n## Fin\n",
    )
    .unwrap();
    fs::write(
        root.join("sub/Target.md"),
        "# Target\n\ntext\n\n## Second part\n\nmore\n",
    )
    .unwrap();
    fs::write(root.join("sub/Up.md"), "# Up\n\n[back](../Home.md)\n").unwrap();
}

/// The link a line holds, as the editor reads it on a click.
fn link(line: &str) -> Link {
    markdown::links(line).remove(0).1
}

fn shown(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Option<PathBuf> {
    shell.read_with(cx, |s, _| s.pane().path.clone())
}

fn caret(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> usize {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).caret())
}

#[gpui::test]
fn a_relative_link_opens_its_note_at_its_heading(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "links-follow");
    fixtures(&root);
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update(cx, |s, cx| s.open_note(&root.join("Home.md"), cx));
    cx.run_until_parked();

    // A note of a folder, with the heading in its address.
    let text = fs::read_to_string(root.join("sub/Target.md")).unwrap();
    shell.update_in(cx, |s, window, cx| {
        s.follow(&link("[x](sub/Target.md#Second%20part)"), window, cx)
    });
    cx.run_until_parked();
    assert_eq!(shown(&shell, cx), Some(root.join("sub/Target.md")));
    assert_eq!(caret(&shell, cx), text.find("## Second part").unwrap());

    // `../` counts from the folder of the note the link is in; `/` from the vault root.
    shell.update_in(cx, |s, window, cx| {
        s.follow(&link("[back](../Home.md)"), window, cx)
    });
    cx.run_until_parked();
    assert_eq!(shown(&shell, cx), Some(root.join("Home.md")));
    shell.update_in(cx, |s, window, cx| {
        s.follow(&link("[x](/sub/Up.md)"), window, cx)
    });
    cx.run_until_parked();
    assert_eq!(shown(&shell, cx), Some(root.join("sub/Up.md")));

    // A heading of the note shown: its line is the one the link jumps to.
    shell.update(cx, |s, cx| s.open_note(&root.join("Home.md"), cx));
    cx.run_until_parked();
    shell.update_in(cx, |s, window, cx| s.follow(&link("[z](#fin)"), window, cx));
    cx.run_until_parked();
    let home = fs::read_to_string(root.join("Home.md")).unwrap();
    assert_eq!(caret(&shell, cx), home.find("## Fin").unwrap());
    assert_eq!(shown(&shell, cx), Some(root.join("Home.md")));

    // A note that is not in the vault is not created: a notice names it instead.
    shell.update_in(cx, |s, window, cx| s.follow(&link("[y](Missing.md)"), window, cx));
    cx.run_until_parked();
    assert!(!root.join("Missing.md").exists());
    assert_eq!(shown(&shell, cx), Some(root.join("Home.md")));
    let noticed = shell.read_with(cx, |s, _| {
        s.toasts.iter().any(|toast| toast.text.contains("Missing.md"))
    });
    assert!(noticed);
}

#[gpui::test]
fn a_renamed_note_takes_its_relative_links_along(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "links-rename");
    fixtures(&root);
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update(cx, |s, cx| s.open_note(&root.join("Home.md"), cx));
    cx.run_until_parked();

    // The file moves in its folder, then the links follow it: the address keeps its form.
    let (from, to) = (root.join("sub/Target.md"), root.join("sub/Renamed.md"));
    fs::rename(&from, &to).unwrap();
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update(cx, |s, cx| s.relink("Target", "Renamed", Some((&from, &to)), cx));
    cx.run_until_parked();
    let home = fs::read_to_string(root.join("Home.md")).unwrap();
    assert!(home.contains("[x](sub/Renamed.md#Second%20part)"), "{home}");
    assert!(home.contains("[y](Missing.md)"), "{home}");
    let up = fs::read_to_string(root.join("sub/Up.md")).unwrap();
    assert_eq!(up, "# Up\n\n[back](../Home.md)\n");
}
