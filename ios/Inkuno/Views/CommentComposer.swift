import SwiftUI

/// A comment being written on a block: the block's line (0-based) and its text (quoted as the passage).
struct CommentDraft: Identifiable {
    let id = UUID()
    let line: Int
    let passage: String
}

/// The small composer for a block comment: the passage quoted, what is said, Cancel and Add. Add writes the
/// desktop's comment syntax after the line (`CommentFormat.commented`), through the editor.
struct CommentComposer: View {
    let draft: CommentDraft
    var onAdd: (String) -> Void

    @Environment(\.dismiss) private var dismiss
    @AppStorage(AppearanceKey.authorName) private var authorName = ""
    @State private var said = ""

    /// The comment text with the author's byline, `Name, HH:MM: text`, as the Comments panel reads it
    /// (`CommentFormat.splitByline`). No byline when no name is set, as the desktop writes it then.
    static func withByline(_ said: String, author: String, now: Date = Date(), calendar: Calendar = .current) -> String {
        let name = AuthorName.clean(author)
        guard !name.isEmpty else { return said }
        let parts = calendar.dateComponents([.hour, .minute], from: now)
        let clock = String(format: "%02d:%02d", parts.hour ?? 0, parts.minute ?? 0)
        return "\(name), \(clock): \(said.trimmingCharacters(in: .whitespacesAndNewlines))"
    }
    @FocusState private var focused: Bool

    /// Whether the passage can take a comment: a non-blank line without comment marks.
    private var canComment: Bool {
        CommentFormat.commented(line: draft.passage, said: "x") != nil
    }

    private var canAdd: Bool {
        canComment && !said.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 12) {
                Text(draft.passage.trimmingCharacters(in: .whitespaces))
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.soft)
                    .lineLimit(3)
                    .padding(.leading, 8)
                    .overlay(alignment: .leading) {
                        RoundedRectangle(cornerRadius: 2)
                            .fill(InkColor.commentDot)
                            .frame(width: 3)
                    }
                TextField("Comment", text: $said, axis: .vertical)
                    .lineLimit(3...8)
                    .font(InkType.body)
                    .focused($focused)
                    .padding(12)
                    .background(InkColor.ground, in: RoundedRectangle(cornerRadius: InkRadius.field, style: .continuous))
                if !canComment {
                    Text("This line cannot take a comment.")
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.red)
                }
                Spacer(minLength: 0)
            }
            .padding(InkMetrics.cardMarginH)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(InkColor.sheet)
            .navigationTitle("Comment")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Add") {
                        onAdd(Self.withByline(said, author: authorName))
                        dismiss()
                    }
                    .disabled(!canAdd)
                }
            }
            .onAppear { focused = true }
        }
        .presentationDetents([.medium])
        .presentationDragIndicator(.visible)
    }
}
