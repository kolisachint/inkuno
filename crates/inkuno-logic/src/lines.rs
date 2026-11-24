//! Line numbers of a note: the 1-based line of a byte offset in its text, and the digits the gutter needs.
//! The numbers are the lines of the `.md` on disk, so a line the editor hides keeps its number and leaves a gap.

/// Where each line of a text starts, to find the line of a byte offset without scanning the text again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    /// The lines of `text`: a text ending in a newline has an empty last line, as an editor shows it.
    pub fn new(text: &str) -> Self {
        let mut starts = vec![0];
        starts.extend(text.match_indices('\n').map(|(i, _)| i + 1));
        Self { starts }
    }

    /// How many lines the text has.
    pub fn count(&self) -> usize {
        self.starts.len()
    }

    /// The 1-based number of the line holding `offset`. An offset past the end is on the last line.
    pub fn number_at(&self, offset: usize) -> usize {
        self.starts.partition_point(|&start| start <= offset)
    }

    /// Whether a line begins at `offset`: a visual row that does not is the rest of a line.
    pub fn starts_line(&self, offset: usize) -> bool {
        self.starts.binary_search(&offset).is_ok()
    }
}

/// The characters the widest of the numbers `1..=lines` takes: the width of the gutter, in digits (at least one).
pub fn digits(lines: usize) -> usize {
    lines.max(1).to_string().len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_text_is_one_line() {
        let index = LineIndex::new("");
        assert_eq!(index.count(), 1);
        assert_eq!(index.number_at(0), 1);
        assert!(index.starts_line(0));
    }

    #[test]
    fn each_offset_maps_to_the_line_it_is_on() {
        // "ab\ncd\nef": line 1 is bytes 0..3, line 2 is 3..6, line 3 is 6..8.
        let index = LineIndex::new("ab\ncd\nef");
        assert_eq!(index.count(), 3);
        let numbers: Vec<usize> = (0..=8).map(|i| index.number_at(i)).collect();
        assert_eq!(numbers, [1, 1, 1, 2, 2, 2, 3, 3, 3]);
        assert!(index.starts_line(0) && index.starts_line(3) && index.starts_line(6));
        assert!(!index.starts_line(1) && !index.starts_line(4));
    }

    #[test]
    fn a_trailing_newline_makes_an_empty_last_line() {
        let index = LineIndex::new("a\n");
        assert_eq!(index.count(), 2);
        assert_eq!(index.number_at(2), 2);
        assert!(index.starts_line(2));
    }

    #[test]
    fn an_offset_past_the_end_is_on_the_last_line() {
        let index = LineIndex::new("a\nb");
        assert_eq!(index.number_at(99), 2);
    }

    #[test]
    fn digits_fit_the_widest_number() {
        assert_eq!(digits(0), 1);
        assert_eq!(digits(1), 1);
        assert_eq!(digits(9), 1);
        assert_eq!(digits(10), 2);
        assert_eq!(digits(999), 3);
        assert_eq!(digits(1000), 4);
    }
}
