import SwiftUI

/// One folder of the vault, pushed from Home or from another folder: folders, then notes,
/// and at the root a Scratch row. An empty folder shows "This folder is empty."
struct TreeView: View {
    @Environment(VaultStore.self) private var vault
    let folder: URL
    @Binding var path: [Route]

    @State private var entries: [VaultEntry] = []
    @State private var childCounts: [URL: Int] = [:]
    @State private var error: String?

    private var isRoot: Bool {
        folder == vault.vaultURL
    }

    private var folders: [VaultEntry] {
        entries.filter { $0.kind == .folder }
    }

    private var notes: [VaultEntry] {
        entries.filter { $0.isNote }
    }

    var body: some View {
        List {
            if !folders.isEmpty {
                Section {
                    ForEach(folders) { entry in
                        NavigationLink(value: Route.tree(entry.url)) {
                            folderRow(entry)
                        }
                    }
                } header: {
                    InkSectionTitle("Folders")
                }
            }
            if !notes.isEmpty {
                Section {
                    ForEach(notes) { entry in
                        NavigationLink(value: Route.note(entry.url)) {
                            NoteRow(entry: entry)
                        }
                    }
                } header: {
                    InkSectionTitle("Notes")
                }
            }
            if isRoot {
                Section {
                    Button {
                        openScratch()
                    } label: {
                        Label("Scratch", systemImage: "pencil.and.scribble")
                            .font(InkType.rowTitle)
                            .foregroundStyle(InkColor.ink)
                    }
                } header: {
                    InkSectionTitle("Scratch")
                }
            }
            if entries.isEmpty && error == nil {
                Section {
                    Text("This folder is empty.")
                        .font(InkType.body)
                        .foregroundStyle(InkColor.soft)
                        .frame(maxWidth: .infinity)
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
        .navigationTitle(isRoot ? "Notes" : folder.lastPathComponent)
        .navigationBarTitleDisplayMode(isRoot ? .large : .inline)
        .task(id: folder) { reload() }
        .refreshable { reload() }
    }

    private func folderRow(_ entry: VaultEntry) -> some View {
        HStack(spacing: 12) {
            Image(systemName: "folder")
                .frame(width: InkMetrics.markerIconSize)
                .foregroundStyle(InkColor.icon)
            Text(entry.title)
                .font(InkType.rowTitle)
                .foregroundStyle(InkColor.ink)
            Spacer(minLength: 8)
            Text("\(childCounts[entry.url] ?? 0)")
                .font(InkType.caption)
                .foregroundStyle(InkColor.faint)
        }
        .frame(minHeight: InkMetrics.rowMinHeight)
    }

    /// Reads the folder and the visible-entry count of each subfolder once, not on every render.
    private func reload() {
        do {
            let listed = try vault.entries(in: folder)
            childCounts = Dictionary(uniqueKeysWithValues: try listed.filter { $0.kind == .folder }.map { entry in
                (entry.url, try vault.entries(in: entry.url).count)
            })
            entries = listed
            error = nil
        } catch {
            entries = []
            childCounts = [:]
            self.error = error.localizedDescription
        }
    }

    private func openScratch() {
        do {
            path.append(.note(try vault.openScratch()))
        } catch {
            self.error = error.localizedDescription
        }
    }
}
