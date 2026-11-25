import SwiftUI

/// The blocks the slash sheet inserts. Markdown only: Math and Mermaid are out of v1.
enum SlashItem: CaseIterable, Identifiable {
    case heading, bullet, numbered, task, quote, code, table, divider

    var id: Self { self }

    var title: String {
        switch self {
        case .heading: return "Heading"
        case .bullet: return "Bullet list"
        case .numbered: return "Numbered list"
        case .task: return "Task"
        case .quote: return "Quote"
        case .code: return "Code block"
        case .table: return "Table"
        case .divider: return "Divider"
        }
    }

    var symbol: String {
        switch self {
        case .heading: return "textformat.size"
        case .bullet: return "list.bullet"
        case .numbered: return "list.number"
        case .task: return "checkmark.square"
        case .quote: return "text.quote"
        case .code: return "chevron.left.forwardslash.chevron.right"
        case .table: return "tablecells"
        case .divider: return "minus"
        }
    }

    /// The Markdown inserted at the caret line.
    var markdown: String {
        switch self {
        case .heading: return "## "
        case .bullet: return "- "
        case .numbered: return "1. "
        case .task: return "- [ ] "
        case .quote: return "> "
        case .code: return "```\n\n```"
        case .table: return "| Column | Column |\n| --- | --- |\n|  |  |"
        case .divider: return "---"
        }
    }

    /// The caret position inside `markdown`, in UTF-16 units: inside the code fence, else the end.
    var caretOffset: Int {
        switch self {
        case .code: return 4
        default: return markdown.utf16.count
        }
    }
}

/// Three columns of block kinds, presented as a medium sheet by the note.
struct SlashSheet: View {
    let onPick: (SlashItem) -> Void

    private let columns = Array(repeating: GridItem(.flexible(), spacing: InkMetrics.gridGap), count: 3)

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVGrid(columns: columns, spacing: InkMetrics.gridGap) {
                    ForEach(SlashItem.allCases) { item in
                        Button {
                            onPick(item)
                        } label: {
                            tile(for: item)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(InkMetrics.cardMarginH)
            }
            .background(InkColor.sheet)
            .navigationTitle("Insert block")
            .navigationBarTitleDisplayMode(.inline)
        }
    }

    private func tile(for item: SlashItem) -> some View {
        VStack(spacing: 6) {
            Image(systemName: item.symbol)
                .font(.system(size: 22, weight: .medium))
                .foregroundStyle(InkColor.icon)
            Text(item.title)
                .font(InkType.caption)
                .foregroundStyle(InkColor.ink)
        }
        .frame(maxWidth: .infinity, minHeight: InkMetrics.tileMinHeight)
        .background(InkColor.chip, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous)
                .stroke(InkColor.chipLine, lineWidth: 1)
        )
        .accessibilityLabel(item.title)
    }
}
