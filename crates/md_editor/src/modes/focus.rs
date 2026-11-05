//! Focus Mode (plan task 8.3, DESIGN §5.2).
//!
//! When Focus Mode is on, every block other than the caret's block is drawn at reduced opacity.
//! The opacity is applied to the whole block item (see `blocks::item_element`), so text, gutters,
//! decorations, backgrounds, tables and images are all dimmed together.

use std::ops::Range;

/// Opacity of a block that does not hold the caret, while Focus Mode is on.
pub const DIM: f32 = 0.3;

/// Whether the caret is in a block drawn over `block_range` of the buffer.
///
/// The range is the block's drawn range, its trailing newline excluded (as the block list
/// stores it). A caret at the end of a block belongs to that block, and a caret at the start of
/// the next block belongs to the next one. This is the same rule the reveal logic uses, so the
/// block that shows its markers is the block at full opacity.
pub fn caret_in_block(block_range: &Range<usize>, caret: usize) -> bool {
    block_range.start <= caret && caret <= block_range.end
}

/// Opacity of a block drawn over `block_range`: 1.0 when Focus Mode is off or the caret is in
/// the block, and [`DIM`] for every other block while Focus Mode is on. Pure, unit-tested.
pub fn block_opacity(focus_on: bool, block_range: &Range<usize>, caret: usize) -> f32 {
    if !focus_on || caret_in_block(block_range, caret) {
        1.0
    } else {
        DIM
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focus_off_dims_nothing() {
        assert_eq!(block_opacity(false, &(0..5), 2), 1.0);
        assert_eq!(block_opacity(false, &(6..9), 2), 1.0);
    }

    #[test]
    fn caret_block_is_full_and_others_are_dim() {
        // "ab\n\ncd": blocks "ab" 0..2 and "cd" 4..6 (the blank line is 3..3).
        assert_eq!(block_opacity(true, &(0..2), 1), 1.0);
        assert_eq!(block_opacity(true, &(4..6), 1), DIM);
        assert_eq!(block_opacity(true, &(4..6), 5), 1.0);
        assert_eq!(block_opacity(true, &(0..2), 5), DIM);
    }

    #[test]
    fn caret_at_end_of_block_belongs_to_that_block() {
        // The caret before the newline that ends block 0..2 is still in block 0.
        assert_eq!(block_opacity(true, &(0..2), 2), 1.0);
        assert!(caret_in_block(&(0..2), 2));
        // The caret after that newline is at the start of block 4..6, not the end of 0..2.
        assert!(!caret_in_block(&(0..2), 3));
        assert_eq!(block_opacity(true, &(4..6), 3), DIM);
    }

    #[test]
    fn caret_at_start_of_block_belongs_to_that_block() {
        assert_eq!(block_opacity(true, &(4..6), 4), 1.0);
        assert_eq!(block_opacity(true, &(0..2), 4), DIM);
    }

    #[test]
    fn blank_block_takes_the_caret_only_at_its_offset() {
        // The blank line between "ab" and "cd" is the empty range 3..3.
        assert_eq!(block_opacity(true, &(3..3), 3), 1.0);
        assert_eq!(block_opacity(true, &(3..3), 2), DIM);
        assert_eq!(block_opacity(true, &(3..3), 4), DIM);
    }

    #[test]
    fn caret_past_all_blocks_dims_every_block() {
        assert_eq!(block_opacity(true, &(0..2), 10), DIM);
        assert_eq!(block_opacity(true, &(4..6), 10), DIM);
    }
}
