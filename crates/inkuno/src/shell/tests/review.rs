//! The Review feed of the right panel (app-ui.md §23): the note's comments and pending changes as one list,
//! oldest first, with the count in its head; one card open at a time; a pick selects the passage; a reply
//! is written after the replies of its comment, on its line; Resolve, Accept and Reject act from the cards.

use super::pending::{settle, text};
use super::*;
use crate::logic::layout::Side;
use crate::logic::pending;
use crate::platform::revisions::Home;
use crate::platform::sidecar;
use crate::shell::review::{Body, Key};

/// The debug names of the feed's cards, by their place in it (`review_card`).
const CARD: [&str; 4] = [
    "review-card-0",
    "review-card-1",
    "review-card-2",
    "review-card-3",
];

/// The note as the feed reads it: a comment by Ana at 10:05, a legacy comment, and a change by Bo proposed at
/// 10:02. The file holds its own text; the change waits beside it (`sidecar`).
const NOTE: &str = "# Plan\n\nkeep<!-- comment @Ana 2026-11-24T10:05Z: later -->\n\nalpha<!-- comment: legacy -->\n\n{--\nold\n--}\n{++\nnew\n++}\nend\n";

/// Opens `display` (the note as the editor shows it, its pending changes marked) in a window: the file is
/// `Plan.md`, its changes are saved beside it with the `created` time each is given.
fn open<'a>(
    name: &str,
    display: &str,
    created: &[&str],
    cx: &'a mut TestAppContext,
) -> (
    PathBuf,
    Entity<Shell>,
    &'a mut gpui::VisualTestContext,
    std::sync::MutexGuard<'static, ()>,
) {
    let (root, shell, cx, config) = start(cx, name);
    let (base, mut changes) = pending::split(display, &[]);
    for (change, time) in changes.iter_mut().zip(created) {
        change.created = Some(time.to_string());
        change.author = "Bo".into();
    }
    let note = root.join("Plan.md");
    fs::write(&note, base).unwrap();
    if !changes.is_empty() {
        sidecar::store(&Home::Vault(root.clone()), &note, &changes).unwrap();
    }
    shell.update(cx, |s, cx| s.rescan(&root, cx));
    shell.update_in(cx, |s, window, cx| s.open_from_nav(&note, window, cx));
    cx.simulate_resize(size(px(1400.), px(900.)));
    cx.run_until_parked();
    shell.update(cx, |s, _| s.prefs.author = "Ana".into());
    shell.update_in(cx, |s, window, cx| {
        s.show_review(window, cx);
    });
    settle(cx);
    (root, shell, cx, config)
}

/// The keys of the feed, in order: `C` for a comment, `P` for a change, with where each starts in the note.
fn order(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> Vec<Key> {
    shell.read_with(cx, |s, _| s.review_items.iter().map(|i| i.key).collect())
}

/// Clicks the feed's card `i`, as a pointer does: it hovers first, then clicks its lower body.
fn pick(cx: &mut gpui::VisualTestContext, i: usize) {
    let card = cx
        .debug_bounds(CARD[i])
        .unwrap_or_else(|| panic!("card {i} is drawn"));
    cx.simulate_mouse_move(
        point(card.left() + px(24.), card.bottom() - px(8.)),
        None,
        gpui::Modifiers::none(),
    );
    cx.run_until_parked();
    let card = cx.debug_bounds(CARD[i]).unwrap();
    cx.simulate_click(card.center(), gpui::Modifiers::none());
    settle(cx);
}

fn click(cx: &mut gpui::VisualTestContext, id: &'static str) {
    let at = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("{id} is drawn"))
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

fn stamped(shell: &Entity<Shell>, cx: &mut gpui::VisualTestContext) -> String {
    without_clock(&text(shell, cx))
}

#[gpui::test]
fn the_feed_lists_the_comments_and_the_changes_oldest_first_and_counts_them(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = open("review-order", NOTE, &["2026-11-24T10:02Z"], cx);
    let legacy = NOTE.find("alpha").unwrap();
    let change = NOTE.find("{--").unwrap();
    let later = NOTE.find("keep").unwrap();
    // Untimed first, in document order (the legacy comment), then by time: the change (10:02), the comment (10:05).
    assert_eq!(
        order(&shell, cx),
        [
            Key::Comment(legacy),
            Key::Change(change),
            Key::Comment(later),
        ]
    );
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_title()),
        "Review · 3 open"
    );
    // Each card says who, where and what: the legacy comment is by Unknown, on line 5, under the heading.
    let (legacy_item, change_item) = shell.read_with(cx, |s, _| {
        (s.review_items[0].clone(), s.review_items[1].clone())
    });
    assert_eq!((legacy_item.author, legacy_item.line), (None, 5));
    assert_eq!(change_item.author.as_deref(), Some("Bo"));
    assert_eq!(
        change_item.line, 7,
        "the change starts on the file's line 7"
    );
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_items[2].section.clone()),
        Some("Plan".to_string())
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn one_card_opens_at_a_time_and_a_pick_selects_its_passage(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-single", NOTE, &["2026-11-24T10:02Z"], cx);
    pick(cx, 0);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_open),
        Some(Key::Comment(NOTE.find("alpha").unwrap()))
    );
    let selected = shell.read_with(cx, |s, cx| s.pane().editor.read(cx).selected().to_string());
    assert_eq!(selected, "alpha", "the commented passage is selected");
    pick(cx, 2);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_open),
        Some(Key::Comment(NOTE.find("keep").unwrap())),
        "the card picked opens, the other closes"
    );
    assert!(
        shell.read_with(cx, |s, _| s.review_reply.as_ref().map(|r| r.0))
            == Some(Key::Comment(NOTE.find("keep").unwrap())),
        "one reply field, under the open comment"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_reply_is_written_after_the_replies_on_the_same_line(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-reply", NOTE, &["2026-11-24T10:02Z"], cx);
    pick(cx, 2);
    shell.update_in(cx, |s, window, cx| {
        let (_, field, _) = s.review_reply.as_ref().expect("a reply field");
        window.focus(&field.focus_handle(cx), cx);
    });
    cx.simulate_input("ça suit");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        stamped(&shell, cx),
        "# Plan\n\nkeep<!-- comment @Ana <T>: later ↳ @Ana <T>: ça suit -->\n\nalpha<!-- comment: legacy -->\n\n{--\nold\n--}\n{++\nnew\n++}\nend\n"
    );
    // The feed still has the comment, open, with its reply listed under it.
    let replies = shell.read_with(cx, |s, _| match &s.review_items[2].body {
        Body::Comment { replies, .. } => replies.len(),
        _ => 0,
    });
    assert_eq!(replies, 1);
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_reply_to_a_legacy_comment_is_in_the_new_form_and_the_root_stays_unknown(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = open("review-legacy", NOTE, &["2026-11-24T10:02Z"], cx);
    pick(cx, 0);
    shell.update_in(cx, |s, window, cx| {
        let (_, field, _) = s.review_reply.as_ref().expect("a reply field");
        window.focus(&field.focus_handle(cx), cx);
    });
    cx.simulate_input("vu");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        stamped(&shell, cx).lines().nth(4),
        Some("alpha<!-- comment: legacy ↳ @Ana <T>: vu -->"),
    );
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_items[0].author.clone()),
        None,
        "the legacy comment is still by Unknown"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn resolve_from_the_card_keeps_the_passage_and_one_undo_brings_it_back(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-resolve", NOTE, &["2026-11-24T10:02Z"], cx);
    pick(cx, 2);
    click(cx, "review-resolve");
    assert_eq!(
        stamped(&shell, cx),
        "# Plan\n\nkeep\n\nalpha<!-- comment: legacy -->\n\n{--\nold\n--}\n{++\nnew\n++}\nend\n"
    );
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_title()),
        "Review · 2 open",
        "the head counts what is open"
    );
    assert!(shell.read_with(cx, |s, _| s.review_open.is_none()));
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        NOTE,
        "one undo step brings the comment back"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn accept_and_reject_from_the_card_are_the_keys_of_the_change(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-decide", NOTE, &["2026-11-24T10:02Z"], cx);
    pick(cx, 1);
    click(cx, "review-accept");
    assert_eq!(
        text(&shell, cx),
        NOTE.replace("{--\nold\n--}\n{++\nnew\n++}\n", "new\n"),
        "accept: the removed lines go, the added stay"
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        NOTE,
        "one undo step brings the change back"
    );
    pick(cx, 1);
    click(cx, "review-reject");
    assert_eq!(
        text(&shell, cx),
        NOTE.replace("{--\nold\n--}\n{++\nnew\n++}\n", "old\n"),
        "reject: the added lines go, the removed stay"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn an_empty_note_says_there_is_nothing_to_review(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-empty", "# Plan\n\nrien\n", &[], cx);
    assert!(cx.debug_bounds("review-empty").is_some(), "the empty view");
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_title()),
        "Review · 0 open"
    );
    let _ = fs::remove_dir_all(root);
}

/// Two comments that touch: the second starts where the first ends (`alpha<!--c1--><!--c2-->`), each on its own.
const TOUCHING: &str = "alpha<!-- comment @Ana 2026-11-24T10:01Z: first --><!-- comment @Ana 2026-11-24T10:05Z: second -->\n";

#[gpui::test]
fn a_reply_on_the_second_of_two_touching_comments_goes_to_the_second(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-touch-reply", TOUCHING, &[], cx);
    let second = TOUCHING.rfind("<!--").unwrap();
    pick(cx, 1);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_open),
        Some(Key::Comment(second))
    );
    shell.update_in(cx, |s, window, cx| {
        let (_, field, _) = s.review_reply.as_ref().expect("a reply field");
        window.focus(&field.focus_handle(cx), cx);
    });
    cx.simulate_input("ça suit");
    cx.simulate_keystrokes("enter");
    settle(cx);
    assert_eq!(
        stamped(&shell, cx),
        "alpha<!-- comment @Ana <T>: first --><!-- comment @Ana <T>: second ↳ @Ana <T>: ça suit -->\n",
        "the reply is on the second comment, the first is as it was"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn resolve_on_the_second_of_two_touching_comments_removes_the_second(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-touch-resolve", TOUCHING, &[], cx);
    pick(cx, 1);
    click(cx, "review-resolve");
    assert_eq!(
        stamped(&shell, cx),
        "alpha<!-- comment @Ana <T>: first -->\n",
        "the second's markup goes, the first stays"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_write_inside_the_second_of_two_touching_comments_writes_the_second(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-touch-write", TOUCHING, &[], cx);
    // The draft's caret is inside the second comment's text.
    let inside = TOUCHING.rfind("second").unwrap();
    shell.update(cx, |s, cx| {
        s.pane()
            .editor
            .update(cx, |e, cx| e.write_comment(inside, "changed", cx))
    });
    settle(cx);
    assert_eq!(
        stamped(&shell, cx),
        "alpha<!-- comment @Ana <T>: first --><!-- comment @Ana <T>: changed -->\n"
    );
    let _ = fs::remove_dir_all(root);
}

/// A note with a pending replace above the line `end`: the gutter and the feed number the file's lines.
#[gpui::test]
fn the_gutter_numbers_the_file_lines_as_the_feed_does_past_a_pending_change(
    cx: &mut TestAppContext,
) {
    let (root, shell, cx, _config) = open("review-gutter", NOTE, &["2026-11-24T10:02Z"], cx);
    let numbers = shell.read_with(cx, |s, cx| s.panes[0].editor.read(cx).file_line_numbers());
    // On disk: `# Plan`, blank, `keep`, blank, `alpha`, blank, `old`, `end`. The marks and the added `new`
    // carry no number; the blank lines are collapsed and carry none.
    assert_eq!(numbers, [1, 3, 5, 7, 8]);
    // Each card's Line N is the gutter's number for its line.
    let lines: Vec<usize> =
        shell.read_with(cx, |s, _| s.review_items.iter().map(|i| i.line).collect());
    assert_eq!(lines, [5, 7, 3], "legacy comment, change, comment");
    for line in lines {
        assert!(numbers.contains(&line), "line {line} is numbered");
    }
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn the_feed_is_built_while_the_review_is_shown_and_counted_without(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-lazy", NOTE, &["2026-11-24T10:02Z"], cx);
    assert_eq!(shell.read_with(cx, |s, _| s.review_items.len()), 3);
    shell.update_in(cx, |s, window, cx| s.show_view(Mode::Outline, window, cx));
    settle(cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_total),
        3,
        "the count still shows on the switcher while the view is hidden"
    );
    // A change to the note while hidden is counted, but the feed is not built again until the view is shown.
    let end = NOTE.len();
    shell.update(cx, |s, cx| {
        s.pane().editor.update(cx, |e, cx| {
            e.absorb(&[(end..end, "{++\nlast\n++}\n".to_string())], cx)
        })
    });
    settle(cx);
    assert_eq!(shell.read_with(cx, |s, _| s.review_total), 4);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_items.len()),
        3,
        "no rebuild while the view is hidden"
    );
    shell.update_in(cx, |s, window, cx| s.show_review(window, cx));
    settle(cx);
    assert_eq!(shell.read_with(cx, |s, _| s.review_items.len()), 4);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_title()),
        "Review · 4 open"
    );
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn a_proposal_in_an_open_note_has_its_time_before_the_note_is_saved(cx: &mut TestAppContext) {
    let base = "# Plan\n\nkeep\n\nend\n";
    let (root, shell, cx, _config) = open("review-proposed", base, &[], cx);
    let current = text(&shell, cx);
    let marked = pending::propose(&current, &current, "# Plan\n\nkept\n\nend\n").expect("a change");
    shell.update(cx, |s, cx| s.absorb_marked(0, &current, &marked, cx));
    settle(cx);
    let times: Vec<Option<String>> = shell.read_with(cx, |s, _| {
        s.review_items.iter().map(|i| i.time.clone()).collect()
    });
    assert_eq!(times.len(), 1);
    assert!(
        times[0].as_deref().is_some_and(|t| t.ends_with('Z')),
        "the change has the minute it was proposed at, not none: {times:?}"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn an_author_none_an_empty_name_or_unknown_is_shown_as_unknown() {
    use crate::shell::review::shown_author;
    assert_eq!(shown_author(None), "Unknown");
    assert_eq!(shown_author(Some("")), "Unknown");
    assert_eq!(shown_author(Some("   ")), "Unknown");
    assert_eq!(shown_author(Some("unknown")), "Unknown");
    assert_eq!(shown_author(Some("UNKNOWN")), "Unknown");
    assert_eq!(shown_author(Some(" Ana ")), "Ana");
}

#[gpui::test]
fn every_new_pane_writes_by_the_revisions_author(cx: &mut TestAppContext) {
    let (root, shell, cx, _config) = open("review-pane-author", NOTE, &[], cx);
    // Read before anything draws: the author is given at creation, not pushed by the next render.
    shell.update_in(cx, |s, window, cx| s.add_tab(window, cx));
    shell.update_in(cx, |s, window, cx| s.split(Side::Right, window, cx));
    let authors = shell.read_with(cx, |s, cx| {
        s.panes
            .iter()
            .map(|pane| pane.editor.read(cx).author().to_string())
            .collect::<Vec<_>>()
    });
    assert!(
        authors.len() >= 3,
        "the note, a new tab and a split: {authors:?}"
    );
    assert!(authors.iter().all(|a| a == "Ana"), "{authors:?}");
    let _ = fs::remove_dir_all(root);
}

/// Clicks the element named `id` (an id made at run time), as a pointer does. The name is leaked: a
/// selector is `'static`, and a test is short-lived.
fn click_named(cx: &mut gpui::VisualTestContext, id: String) {
    let id: &'static str = Box::leak(id.into_boxed_str());
    let at = cx
        .debug_bounds(id)
        .unwrap_or_else(|| panic!("{id} is drawn"))
        .center();
    cx.simulate_click(at, gpui::Modifiers::none());
    settle(cx);
}

/// Clicks the card named `name` (its debug selector), as `pick` does for a numbered card.
fn pick_named(cx: &mut gpui::VisualTestContext, name: String) {
    let name: &'static str = Box::leak(name.into_boxed_str());
    let card = cx
        .debug_bounds(name)
        .unwrap_or_else(|| panic!("{name} is drawn"));
    cx.simulate_mouse_move(
        point(card.left() + px(24.), card.bottom() - px(8.)),
        None,
        gpui::Modifiers::none(),
    );
    cx.run_until_parked();
    let card = cx.debug_bounds(name).unwrap();
    cx.simulate_click(card.center(), gpui::Modifiers::none());
    settle(cx);
}

/// Three changes: the first two proposed in one minute by one author (one run), the third a minute later.
const THREE_CHANGES: &str = "# Plan\n\nkeep one\n\n{--\nold one\n--}\n{++\nnew one\n++}\n\nkeep two\n\n{--\nold two\n--}\n{++\nnew two\n++}\n\nkeep three\n\n{--\nold three\n--}\n{++\nnew three\n++}\n";

#[gpui::test]
fn changes_proposed_together_are_one_card_and_a_single_change_is_not(cx: &mut TestAppContext) {
    let times = [
        "2026-11-24T10:02Z",
        "2026-11-24T10:02Z",
        "2026-11-24T10:09Z",
    ];
    let (root, shell, cx, _config) = open("review-group-card", THREE_CHANGES, &times, cx);
    assert_eq!(
        shell.read_with(cx, |s, _| s.review_title()),
        "Review · 3 open",
        "the count is still per change"
    );
    assert!(
        cx.debug_bounds("review-group-0").is_some(),
        "the run of two is one card"
    );
    assert!(
        cx.debug_bounds("review-card-1").is_some(),
        "the change of its own is a card of its own"
    );
    assert!(cx.debug_bounds("review-card-2").is_none());
    let _ = fs::remove_dir_all(root);
}

#[gpui::test]
fn accept_all_of_a_group_takes_its_changes_and_leaves_the_others_pending(cx: &mut TestAppContext) {
    let times = [
        "2026-11-24T10:02Z",
        "2026-11-24T10:02Z",
        "2026-11-24T10:09Z",
    ];
    let (root, shell, cx, _config) = open("review-group-accept", THREE_CHANGES, &times, cx);
    let first = match order(&shell, cx)[0] {
        Key::Change(at) => at,
        Key::Comment(_) => panic!("the first item is a change"),
    };
    pick_named(cx, "review-group-0".to_string());
    click_named(cx, format!("review-group-{first}-accept-all"));
    assert_eq!(
        text(&shell, cx),
        THREE_CHANGES
            .replace("{--\nold one\n--}\n{++\nnew one\n++}\n", "new one\n")
            .replace("{--\nold two\n--}\n{++\nnew two\n++}\n", "new two\n"),
        "the two of the run are accepted; the removed lines go, the added stay"
    );
    assert_eq!(
        shell.read_with(cx, |s, cx| s.pane().editor.read(cx).pending_changes()),
        1,
        "the third change is still pending"
    );
    cx.simulate_keystrokes("secondary-z");
    settle(cx);
    assert_eq!(
        text(&shell, cx),
        THREE_CHANGES,
        "one undo step brings the two back"
    );
    let _ = fs::remove_dir_all(root);
}
