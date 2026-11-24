import InkunoCore
import SwiftUI

/// The stack root: Today, New note, Search and Notes, then recall cards from a month ago.
/// No tab bar. The file tree is push navigation.
struct HomeView: View {
    @Environment(VaultStore.self) private var vault
    @Binding var path: [Route]
    @Binding var showPalette: Bool

    @State private var recall: [RecalledNote] = []
    @State private var error: String?

    private let columns = [
        GridItem(.flexible(), spacing: InkMetrics.gridGap),
        GridItem(.flexible(), spacing: InkMetrics.gridGap),
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                writeCard
                tiles
                if !recall.isEmpty {
                    InkSectionTitle("From a month ago")
                        .padding(.horizontal, InkMetrics.cardMarginH + 4)
                        .padding(.top, 8)
                        .padding(.bottom, 8)
                    recallCards
                }
            }
            .padding(.bottom, 24)
        }
        .background(InkColor.ground)
        .navigationTitle("Inkuno")
        .navigationBarTitleDisplayMode(.large)
        .refreshable {
            try? vault.rebuildIndex()
            reloadRecall()
            showPalette = true
        }
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    showPalette = true
                } label: {
                    Image(systemName: "magnifyingglass")
                }
                .accessibilityLabel("Search")
            }
            ToolbarItem(placement: .topBarTrailing) {
                Button {
                    path.append(.settings)
                } label: {
                    Image(systemName: "gearshape")
                }
                .accessibilityLabel("Settings")
            }
        }
        .onAppear(perform: reloadRecall)
        .alert("Inkuno", isPresented: Binding(get: { error != nil }, set: { if !$0 { error = nil } })) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(error ?? "")
        }
    }

    private var writeCard: some View {
        Button {
            openToday()
        } label: {
            HStack {
                Text("Write here…")
                    .font(InkType.rowTitle)
                    .foregroundStyle(InkColor.soft)
                Spacer()
            }
            .padding(.horizontal, InkMetrics.cardPaddingH)
            .padding(.vertical, InkMetrics.cardPaddingV)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(InkColor.sheet, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
        }
        .buttonStyle(.plain)
        .padding(.horizontal, InkMetrics.cardMarginH)
        .padding(.top, 8)
        .padding(.bottom, InkMetrics.cardMarginBottom)
        .accessibilityIdentifier("home.write")
    }

    private var tiles: some View {
        LazyVGrid(columns: columns, spacing: InkMetrics.gridGap) {
            tile("Today", systemImage: "calendar", id: "today") { openToday() }
            tile("New note", systemImage: "square.and.pencil", id: "newNote") { newNote() }
            tile("Search", systemImage: "magnifyingglass", id: "search") { showPalette = true }
            tile("Notes", systemImage: "folder", id: "notes") { openNotes() }
        }
        .padding(.horizontal, InkMetrics.cardMarginH)
        .padding(.bottom, InkMetrics.cardMarginBottom)
    }

    private func tile(_ title: String, systemImage: String, id: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 10) {
                Image(systemName: systemImage)
                    .font(.system(size: 20, weight: .medium))
                    .foregroundStyle(InkColor.accent)
                Spacer(minLength: 0)
                Text(title)
                    .font(InkType.rowTitle)
                    .foregroundStyle(InkColor.ink)
            }
            .padding(InkMetrics.cardPaddingH)
            .frame(maxWidth: .infinity, minHeight: InkMetrics.tileMinHeight, alignment: .leading)
            .background(InkColor.sheet, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("home.tile.\(id)")
    }

    private var recallCards: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: InkMetrics.gridGap) {
                ForEach(recall, id: \.path) { note in
                    Button {
                        if let url = vault.noteURL(path: note.path) { path.append(.note(url)) }
                    } label: {
                        VStack(alignment: .leading, spacing: 6) {
                            Text(note.name)
                                .font(InkType.rowTitle)
                                .foregroundStyle(InkColor.ink)
                                .lineLimit(2)
                            Spacer(minLength: 0)
                            Text(ago(days: note.days))
                                .font(InkType.caption)
                                .foregroundStyle(InkColor.faint)
                        }
                        .padding(InkMetrics.cardPaddingH)
                        .frame(width: InkMetrics.recallWidth, height: 104, alignment: .topLeading)
                        .background(InkColor.sheet, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
                    }
                    .buttonStyle(.plain)
                }
            }
            .padding(.horizontal, InkMetrics.cardMarginH)
        }
    }

    // MARK: Actions

    private func openToday() {
        perform {
            path.append(.note(try vault.openToday()))
        }
    }

    private func newNote() {
        perform {
            guard let root = vault.vaultURL else { throw VaultError.noVault }
            let url = try vault.createNote(at: vault.newNoteURL(in: root))
            path.append(.note(url))
        }
    }

    private func openNotes() {
        guard let root = vault.vaultURL else { return }
        path.append(.tree(root))
    }

    private func reloadRecall() {
        recall = (try? vault.recalledNotes()) ?? []
    }

    private func perform(_ work: () throws -> Void) {
        do {
            try work()
        } catch {
            self.error = error.localizedDescription
        }
    }
}
