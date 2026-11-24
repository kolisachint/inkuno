import Foundation
import InkunoCore

/// A comment of the note, as the Comments panel lists it.
struct NoteComment: Identifiable {
    var id: Int
    /// The commented text; empty for a comment on its own.
    var passage: String
    var who: String
    var time: String
    var text: String
    /// 0-based line number, for `onSelectLine`.
    var line: Int
}

/// The comments of a note: `passage<!-- comment: text -->` (core `comment`, `markdown/mod.rs`). The
/// parse, the write and the outline are the core's; what is left here is what the desktop keeps in
/// its shell and editor, not in the core.
enum CommentFormat {
    /// The comments outside code blocks, in order (core `allComments`).
    static func parse(_ text: String) -> [NoteComment] {
        allComments(text: text).enumerated().map { index, comment in
            let (who, time, said) = splitByline(comment.said)
            return NoteComment(
                id: index, passage: comment.noted, who: who, time: time, text: said,
                line: lineNumber(of: comment.startUtf16, in: text)
            )
        }
    }

    /// The line with a comment written after it: `passage<!-- comment: said -->`. The passage is the part of
    /// the line after its last comment, so a line can take another comment. Nil when the line is blank,
    /// or the core refuses the comment (empty text, comment marks in the passage).
    static func commented(line: String, said: String) -> String? {
        var passage = line
        if let close = line.range(of: "-->", options: .backwards) {
            passage = String(line[close.upperBound...])
        }
        guard !passage.trimmingCharacters(in: .whitespaces).isEmpty else { return nil }
        return try? commentOn(text: line, passage: passage, comment: said)
    }

    /// Resolve: drop the `<!-- comment: … -->` markup, keep the passage. A comment alone on its
    /// line takes the line with it.
    // ponytail: the desktop does this in its editor (crates/inkuno/src/views/editor/links.rs), which the core
    // does not export; the rule is mirrored here.
    static func resolve(in text: String, comment: NoteComment) -> String {
        var lines = text.components(separatedBy: "\n")
        guard lines.indices.contains(comment.line) else { return text }
        let line = lines[comment.line]
        guard let openRange = line.range(of: "<!-- comment:"),
              let closeRange = line.range(of: "-->", range: openRange.upperBound..<line.endIndex) else {
            return text
        }
        let without = String(line[..<openRange.lowerBound]) + String(line[closeRange.upperBound...])
        if without.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            lines.remove(at: comment.line)
        } else {
            lines[comment.line] = without
        }
        return lines.joined(separator: "\n")
    }

    /// The 0-based line of a UTF-16 offset of `text`: the line breaks before it.
    private static func lineNumber(of offset: UInt64, in text: String) -> Int {
        text.utf16.prefix(Int(offset)).reduce(0) { $1 == 10 ? $0 + 1 : $0 }
    }

    /// The byline a comment may open with, `Name, HH:MM: ` (who and when it was written). The core gives
    /// what is said; the panel shows the name and the time apart, as the desktop's shell does.
    // ponytail: the desktop splits this in its shell (crates/inkuno/src/shell/comments.rs, `split_byline`),
    // which the core does not export; the rule is mirrored here.
    static func splitByline(_ said: String) -> (who: String, time: String, text: String) {
        guard let headEnd = said.range(of: ": "),
              let split = String(said[..<headEnd.lowerBound]).range(of: ", ", options: .backwards) else {
            return ("", "", said)
        }
        let name = String(said[..<split.lowerBound])
        let time = String(said[split.upperBound..<headEnd.lowerBound])
        let text = String(said[headEnd.upperBound...])
        let clock = time.count == 5 && time[time.index(time.startIndex, offsetBy: 2)] == ":"
            && time.enumerated().allSatisfy { i, c in i == 2 || c.isNumber }
        guard clock, !name.isEmpty, name.count <= 40, !name.contains("\n") else {
            return ("", "", said)
        }
        return (name, time, text)
    }
}
