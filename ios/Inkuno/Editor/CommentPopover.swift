import InkunoCore
import SwiftUI

/// The comment a tap on its passage opens: the passage quoted, who and when, the text, its replies, a Reply
/// field and Resolve. Reply and Resolve are the core's, through `onReply` and `onResolve` (`CommentFormat`).
/// It is `CommentCard` without Go to line, since the passage is on screen behind it.
struct CommentPopover: View {
    /// The comment shown: its passage, author, time, text and replies.
    let comment: NoteComment
    /// Writes the reply; true when the note was written, so the field is cleared.
    var onReply: (String) -> Bool
    /// Marks the comment resolved in the note.
    var onResolve: () -> Void

    /// The reply the field holds, not yet sent.
    @State private var reply = ""

    /// True when the reply field holds text, so Reply can send it.
    private var canReply: Bool {
        !reply.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    /// The popover's content: the comment with its replies, then the Reply field and Resolve.
    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                if !comment.passage.isEmpty {
                    Text(comment.passage)
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.soft)
                        .padding(.leading, 8)
                        .overlay(alignment: .leading) {
                            RoundedRectangle(cornerRadius: 2)
                                .fill(InkColor.commentDot)
                                .frame(width: 3)
                        }
                }
                byline(name: CommentFormat.displayName(comment.author), time: comment.time)
                Text(comment.text.isEmpty ? "(empty)" : comment.text)
                    .font(InkType.body)
                    .foregroundStyle(InkColor.ink)
                ForEach(Array(comment.replies.enumerated()), id: \.offset) { _, item in
                    VStack(alignment: .leading, spacing: 2) {
                        byline(name: CommentFormat.displayName(item.author), time: item.time)
                        Text(item.text)
                            .font(InkType.body)
                            .foregroundStyle(InkColor.ink)
                    }
                    .padding(.leading, 12)
                    .overlay(alignment: .leading) {
                        Rectangle()
                            .fill(InkColor.line)
                            .frame(width: 2)
                    }
                }
                replyField
                Button("Resolve", action: onResolve)
                    .buttonStyle(.bordered)
                    .tint(InkColor.accent)
                    .controlSize(.small)
                    .padding(.top, 4)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(InkMetrics.cardPaddingH)
        }
        .background(InkColor.sheet)
    }

    /// The author's initial in a dot, the name, and the relative time when there is one.
    private func byline(name: String, time: String?) -> some View {
        HStack(spacing: 6) {
            Text(String(name.prefix(1)).uppercased())
                .font(InkType.caption)
                .foregroundStyle(InkColor.sheet)
                .frame(width: 20, height: 20)
                .background(Circle().fill(InkColor.accent))
            Text(name)
                .font(InkType.caption)
                .foregroundStyle(InkColor.ink)
            if let time {
                Text(CommentFormat.relativeTime(time))
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.faint)
            }
        }
    }

    /// The reply field and Send, which writes the reply through `onReply`.
    private var replyField: some View {
        HStack(spacing: 8) {
            TextField("Reply", text: $reply)
                .font(InkType.body)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(InkColor.ground, in: RoundedRectangle(cornerRadius: InkRadius.field, style: .continuous))
            Button("Send") {
                if onReply(reply) { reply = "" }
            }
            .buttonStyle(.bordered)
            .tint(InkColor.accent)
            .controlSize(.small)
            .disabled(!canReply)
        }
        .padding(.top, 4)
    }
}
