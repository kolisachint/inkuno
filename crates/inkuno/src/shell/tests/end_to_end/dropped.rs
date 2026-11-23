//! Files dropped on the note: copied into its `assets/` folder and linked, one undo step, never
//! overwritten. The drop is handed to the editor directly (a gpui drop cannot be built in a test).

use super::*;

pub(super) fn run(j: &Journey, cx: &mut gpui::VisualTestContext) {
    let (shell, root) = (&j.shell, &j.root);
    let text = |cx: &mut gpui::VisualTestContext| j.text(cx);
    let drop_at_end = |cx: &mut gpui::VisualTestContext, files: Vec<PathBuf>| {
        let at = j.text(cx).len();
        shell.update(cx, |s, cx| {
            s.pane()
                .editor
                .update(cx, |e, cx| e.drop_paths(&files, at, cx))
        });
        cx.run_until_parked();
    };
    let assets = root.join("assets");
    let depot = root.join("depot");
    fs::create_dir_all(&depot).unwrap();
    let png = b"\x89PNG\r\n\x1a\nfirst".to_vec();
    fs::write(depot.join("photo de classe.png"), &png).unwrap();

    j.load(cx, "# Titre\n\nTexte", 0);
    let before = text(cx);
    drop_at_end(cx, vec![depot.join("photo de classe.png")]);
    assert_eq!(
        text(cx),
        format!("{before}\n![photo de classe.png](assets/photo%20de%20classe.png)")
    );
    assert_eq!(fs::read(assets.join("photo de classe.png")).unwrap(), png);
    // One undo step takes the drop back.
    cx.simulate_keystrokes("secondary-z");
    assert_eq!(text(cx), before);

    // The same bytes again: the file is reused, no second copy.
    drop_at_end(cx, vec![depot.join("photo de classe.png")]);
    assert!(text(cx).ends_with("![photo de classe.png](assets/photo%20de%20classe.png)"));
    assert_eq!(fs::read_dir(&assets).unwrap().count(), 1);

    // Other bytes under the same name: the copy gets the next free name, the old file stays.
    let other = depot.join("other/photo de classe.png");
    fs::create_dir_all(other.parent().unwrap()).unwrap();
    fs::write(&other, b"\x89PNG second").unwrap();
    drop_at_end(cx, vec![other]);
    assert!(text(cx).ends_with("![photo de classe 2.png](assets/photo%20de%20classe%202.png)"));
    assert_eq!(fs::read(assets.join("photo de classe.png")).unwrap(), png);
    assert_eq!(
        fs::read(assets.join("photo de classe 2.png")).unwrap(),
        b"\x89PNG second"
    );

    // Any other file is linked, not drawn; a file that cannot be read is skipped.
    fs::write(depot.join("rapport.pdf"), b"%PDF-1.4").unwrap();
    drop_at_end(
        cx,
        vec![depot.join("rapport.pdf"), depot.join("absent.png")],
    );
    assert!(text(cx).ends_with("[rapport.pdf](assets/rapport.pdf)"));
    assert!(!assets.join("absent.png").exists());
    cx.simulate_keystrokes("secondary-z");
    assert!(!text(cx).contains("rapport.pdf"));
}
