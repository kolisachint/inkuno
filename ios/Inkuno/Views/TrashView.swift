import SwiftUI

/// The vault's Trash: trashed notes with Restore buttons. Restoring brings a note back to the folder it
/// came from, numbered when taken (`platform/vault.rs` `restore`), then shows a "Restored" notice.
struct TrashView: View {
    /// The notice the stack shows over its screens (see `RootView`).
    @Binding var notice: InkNotice?
    var onOpen: (URL) -> Void = { _ in }

    @Environment(VaultStore.self) private var vault
    @State private var trashed: [VaultEntry] = []
    @State private var error: String?

    var body: some View {
        List {
            if trashed.isEmpty, error == nil {
                Section {
                    Text("Trash is empty.")
                        .font(InkType.body)
                        .foregroundStyle(InkColor.faint)
                        .frame(maxWidth: .infinity)
                }
            } else {
                Section {
                    ForEach(trashed) { entry in
                        HStack(spacing: 12) {
                            Image(systemName: "doc.text")
                                .frame(width: InkMetrics.markerIconSize)
                                .foregroundStyle(InkColor.icon)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(entry.title)
                                    .font(InkType.rowTitle)
                                    .foregroundStyle(InkColor.ink)
                                    .lineLimit(1)
                                Text(vault.trashOrigin(of: entry.url))
                                    .font(InkType.caption)
                                    .foregroundStyle(InkColor.faint)
                            }
                            Spacer(minLength: 8)
                            Button("Restore") { restore(entry) }
                                .font(.system(size: 15, weight: .semibold))
                                .foregroundStyle(.white)
                                .padding(.horizontal, 14)
                                .frame(minHeight: 36)
                                .background(InkColor.accent, in: RoundedRectangle(cornerRadius: InkRadius.ctaSmall, style: .continuous))
                                .buttonStyle(.plain)
                        }
                        .frame(minHeight: InkMetrics.rowMinHeight)
                        .swipeActions(edge: .trailing, allowsFullSwipe: true) {
                            Button("Restore") { restore(entry) }
                                .tint(InkColor.accent)
                        }
                    }
                } footer: {
                    Text("Restore brings a note back to the folder it came from.")
                }
            }
            if let error {
                Section {
                    Text(error)
                        .font(InkType.caption)
                        .foregroundStyle(InkColor.red)
                }
            }
        }
        .listStyle(.insetGrouped)
        .scrollContentBackground(.hidden)
        .background(InkColor.ground)
        .navigationTitle("Trash")
        .navigationBarTitleDisplayMode(.large)
        .task { reload() }
        .refreshable { reload() }
    }

    private func reload() {
        do {
            trashed = try vault.trashedNotes()
            error = nil
        } catch {
            trashed = []
            self.error = error.localizedDescription
        }
    }

    private func restore(_ entry: VaultEntry) {
        do {
            let restored = try vault.restoreFromTrash(at: entry.url)
            reload()
            notice = InkNotice("Restored")
            onOpen(restored)
        } catch {
            self.error = error.localizedDescription
        }
    }
}
