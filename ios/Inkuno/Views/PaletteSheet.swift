import SwiftUI

/// Search over note names, as a full sheet. An empty query shows the six most recent notes.
/// A query starting with ">" lists commands. Content search is not in this skeleton yet.
struct PaletteSheet: View {
    @Environment(VaultStore.self) private var vault
    @Environment(\.dismiss) private var dismiss

    /// Called with the route to push; the presenter dismisses the sheet.
    let onOpen: (Route) -> Void

    @State private var query = ""
    @State private var notes: [VaultEntry] = []
    @State private var error: String?

    private enum Command: CaseIterable {
        case today, notes, settings

        var title: String {
            switch self {
            case .today: return "Today"
            case .notes: return "Notes"
            case .settings: return "Settings"
            }
        }
    }

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

    private var matches: [VaultEntry] {
        notes.filter { $0.title.localizedStandardContains(needle) }
            .sorted { $0.title.localizedStandardCompare($1.title) == .orderedAscending }
    }

    private var matchingCommands: [Command] {
        Command.allCases.filter { needle.isEmpty || $0.title.localizedStandardContains(needle) }
    }

    var body: some View {
        NavigationStack {
            List {
                if isCommandMode {
                    Section {
                        ForEach(matchingCommands, id: \.self) { command in
                            Button(command.title) { run(command) }
                        }
                    } header: {
                        InkSectionTitle("Commands")
                    }
                } else if needle.isEmpty {
                    Section {
                        ForEach(recent) { entry in
                            noteButton(entry)
                        }
                    } header: {
                        InkSectionTitle("Recent")
                    }
                } else {
                    Section {
                        ForEach(matches) { entry in
                            noteButton(entry)
                        }
                        Button("Create “\(needle)”") { create() }
                    } header: {
                        InkSectionTitle("Names")
                    } footer: {
                        if matches.isEmpty {
                            Text("No matches")
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

    private func noteButton(_ entry: VaultEntry) -> some View {
        Button {
            onOpen(.note(entry.url))
        } label: {
            NoteRow(entry: entry)
        }
    }

    private func reload() {
        do {
            notes = try vault.allNotes()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func run(_ command: Command) {
        do {
            switch command {
            case .today:
                onOpen(.note(try vault.openToday()))
            case .notes:
                guard let root = vault.vaultURL else { throw VaultError.noVault }
                onOpen(.tree(root))
            case .settings:
                onOpen(.settings)
            }
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
