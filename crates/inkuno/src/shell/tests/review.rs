//! The Review feed of the right panel (app-ui.md §23): the note's comments and pending changes as one list,
//! oldest first, with the count in its head; one card open at a time; a pick selects the passage; a reply
//! is written after the replies of its comment, on its line; Resolve, Accept and Reject act from the cards.

use super::pending::{settle, text};
use super::*;
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
