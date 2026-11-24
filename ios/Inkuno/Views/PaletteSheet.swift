import SwiftUI

/// Note search and `>` commands, as a full sheet. An empty query shows the six most recent notes
/// (by mtime). A query matches names with fuzzy ranking (`rank.rs` `fuzzy`), then bodies; ">"
/// switches to commands. `currentNote` (with `onPanelTab`) enables the This-note commands when a
/// note is open; without it those commands are hidden.
struct PaletteSheet: View {
    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss

    /// Called with the route to push; the presenter dismisses the sheet.
    let onOpen: (Route) -> Void
    var currentNote: URL? = nil
    var onPanelTab: ((PanelTab) -> Void)? = nil

    @State private var query = ""
    @State private var notes: [VaultEntry] = []
    @State private var bodies: [URL: String] = [:]
    @State private var error: String?

    private var isCommandMode: Bool {
        query.hasPrefix(">")
    }

    /// The query without the ">" prefix, trimmed.
    private var needle: String {
        (isCommandMode ? String(query.dropFirst()) : query).trimmingCharacters(in: .whitespaces)
    }

    private var recent: [VaultEntry] {
        notes.sorted { ($0.modified ?? .distantPast) > ($1.modified ?? .distantPast) }
            .prefix(6)
            .map { $0 }
    }

    private struct NameHit {
        var entry: VaultEntry
        var score: Int
    }

    private var nameHits: [VaultEntry] {
        notes.compactMap { entry in
            PaletteRank.fuzzy(query: needle, name: entry.title).map { NameHit(entry: entry, score: $0) }
        }
        .sorted { $0.score > $1.score }
        .map(\.entry)
    }

    private var contentHits: [VaultEntry] {
        let folded = PaletteRank.fold(needle)
        let named = Set(nameHits.map(\.id))
        return notes.filter { entry in
            !named.contains(entry.id)
                && bodies[entry.url].map { PaletteRank.fold($0).contains(folded) } == true
        }
        .sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }

    private var showCreateRow: Bool {
        !needle.isEmpty && !notes.contains { $0.title.lowercased() == needle.lowercased() }
    }

    private var commands: [PaletteCommand] {
        PaletteCommand.all(for: currentNote, hasPanelHost: onPanelTab != nil)
            .filter { needle.isEmpty || $0.title.localizedStandardContains(needle) }
    }

    var body: some View {
        NavigationStack {
            List {
                if isCommandMode {
                    commandSection
                } else if needle.isEmpty {
                    Section {
                        ForEach(recent) { entry in
                            noteButton(entry)
                        }
                    } header: {
                        InkSectionTitle("Recent")
                    }
                } else if needle.hasPrefix("@") {
                    Section {
                        Button("Today's note") { openToday() }
                        ForEach(contentHits) { entry in
                            noteButton(entry, snippet: needle)
                        }
                    } header: {
                        InkSectionTitle("Names")
                    } footer: {
                        if nameHits.isEmpty, contentHits.isEmpty {
                            Text("No matches")
                        }
                    }
                } else {
                    if !nameHits.isEmpty || showCreateRow {
                        Section {
                            ForEach(nameHits) { entry in
                                noteButton(entry)
                            }
                            if showCreateRow {
                                Button("Create “\(needle)”") { create() }
                            }
                        } header: {
                            InkSectionTitle("Names")
                        } footer: {
                            if nameHits.isEmpty, contentHits.isEmpty {
                                Text("No matches")
                            }
                        }
                    }
                    if !contentHits.isEmpty {
                        Section {
                            ForEach(contentHits) { entry in
                                noteButton(entry, snippet: needle)
                            }
                        } header: {
                            InkSectionTitle("Content")
                        }
                    }
                    if nameHits.isEmpty, contentHits.isEmpty, !showCreateRow {
                        Section {
                            Text("No matches")
                                .font(InkType.body)
                                .foregroundStyle(InkColor.faint)
                        }
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
            .searchable(text: $query, placement: .navigationBarDrawer(displayMode: .always), prompt: "Find a note, or > for commands")
            .navigationTitle("Search")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
            .task { reload() }
        }
    }

    // MARK: Commands

    @ViewBuilder
    private var commandSection: some View {
        let groups = Dictionary(grouping: commands, by: \.group)
        ForEach(PaletteCommand.Group.allCases) { group in
            if let rows = groups[group], !rows.isEmpty {
                Section {
                    ForEach(rows) { command in
                        Button(command.title) { run(command) }
                    }
                } header: {
                    InkSectionTitle(group.title)
                }
            }
        }
        if commands.isEmpty {
            Section {
                Text("No matches")
                    .font(InkType.body)
                    .foregroundStyle(InkColor.faint)
            }
        }
    }

    private func noteButton(_ entry: VaultEntry, snippet: String? = nil) -> some View {
        Button {
            onOpen(.note(entry.url))
        } label: {
            if let snippet, !snippet.isEmpty, snippet.first != "@",
               let body = bodies[entry.url] {
                NoteRow(entry: entry, snippet: PaletteSheet.snippet(in: body, query: snippet))
            } else {
                NoteRow(entry: entry)
            }
        }
    }

    /// The matching line with the word marked, trimmed to one line (`rank.rs` `snippet`).
    static func snippet(in body: String, query: String) -> String? {
        let folded = PaletteRank.fold(body)
        let needle = PaletteRank.fold(query)
        guard let range = folded.range(of: needle) else { return nil }
        let distance = folded.distance(from: folded.startIndex, to: range.lowerBound)
        let index = body.index(body.startIndex, offsetBy: distance, limitedBy: body.endIndex) ?? body.startIndex
        let lineStart = body[..<index].lastIndex(of: "\n").map { body.index(after: $0) } ?? body.startIndex
        let lineEnd = body[index...].firstIndex(of: "\n") ?? body.endIndex
        var line = String(body[lineStart..<lineEnd]).trimmingCharacters(in: .whitespaces)
        if line.count > 72 {
            line = "…" + line.prefix(70) + "…"
        }
        return line.isEmpty ? nil : line
    }

    // MARK: Data

    private func reload() {
        do {
            notes = try vault.allNotes()
            bodies = Dictionary(uniqueKeysWithValues: notes.map { ($0.url, (try? vault.readNote(at: $0.url)) ?? "") })
            error = nil
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func run(_ command: PaletteCommand) {
        do {
            switch command.action {
            case .today:
                onOpen(.note(try vault.openToday()))
            case .newNote:
                guard let root = vault.vaultURL else { throw VaultError.noVault }
                onOpen(.note(try vault.createNote(at: vault.newNoteURL(in: root))))
            case .settings:
                onOpen(.settings)
            case .trash:
                onOpen(.trash)
            case .saveVersion:
                guard let note = currentNote else { throw VaultError.noVault }
                if try vault.saveVersion(of: note) == nil {
                    error = "Already saved: nothing changed."
                }
            case .showPanel(let tab):
                onPanelTab?(tab)
                dismiss()
            }
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func openToday() {
        do {
            onOpen(.note(try vault.openToday()))
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func create() {
        do {
            guard let root = vault.vaultURL else { throw VaultError.noVault }
            // A name cannot contain "/" or ":" on the Files side.
            let name = String(needle.filter { $0 != "/" && $0 != ":" })
            let url = try vault.createNote(at: root.appendingPathComponent(name).appendingPathExtension("md"))
            onOpen(.note(url))
        } catch {
            self.error = error.localizedDescription
        }
    }
}

/// One `>` command of the palette.
struct PaletteCommand: Identifiable {
    enum Group: String, CaseIterable, Identifiable {
        case notes, thisNote, window

        var id: String { rawValue }

        var title: String {
            switch self {
            case .notes: return "Notes"
            case .thisNote: return "This note"
            case .window: return "Window"
            }
        }
    }

    enum Action {
        case today, newNote, settings, trash
        case saveVersion
        case showPanel(PanelTab)
    }

    var id: String { title }
    var group: Group
    var title: String
    var action: Action

    /// The full command list. The This-note commands (Save version, Outline/Comments/Versions/
    /// Backlinks) appear only when a note is open; the panel ones also need a host for the sheet.
    static func all(for currentNote: URL?, hasPanelHost: Bool) -> [PaletteCommand] {
        var list = [
            PaletteCommand(group: .notes, title: "Today", action: .today),
            PaletteCommand(group: .notes, title: "New note", action: .newNote),
            PaletteCommand(group: .window, title: "Open Settings", action: .settings),
            PaletteCommand(group: .window, title: "Restore from Trash", action: .trash),
        ]
        if currentNote != nil {
            list.append(PaletteCommand(group: .thisNote, title: "Save version", action: .saveVersion))
            if hasPanelHost {
                for tab in PanelTab.allCases {
                    list.append(PaletteCommand(group: .thisNote, title: "Show \(tab.title.lowercased())", action: .showPanel(tab)))
                }
            }
        }
        return list
    }
}
