//! Offsets: the Rust side counts bytes, UITextView counts UTF-16 code units. Every record that
//! crosses the FFI carries UTF-16 offsets (`_utf16`); these helpers convert the rest.

/// The UTF-16 offset of byte `at` of `text`. `at` is clamped to the text and rounded down to a
/// character boundary, so a position inside a character maps to the start of that character.
pub(crate) fn offset_utf16(text: &str, at: usize) -> u64 {
    let mut at = at.min(text.len());
    while !text.is_char_boundary(at) {
        at -= 1;
    }
    text[..at].encode_utf16().count() as u64
}

/// The byte offset of the UTF-16 offset `at` of `text`. Past the end: the end of the text. An offset
/// inside a surrogate pair (an emoji split in two) maps to the start of that character.
pub(crate) fn byte_offset(text: &str, at: u64) -> usize {
    let mut units = 0u64;
    for (i, c) in text.char_indices() {
        let width = c.len_utf16() as u64;
        if units + width > at {
            return i;
        }
        units += width;
    }
    text.len()
}

/// The UTF-16 length of `text`.
pub(crate) fn utf16_len(text: &str) -> u64 {
    text.encode_utf16().count() as u64
}

/// The UTF-16 offset of the UTF-8 byte offset `offset` of `text` (clamped, see `offset_utf16`).
#[uniffi::export]
pub fn utf8_to_utf16(text: String, offset: u64) -> u64 {
    offset_utf16(&text, offset as usize)
}

/// The UTF-8 byte offset of the UTF-16 offset `offset` of `text` (see `byte_offset`).
#[uniffi::export]
pub fn utf16_to_utf8(text: String, offset: u64) -> u64 {
    byte_offset(&text, offset) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_offsets_are_equal() {
        assert_eq!(utf8_to_utf16("hello".into(), 3), 3);
        assert_eq!(utf16_to_utf8("hello".into(), 3), 3);
    }

    #[test]
    fn accents_take_one_unit_and_two_bytes() {
        // "é" is 2 bytes and 1 unit; "thème" ends at byte 6 (unit 5).
        let text = "thème rouge";
        assert_eq!(utf8_to_utf16(text.into(), 6), 5);
        assert_eq!(utf16_to_utf8(text.into(), 5), 6);
        assert_eq!(utf16_to_utf8(text.into(), 7), 8);
    }

    #[test]
    fn emoji_take_two_units_and_four_bytes() {
        // "a😀b": 😀 is 4 bytes, 2 units.
        let text = "a😀b";
        assert_eq!(utf8_to_utf16(text.into(), 1), 1);
        assert_eq!(utf8_to_utf16(text.into(), 5), 3);
        assert_eq!(utf16_to_utf8(text.into(), 3), 5);
        assert_eq!(utf16_to_utf8(text.into(), 1), 1);
    }

    #[test]
    fn a_position_inside_a_surrogate_pair_rounds_down() {
        let text = "a😀b";
        // Unit 2 is the second half of 😀: the byte where 😀 starts.
        assert_eq!(utf16_to_utf8(text.into(), 2), 1);
        // A byte inside 😀 rounds down to its start (unit 1).
        assert_eq!(utf8_to_utf16(text.into(), 2), 1);
        assert_eq!(utf8_to_utf16(text.into(), 3), 1);
    }

    #[test]
    fn out_of_range_offsets_are_clamped() {
        assert_eq!(utf16_to_utf8("ab".into(), 99), 2);
        assert_eq!(utf8_to_utf16("ab".into(), 99), 2);
        assert_eq!(utf16_to_utf8("".into(), 0), 0);
        assert_eq!(utf8_to_utf16("".into(), 0), 0);
    }

    #[test]
    fn round_trip_on_every_char_boundary() {
        let text = "é 😀 中文 **x**\n✓";
        for (i, _) in text.char_indices() {
            let units = utf8_to_utf16(text.into(), i as u64);
            assert_eq!(utf16_to_utf8(text.into(), units) as usize, i);
        }
        assert_eq!(utf16_len(text), text.encode_utf16().count() as u64);
    }
}
