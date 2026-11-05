//! Typewriter Mode (plan task 8.4).
//!
//! When Typewriter Mode is on, the caret line is scrolled to a fixed fraction of the viewport
//! height (the centre by default) after each caret move or edit. The block list adds a spacer
//! item of half the viewport at the top and at the bottom, so the first and last lines can
//! reach that position too. The scroll is animated over [`ANIMATION_MS`].
//!
//! The per-frame step runs from `BlockHost::paint` (see `block_list.rs`), after the visible
//! blocks have been painted, because the caret's position is only known then. Scrolling in
//! paint takes effect on the next frame, so the step asks for one more frame while it moves.

use std::time::Instant;

use gpui::{px, App, Entity, ListOffset, Window};

use crate::editor::Editor;

/// Where the caret line sits in the viewport, as a fraction of its height (0 is the top).
pub const TYPEWRITER_FRACTION: f32 = 0.5;

/// Length of the scroll animation after a caret move.
pub const ANIMATION_MS: f32 = 120.0;

/// Frames to wait for the caret's block to be drawn after bringing its item into view.
const MAX_RETRIES: u8 = 8;

/// Typewriter scroll state, held by `Editor` (field `typewriter`). Reset when Typewriter Mode
/// is switched on or off.
#[derive(Default)]
pub struct TypewriterState {
    /// Buffer version and caret offset the caret was last centred for.
    key: Option<(u64, usize)>,
    /// The scroll animation in progress, if any.
    anim: Option<Animation>,
    /// Frames spent waiting for the caret's block to be drawn.
    retries: u8,
}

/// One scroll animation: when it started and how much of the distance it has covered.
struct Animation {
    started: Instant,
    /// Eased progress in `0..=1` already applied to the scroll.
    progress: f32,
}

/// Scroll offset change that puts a line's centre at `fraction` of the viewport height.
///
/// `caret_y` is the line's top in the same space as the viewport top (for example, content
/// coordinates, where the viewport starts at the scroll offset). The result is the scroll
/// offset that puts the line there. Pure, unit-tested.
pub fn target_scroll(caret_y: f32, line_height: f32, viewport_h: f32, fraction: f32) -> f32 {
    caret_y + line_height / 2.0 - fraction * viewport_h
}

/// Height of the spacer item at each end of the list, for a viewport this tall.
///
/// It is half the viewport, so the first and the last block can each be centred. The spacers
/// are list items rather than list padding: GPUI lays items out only from the scroll anchor
/// downwards, so padding above the first block would stay blank and hide the text above the
/// caret. Pure, unit-tested.
pub fn spacer_height(viewport_h: f32) -> f32 {
    viewport_h / 2.0
}

/// Ease-out (cubic) interpolation from `from` to `to` at time `t` in `0..=1`. Pure.
pub fn ease(from: f32, to: f32, t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    from + (to - from) * (1.0 - (1.0 - t).powi(3))
}

/// The part of `remaining` to scroll in one frame, when the eased progress goes from `prev` to
/// `now` (both in `0..=1`). Scrolling this share of what is left each frame adds up to the
/// whole distance by the end of the animation, even when the target has moved. Pure.
pub fn frame_step(remaining: f32, prev: f32, now: f32) -> f32 {
    if prev >= 1.0 {
        return remaining;
    }
    remaining * ((now - prev) / (1.0 - prev)).clamp(0.0, 1.0)
}

/// Centre the caret line, if the caret moved or the text changed since the last centring. Runs
/// one animation step per frame. Called from `BlockHost::paint` when Typewriter Mode is on.
pub(crate) fn step(editor: &Entity<Editor>, window: &mut Window, cx: &mut App) {
    let (enabled, version, head) = {
        let editor = editor.read(cx);
        (
            editor.modes().typewriter,
            editor.buffer().version(),
            editor.selection.selection.head,
        )
    };
    if !enabled {
        return;
    }
    let (pending, animating, retries) = {
        let state = &editor.read(cx).typewriter;
        (
            state.key != Some((version, head)),
            state.anim.is_some(),
            state.retries,
        )
    };
    if !pending && !animating {
        return;
    }

    let list = editor.read(cx).blocks.list.clone();
    let viewport = list.viewport_bounds();
    let caret = editor
        .read(cx)
        .layouts
        .block_for_offset(head)
        .and_then(|block| {
            block
                .line_for_offset(head)
                .map(|line| (line.origin.y, block.snapshot.line_height))
        });
    let Some((line_top, line_height)) = caret else {
        // The caret's block is not drawn (it is scrolled out of view). Bring its item to the top
        // of the list. It is drawn next frame and centred then.
        let index = editor.read(cx).blocks.item_index_at(head);
        if let (true, Some(index)) = (retries < MAX_RETRIES, index) {
            list.scroll_to(ListOffset {
                item_ix: index,
                offset_in_item: px(0.),
            });
            editor.update(cx, |editor, _cx| editor.typewriter.retries += 1);
            request_frame(editor, window);
        }
        return;
    };

    let delta = target_scroll(
        f32::from(line_top - viewport.origin.y),
        f32::from(line_height),
        f32::from(viewport.size.height),
        TYPEWRITER_FRACTION,
    );
    // A new caret position starts a new animation. Otherwise continue the one in progress.
    let anim = editor.update(cx, |editor, _cx| {
        let state = &mut editor.typewriter;
        if pending {
            state.key = Some((version, head));
            state.retries = 0;
            state.anim = None;
        }
        state.anim.take()
    });
    let anim = match anim {
        Some(anim) => anim,
        None => Animation {
            started: Instant::now(),
            progress: 0.0,
        },
    };

    let t = anim.started.elapsed().as_secs_f32() * 1000.0 / ANIMATION_MS;
    let progress = ease(0.0, 1.0, t);
    let done = t >= 1.0 || delta.abs() < 0.5;
    let step = if done {
        delta
    } else {
        frame_step(delta, anim.progress, progress)
    };
    list.scroll_by(px(step));
    let next = (!done).then_some(Animation {
        started: anim.started,
        progress,
    });
    editor.update(cx, |editor, _cx| editor.typewriter.anim = next);
    if !done || step != 0.0 {
        // The list was painted before this scroll, so paint again to show it.
        request_frame(editor, window);
    }
}

/// Redraw the editor on the next frame.
fn request_frame(editor: &Entity<Editor>, window: &mut Window) {
    let entity = editor.entity_id();
    window.on_next_frame(move |_window, cx| cx.notify(entity));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_scroll_centres_a_line_at_the_fraction() {
        // A 20 px line whose centre is at the middle of a 400 px viewport needs no scroll.
        assert_eq!(target_scroll(190.0, 20.0, 400.0, 0.5), 0.0);
        // A line lower in the viewport needs a positive scroll to move it up.
        assert!(target_scroll(300.0, 20.0, 400.0, 0.5) > 0.0);
        assert!(target_scroll(50.0, 20.0, 400.0, 0.5) < 0.0);
        // The fraction moves the target: 0 puts the line at the top.
        assert_eq!(target_scroll(100.0, 20.0, 400.0, 0.0), 110.0);
    }

    #[test]
    fn first_and_last_lines_reach_the_centre_with_padding() {
        for &(line, viewport, lines) in &[
            (20.0_f32, 400.0_f32, 3_usize),
            (20.0, 400.0, 300),
            (24.0, 800.0, 1),
            (2.0, 400.0, 3),
            (17.5, 333.0, 2),
            (30.0, 120.0, 1000),
        ] {
            let pad = spacer_height(viewport);
            let content = lines as f32 * line + 2.0 * pad;
            let max_scroll = (content - viewport).max(0.0);
            // The first line is drawn just below the top spacer.
            let first = target_scroll(pad, line, viewport, TYPEWRITER_FRACTION);
            assert!(
                (0.0..=max_scroll).contains(&first),
                "first line: target {first} outside 0..={max_scroll} ({lines} lines)"
            );
            // The last line is drawn just above the bottom spacer.
            let last_y = pad + (lines as f32 - 1.0) * line;
            let last = target_scroll(last_y, line, viewport, TYPEWRITER_FRACTION);
            assert!(
                (0.0..=max_scroll).contains(&last),
                "last line: target {last} outside 0..={max_scroll} ({lines} lines)"
            );
        }
    }

    #[test]
    fn without_padding_the_first_line_cannot_reach_the_centre() {
        // That is why the padding is needed: the first line would need a negative scroll.
        assert!(target_scroll(0.0, 20.0, 400.0, TYPEWRITER_FRACTION) < 0.0);
    }

    #[test]
    fn ease_goes_from_start_to_end_and_slows_down() {
        assert_eq!(ease(0.0, 10.0, 0.0), 0.0);
        assert_eq!(ease(0.0, 10.0, 1.0), 10.0);
        assert_eq!(ease(10.0, 20.0, 0.0), 10.0);
        assert_eq!(ease(0.0, 10.0, 2.0), 10.0, "t is clamped");
        // Ease-out covers more than half the distance in the first half of the time.
        assert!(ease(0.0, 1.0, 0.5) > 0.5);
        let mut prev = 0.0;
        for i in 1..=20 {
            let now = ease(0.0, 1.0, i as f32 / 20.0);
            assert!(now >= prev, "ease never goes back");
            prev = now;
        }
    }

    #[test]
    fn frame_steps_add_up_to_the_whole_distance() {
        let mut remaining = 100.0_f32;
        let mut prev = 0.0_f32;
        for frame in 1..=12 {
            let now = ease(0.0, 1.0, frame as f32 / 12.0);
            let step = frame_step(remaining, prev, now);
            assert!(step >= 0.0);
            remaining -= step;
            prev = now;
        }
        assert!(remaining.abs() < 1e-3, "remaining {remaining}");
        assert_eq!(frame_step(40.0, 0.3, 0.3), 0.0, "no time passed, no move");
        assert_eq!(frame_step(40.0, 1.0, 1.0), 40.0, "finished, move the rest");
    }
}
