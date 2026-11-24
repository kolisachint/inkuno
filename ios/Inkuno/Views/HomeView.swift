import InkunoCore
import SwiftUI

/// The stack root: Today, New note, Search and Notes, then recall cards from a month ago.
/// No tab bar. The file tree is push navigation.
struct HomeView: View {
    @Environment(VaultStore.self) private var vault
    @Binding var path: [Route]
    @Binding var showPalette: Bool

    @State private var recall: [RecalledNote] = []
    @State private var todaySubtitle = Self.notWrittenYet
    @State private var noteCount = 0
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
                // The pull hint stays even when there is nothing to reread (prototype :644).
                Text("Pull down on any screen to search and run commands")
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.soft)
                    .multilineTextAlignment(.center)
                    .frame(maxWidth: .infinity)
                    .padding(.horizontal, InkMetrics.cardMarginH + 8)
                    .padding(.top, recall.isEmpty ? 8 : 12)
                    .padding(.bottom, 8)
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

    /// The Today tile's subtitle: the daily note's title, or "Not written yet" (prototype :636).
    static func todaySubtitle(todayTitle: String?) -> String {
        todayTitle ?? notWrittenYet
    }

    private static let notWrittenYet = "Not written yet"

    /// The Notes tile's subtitle: "N notes" (prototype :639).
    static func notesSubtitle(count: Int) -> String {
        "\(count) notes"
    }

    private var writeCard: some View {
        Button {
            openToday()
        } label: {
            HStack(spacing: 10) {
                Image(systemName: "square.and.pencil")
                    .font(.system(size: 20))
                    .foregroundStyle(InkColor.soft)
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
    }

    private var tiles: some View {
        LazyVGrid(columns: columns, spacing: InkMetrics.gridGap) {
            tile("Today", subtitle: todaySubtitle, systemImage: "calendar") { openToday() }
            tile("New note", subtitle: "In Notes", systemImage: "square.and.pencil") { newNote() }
            tile("Search", subtitle: "Or pull down", systemImage: "magnifyingglass") { showPalette = true }
            tile("Notes", subtitle: Self.notesSubtitle(count: noteCount), systemImage: "folder") { openNotes() }
        }
        .padding(.horizontal, InkMetrics.cardMarginH)
        .padding(.bottom, InkMetrics.cardMarginBottom)
    }

    private func tile(_ title: String, subtitle: String, systemImage: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            VStack(alignment: .leading, spacing: 4) {
                Image(systemName: systemImage)
                    .font(.system(size: 20, weight: .medium))
                    .foregroundStyle(InkColor.accent)
                Spacer(minLength: 0)
                Text(title)
                    .font(InkType.rowTitle)
                    .fontWeight(.semibold)
                    .foregroundStyle(InkColor.ink)
                Text(subtitle)
                    .font(InkType.caption)
                    .foregroundStyle(InkColor.soft)
                    .lineLimit(1)
            }
            .padding(InkMetrics.cardPaddingH)
            // Spec: tile min height 112pt, radius 16 (radius is `InkRadius.card`).
            .frame(maxWidth: .infinity, minHeight: 112, alignment: .leading)
            .background(InkColor.sheet, in: RoundedRectangle(cornerRadius: InkRadius.card, style: .continuous))
        }
        .buttonStyle(.plain)
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

    /// The daily note's title when it is on disk already; nil when it is not written yet.
    private func todayTitle() -> String? {
        guard let root = vault.vaultURL else { return nil }
        let url = root.appendingPathComponent(VaultStore.dailyNoteFileName(for: .now))
        guard FileManager.default.fileExists(atPath: url.path) else { return nil }
        return url.deletingPathExtension().lastPathComponent
    }

    private func reloadRecall() {
        recall = (try? vault.recalledNotes()) ?? []
        noteCount = (try? vault.allNotes().count) ?? 0
        todaySubtitle = Self.todaySubtitle(todayTitle: todayTitle())
    }

    private func perform(_ work: () throws -> Void) {
        do {
            try work()
        } catch {
            self.error = error.localizedDescription
        }
    }
}
