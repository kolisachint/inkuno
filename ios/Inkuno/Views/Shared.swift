import SwiftUI

/// The square app mark shown on onboarding (68pt).
struct InkMark: View {
    var body: some View {
        RoundedRectangle(cornerRadius: 16, style: .continuous)
            .fill(InkColor.accent)
            .frame(width: 68, height: 68)
            .overlay {
                Image(systemName: "pencil.line")
                    .font(.system(size: 30, weight: .semibold))
                    .foregroundStyle(.white)
            }
            .accessibilityHidden(true)
    }
}

/// Section header in the prototype's style: 13pt, uppercase, tracked.
struct InkSectionTitle: View {
    let text: String

    init(_ text: String) {
        self.text = text
    }

    var body: some View {
        Text(text.uppercased())
            .font(InkType.sectionHeader)
            .tracking(InkType.sectionTracking)
            .foregroundStyle(InkColor.soft)
    }
}

enum InkFormat {
    private static let relative: RelativeDateTimeFormatter = {
        let formatter = RelativeDateTimeFormatter()
        formatter.unitsStyle = .full
        return formatter
    }()

    /// "3 days ago", "1 month ago". Empty when the date is unknown.
    static func age(_ date: Date?, now: Date = .now) -> String {
        guard let date else { return "" }
        return relative.localizedString(for: date, relativeTo: now)
    }
}

/// A note row: icon, title, age. Used by the tree and the palette.
struct NoteRow: View {
    let entry: VaultEntry
    /// A matching content line, shown under the title in palette Content results.
    var snippet: String? = nil

    var body: some View {
        HStack(spacing: 12) {
            Image(systemName: entry.kind == .cloudPlaceholder ? "icloud.and.arrow.down" : "doc.text")
                .frame(width: InkMetrics.markerIconSize)
                .foregroundStyle(InkColor.icon)
            VStack(alignment: .leading, spacing: 2) {
                Text(entry.title)
                    .font(InkType.rowTitle)
                    .foregroundStyle(InkColor.ink)
                    .lineLimit(1)
                if let snippet {
                    Text(snippet)
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.soft)
                        .lineLimit(1)
                } else if entry.kind == .cloudPlaceholder {
                    Text("In iCloud")
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.faint)
                }
            }
            Spacer(minLength: 8)
            Text(InkFormat.age(entry.modified))
                .font(InkType.caption)
                .foregroundStyle(InkColor.faint)
        }
        .frame(minHeight: InkMetrics.rowMinHeight)
    }
}
