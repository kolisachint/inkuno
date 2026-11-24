import InkunoCore
import SwiftUI

/// The bottom pill while a note has pending changes: "N changes · Review".
struct PendingPill: View {
    let noteURL: URL
    var onReview: () -> Void

    @Environment(VaultStore.self) private var vault
    @State private var count = 0

    var body: some View {
        Group {
            if count > 0 {
                Button(action: onReview) {
                    Text("\(count) change\(count == 1 ? "" : "s") · Review")
                        .font(.system(size: 14, weight: .semibold))
                        .foregroundStyle(InkColor.toastInk)
                        .padding(.horizontal, 16)
                        .padding(.vertical, 10)
                        .background(InkColor.toast, in: Capsule())
                        .shadow(radius: 8)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("\(count) pending changes. Review.")
            }
        }
        .task(id: noteURL) { reload() }
        .onReceive(NotificationCenter.default.publisher(for: .inkunoNoteChanged)) { note in
            guard (note.object as? URL) == noteURL else { return }
            reload()
        }
    }

    private func reload() {
        count = vault.pendingCount(for: noteURL)
    }
}

/// The pending-changes review sheet: Reject all / Accept all and one card per change (old struck,
/// new). Swipe leading accepts, trailing rejects. Functional: accept writes `new` over `old` in
/// the note file, reject leaves the file; both update the sidecar list.
struct PendingSheet: View {
    let noteURL: URL

    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss

    @State private var entries: [PendingEntry] = []
    @State private var error: String?

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                HStack {
                    Text("\(entries.count) pending change\(entries.count == 1 ? "" : "s")")
                        .font(.system(size: 17, weight: .semibold))
                        .foregroundStyle(InkColor.ink)
                    Spacer(minLength: 8)
                }
                .padding(.horizontal, InkMetrics.cardMarginH)
                .padding(.top, 12)
                .padding(.bottom, 12)

                HStack(spacing: 8) {
                    Button("Reject all") { resolveAll(accept: false) }
                        .frame(maxWidth: .infinity, minHeight: 36)
                        .background(InkColor.segment, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                    Button("Accept all") { resolveAll(accept: true) }
                        .frame(maxWidth: .infinity, minHeight: 36)
                        .foregroundStyle(.white)
                        .background(InkColor.accent, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                }
                .font(.system(size: 15))
                .buttonStyle(.plain)
                .padding(.horizontal, InkMetrics.cardMarginH)
                .padding(.bottom, 12)

                ScrollView {
                    LazyVStack(spacing: 10) {
                        ForEach(entries, id: \.id) { entry in
                            PendingCard(entry: entry,
                                        onAccept: { resolve(entry, accept: true) },
                                        onReject: { resolve(entry, accept: false) })
                        }
                    }
                    .padding(.horizontal, InkMetrics.cardMarginH)
                    if entries.isEmpty {
                        Text("No pending changes.")
                            .font(InkType.body)
                            .foregroundStyle(InkColor.faint)
                            .padding(.top, 24)
                    }
                    Text("Swipe a change right to accept, left to reject. The note file changes only when you accept.")
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.soft)
                        .padding(.horizontal, InkMetrics.cardMarginH + 4)
                        .padding(.vertical, 12)
                }
            }
            .background(InkColor.sheet)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .alert("Inkuno", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
                Button("OK", role: .cancel) {}
            } message: {
                Text(error ?? "")
            }
            .task { reload() }
        }
        .presentationDetents([.medium, .large])
    }

    private func reload() {
        entries = vault.pendingEntries(for: noteURL)
        if entries.isEmpty {
            // All settled elsewhere: follow the prototype and close the sheet.
            dismiss()
        }
    }

    private func resolve(_ entry: PendingEntry, accept: Bool) {
        do {
            if accept {
                guard try vault.acceptPendingEntry(id: entry.id, for: noteURL) else {
                    error = "That change is gone or changed: nothing was \(accept ? "accepted" : "rejected")."
                    return
                }
            } else {
                try vault.rejectPendingEntry(id: entry.id, for: noteURL)
            }
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: noteURL)
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func resolveAll(accept: Bool) {
        do {
            try vault.resolveAllPending(for: noteURL, accept: accept)
            NotificationCenter.default.post(name: .inkunoNoteChanged, object: noteURL)
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }
}

private struct PendingCard: View {
    let entry: PendingEntry
    var onAccept: () -> Void
    var onReject: () -> Void

    var body: some View {
        ZStack {
            // Swipe affordances behind the card (green leading, red trailing).
            HStack {
                Label("Accept", systemImage: "checkmark")
                    .foregroundStyle(InkColor.addInk)
                Spacer(minLength: 0)
                Label("Reject", systemImage: "xmark")
                    .foregroundStyle(InkColor.deleteInk)
            }
            .font(.system(size: 15, weight: .semibold))
            .padding(.horizontal, 16)
            .background {
                HStack(spacing: 0) {
                    InkColor.addWash
                    InkColor.deleteWash
                }
                .clipShape(RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
            }

            VStack(alignment: .leading, spacing: 2) {
                if !entry.old.isEmpty {
                    Text(entry.old)
                        .font(InkType.mono)
                        .foregroundStyle(InkColor.deleteInk)
                        .strikethrough()
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                if !entry.new.isEmpty {
                    Text(entry.new)
                        .font(InkType.mono)
                        .foregroundStyle(InkColor.addInk)
                        .frame(maxWidth: .infinity, alignment: .leading)
                }
                if !entry.author.isEmpty {
                    Text(entry.author)
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.faint)
                        .padding(.top, 2)
                }
                HStack(spacing: 8) {
                    Button("Reject", action: onReject)
                        .frame(minHeight: 34)
                        .padding(.horizontal, 12)
                        .background(InkColor.segment, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                    Button("Accept", action: onAccept)
                        .frame(minHeight: 34)
                        .padding(.horizontal, 12)
                        .background(InkColor.segment, in: RoundedRectangle(cornerRadius: 9, style: .continuous))
                    Spacer(minLength: 0)
                }
                .font(.system(size: 14))
                .buttonStyle(.plain)
                .padding(.top, 8)
            }
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(InkColor.sheet, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous)
                    .stroke(InkColor.line, lineWidth: 1)
            }
            .offset(x: dragOffset)
            .gesture(swipeGesture)
        }
    }

    @State private var dragOffset: CGFloat = 0

    private var swipeGesture: some Gesture {
        DragGesture(minimumDistance: 20, coordinateSpace: .local)
            .onChanged { value in
                guard abs(value.translation.height) < 30 else { return }
                dragOffset = value.translation.width
            }
            .onEnded { value in
                let dx = value.translation.width
                withAnimation(.easeOut(duration: 0.18)) { dragOffset = 0 }
                if dx > 90 {
                    onAccept()
                } else if dx < -90 {
                    onReject()
                }
            }
    }
}

extension Notification.Name {
    /// Posted with the note URL as object after pending changes change on disk.
    static let inkunoNoteChanged = Notification.Name("inkunoNoteChanged")
}
