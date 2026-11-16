//! The Versions panel and the compare (`app-ui.md` §19): the panel lists the versions of the note of the
//! active tab, newest first; Enter compares one with the note as it is now, in a read-only tab of its own,
//! where the difference is drawn as pending changes. Each path is driven with keystrokes or the shell's
//! own gestures, on disk.

use super::*;
use crate::logic::revision::Kind;
use crate::platform::revisions::{self as history, Home};

/// The notice of every change refused in a comparison.
pub(super) const COMPARISON: &str = "This is a comparison: it cannot be changed.";
/// The notice of a comparison refused for pending changes.
const PENDING: &str =
    "This version or the note holds pending changes: accept or reject them first.";

pub(super) fn open(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, path: &Path) {
    shell.update_in(cx, |s, window, cx| s.open_from_nav(path, window, cx));
    cx.run_until_parked();
}

/// Lets the debounced write happen, and the window draw what changed.
pub(super) fn settle(cx: &mut gpui::VisualTestContext) {
    cx.executor().advance_clock(Duration::from_millis(500));
    cx.run_until_parked();
    frame(cx);
}

/// Draws the window once more, so that the panel's rows follow the tab.
pub(super) fn frame(cx: &mut gpui::VisualTestContext) {
    cx.update(|window, _| window.refresh());
    cx.run_until_parked();
}

/// `Ctrl+Alt+S`, then the line on what changed (`memo`, empty to skip), then Enter.
pub(super) fn save_version(cx: &mut gpui::VisualTestContext, memo: &str) {
    cx.simulate_keystrokes("secondary-alt-s");
    cx.run_until_parked();
    if !memo.is_empty() {
        cx.simulate_input(memo);
    }
    cx.simulate_keystrokes("enter");
    cx.run_until_parked();
}

pub(super) fn show_versions(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Versions, false, window, cx)
    });
    frame(cx);
}

/// The panel's row names, newest first.
pub(super) fn names(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<String> {
    shell.read_with(cx, |s, _| {
        s.nav_rows().into_iter().map(|r| r.name).collect()
    })
}

/// The file of the panel's row `ix`.
pub(super) fn row_path(
    shell: &Entity<Shell>,
    cx: &mut gpui::VisualTestContext,
    ix: usize,
) -> PathBuf {
    shell.read_with(cx, |s, _| s.nav_rows()[ix].path.clone())
}

/// The text of the active tab's editor.
pub(super) fn text(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    shell.read_with(cx, |s, cx| s.pane().editor.read(cx).text().to_string())
}

/// How many tabs of the window are comparisons.
pub(super) fn compares(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> usize {
    shell.read_with(cx, |s, _| {
        s.panes.iter().filter(|p| p.compare.is_some()).count()
    })
}

/// The tab that is a comparison.
pub(super) fn compare_tab(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> usize {
    shell.read_with(cx, |s, _| {
        s.panes
            .iter()
            .position(|p| p.compare.is_some())
            .expect("a comparison is open")
    })
}

/// Every file under `dir`, to see that nothing was written there.
pub(super) fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(folder) = todo.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                todo.push(path);
            } else {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Puts the caret at byte `at`.
pub(super) fn put(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, at: usize) {
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| e.set_view(at, 0., cx))
    });
}

/// Whether the shell shows the message `what` now.
pub(super) fn says(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, what: &str) -> bool {
    shell.read_with(cx, |s, _| s.toasts.iter().any(|t| t.text == what))
}

/// The panel's first row, compared from the keys: the panel shown, then down, then Enter.
fn compare_first_by_keys(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) {
    show_versions(shell, cx);
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("enter");
    settle(cx);
}

/// A note `Plan.md` whose first version has the text `first`, and whose text is `now` now (typed or
/// loaded, not saved: the compare reads what the tab shows). The panel is shown with its rows.
fn one_version_then_edit<'a>(
    cx: &'a mut TestAppContext,
    name: &str,
    first: &str,
    now: &str,
) -> (
    PathBuf,
    Entity<Shell>,
    &'a mut gpui::VisualTestContext,
    std::sync::MutexGuard<'static, ()>,
) {
    let (root, shell, cx, config) = start(cx, name);
    let plan = root.join("Plan.md");
    fs::write(&plan, first).unwrap();
    open(&shell, cx, &plan);
    save_version(cx, "");
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.load(now.to_string(), 0, cx))
    });
    settle(cx);
    (root, shell, cx, config)
}

/// The text the compare of `one_version_then_edit` shows, in the version's order.
const CHANGED: &str = "# Plan\nkeep\n{--\nchange me\n--}\n{++\nchanged\n++}\nend\n{--\ngone\n--}\nlast\n{++\nnew\n++}\n";

#[gpui::test]
fn the_palette_offers_the_panel_and_it_lists_newest_first(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-list");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n\none idea\n").unwrap();
    open(&shell, cx, &plan);
    let offered = shell.update_in(cx, |s, window, cx| {
        s.palette_commands(window, cx)
            .0
            .iter()
            .map(|c| c.action)
            .collect::<Vec<_>>()
    });
    assert!(offered.contains(&"app::ShowVersions"));

    save_version(cx, "dropped the plugin idea");
    assert!(shell.read_with(cx, |s, _| s.told(Tone::Done, "Revision 1 saved")));
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_input("two\n");
    settle(cx);
    save_version(cx, "");
    // An automatic version, as an agent's write leaves it.
    let home = Home::Vault(root.clone());
    history::save_as(
        &home,
        &plan,
        "assistant",
        Some("proposed, pending"),
        "# Plan\n\none idea\ntwo\nthree\n",
        history::now(),
        Kind::Auto,
    )
    .unwrap();

    show_versions(&shell, cx);
    assert_eq!(shell.read_with(cx, |s, _| s.nav.mode), Mode::Versions);
    let me = shell.read_with(cx, |s, _| s.revision_author());
    assert_eq!(
        names(&shell, cx),
        [
            "#3 · assistant · auto · proposed, pending".to_string(),
            format!("#2 · {me}"),
            format!("#1 · {me} · dropped the plugin idea"),
        ]
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_panel_says_why_it_has_no_rows(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-empty");
    // A new note, with no file: there is no note to have a history.
    show_versions(&shell, cx);
    assert!(names(&shell, cx).is_empty());
    assert_eq!(
        shell.read_with(cx, |s, _| s.versions_said()),
        "No note is open."
    );
    // A note of the vault with no version yet: the chord is the one in force.
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\n").unwrap();
    open(&shell, cx, &plan);
    show_versions(&shell, cx);
    let key = shell.read_with(cx, |_, cx| crate::ui::keys::of(cx, &SaveRevision));
    assert!(key.ends_with('S'), "{key}");
    assert_eq!(
        shell.read_with(cx, |s, _| s.versions_said()),
        format!("No version yet. Save one with {key}.")
    );
    // A note outside the vault and the scratch folder: it has no history.
    shell.update_in(cx, |s, _, cx| {
        s.pane_mut().path = Some(PathBuf::from("/elsewhere/Note.md"));
        s.sync_versions(cx);
    });
    assert_eq!(
        shell.read_with(cx, |s, _| s.versions_said()),
        "This note has no history."
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_list_follows_the_tab_and_gains_a_row_after_a_save(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-follow");
    let plan = root.join("Plan.md");
    let other = root.join("Other.md");
    fs::write(&plan, "# Plan\n\none\n").unwrap();
    fs::write(&other, "# Other\n").unwrap();
    open(&shell, cx, &plan);
    save_version(cx, "first");
    show_versions(&shell, cx);
    assert_eq!(names(&shell, cx).len(), 1);

    // Another tab, with no version yet: the list is that note's.
    open(&shell, cx, &other);
    frame(cx);
    assert!(names(&shell, cx).is_empty());
    assert!(shell.read_with(cx, |s, _| s.versions_said().starts_with("No version yet.")));

    // Back to the first: its list again, and a save adds a row while the panel shows.
    open(&shell, cx, &plan);
    frame(cx);
    assert_eq!(names(&shell, cx).len(), 1);
    cx.simulate_keystrokes("secondary-end");
    cx.simulate_input("two\n");
    settle(cx);
    save_version(cx, "second");
    frame(cx);
    assert_eq!(names(&shell, cx).len(), 2);
    assert!(names(&shell, cx)[0].starts_with("#2 · "));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn enter_compares_a_version_with_the_note_now(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-compare",
        "# Plan\nkeep\nchange me\nend\ngone\nlast\n",
        "# Plan\nkeep\nchanged\nend\nlast\nnew\n",
    );
    let before = shell.read_with(cx, |s, _| s.panes.len());
    compare_first_by_keys(&shell, cx);

    assert_eq!(shell.read_with(cx, |s, _| s.panes.len()), before + 1);
    let tab = compare_tab(&shell, cx);
    assert_eq!(
        shell.read_with(cx, |s, cx| s.tab_name(tab, cx)),
        "Plan · #1 ↔ now"
    );
    // One changed line, one removed line, one added line: three pending changes, in the version's order.
    assert_eq!(text(&shell, cx), CHANGED);
    assert!(shell.read_with(cx, |s, cx| {
        let pane = &s.panes[tab];
        let editor = pane.editor.read(cx);
        editor.is_readonly() && editor.pending_changes() == 3 && pane.path.is_none() && !pane.dirty
    }));
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_comparison_changes_nothing_and_says_so(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-readonly",
        "# Plan\nkeep\nchange me\nend\ngone\nlast\n",
        "# Plan\nkeep\nchanged\nend\nlast\nnew\n",
    );
    compare_first_by_keys(&shell, cx);
    // In a kept line, where no change is: only the comparison's own rule refuses it.
    put(&shell, cx, CHANGED.find("keep").unwrap() + 2);
    cx.simulate_input("x");
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("backspace");
    cx.simulate_keystrokes("delete");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        CHANGED,
        "a kept line changes nothing either"
    );
    assert!(says(&shell, cx, COMPARISON));
    // The caret in a change: an added line, then a removed one.
    put(&shell, cx, CHANGED.find("{++").unwrap() + 4);
    cx.simulate_input("x");
    cx.simulate_keystrokes("enter");
    cx.simulate_keystrokes("backspace");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        CHANGED,
        "typing, Enter and Backspace change nothing"
    );
    assert!(says(&shell, cx, COMPARISON));

    // Paste, in a kept line: the clipboard's text is refused too.
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("Z".into())));
    put(&shell, cx, CHANGED.find("keep").unwrap() + 2);
    cx.simulate_keystrokes("secondary-v");
    settle(cx);
    assert_eq!(text(&shell, cx), CHANGED, "paste changes nothing");

    // Cut over the whole text: refused, and the clipboard keeps its text.
    cx.simulate_keystrokes("secondary-a");
    cx.simulate_keystrokes("secondary-x");
    settle(cx);
    assert_eq!(text(&shell, cx), CHANGED, "cut changes nothing");
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
        Some("Z"),
        "a refused cut writes no clipboard"
    );

    // Find and replace: the replacement is refused too.
    cx.simulate_keystrokes("secondary-f");
    cx.simulate_input("changed");
    cx.simulate_keystrokes("secondary-r");
    cx.simulate_input("x");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(text(&shell, cx), CHANGED, "replace changes nothing");

    // Save a version: not a version of anything.
    cx.simulate_keystrokes("secondary-alt-s");
    settle(cx);
    assert!(says(
        &shell,
        cx,
        "This is a comparison: it cannot be saved as a version."
    ));
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));

    // Accept and reject the change under the caret, then undo: nothing moves.
    put(&shell, cx, CHANGED.find("{++").unwrap() + 4);
    cx.simulate_keystrokes("secondary-enter");
    cx.simulate_keystrokes("secondary-backspace");
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        CHANGED,
        "accept, reject and undo change nothing"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_comparison_is_never_dirty_asks_nothing_and_writes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-closed",
        "# Plan\nkeep\nchange me\nend\ngone\nlast\n",
        "# Plan\nkeep\nchanged\nend\nlast\nnew\n",
    );
    let scratch = platform::scratch::dir();
    let before_files = (files_under(&root), files_under(&scratch));
    // The layout's paths, without the mark of the tab shown (a comparison is shown, so no tab is marked).
    let paths = |shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext| {
        shell.read_with(cx, |s, _| s.panes_state().replace('*', ""))
    };
    let before_state = paths(&shell, cx);
    let before_recent = shell.read_with(cx, |s, _| s.recent.clone());
    compare_first_by_keys(&shell, cx);
    settle(cx);
    let tab = compare_tab(&shell, cx);
    assert!(shell.read_with(cx, |s, _| !s.panes[tab].dirty));
    // The changes of a comparison are not the note's: the hint does not count them.
    assert!(cx.debug_bounds("pending-hint").is_none());
    // Not in the layout that the next start restores, nor in the recent notes.
    assert_eq!(paths(&shell, cx), before_state);
    assert_eq!(shell.read_with(cx, |s, _| s.recent.clone()), before_recent);

    // Closed: no question, the tab goes, and nothing is written.
    shell.update_in(cx, |s, window, cx| s.close_tab(tab, window, cx));
    settle(cx);
    assert!(shell.read_with(cx, |s, _| s.palette.is_none()));
    assert_eq!(compares(&shell, cx), 0);
    assert_eq!((files_under(&root), files_under(&scratch)), before_files);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn equal_texts_say_so_and_open_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) =
        one_version_then_edit(cx, "versions-same", "# Plan\nsame\n", "# Plan\nsame\n");
    let tabs = shell.read_with(cx, |s, _| s.panes.len());
    compare_first_by_keys(&shell, cx);
    assert!(says(&shell, cx, "Nothing changed since version 1."));
    assert_eq!(shell.read_with(cx, |s, _| s.panes.len()), tabs);
    assert_eq!(compares(&shell, cx), 0);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_text_with_pending_marks_is_refused(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(cx, "versions-marks", "a\n", "a\n");
    // A version whose text holds a change's marks (written by hand, or by an older write).
    let home = Home::Vault(root.clone());
    history::save(
        &home,
        &root.join("Plan.md"),
        "tester",
        None,
        "a\n{++\nx\n++}\n",
        history::now(),
    )
    .unwrap();
    show_versions(&shell, cx);
    // Row 0 is the newest, the marked one: its compare is refused, with no tab.
    cx.simulate_keystrokes("down");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert!(says(&shell, cx, PENDING));
    assert_eq!(compares(&shell, cx), 0);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_same_version_opens_one_tab_and_its_text_is_brought_up_to_date(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-twice",
        "# Plan\nkeep\nchange me\nend\ngone\nlast\n",
        "# Plan\nkeep\nchanged\nend\nlast\nnew\n",
    );
    compare_first_by_keys(&shell, cx);
    assert_eq!(compares(&shell, cx), 1);
    // Back to the note: the same version again is the same tab.
    let note = shell.read_with(cx, |s, _| {
        s.panes.iter().position(|p| p.compare.is_none()).unwrap()
    });
    shell.update_in(cx, |s, window, cx| s.focus_pane(note, window, cx));
    frame(cx);
    let path = row_path(&shell, cx, 0);
    shell.update_in(cx, |s, window, cx| s.compare_version(&path, window, cx));
    settle(cx);
    assert_eq!(compares(&shell, cx), 1);
    assert_eq!(text(&shell, cx), CHANGED);

    // The note changed meanwhile: the same tab shows the compare of the new text, not the old one.
    shell.update_in(cx, |s, window, cx| s.focus_pane(note, window, cx));
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| {
            e.load(
                "# Plan\nkeep\nchange me\nend\ngone\nlast\nmore\n".to_string(),
                0,
                cx,
            )
        })
    });
    shell.update_in(cx, |s, window, cx| s.compare_version(&path, window, cx));
    settle(cx);
    assert_eq!(compares(&shell, cx), 1);
    assert!(
        text(&shell, cx).ends_with("{++\nmore\n++}\n"),
        "{}",
        text(&shell, cx)
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_row_menu_copies_the_text_and_cannot_trash_a_version(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-menu");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\nfirst\n").unwrap();
    open(&shell, cx, &plan);
    save_version(cx, "");
    show_versions(&shell, cx);
    let path = row_path(&shell, cx, 0);

    // The menu of a version: its compare first, then its copy, then its restore.
    shell.update_in(cx, |s, _, cx| {
        s.menu = Some(nav::Menu {
            target: Some(path.clone()),
            ..Default::default()
        });
        let groups = s.menu_groups(cx);
        let actions: Vec<&nav::Do> = groups.iter().flatten().map(|(_, what)| what).collect();
        assert!(matches!(actions[0], nav::Do::CompareVersion));
        assert!(matches!(actions[1], nav::Do::CopyVersion));
        assert!(matches!(actions[2], nav::Do::RestoreVersion));
        assert_eq!(actions.len(), 3);
    });

    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::CopyVersion, Some(path.clone()), window, cx)
    });
    assert_eq!(
        cx.read_from_clipboard().and_then(|c| c.text()).as_deref(),
        Some("# Plan\nfirst\n")
    );

    // Trash, rename and the like are not the panel's: the version file stays where it is.
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::Trash, Some(path.clone()), window, cx);
        s.menu_do(nav::Do::Rename, Some(path.clone()), window, cx);
    });
    settle(cx);
    assert!(path.is_file(), "{}", path.display());
    assert!(root.join(".inkuno/revisions/Plan.md/0001.md").is_file());
    let _ = fs::remove_dir_all(root);
}

/// The text of tab `tab`'s editor.
fn text_of(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, tab: usize) -> String {
    shell.read_with(cx, |s, cx| s.panes[tab].editor.read(cx).text().to_string())
}

/// A version chosen in the panel is not the tree's selection: the tree's keys find nothing to act on.
#[gpui::test]
fn a_version_chosen_in_the_panel_is_not_the_selection_of_the_tree(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-tree-keys",
        "# Plan\nfirst\n",
        "# Plan\nfirst\nsecond\n",
    );
    show_versions(&shell, cx);
    let version = row_path(&shell, cx, 0);
    let before = fs::read(&version).unwrap();
    let files = files_under(&root);
    // A click on the version row.
    shell.update_in(cx, |s, window, cx| s.nav_select(0, window, cx));
    assert_eq!(
        shell.read_with(cx, |s, _| s.nav.sel.clone()),
        Some(version.clone())
    );
    // The tree is shown: the selection of the panel does not come with it.
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Tree, false, window, cx)
    });
    frame(cx);
    assert_eq!(shell.read_with(cx, |s, _| s.nav.sel.clone()), None);
    // Delete, F2 and Ctrl+D, the tree's keys: none of them reaches the version.
    cx.simulate_keystrokes("delete");
    cx.simulate_keystrokes("f2 escape");
    cx.simulate_keystrokes("secondary-d");
    settle(cx);
    assert_eq!(fs::read(&version).unwrap(), before);
    assert_eq!(files_under(&root), files);
    let _ = fs::remove_dir_all(root);
}

/// Every file gesture on a history file is refused, whatever the selection or the panel shows.
#[gpui::test]
fn a_history_file_is_refused_by_every_file_gesture_whatever_the_panel(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-history-refused",
        "# Plan\nfirst\n",
        "# Plan\nfirst\nsecond\n",
    );
    show_versions(&shell, cx);
    let version = row_path(&shell, cx, 0);
    let before = fs::read(&version).unwrap();
    let files = files_under(&root);
    // The tree shown, with the version as its target (a stale selection, a menu of another panel).
    shell.update_in(cx, |s, window, cx| {
        s.show_nav(Mode::Tree, false, window, cx);
        for what in [
            nav::Do::Trash,
            nav::Do::Rename,
            nav::Do::Duplicate,
            nav::Do::MoveTo,
            nav::Do::Cut,
        ] {
            s.menu_do(what, Some(version.clone()), window, cx);
        }
    });
    settle(cx);
    assert!(says(
        &shell,
        cx,
        "The history of the notes is kept by the app: nothing is done to it."
    ));
    assert_eq!(fs::read(&version).unwrap(), before);
    assert_eq!(files_under(&root), files);
    let _ = fs::remove_dir_all(root);
}

/// A version whose file is garbage is one quiet row: the others list, and Enter opens nothing.
#[gpui::test]
fn one_unreadable_version_hides_neither_the_others_nor_opens_anything(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-unreadable");
    let plan = root.join("Plan.md");
    fs::write(&plan, "# Plan\none\n").unwrap();
    open(&shell, cx, &plan);
    save_version(cx, "");
    for word in ["two", "three"] {
        cx.simulate_keystrokes("secondary-end");
        cx.simulate_input(&format!("{word}\n"));
        settle(cx);
        save_version(cx, "");
    }
    // The middle version's file is overwritten with garbage (a hand edit, a bad copy).
    let middle = root.join(".inkuno/revisions/Plan.md/0002.md");
    fs::write(&middle, "garbage, not a revision\n").unwrap();
    show_versions(&shell, cx);
    let rows = names(&shell, cx);
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert!(rows[0].starts_with("#3 · "), "{rows:?}");
    assert_eq!(rows[1], "#2 · cannot be read");
    assert!(rows[2].starts_with("#1 · "), "{rows:?}");
    // Enter on the marked row opens nothing, and says why.
    cx.simulate_keystrokes("down down enter");
    settle(cx);
    assert_eq!(compares(&shell, cx), 0);
    assert!(says(
        &shell,
        cx,
        "#2 · this version cannot be read, so it cannot be compared."
    ));
    // A refusal changes no row.
    assert_eq!(names(&shell, cx).len(), 3);
    let _ = fs::remove_dir_all(root);
}

/// A folder link and a wiki link followed from a comparison never load into it.
#[gpui::test]
fn a_comparison_never_becomes_a_note_or_a_list(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-no-note",
        "# Plan\nkeep\nchange me\nend\n",
        "# Plan\nkeep\nchanged\nend\nmore\n",
    );
    fs::create_dir_all(root.join("dossier")).unwrap();
    fs::write(root.join("dossier/Inside.md"), "# Inside\n").unwrap();
    fs::write(root.join("Other.md"), "# Other\n").unwrap();
    compare_first_by_keys(&shell, cx);
    let compare = compare_tab(&shell, cx);
    let compared = text(&shell, cx);
    shell.update(cx, |s, cx| s.rescan(&root, cx));

    // A folder link followed from the comparison: its list page takes a tab of its own.
    shell.update_in(cx, |s, window, cx| {
        s.follow(
            &crate::logic::markdown::Link::Wiki("dossier".into()),
            window,
            cx,
        )
    });
    cx.run_until_parked();
    assert_ne!(shell.read_with(cx, |s, _| s.active), compare);
    // A note opened from that list (what its cell does).
    let inside = root.join("dossier/Inside.md");
    shell.update(cx, |s, cx| s.open_note(&inside, cx));
    cx.run_until_parked();
    // The comparison is intact: still a comparison, read-only, with its own text.
    let intact = shell.read_with(cx, |s, cx| {
        let tab = &s.panes[compare];
        (
            tab.compare.is_some(),
            tab.editor.read(cx).is_readonly(),
            tab.path.clone(),
        )
    });
    assert_eq!((intact.0, intact.1), (true, true));
    assert_eq!(intact.2, None);
    assert_eq!(text_of(&shell, cx, compare), compared);
    // The note is in another tab, and it is editable.
    let note = shell.read_with(cx, |s, cx| {
        (
            s.active,
            s.pane().compare.is_none(),
            s.pane().path.clone(),
            s.pane().editor.read(cx).is_readonly(),
        )
    });
    assert_ne!(note.0, compare);
    assert!(note.1);
    assert_eq!(note.2, Some(inside.clone()));
    assert!(!note.3);

    // A wiki link to a note, followed from the comparison: opens in another tab.
    shell.update_in(cx, |s, window, cx| s.focus_pane(compare, window, cx));
    shell.update_in(cx, |s, window, cx| {
        s.follow(
            &crate::logic::markdown::Link::Wiki("Other".into()),
            window,
            cx,
        )
    });
    cx.run_until_parked();
    let other = root.join("Other.md");
    let (active, path, compared_now) = shell.read_with(cx, |s, cx| {
        (
            s.active,
            s.pane().path.clone(),
            s.panes[compare].compare.is_some() && s.panes[compare].editor.read(cx).is_readonly(),
        )
    });
    assert_ne!(active, compare);
    assert_eq!(path, Some(other));
    assert!(compared_now);
    assert_eq!(text_of(&shell, cx, compare), compared);
    let _ = fs::remove_dir_all(root);
}

/// Second line of defence: a note loaded into a comparison tab makes it an editable note.
#[gpui::test]
fn a_note_loaded_into_a_comparison_tab_is_an_editable_note(cx: &mut TestAppContext) {
    // The second line of defence: a route that loads a note into the comparison makes the tab a note.
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-second-line",
        "# Plan\nkeep\nchange me\n",
        "# Plan\nkeep\nchanged\n",
    );
    fs::write(root.join("Other.md"), "# Other\n").unwrap();
    compare_first_by_keys(&shell, cx);
    let compare = compare_tab(&shell, cx);
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update(cx, |s, cx| s.open_note(&root.join("Other.md"), cx));
    cx.run_until_parked();
    let (compared, readonly, path) = shell.read_with(cx, |s, cx| {
        let tab = &s.panes[compare];
        (
            tab.compare.is_some(),
            tab.editor.read(cx).is_readonly(),
            tab.path.clone(),
        )
    });
    assert!(!compared);
    assert!(!readonly);
    assert_eq!(path, Some(root.join("Other.md")));
    let _ = fs::remove_dir_all(root);
}

/// Pasting a picture into a comparison writes no file and changes no text.
#[gpui::test]
fn an_image_pasted_into_a_comparison_is_not_written(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-paste",
        "# Plan\nkeep\nchange me\n",
        "# Plan\nkeep\nchanged\n",
    );
    compare_first_by_keys(&shell, cx);
    let text_before = text(&shell, cx);
    let files = files_under(&root);
    let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\"/>";
    let image = gpui::Image::from_bytes(gpui::ImageFormat::Svg, svg.as_bytes().to_vec());
    cx.write_to_clipboard(gpui::ClipboardItem::new_image(&image));
    cx.simulate_keystrokes("secondary-v");
    settle(cx);
    assert_eq!(files_under(&root), files);
    assert_eq!(text(&shell, cx), text_before);
    assert!(says(&shell, cx, COMPARISON));
    let _ = fs::remove_dir_all(root);
}

/// The saved layout marks the note a shown comparison belongs to.
#[gpui::test]
fn a_shown_comparison_saves_its_note_as_the_shown_tab(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = start(cx, "versions-layout");
    let other = root.join("Other.md");
    let plan = root.join("Plan.md");
    fs::write(&other, "# Other\n").unwrap();
    fs::write(&plan, "# Plan\nkeep\nchange me\n").unwrap();
    open(&shell, cx, &other);
    open(&shell, cx, &plan);
    save_version(cx, "");
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| {
            e.load("# Plan\nkeep\nchanged\n".to_string(), 0, cx)
        })
    });
    settle(cx);
    compare_first_by_keys(&shell, cx);
    assert_eq!(compares(&shell, cx), 1);
    // The tabs are Other, Plan, and the comparison shown: the layout marks Plan, the note it compares.
    let state = shell.read_with(cx, |s, _| s.panes_state());
    let (_, groups) = crate::shell::pane::read_panes(&state);
    assert_eq!(groups.len(), 1);
    let (tabs, shown) = &groups[0];
    assert_eq!(tabs, &[other, plan.clone()]);
    assert_eq!(tabs[*shown], plan);
    let _ = fs::remove_dir_all(root);
}

/// While a comparison is shown, the panel lists its note and marks the compared version.
#[gpui::test]
fn the_panel_lists_the_versions_of_the_note_a_comparison_belongs_to(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-follow-compare",
        "# Plan\nkeep\nchange me\n",
        "# Plan\nkeep\nchanged\n",
    );
    compare_first_by_keys(&shell, cx);
    show_versions(&shell, cx);
    assert_eq!(names(&shell, cx).len(), 1);
    assert_eq!(shell.read_with(cx, |s, _| s.versions_said()), String::new());
    // The row of the compared version is the current one.
    let path = row_path(&shell, cx, 0);
    assert!(shell.read_with(cx, |s, _| s.version_current(&path)));
    let _ = fs::remove_dir_all(root);
}

/// A revision written from a terminal shows in the open panel once the window hears of it.
#[gpui::test]
fn a_revision_written_from_a_terminal_reaches_the_open_panel(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-terminal",
        "# Plan\nfirst\n",
        "# Plan\nfirst\n",
    );
    show_versions(&shell, cx);
    assert_eq!(names(&shell, cx).len(), 1);
    // `inkuno rev save` writes a version from a terminal: the app's window hears of it, and nothing else.
    let plan = root.join("Plan.md");
    history::save(
        &Home::Vault(root.clone()),
        &plan,
        "bo",
        None,
        "# Plan\nsecond\n",
        0,
    )
    .unwrap();
    shell.update_in(cx, |s, _, cx| s.versions_moved(cx));
    frame(cx);
    let names = names(&shell, cx);
    assert_eq!(names.len(), 2, "{names:?}");
    assert!(names[0].starts_with("#2 · bo"), "{names:?}");
    let _ = fs::remove_dir_all(root);
}

/// Restores version `number` from the panel's menu, as the person does: the panel shown, the row's
/// menu's own action, then the settling of the write.
fn restore_from_the_menu(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext, number: u32) {
    show_versions(shell, cx);
    let rows = names(shell, cx);
    let prefix = format!("#{number} · ");
    let ix = rows
        .iter()
        .position(|row| row.starts_with(&prefix))
        .expect("the version is listed");
    let path = row_path(shell, cx, ix);
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::RestoreVersion, Some(path), window, cx)
    });
    settle(cx);
}

/// The history of `Plan.md` under `root`, oldest first.
fn history_of(root: &Path) -> Vec<crate::logic::revision::Revision> {
    history::all(&Home::Vault(root.to_path_buf()), &root.join("Plan.md")).unwrap()
}

#[gpui::test]
fn a_restore_proposes_the_version_as_pending_changes_in_one_undo_step(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) =
        one_version_then_edit(cx, "versions-restore", "# Plan\none\n", "# Plan\ntwo\n");
    restore_from_the_menu(&shell, cx, 1);
    let now = text(&shell, cx);
    assert_eq!(
        crate::logic::pending::accept_all(&now),
        "# Plan\none\n",
        "accepting every change gives the version"
    );
    assert_eq!(
        crate::logic::pending::reject_all(&now),
        "# Plan\ntwo\n",
        "rejecting every change gives the note as it was"
    );
    assert_eq!(crate::logic::pending::areas(&now).len(), 1);
    // One undo takes the whole restore back.
    shell.update_in(cx, |s, window, cx| {
        let at = s.active;
        s.focus_pane(at, window, cx)
    });
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(text(&shell, cx), "# Plan\ntwo\n", "one undo takes it back");
    // Two automatic revisions surround the restore, by the person's author.
    let me = shell.read_with(cx, |s, _| s.revision_author());
    let found = history_of(&root);
    assert_eq!(
        found.iter().map(|rev| rev.kind).collect::<Vec<_>>(),
        [Kind::Checkpoint, Kind::Auto, Kind::Auto]
    );
    assert_eq!(found[1].note.as_deref(), Some("before restoring version 1"));
    assert_eq!(found[1].author, me);
    assert_eq!(found[1].text, "# Plan\ntwo\n", "the text before");
    assert_eq!(
        found[2].note.as_deref(),
        Some("version 1 restored, pending")
    );
    assert_eq!(found[2].author, me);
    assert_eq!(
        found[2].text, "# Plan\none\n",
        "the restored text, accepted"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn restoring_the_text_the_note_already_has_says_so_and_changes_nothing(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-restore-same",
        "# Plan\none\n",
        "# Plan\none\n",
    );
    restore_from_the_menu(&shell, cx, 1);
    assert!(says(&shell, cx, "The note is this version already."));
    assert_eq!(text(&shell, cx), "# Plan\none\n");
    assert_eq!(history_of(&root).len(), 1, "no revision is added");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_restore_is_refused_while_the_note_has_pending_changes(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-restore-pending",
        "# Plan\none\n",
        "# Plan\ntwo\n",
    );
    restore_from_the_menu(&shell, cx, 1);
    let once = text(&shell, cx);
    // The second restore is of the same version, from its row: the note holds changes now.
    restore_from_the_menu(&shell, cx, 1);
    assert!(says(
        &shell,
        cx,
        "Accept or reject the pending changes first."
    ));
    assert_eq!(text(&shell, cx), once, "the note is not changed");
    assert_eq!(history_of(&root).len(), 3, "no revision is added");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_restore_is_refused_in_a_comparison(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-restore-compare",
        "# Plan\none\n",
        "# Plan\ntwo\n",
    );
    compare_first_by_keys(&shell, cx);
    assert_eq!(compares(&shell, cx), 1);
    let path = row_path(&shell, cx, 0);
    shell.update_in(cx, |s, window, cx| {
        s.menu_do(nav::Do::RestoreVersion, Some(path), window, cx)
    });
    settle(cx);
    assert!(says(&shell, cx, COMPARISON));
    assert_eq!(compares(&shell, cx), 1);
    assert_eq!(history_of(&root).len(), 1, "no revision is added");
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_unreadable_version_has_no_restore_entry(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = one_version_then_edit(
        cx,
        "versions-restore-unreadable",
        "# Plan\none\n",
        "# Plan\ntwo\n",
    );
    let file = root.join(".inkuno/revisions/Plan.md/0001.md");
    fs::write(&file, "garbage, not a revision\n").unwrap();
    show_versions(&shell, cx);
    assert_eq!(names(&shell, cx), ["#1 · cannot be read"]);
    let path = row_path(&shell, cx, 0);
    shell.update_in(cx, |s, _, cx| {
        s.menu = Some(nav::Menu {
            target: Some(path.clone()),
            ..Default::default()
        });
        let groups = s.menu_groups(cx);
        let actions: Vec<&nav::Do> = groups.iter().flatten().map(|(_, what)| what).collect();
        assert!(matches!(actions[0], nav::Do::CompareVersion));
        assert!(matches!(actions[1], nav::Do::CopyVersion));
        assert_eq!(actions.len(), 2, "no restore of a version with no text");
    });
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_version_is_dated_in_local_time_to_the_minute(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) =
        one_version_then_edit(cx, "versions-local-date", "# Plan\none\n", "# Plan\none\n");
    show_versions(&shell, cx);
    let path = row_path(&shell, cx, 0);
    // The stored time is UTC; the row shows the local time of the same instant, built with the
    // platform's own conversion, so the test does not depend on the machine's zone.
    let rev = history::get(&Home::Vault(root.clone()), &root.join("Plan.md"), 1).unwrap();
    let unix = crate::logic::revision::unix_of(&rev.date).unwrap();
    let (year, month, day, hour, minute) = crate::platform::clock::local(unix);
    let expected = format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}");
    assert_eq!(shell.read_with(cx, |s, _| s.version_date(&path)), expected);
    let _ = fs::remove_dir_all(root);
}
