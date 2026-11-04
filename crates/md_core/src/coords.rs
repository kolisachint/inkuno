//! Coordinate conversion between byte offsets and (line, byte column) points (task 2.2).
//!
//! Lines are split on `\n` only. A line's column is its byte offset from the start of the
//! line, so a multi-byte character counts as several columns. `\r` is an ordinary byte.

use crate::buffer::Buffer;

/// A position in the text: 0-based line and byte column within that line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Point {
    pub line: usize,
    pub column: usize,
}

/// Convert a byte offset into a [`Point`].
///
/// Panics if `offset` is past the end of `text` or not on a char boundary.
pub fn offset_to_point(text: &str, offset: usize) -> Point {
    assert!(
        text.is_char_boundary(offset),
        "offset {offset} is out of bounds or not on a char boundary"
    );
    offset_to_point_from_bytes(text.bytes(), offset)
}

/// Convert a [`Point`] into a byte offset.
///
/// The column is clamped to the end of its line, so a column past the line end gives the
/// offset of the line's `\n` (or the end of the text on the last line). A column that falls
/// inside a multi-byte character rounds down to that character's start. A line past the last
/// line gives `text.len()`.
pub fn point_to_offset(text: &str, point: Point) -> usize {
    point_to_offset_from_bytes(text.bytes(), text.len(), point)
}

/// Shared implementation of [`offset_to_point`] over any byte stream that starts at offset 0.
/// The caller must have checked that `offset` is a char boundary within the stream.
pub(crate) fn offset_to_point_from_bytes(bytes: impl Iterator<Item = u8>, offset: usize) -> Point {
    let mut line = 0;
    let mut line_start = 0;
    for (i, b) in bytes.take(offset).enumerate() {
        if b == b'\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    Point {
        line,
        column: offset - line_start,
    }
}

/// Shared implementation of [`point_to_offset`] over any byte stream that starts at offset 0.
/// `total` is the stream's length in bytes.
pub(crate) fn point_to_offset_from_bytes(
    bytes: impl Iterator<Item = u8>,
    total: usize,
    point: Point,
) -> usize {
    let mut bytes = bytes.enumerate();

    // Find where the target line starts.
    let mut line = 0;
    let mut line_start = 0;
    while line < point.line {
        match bytes.next() {
            Some((i, b'\n')) => {
                line += 1;
                line_start = i + 1;
            }
            Some(_) => {}
            None => return total,
        }
    }

    // Walk the target line, stopping at the requested column or the line end.
    let mut result = line_start;
    for (i, b) in bytes {
        if i - line_start > point.column {
            return result;
        }
        if b == b'\n' {
            return i;
        }
        if (b & 0xC0) != 0x80 {
            result = i;
        }
    }
    // Reached the end of the text without a newline.
    if total - line_start <= point.column {
        total
    } else {
        result
    }
}

impl Buffer {
    /// Convert a byte offset into a [`Point`]. Panics like [`offset_to_point`].
    /// Cost is linear in `offset`.
    pub fn offset_to_point(&self, offset: usize) -> Point {
        assert!(
            offset <= self.len() && self.is_char_boundary(offset),
            "offset {offset} is out of bounds or not on a char boundary"
        );
        offset_to_point_from_bytes(self.rope_bytes(), offset)
    }

    /// Convert a [`Point`] into a byte offset. Clamps like [`point_to_offset`].
    /// Cost is linear in the target line's position.
    pub fn point_to_offset(&self, point: Point) -> usize {
        point_to_offset_from_bytes(self.rope_bytes(), self.len(), point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Inputs covering empty text, empty lines, a trailing newline, multi-byte characters
    /// and emoji.
    const INPUTS: &[&str] = &[
        "",
        "\n",
        "\n\n\n",
        "a",
        "a\n",
        "a\n\nb\n",
        "hello\nworld",
        "héllo\n日本語\n\n🎉 end",
        "🎉\n🎉🎉\n",
    ];

    fn p(line: usize, column: usize) -> Point {
        Point { line, column }
    }

    #[test]
    fn offset_to_point_known_values() {
        let text = "a\nbc\n\nd";
        assert_eq!(offset_to_point(text, 0), p(0, 0));
        assert_eq!(offset_to_point(text, 1), p(0, 1)); // at the '\n'
        assert_eq!(offset_to_point(text, 2), p(1, 0));
        assert_eq!(offset_to_point(text, 4), p(1, 2)); // at the second '\n'
        assert_eq!(offset_to_point(text, 5), p(2, 0)); // empty line
        assert_eq!(offset_to_point(text, 6), p(3, 0));
        assert_eq!(offset_to_point(text, 7), p(3, 1)); // end
    }

    #[test]
    fn offset_to_point_multibyte_columns_are_bytes() {
        // 'é' is 2 bytes, '日' 3 bytes, '🎉' 4 bytes.
        let text = "é日\n🎉x";
        assert_eq!(offset_to_point(text, 2), p(0, 2)); // after é
        assert_eq!(offset_to_point(text, 5), p(0, 5)); // after 日
        assert_eq!(offset_to_point(text, 6), p(1, 0));
        assert_eq!(offset_to_point(text, 10), p(1, 4)); // after 🎉
        assert_eq!(offset_to_point(text, 11), p(1, 5));
    }

    #[test]
    fn point_to_offset_known_values() {
        let text = "a\nbc\n\nd";
        assert_eq!(point_to_offset(text, p(0, 0)), 0);
        assert_eq!(point_to_offset(text, p(1, 1)), 3);
        assert_eq!(point_to_offset(text, p(2, 0)), 5);
        assert_eq!(point_to_offset(text, p(3, 0)), 6);
    }

    #[test]
    fn point_to_offset_clamps_column_to_line_length() {
        let text = "ab\ncde\n";
        assert_eq!(point_to_offset(text, p(0, 99)), 2); // '\n' of line 0
        assert_eq!(point_to_offset(text, p(1, 99)), 6); // '\n' of line 1
                                                        // Last, empty line: clamps to end of text.
        assert_eq!(point_to_offset(text, p(2, 99)), 7);
    }

    #[test]
    fn point_to_offset_clamps_line_past_end_to_text_end() {
        assert_eq!(point_to_offset("ab\ncd", p(5, 0)), 5);
        assert_eq!(point_to_offset("", p(1, 0)), 0);
    }

    #[test]
    fn point_to_offset_column_inside_char_rounds_down() {
        // 'é' occupies bytes 1..3 of "aé!".
        assert_eq!(point_to_offset("aé!", p(0, 2)), 1);
        assert_eq!(point_to_offset("aé!", p(0, 3)), 3);
    }

    #[test]
    fn round_trip_on_every_offset_of_every_input() {
        for text in INPUTS {
            for offset in 0..=text.len() {
                if !text.is_char_boundary(offset) {
                    continue;
                }
                let point = offset_to_point(text, offset);
                assert_eq!(
                    point_to_offset(text, point),
                    offset,
                    "round trip failed for {text:?} at offset {offset} (point {point:?})"
                );
            }
        }
    }

    #[test]
    fn points_are_ordered_by_line_then_column() {
        assert!(p(0, 9) < p(1, 0));
        assert!(p(1, 0) < p(1, 1));
    }

    #[test]
    #[should_panic]
    fn offset_past_end_panics() {
        offset_to_point("abc", 4);
    }

    #[test]
    #[should_panic]
    fn offset_inside_char_panics() {
        offset_to_point("é", 1);
    }

    #[test]
    fn buffer_conversions_match_free_functions() {
        for text in INPUTS {
            let buffer = Buffer::new(text);
            for offset in 0..=text.len() {
                if !text.is_char_boundary(offset) {
                    continue;
                }
                let point = offset_to_point(text, offset);
                assert_eq!(buffer.offset_to_point(offset), point, "{text:?} @ {offset}");
                assert_eq!(buffer.point_to_offset(point), offset, "{text:?} @ {offset}");
            }
            // Clamping behaves the same as the free function.
            assert_eq!(
                buffer.point_to_offset(p(0, 999)),
                point_to_offset(text, p(0, 999))
            );
            assert_eq!(
                buffer.point_to_offset(p(99, 0)),
                point_to_offset(text, p(99, 0))
            );
        }
    }
}
