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

/// A notice shown over the bottom of the screen: a message and, optionally, one action button (Undo).
/// `inkNotice(_:)` shows it for its duration, then clears it.
struct InkNotice: Identifiable {
    /// Tells this notice from the others: a new id restarts the toast and its timer.
    let id = UUID()
    /// The text the toast shows.
    let message: String
    /// How long the notice stays on screen, in seconds.
    let duration: Double
    /// The action button's title, when there is an action.
    let actionTitle: String?
    /// What the action button does.
    let action: (() -> Void)?

    /// A notice with no action, shown for `duration` seconds.
    init(_ message: String, duration: Double = 2) {
        self.message = message
        self.duration = duration
        self.actionTitle = nil
        self.action = nil
    }

    /// A notice with one action button, such as Undo.
    init(_ message: String, duration: Double, actionTitle: String, action: @escaping () -> Void) {
        self.message = message
        self.duration = duration
        self.actionTitle = actionTitle
        self.action = action
    }
}

extension View {
    /// Shows `notice` over the bottom of the view until its duration is up, then clears it.
    func inkNotice(_ notice: Binding<InkNotice?>) -> some View {
        modifier(InkNoticeModifier(notice: notice))
    }
}

/// The toast behind `inkNotice(_:)`: a capsule above the bottom bar, faded in and out.
private struct InkNoticeModifier: ViewModifier {
    /// The notice on screen, or nil. The toast clears it when its duration ends or its action runs.
    @Binding var notice: InkNotice?
    /// The system Reduce Motion setting, which the toast's animation follows (`InkMotion.resolved`).
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    func body(content: Content) -> some View {
        content
            .overlay(alignment: .bottom) {
                if let notice {
                    HStack(spacing: 10) {
                        Text(notice.message)
                        if let title = notice.actionTitle, let action = notice.action {
                            Button(title) {
                                action()
                                self.notice = nil
                            }
                            .fontWeight(.semibold)
                            .frame(minHeight: 36)
                        }
                    }
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.toastInk)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
                    .background(InkColor.toast, in: Capsule())
                    .padding(.bottom, 96)
                    .transition(.opacity)
                    .id(notice.id)
                }
            }
            .animation(InkMotion.resolved(InkMotion.toast, reduceMotion: reduceMotion), value: notice?.id)
            .task(id: notice?.id) {
                guard let current = notice else { return }
                try? await Task.sleep(for: .seconds(current.duration))
                guard !Task.isCancelled else { return }
                notice = nil
            }
    }
}
