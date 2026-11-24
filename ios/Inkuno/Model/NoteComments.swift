import Foundation
import InkunoCore

/// A comment of the note, as the Comments panel lists it.
struct NoteComment: Identifiable {
    var id: Int
    /// The commented text; empty for a comment on its own.
    var passage: String
    /// Who wrote it and when (UTC, `2026-11-24T10:02Z`); nil when unknown (the legacy form, or `Unknown`).
    var author: String?
    var time: String?
    var text: String
    var replies: [ReplyInfo]
    /// Where the comment starts, in UTF-16 units of the note: what the core finds it by.
    var startUtf16: UInt64
    /// 0-based line number, for `onSelectLine`.
    var line: Int
}

/// The comments of a note: `passage<!-- comment @name time: text ↳ @name time: reply -->`. The parse, the
/// write, the replies and Resolve are the core's (`inkuno-logic/src/markdown/comments.rs`); what is left
/// here is the passage the editor writes after, the line number and the relative time.
enum CommentFormat {
    /// The comments outside code blocks, in order (core `allComments`).
    static func parse(_ text: String) -> [NoteComment] {
        allComments(text: text).enumerated().map { index, comment in
            NoteComment(
                id: index, passage: comment.noted, author: comment.author, time: comment.time,
                text: comment.said, replies: comment.replies, startUtf16: comment.startUtf16,
                line: lineNumber(of: comment.startUtf16, in: text)
            )
        }
    }

    /// The line with a comment written after it, by `author` at `at` (core `commentAddBy`): the passage is the
    /// part of the line after its last comment, so a line can take another comment. Nil when the line is blank,
    /// or the core refuses the comment (empty text, comment marks in the passage).
    static func commented(
        line: String, said: String, author: String = AuthorName.resolved(), at: Date = Date()
    ) -> String? {
        var passage = line
        if let close = line.range(of: "-->", options: .backwards) {
            passage = String(line[close.upperBound...])
        }
        guard !passage.trimmingCharacters(in: .whitespaces).isEmpty else { return nil }
        return try? commentAddBy(
            text: line, passage: passage, comment: said, author: author,
            unixTime: Int64(at.timeIntervalSince1970)
        )
    }

    /// The note with the comment resolved: its markup goes, the passage stays (core `commentResolve`).
    static func resolve(in text: String, comment: NoteComment) throws -> String {
        try commentResolve(text: text, commentStartUtf16: comment.startUtf16)
    }

    /// The note with `said` added as a reply to the comment, by `author` at `at` (core `commentReply`).
    static func reply(
        in text: String, comment: NoteComment, said: String, author: String = AuthorName.resolved(), at: Date = Date()
    ) throws -> String {
        try commentReply(
            text: text, commentStartUtf16: comment.startUtf16, reply: said, author: author,
            unixTime: Int64(at.timeIntervalSince1970)
        )
    }

    /// The name a comment or reply is shown under: its author, or `Unknown`.
    static func displayName(_ author: String?) -> String {
        author ?? "Unknown"
    }

    /// The relative time of a stored UTC minute (`2026-11-24T10:02Z`), as the panel shows it; the stamp
    /// itself when it does not read as one.
    static func relativeTime(_ stamp: String, now: Date = Date()) -> String {
        guard let date = stampFormatter.date(from: stamp) else { return stamp }
        return RelativeDateTimeFormatter().localizedString(for: date, relativeTo: now)
    }

    private static let stampFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(identifier: "UTC")
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm'Z'"
        return formatter
    }()

    /// The 0-based line of a UTF-16 offset of `text`: the line breaks before it.
    private static func lineNumber(of offset: UInt64, in text: String) -> Int {
        text.utf16.prefix(Int(offset)).reduce(0) { $1 == 10 ? $0 + 1 : $0 }
    }
}
