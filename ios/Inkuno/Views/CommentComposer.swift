import SwiftUI

/// A comment being written on a block: the block's line (0-based), its text (quoted as the passage) and
/// whether it is a table line (its `|` is escaped).
struct CommentDraft: Identifiable {
    let id = UUID()
    let line: Int
    let passage: String
    let inTable: Bool
}

/// The small composer for a block comment: the passage quoted, what is said, Cancel and Add. Add writes the
/// desktop's comment syntax after the line (`CommentFormat.commented`), through the editor.
struct CommentComposer: View {
    let draft: CommentDraft
    var onAdd: (String) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var said = ""
    @FocusState private var focused: Bool

    /// Whether the passage can take a comment: a non-blank line without comment marks.
    private var canComment: Bool {
        CommentFormat.commented(line: draft.passage, said: "x", inTable: draft.inTable) != nil
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
                        onAdd(said)
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
