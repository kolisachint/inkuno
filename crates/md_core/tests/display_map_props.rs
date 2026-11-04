//! Property tests for the display map (task 2.8, DESIGN §3.2 and §4.1).
//!
//! Random documents and caret positions are checked against the invariants of
//! `DisplayMap`. The runner uses a fixed seed, so `cargo test` is repeatable.

use md_core::display_map::{Bias, Direction, DisplayMap};
use proptest::prelude::*;
use proptest::test_runner::RngSeed;

/// Tokens that build random documents. They include inline delimiters, a
/// multi-byte character and newlines.
const ALPHABET: &[&str] = &[
    "a", "b", "é", " ", "*", "**", "`", "~~", "$", "[", "]", "(", ")", "!", "\n",
];

fn document() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(ALPHABET), 0..40).prop_map(|parts| parts.concat())
}

/// Char boundaries of `text`, including 0 and `text.len()`.
fn boundaries(text: &str) -> Vec<usize> {
    (0..=text.len())
        .filter(|&i| text.is_char_boundary(i))
        .collect()
}

/// Pick a char boundary of `text` from an arbitrary index.
fn pick_boundary(text: &str, pick: usize) -> usize {
    let bounds = boundaries(text);
    bounds[pick % bounds.len()]
}

/// True when `offset` is strictly inside a hidden range (not a caret stop).
fn strictly_inside(map: &DisplayMap, offset: usize) -> bool {
    map.hidden()
        .iter()
        .any(|r| r.start < offset && offset < r.end)
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        rng_seed: RngSeed::Fixed(42),
        ..ProptestConfig::default()
    })]

    /// Every visible offset maps to a display position and back, with either bias.
    #[test]
    fn visible_offsets_round_trip(
        text in document(),
        caret in prop::option::of(any::<usize>()),
    ) {
        let caret = caret.map(|pick| pick_boundary(&text, pick));
        let map = DisplayMap::for_text(&text, caret);
        for s in boundaries(&text) {
            if strictly_inside(&map, s) {
                continue;
            }
            let d = map.to_display(s);
            let left = map.to_source(d, Bias::Left);
            let right = map.to_source(d, Bias::Right);
            prop_assert!(
                left == s || right == s,
                "source {} -> display {} -> left {} right {} (text {:?}, caret {:?})",
                s, d, left, right, text, caret
            );
        }
    }

    /// to_display never decreases as the source offset grows.
    #[test]
    fn to_display_is_monotonic(
        text in document(),
        caret in prop::option::of(any::<usize>()),
    ) {
        let caret = caret.map(|pick| pick_boundary(&text, pick));
        let map = DisplayMap::for_text(&text, caret);
        let mut prev = 0;
        for s in 0..=text.len() {
            let d = map.to_display(s);
            prop_assert!(d >= prev, "to_display({}) = {} < {}", s, d, prev);
            prev = d;
        }
    }

    /// The display string is the source minus the hidden bytes.
    #[test]
    fn display_len_is_source_minus_hidden(
        text in document(),
        caret in prop::option::of(any::<usize>()),
    ) {
        let caret = caret.map(|pick| pick_boundary(&text, pick));
        let map = DisplayMap::for_text(&text, caret);
        let hidden: usize = map.hidden().iter().map(|r| r.end - r.start).sum();
        prop_assert_eq!(map.display_len(), text.len() - hidden);
    }

    /// Caret movement from any caret stop lands on a char boundary that is not
    /// strictly inside a hidden range, in both directions.
    #[test]
    fn move_caret_never_lands_inside_hidden(
        text in document(),
        caret in prop::option::of(any::<usize>()),
        pick in any::<usize>(),
    ) {
        let caret = caret.map(|c| pick_boundary(&text, c));
        let map = DisplayMap::for_text(&text, caret);
        let start = pick_boundary(&text, pick);
        for dir in [Direction::Left, Direction::Right] {
            let next = map.move_caret(&text, start, dir);
            prop_assert!(next <= text.len());
            prop_assert!(text.is_char_boundary(next), "{:?} not a char boundary", next);
            prop_assert!(
                !strictly_inside(&map, next),
                "{:?} from {} landed inside a hidden range {:?}",
                dir, start, map.hidden()
            );
        }
    }

    /// Building the map never panics, and every hidden range lies inside the
    /// text on char boundaries.
    #[test]
    fn for_text_never_panics_and_hides_valid_ranges(
        text in document(),
        caret in prop::option::of(any::<usize>()),
    ) {
        let caret = caret.map(|pick| pick_boundary(&text, pick));
        let map = DisplayMap::for_text(&text, caret);
        for r in map.hidden() {
            prop_assert!(r.start < r.end && r.end <= text.len());
            prop_assert!(text.is_char_boundary(r.start) && text.is_char_boundary(r.end));
        }
    }
}
