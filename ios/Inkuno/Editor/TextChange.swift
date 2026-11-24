import Foundation

/// The changed part of an edit, found by comparing the old and the new text: everything before
/// `prefix` and after the changed part is the same in both. The editor uses it to restyle only the
/// lines an edit touched, whatever made the edit (typing, paste, undo, a replaced range).
struct TextChange: Equatable {
    /// UTF-16 length of the unchanged start.
    let prefix: Int
    /// End of the changed part in the old text.
    let oldEnd: Int
    /// End of the changed part in the new text.
    let newEnd: Int

    init(from old: NSString, to new: NSString) {
        let oldLength = old.length
        let newLength = new.length
        let limit = min(oldLength, newLength)
        var start = 0
        while start < limit && old.character(at: start) == new.character(at: start) {
            start += 1
        }
        var suffix = 0
        while suffix < limit - start
            && old.character(at: oldLength - 1 - suffix) == new.character(at: newLength - 1 - suffix) {
            suffix += 1
        }
        prefix = start
        oldEnd = oldLength - suffix
        newEnd = newLength - suffix
    }

    var isChanged: Bool {
        prefix < oldEnd || prefix < newEnd
    }

    /// Maps a position of the old text to the new text. A position inside the changed part maps to its end.
    func map(_ position: Int) -> Int {
        if position <= prefix {
            return position
        }
        if position >= oldEnd {
            return position + (newEnd - oldEnd)
        }
        return newEnd
    }
}
