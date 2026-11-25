import SwiftUI
import UIKit

/// One folder of the vault, pushed from Home or from another folder: folders, then notes,
/// and at the root the Scratch section (new scratch note, then the scratch notes, newest first).
/// An empty folder shows "This folder is empty."
/// Long-press a row for its context menu: Rename, Move, Duplicate (notes), Copy link (notes),
/// Move to Trash. Renaming refuses a taken name, so nothing is overwritten.
struct TreeView: View {
    @Environment(VaultStore.self) private var vault
    let folder: URL
    @Binding var path: [Route]

    @State private var entries: [VaultEntry] = []
    @State private var childCounts: [URL: Int] = [:]
    @State private var error: String?
    @State private var renaming: VaultEntry?
    @State private var renameText = ""
    @State private var moving: VaultEntry?
    @State private var notice: String?
    @State private var scratches: [ScratchItem] = []

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
                        .contextMenu { rowMenu(for: entry) }
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
                        .contextMenu { rowMenu(for: entry) }
                    }
                } header: {
                    InkSectionTitle("Notes")
                }
            }
            if isRoot {
                Section {
                    Button {
                        newScratch()
                    } label: {
                        Label("New scratch", systemImage: "pencil.and.scribble")
                            .font(InkType.rowTitle)
                            .foregroundStyle(InkColor.ink)
                    }
                    ForEach(scratches) { scratch in
                        NavigationLink(value: Route.note(scratch.url)) {
                            scratchRow(scratch)
                        }
                        .contextMenu {
                            Button(role: .destructive) {
                                trashScratch(scratch.url)
                            } label: {
                                Label("Move to Trash", systemImage: "trash")
                            }
                        }
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
        .alert("Rename", isPresented: Binding(get: { renaming != nil }, set: { if !$0 { renaming = nil } })) {
            TextField("Name", text: $renameText)
            Button("Cancel", role: .cancel) {}
            Button("Rename") { rename() }
        } message: {
            Text("A name cannot contain \"/\" or \":\". Taken names are refused.")
        }
        .sheet(item: $moving) { entry in
            MoveSheet(entry: entry) { destination in
                move(entry, to: destination)
            }
        }
        .overlay(alignment: .bottom) {
            if let notice {
                Text(notice)
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.toastInk)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 8)
                    .background(InkColor.toast, in: Capsule())
                    .padding(.bottom, 16)
            }
        }
    }

    // MARK: Context menu

    @ViewBuilder
    private func rowMenu(for entry: VaultEntry) -> some View {
        Button {
            renaming = entry
            renameText = entry.title
        } label: {
            Label("Rename", systemImage: "pencil")
        }
        Button {
            moving = entry
        } label: {
            Label("Move…", systemImage: "folder")
        }
        if entry.isNote {
            Button {
                duplicate(entry)
            } label: {
                Label("Duplicate", systemImage: "plus.square.on.square")
            }
            Button {
                copyLink(entry)
            } label: {
                Label("Copy link", systemImage: "link")
            }
        }
        Divider()
        Button(role: .destructive) {
            trash(entry)
        } label: {
            Label("Move to Trash", systemImage: "trash")
        }
    }

    // MARK: Actions

    private func rename() {
        guard let entry = renaming else { return }
        renaming = nil
        do {
            let renamed = try vault.renameItem(at: entry.url, to: renameText)
            if entry.isNote, path.last == .note(entry.url) {
                path[path.count - 1] = .note(renamed)
            }
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func move(_ entry: VaultEntry, to destination: URL) {
        moving = nil
        do {
            try vault.moveItem(at: entry.url, toFolder: destination)
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func duplicate(_ entry: VaultEntry) {
        do {
            try vault.duplicateItem(at: entry.url)
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func copyLink(_ entry: VaultEntry) {
        if let link = vault.copyLink(for: entry.url) {
            UIPasteboard.general.string = link
            flash("Copied \(link)")
        }
    }

    private func trash(_ entry: VaultEntry) {
        do {
            try vault.trashItem(at: entry.url)
            if path.last == .note(entry.url), path.count > 1 {
                _ = path.popLast()
            }
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func flash(_ message: String) {
        notice = message
        Task { @MainActor in
            try? await Task.sleep(for: .seconds(2))
            if notice == message { notice = nil }
        }
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
            scratches = isRoot ? scratchItems() : []
            error = nil
        } catch {
            entries = []
            childCounts = [:]
            self.error = error.localizedDescription
        }
    }

    private func scratchItems() -> [ScratchItem] {
        ScratchStore.list().map { url in
            ScratchItem(
                url: url,
                title: ScratchStore.title(of: url),
                modified: try? url.resourceValues(forKeys: [.contentModificationDateKey]).contentModificationDate
            )
        }
    }

    private func newScratch() {
        do {
            path.append(.note(try ScratchStore.create()))
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func trashScratch(_ url: URL) {
        do {
            try ScratchStore.trash(url)
            reload()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func scratchRow(_ scratch: ScratchItem) -> some View {
        HStack(spacing: 12) {
            Image(systemName: "pencil.and.scribble")
                .frame(width: InkMetrics.markerIconSize)
                .foregroundStyle(InkColor.icon)
            Text(scratch.title)
                .font(InkType.rowTitle)
                .foregroundStyle(InkColor.ink)
                .lineLimit(1)
            Spacer(minLength: 8)
            Text(InkFormat.age(scratch.modified))
                .font(InkType.caption)
                .foregroundStyle(InkColor.faint)
        }
        .frame(minHeight: InkMetrics.rowMinHeight)
    }
}

/// A scratch note in the Scratch section: its file, its title (first line) and its last change.
private struct ScratchItem: Identifiable {
    let url: URL
    let title: String
    let modified: Date?

    var id: URL { url }
}

/// The Move destination picker: every folder of the vault.
private struct MoveSheet: View {
    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss
    let entry: VaultEntry
    var onPick: (URL) -> Void

    @State private var folders: [URL] = []

    var body: some View {
        NavigationStack {
            List {
                Section {
                    ForEach(folders, id: \.self) { folder in
                        Button {
                            onPick(folder)
                            dismiss()
                        } label: {
                            HStack(spacing: 12) {
                                Image(systemName: "folder")
                                    .frame(width: InkMetrics.markerIconSize)
                                    .foregroundStyle(InkColor.icon)
                                Text(displayName(folder))
                                    .font(InkType.rowTitle)
                                    .foregroundStyle(InkColor.ink)
                                    .lineLimit(1)
                                Spacer(minLength: 8)
                            }
                            .frame(minHeight: InkMetrics.rowMinHeight)
                        }
                        .disabled(folder == entry.url.deletingLastPathComponent())
                    }
                } header: {
                    InkSectionTitle("Move “\(entry.title)” to")
                }
            }
            .listStyle(.insetGrouped)
            .scrollContentBackground(.hidden)
            .background(InkColor.ground)
            .navigationTitle("Move")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
            }
            .task {
                folders = (try? vault.allFolders()) ?? []
            }
        }
        .presentationDetents([.medium])
    }

    private func displayName(_ folder: URL) -> String {
        folder == vault.vaultURL ? "Notes" : folder.lastPathComponent
    }
}
