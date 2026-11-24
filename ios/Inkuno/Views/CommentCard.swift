import InkunoCore
import SwiftUI

/// One comment in the Comments panel: the passage it is on, who wrote it and when, the text, its replies,
/// a reply field, Resolve and Go to line. Resolve and the reply are the core's, through `onResolve` and
/// `onReply` (`CommentFormat`).
struct CommentCard: View {
    let comment: NoteComment
    /// Writes the reply; true when the note was written, so the field is cleared.
    var onReply: (String) -> Bool
    var onResolve: () -> Void
    var onGoToLine: () -> Void

    @State private var reply = ""

    private var canReply: Bool {
        !reply.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var name: String {
        CommentFormat.displayName(comment.author)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
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
            byline(name: name, time: comment.time)
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
                .padding(.top, 4)
            }
            replyField
            HStack(spacing: 8) {
                Button("Resolve", action: onResolve)
                    .buttonStyle(.bordered)
                    .tint(InkColor.accent)
                    .controlSize(.small)
                Button("Go to line", action: onGoToLine)
                    .buttonStyle(.bordered)
                    .controlSize(.small)
            }
            .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, InkMetrics.cardPaddingH)
        .padding(.vertical, 10)
        .background(InkColor.ground, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
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

    private var replyField: some View {
        HStack(spacing: 8) {
            TextField("Reply", text: $reply)
                .font(InkType.body)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(InkColor.sheet, in: RoundedRectangle(cornerRadius: InkRadius.field, style: .continuous))
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
