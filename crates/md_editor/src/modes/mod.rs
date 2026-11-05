//! Editor modes (phase 8). Modes are views over the same buffer (DESIGN §5).
//!
//! 8.4 adds the skeleton: the [`Modes`] flags on `Editor`, the three toggle actions, and the
//! Typewriter Mode implementation in [`typewriter`]. Source Mode (8.1, [`source`]) draws the
//! buffer as plain source. Focus Mode (8.3, [`focus`]) dims blocks other than the caret's. Key
//! bindings come in 8.5, so the toggles are not bound to any key here.
//!
//! Task 8.2: toggling Source Mode keeps the top visible block in view, see [`ScrollAnchor`]. The
//! caret is a source offset, so it stays where it is.
//!
//! Debug aids until 8.5: `NOTEAPP_TYPEWRITER=1`, `NOTEAPP_SOURCE=1` and `NOTEAPP_FOCUS=1` start
//! the editor with that mode on ([`startup_modes`]). Remove them when 8.5 binds the keys.

pub mod focus;
pub mod source;
pub mod typewriter;

use gpui::{actions, px, Div, Entity, ListOffset};

use crate::editor::Editor;
use crate::keys;

actions!(
    md_editor_modes,
    [ToggleSourceMode, ToggleFocusMode, ToggleTypewriterMode]
);

/// The mode flags. Each mode is independent, so Focus and Typewriter can be on together.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Modes {
    pub source: bool,
    pub focus: bool,
    pub typewriter: bool,
}

/// The modes an editor starts with. All off, except Typewriter when `NOTEAPP_TYPEWRITER=1` and
/// Source when `NOTEAPP_SOURCE=1`, Focus when `NOTEAPP_FOCUS=1`.
pub fn startup_modes() -> Modes {
    let on = |name: &str| std::env::var_os(name).is_some_and(|v| v.to_str() == Some("1"));
    Modes {
        source: on("NOTEAPP_SOURCE"),
        typewriter: on("NOTEAPP_TYPEWRITER"),
        focus: on("NOTEAPP_FOCUS"),
    }
}

/// The block at the top of the list and how far it is scrolled past its top (task 8.2). `block`
/// counts blocks only, not the Typewriter spacers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAnchor {
    pub block: usize,
    pub offset_px: f32,
}

/// The anchor for list offset `top`, when the list has `lead` spacer items before its blocks
/// and `blocks` blocks. A top inside a spacer is the first block, at its top. Pure.
pub fn anchor_for(top: ListOffset, lead: usize, blocks: usize) -> ScrollAnchor {
    if top.item_ix < lead {
        return ScrollAnchor {
            block: 0,
            offset_px: 0.,
        };
    }
    ScrollAnchor {
        block: (top.item_ix - lead).min(blocks.saturating_sub(1)),
        offset_px: f32::from(top.offset_in_item),
    }
}

/// The list offset that shows `anchor`, for a list with `lead` spacer items before its blocks.
/// It is the inverse of [`anchor_for`] for a top past the spacers. Pure.
pub fn offset_for(anchor: ScrollAnchor, lead: usize) -> ListOffset {
    ListOffset {
        item_ix: anchor.block + lead,
        offset_in_item: px(anchor.offset_px),
    }
}

impl Editor {
    /// The mode flags currently set.
    pub fn modes(&self) -> Modes {
        self.modes
    }

    /// Set the mode flags and redraw. Turning Typewriter on or off resets its scroll state, so
    /// the caret is centred again on the next frame.
    ///
    /// Turning Source Mode on or off (task 8.2) keeps the top visible block at the top. The
    /// selection is not touched, so the caret keeps its source offset. Heights change with the
    /// mode, so the list is remeasured first, then scrolled back to the saved block.
    pub fn set_modes(&mut self, modes: Modes, cx: &mut gpui::Context<Self>) {
        if self.modes.typewriter != modes.typewriter {
            self.typewriter = typewriter::TypewriterState::default();
        }
        if self.modes.source != modes.source {
            let anchor = self.blocks.top_anchor();
            self.modes = modes;
            self.blocks.list.remeasure();
            self.blocks.scroll_to_anchor(anchor);
        } else {
            self.modes = modes;
        }
        cx.notify();
    }
}

/// Wire the mode toggle actions to the editor root. Called from `Editor::render` (HOOK 8.4).
pub fn attach(root: Div, editor: &Entity<Editor>) -> Div {
    let root = keys::on::<ToggleSourceMode>(root, editor, |e, cx| {
        let modes = Modes {
            source: !e.modes.source,
            ..e.modes
        };
        e.set_modes(modes, cx);
    });
    let root = keys::on::<ToggleFocusMode>(root, editor, |e, cx| {
        let modes = Modes {
            focus: !e.modes.focus,
            ..e.modes
        };
        e.set_modes(modes, cx);
    });
    keys::on::<ToggleTypewriterMode>(root, editor, |e, cx| {
        let modes = Modes {
            typewriter: !e.modes.typewriter,
            ..e.modes
        };
        e.set_modes(modes, cx);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_start_off() {
        assert_eq!(
            Modes::default(),
            Modes {
                source: false,
                focus: false,
                typewriter: false
            }
        );
    }

    #[test]
    fn anchor_round_trips_past_the_spacers() {
        let top = ListOffset {
            item_ix: 1 + 412,
            offset_in_item: px(7.5),
        };
        let anchor = anchor_for(top, 1, 1000);
        assert_eq!(
            anchor,
            ScrollAnchor {
                block: 412,
                offset_px: 7.5
            }
        );
        assert_eq!(offset_for(anchor, 1), top);
    }

    #[test]
    fn anchor_inside_a_spacer_is_the_first_block() {
        let top = ListOffset {
            item_ix: 0,
            offset_in_item: px(3.),
        };
        assert_eq!(
            anchor_for(top, 1, 1000),
            ScrollAnchor {
                block: 0,
                offset_px: 0.
            }
        );
    }

    /// Task 8.2 on a 1,000-paragraph document (2,000 lines): toggling Source Mode on and off keeps
    /// the caret's source offset and the top visible block. Blank lines are blocks too, so
    /// paragraph `i` is block `2 * i`. The helpers are checked here, not GPUI paint: a toggle does
    /// not write the selection, and the anchor restores the same list offset.
    #[test]
    fn source_toggle_keeps_caret_and_top_block_in_a_long_document() {
        use crate::block_list::blocks_for;
        use crate::inline_render::document_hidden_ranges;

        let text: String = (0..1000).map(|i| format!("Line {i}\n\n")).collect();
        let blocks = blocks_for(&text);
        assert_eq!(blocks.len(), 2000);

        // Caret at the start of paragraph 700 (block 1400). The top visible list item is block
        // 824 (paragraph 412), 7.5 px into it, below one Typewriter spacer.
        let caret = blocks[1400].range.start;
        let block_of = |offset: usize| blocks.iter().position(|b| b.range.contains(&offset));
        assert_eq!(block_of(caret), Some(1400));
        let top = ListOffset {
            item_ix: 1 + 824,
            offset_in_item: px(7.5),
        };
        let anchor = anchor_for(top, 1, blocks.len());
        assert_eq!(anchor.block, 824);

        // Source Mode on: the caret is unchanged and its block is unchanged. The anchor
        // round-trips to the same list offset.
        let on_hidden = source::hidden_range_count(true, &text, Some(caret));
        let off_hidden = source::hidden_range_count(false, &text, Some(caret));
        assert_eq!(on_hidden, 0);
        assert_eq!(off_hidden, document_hidden_ranges(&text, Some(caret)).len());
        assert_eq!(block_of(caret), Some(1400), "caret block after toggle on");
        assert_eq!(offset_for(anchor, 1), top, "top block after toggle on");

        // Source Mode off: the same again.
        assert_eq!(block_of(caret), Some(1400), "caret block after toggle off");
        assert_eq!(offset_for(anchor, 1), top, "top block after toggle off");
        assert_eq!(anchor_for(offset_for(anchor, 1), 1, blocks.len()), anchor);
    }
}
